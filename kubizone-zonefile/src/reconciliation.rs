use futures::StreamExt;
use kubizone_crds::{Record, RecordSpec, Zone};
use kubizone_zonefile_crds::{ZoneFile, ZoneFileSpec, TARGET_ZONEFILE_LABEL};

use k8s_openapi::{api::core::v1::ConfigMap, serde_json::json};
use kube::{
    api::{ListParams, Patch, PatchParams},
    core::ObjectMeta,
    runtime::{controller::Action, watcher, Controller},
    Api, Client, Resource, ResourceExt,
};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tracing::log::*;

struct Data {
    client: Client,
}

pub const CONTROLLER_NAME: &str = "kubi.zone/zonefile";
use kubizone_crds::PARENT_ZONE_LABEL;

/// Builds the actual [zone file](https://datatracker.ietf.org/doc/html/rfc1035#section-5)
/// based on [`Record`]s and [`Zone`]s pointing to the [`Zone`] referenced by [`ZoneFile`].
async fn build_zonefile(
    client: Client,
    zonefile: &ZoneFile,
    origin: &str,
    serial: u32,
) -> Result<String, kube::Error> {
    let label = format!("{PARENT_ZONE_LABEL}={}", zonefile.zone_ref().to_string());
    debug!("generating zone by finding records matching {label}");
    let zone_ref = ListParams::default().labels(&label);

    // TODO: Implement sub-zone building, by either listing namservers
    // (if any), or including the zone's records directly.

    // Suffix of the origin, with a '.' in front.
    // We remove suffixes matching this from all records in order to
    // simplify the output zonefiles, as the origin is assumed.
    let origin_suffix = &format!(".{origin}");

    // Transform the record FQDNs into zone-dependent domain names,
    // without the .origin suffix (see above).
    //
    // For example, in the zone "example.org.", the following
    // transformations will take place:
    //   www.example.org. => www
    //   ftp.users.example.org. => ftp.users
    let mut records: Vec<_> = Api::<Record>::all(client.clone())
        .list(&zone_ref)
        .await?
        .into_iter()
        .map(|record| {
            let shortened_name = record
                .spec
                .domain_name
                .strip_suffix(origin_suffix)
                .unwrap_or(&record.spec.domain_name)
                .to_string();

            (shortened_name, record)
        })
        .collect();

    // Sort the zones by *reversed* fqdn in *reverse* order, putting the longer fqdns on top.
    //
    // Reversing the fqdns sorts by domain suffix.
    //
    // Reversing the order puts the longer domains first, letting us use `Iterator::find`
    // to get the longest matching suffix.
    records.sort_by(|(a, _), (b, _)| {
        b.chars()
            .rev()
            .collect::<Vec<_>>()
            .cmp(&a.chars().rev().collect())
    });

    // We use the longest domain name in the list for
    // aligning the text in the output zonefile
    let longest_name = records
        .iter()
        .map(|(name, _)| name.len())
        .max()
        .unwrap_or_default();

    let serialized_records = records
        .into_iter()
        .map(|(name, record)| {
            let RecordSpec {
                type_,
                class,
                ttl,
                rdata,
                ..
            } = record.spec;

            format!(
                "{name:>width$} {ttl:<8} {class:<5} {type_:<6} {rdata}",
                width = longest_name,
                ttl = ttl.unwrap_or(zonefile.spec.ttl)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let ZoneFileSpec {
        refresh,
        retry,
        expire,
        negative_response_cache,
        ..
    } = &zonefile.spec;

    let zone = indoc::formatdoc! {"
        $ORIGIN {origin}

        {origin} IN SOA ns.{origin} noc.{origin} (
            {serial}
            {refresh}
            {retry}
            {expire}
            {negative_response_cache}
        )

        {serialized_records}
    "};

    Ok(zone)
}

/// Applied a [`TARGET_ZONEFILE_LABEL`] label which references our zonefile.
/// This label is monitored by our controller, causing reconciliation loops
/// to fire for [`ZoneFile`]s referenced by [`Zone`]s, when the zone itself
/// is updated.
async fn apply_zonefile_backref(
    client: Client,
    zonefile: &ZoneFile,
    zone: &Zone,
) -> Result<(), kube::Error> {
    let zonefile_ref = format!(
        "{}.{}",
        zonefile.name_any(),
        zonefile.namespace().as_ref().unwrap()
    );

    if zone.labels().get(TARGET_ZONEFILE_LABEL) != Some(&zonefile_ref) {
        info!(
            "updating zone {}'s {TARGET_ZONEFILE_LABEL} to {zonefile_ref}",
            zonefile.name_any()
        );

        Api::<Zone>::namespaced(client, zone.namespace().as_ref().unwrap())
            .patch_metadata(
                &zone.name_any(),
                &PatchParams::apply(CONTROLLER_NAME),
                &Patch::Merge(json!({
                    "metadata": {
                        "labels": {
                            TARGET_ZONEFILE_LABEL: zonefile_ref
                        },
                    }
                })),
            )
            .await?;
    }

    Ok(())
}

async fn reconcile_zonefiles(
    zonefile: Arc<ZoneFile>,
    ctx: Arc<Data>,
) -> Result<Action, kube::Error> {
    let zone = Api::<Zone>::namespaced(
        ctx.client.clone(),
        &zonefile
            .spec
            .zone_ref
            .namespace
            .as_ref()
            .or(zonefile.namespace().as_ref())
            .cloned()
            .unwrap(),
    )
    .get(&zonefile.spec.zone_ref.name)
    .await?;

    apply_zonefile_backref(ctx.client.clone(), &zonefile, &zone).await?;

    let Some(zone_hash) = zone.status.as_ref().and_then(|zone| zone.hash.as_ref()) else {
        debug!("zone {} has not yet computed its hash, requeuing", zone.name_any());
        return Ok(Action::requeue(Duration::from_secs(5)))
    };

    let last_hash = zonefile
        .status
        .as_ref()
        .and_then(|status| status.hash.as_ref());

    if last_hash != Some(zone_hash) {
        info!(
            "zone {}'s hash is not equal to zonefile {}'s ({zone_hash} != {last_hash:?}), regenerating configmap and rotating serial.",
            zone.name_any(),
            zonefile.name_any(),
        );

        // Compute a serial based on the current datetime in UTC as per:
        // https://datatracker.ietf.org/doc/html/rfc1912#section-2.2
        let now = time::OffsetDateTime::now_utc();
        #[rustfmt::skip]
        let now_serial
            = now.year()  as u32 * 1000000
            + now.month() as u32 * 10000
            + now.day()   as u32 * 100;

        // If it's a new day, use YYYYMMDD00, otherwise just use the increment
        // of the old serial.
        let next_serial = std::cmp::max(now_serial, zonefile.spec.serial + 1);

        let owner_reference = zonefile.controller_owner_ref(&()).unwrap();

        let configmap_name = format!("{}-{next_serial}", zonefile.name_any());

        let config_map = ConfigMap {
            metadata: ObjectMeta {
                name: Some(configmap_name.clone()),
                namespace: zonefile.namespace(),
                owner_references: Some(vec![owner_reference]),
                ..ObjectMeta::default()
            },
            data: Some(BTreeMap::from([(
                "zonefile".to_string(),
                build_zonefile(
                    ctx.client.clone(),
                    &zonefile,
                    &zone.spec.domain_name,
                    next_serial,
                )
                .await?,
            )])),
            ..Default::default()
        };

        Api::<ConfigMap>::namespaced(ctx.client.clone(), zonefile.namespace().as_ref().unwrap())
            .patch(
                &configmap_name,
                &PatchParams::apply(CONTROLLER_NAME),
                &Patch::Apply(config_map),
            )
            .await?;

        info!(
            "updating zone {}'s serial (before: {}, now: {next_serial})",
            zone.name_any(),
            zonefile.spec.serial
        );

        // We apply the serial patch first. That way, if the hash status
        // application fails, the failure mode is that serial gets bumped
        // again on the next reconciliation loop, and then the hash update
        // hopefully works the second time around.
        //
        // It'd be better to be able to update both serial and hash in an atomic
        // fashion, but none of the attempts I've made have succeeded.
        Api::<ZoneFile>::namespaced(ctx.client.clone(), zonefile.namespace().as_ref().unwrap())
            .patch(
                &zonefile.name_any(),
                &PatchParams::apply(CONTROLLER_NAME),
                &Patch::Merge(json!({
                    "spec": {
                        "serial": next_serial,
                    },
                })),
            )
            .await?;

        Api::<ZoneFile>::namespaced(ctx.client.clone(), zonefile.namespace().as_ref().unwrap())
            .patch_status(
                &zonefile.name_any(),
                &PatchParams::apply(CONTROLLER_NAME),
                &Patch::Merge(json!({
                    "status": {
                        "hash": zone_hash,
                        "configMap": configmap_name
                    },
                })),
            )
            .await?;
    }

    Ok(Action::requeue(Duration::from_secs(30)))
}

fn zonefile_error_policy(zone: Arc<ZoneFile>, error: &kube::Error, _ctx: Arc<Data>) -> Action {
    error!(
        "zonefile {} reconciliation encountered error: {error}",
        zone.name_any()
    );
    Action::requeue(Duration::from_secs(60))
}

pub async fn reconcile(client: Client) {
    let zonefiles = Api::<ZoneFile>::all(client.clone());

    let zone_controller = Controller::new(zonefiles, watcher::Config::default())
        .watches(
            Api::<Zone>::all(client.clone()),
            watcher::Config::default(),
            kubizone_crds::watch_reference(TARGET_ZONEFILE_LABEL),
        )
        .shutdown_on_signal()
        .run(
            reconcile_zonefiles,
            zonefile_error_policy,
            Arc::new(Data {
                client: client.clone(),
            }),
        )
        .for_each(|res| async move {
            match res {
                Ok(o) => info!("reconciled {:?}", o),
                Err(e) => warn!("reconcile failed: {}", e),
            }
        });

    zone_controller.await;
}
