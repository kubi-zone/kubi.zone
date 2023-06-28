use dnsetes_crds::{DNSRecord, DNSRecordSpec, DNSZone, DNSZoneSpec};
use dnsetes_zonefile_crds::{ZoneFile, TARGET_ZONEFILE_LABEL};
use futures::StreamExt;

use k8s_openapi::{api::core::v1::ConfigMap, serde_json::json};
use kube::{
    api::{ListParams, Patch, PatchParams},
    core::ObjectMeta,
    runtime::{controller::Action, reflector::ObjectRef, watcher, Controller},
    Api, Client, Resource, ResourceExt,
};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tracing::log::*;

struct Data {
    client: Client,
}

pub const CONTROLLER_NAME: &str = "dnsetes.pius.dev/zonefile";
use dnsetes_crds::PARENT_ZONE_LABEL;

async fn build_zonefile(client: Client, zone: &DNSZone) -> Result<String, kube::Error> {
    let zone_ref = ListParams::default().labels(&format!(
        "{PARENT_ZONE_LABEL}={}",
        zone.zone_ref().to_string()
    ));

    // Get a hash of the collective child zones and records and use
    // as the basis for detecting change.
    /*
       let child_zones: Vec<_> = Api::<DNSZone>::all(client.clone())
           .list(&zone_ref)
           .await?
           .into_iter()
           .map(|zone| zone.spec)
           .collect();
    */

    let records = Api::<DNSRecord>::all(client.clone())
        .list(&zone_ref)
        .await?
        .into_iter()
        .map(|record| {
            let DNSRecordSpec {
                name,
                type_,
                class,
                ttl,
                rdata,
                ..
            } = record.spec;

            format!(
                "{name} {ttl} {class} {type_} {rdata}",
                ttl = ttl.unwrap_or(zone.spec.ttl)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let DNSZoneSpec {
        name,
        serial,
        refresh,
        retry,
        expire,
        negative_response_cache,
        ..
    } = &zone.spec;

    let zone = indoc::formatdoc! {"
        $ORIGIN {name}

        {name} IN SOA ns.{name} noc.{name} (
            {serial}
            {refresh}
            {retry}
            {expire}
            {negative_response_cache}
        )

        {records}
    "};

    Ok(zone)
}

async fn reconcile_zonefiles(
    zonefile: Arc<ZoneFile>,
    ctx: Arc<Data>,
) -> Result<Action, kube::Error> {
    let zone = Api::<DNSZone>::namespaced(
        ctx.client.clone(),
        &zonefile
            .spec
            .zone
            .namespace
            .as_ref()
            .or(zonefile.namespace().as_ref())
            .cloned()
            .unwrap(),
    )
    .get(&zonefile.spec.zone.name)
    .await?;

    // Create a label on the DNSZone pointing back to our ZoneFile so we
    // our reconciliation loop triggers, if the DNSZone updates.
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

        Api::<DNSZone>::namespaced(
            ctx.client.clone(),
            &zone.metadata.namespace.as_ref().unwrap(),
        )
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

    let last_serial = zonefile.status.as_ref().and_then(|status| status.serial);

    if last_serial != Some(zone.spec.serial) {
        info!(
            "zone {}'s serial is not equal to zonefile {}'s ({} != {last_serial:?}), regenerating configmap",
            zone.name_any(),
            zonefile.name_any(),
            zone.spec.serial
        );

        let owner_reference = zonefile.controller_owner_ref(&()).unwrap();

        let config_map = ConfigMap {
            metadata: ObjectMeta {
                name: Some(zonefile.name_any()),
                namespace: zonefile.namespace(),
                owner_references: Some(vec![owner_reference]),
                ..ObjectMeta::default()
            },
            data: Some(BTreeMap::from([(
                "zonefile".to_string(),
                build_zonefile(ctx.client.clone(), &zone).await?,
            )])),
            ..Default::default()
        };

        Api::<ConfigMap>::namespaced(ctx.client.clone(), zonefile.namespace().as_ref().unwrap())
            .patch(
                &zonefile.name_any(),
                &PatchParams::apply(CONTROLLER_NAME),
                &Patch::Apply(config_map),
            )
            .await?;

        Api::<ZoneFile>::namespaced(ctx.client.clone(), zonefile.namespace().as_ref().unwrap())
            .patch_status(
                &zonefile.name_any(),
                &PatchParams::apply(CONTROLLER_NAME),
                &Patch::Merge(json!({
                    "status": {
                        "serial": zone.spec.serial
                    }
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
            Api::<DNSZone>::all(client.clone()),
            watcher::Config::default(),
            |zone| {
                let parent = zone.labels().get(TARGET_ZONEFILE_LABEL)?;

                let (name, namespace) = parent.split_once(".")?;

                Some(ObjectRef::new(name).within(namespace))
            },
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
