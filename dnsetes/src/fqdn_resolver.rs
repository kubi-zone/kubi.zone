use futures::StreamExt;
use k8s_openapi::serde_json::json;
use std::{
    collections::{hash_map::DefaultHasher, BTreeSet},
    hash::{Hash, Hasher},
    sync::Arc,
    time::Duration,
};
use tokio::select;

use dnsetes_crds::{DNSRecord, DNSZone};
use kube::{
    api::{ListParams, Patch, PatchParams},
    runtime::{controller::Action, reflector::ObjectRef, watcher, Controller},
    Api, Client, ResourceExt,
};
use tracing::log::*;

struct Data {
    client: Client,
}

const CONTROLLER_NAME: &str = "dnsetes.pius.dev/zone-resolver";
const PARENT_ZONE_LABEL: &str = "dnsetes.pius.dev/parent-zone";

async fn set_zone_fqdn(client: Client, zone: &DNSZone, fqdn: &str) -> Result<(), kube::Error> {
    if !zone
        .status
        .as_ref()
        .and_then(|status| status.fqdn.as_ref())
        .is_some_and(|current_fqdn| current_fqdn == &fqdn)
    {
        info!("updating fqdn for zone {} to {}", zone.name_any(), fqdn);
        Api::<DNSZone>::namespaced(client, &zone.metadata.namespace.as_ref().unwrap())
            .patch_status(
                &zone.name_any(),
                &PatchParams::apply(CONTROLLER_NAME),
                &Patch::Merge(json!({
                    "status": {
                        "fqdn": fqdn,
                    }
                })),
            )
            .await?;
    } else {
        debug!("not updating fqdn for zone {} {fqdn}", zone.name_any())
    }
    Ok(())
}
async fn set_zone_parent_ref(
    client: Client,
    zone: &Arc<DNSZone>,
    parent_ref: String,
) -> Result<(), kube::Error> {
    if !zone
        .labels()
        .get(PARENT_ZONE_LABEL)
        .is_some_and(|current_parent| current_parent == &parent_ref)
    {
        info!(
            "updating zone {}'s {PARENT_ZONE_LABEL} to {parent_ref}",
            zone.name_any()
        );

        Api::<DNSZone>::namespaced(client, &zone.metadata.namespace.as_ref().unwrap())
            .patch_metadata(
                &zone.name_any(),
                &PatchParams::apply(CONTROLLER_NAME),
                &Patch::Merge(json!({
                    "metadata": {
                        "labels": {
                            PARENT_ZONE_LABEL: parent_ref
                        },
                    }
                })),
            )
            .await?;
    } else {
        debug!(
            "not updating zone {}'s {PARENT_ZONE_LABEL} since it is already {parent_ref}",
            zone.name_any()
        )
    }
    Ok(())
}

async fn set_record_fqdn(
    client: Client,
    record: &DNSRecord,
    fqdn: &str,
) -> Result<(), kube::Error> {
    if !record
        .status
        .as_ref()
        .and_then(|status| status.fqdn.as_ref())
        .is_some_and(|current_fqdn| current_fqdn == &fqdn)
    {
        info!("updating fqdn for record {} to {}", record.name_any(), fqdn);
        Api::<DNSRecord>::namespaced(client, &record.metadata.namespace.as_ref().unwrap())
            .patch_status(
                &record.name_any(),
                &PatchParams::apply(CONTROLLER_NAME),
                &Patch::Merge(json!({
                    "status": {
                        "fqdn": fqdn,
                    }
                })),
            )
            .await?;
    } else {
        debug!("not updating fqdn for record {} {fqdn}", record.name_any())
    }
    Ok(())
}

async fn set_record_parent_ref(
    client: Client,
    record: &Arc<DNSRecord>,
    parent_ref: String,
) -> Result<(), kube::Error> {
    if !record
        .labels()
        .get(PARENT_ZONE_LABEL)
        .is_some_and(|current_parent| current_parent == &parent_ref)
    {
        info!(
            "updating record {}'s {PARENT_ZONE_LABEL} to {parent_ref}",
            record.name_any()
        );
        Api::<DNSRecord>::namespaced(client, &record.metadata.namespace.as_ref().unwrap())
            .patch_metadata(
                &record.name_any(),
                &PatchParams::apply(CONTROLLER_NAME),
                &Patch::Merge(json!({
                    "metadata": {
                        "labels": {
                            PARENT_ZONE_LABEL: parent_ref
                        },
                    }
                })),
            )
            .await?;
    } else {
        debug!(
            "not updating record {}'s {PARENT_ZONE_LABEL} since it is already {parent_ref}",
            record.name_any()
        )
    }
    Ok(())
}

async fn rotate_zone_serial(zone: Arc<DNSZone>, client: Client) -> Result<(), kube::Error> {
    // Reference to this zone, which other zones and records
    // will use to refer to it by.
    let zone_ref = ListParams::default().labels(&format!(
        "{PARENT_ZONE_LABEL}={}-{}",
        zone.namespace().as_ref().unwrap(),
        zone.name_any()
    ));

    // Get a hash of the collective child zones and records and use
    // as the basis for detecting change.
    let child_zones: BTreeSet<_> = Api::<DNSZone>::all(client.clone())
        .list(&zone_ref)
        .await?
        .into_iter()
        .map(|zone| zone.spec)
        .collect();

    let child_records: BTreeSet<_> = Api::<DNSRecord>::all(client.clone())
        .list(&zone_ref)
        .await?
        .into_iter()
        .map(|record| record.spec)
        .collect();

    let mut hasher = DefaultHasher::new();
    (child_zones, child_records).hash(&mut hasher);

    let hash = hasher.finish().to_string();

    let last_hash = zone.status.as_ref().and_then(|status| status.hash.as_ref());

    if last_hash != Some(&hash) {
        info!(
            "zone {}'s hash (before: {last_hash:?}, now: {hash}) changed, updating serial",
            zone.name_any()
        );
        // Compute a serial based on the current datetime in UTC.
        let now = time::OffsetDateTime::now_utc();

        #[rustfmt::skip]
        let now_serial
            = now.year()  as u32 * 1000000
            + now.month() as u32 * 10000
            + now.day()   as u32 * 100;

        let next_serial = std::cmp::max(now_serial, zone.spec.serial + 1);

        info!(
            "updating zone {}'s serial (before: {}, now: {next_serial})",
            zone.name_any(),
            zone.spec.serial
        );

        // We apply the serial patch first, because in that case if the hash status
        // application fails, the failure mode is that serial gets bumped again, and
        // then the hash update hopefully works the second time around.
        //
        // It'd be better to be able to update both serial and hash in an atomic
        // fashion, but none of the attempts I've made have succeeded.
        Api::<DNSZone>::namespaced(client.clone(), &zone.namespace().as_ref().unwrap())
            .patch(
                &zone.name_any(),
                &PatchParams::apply(CONTROLLER_NAME),
                &Patch::Merge(json!({
                    "spec": {
                        "serial": next_serial,
                    },
                })),
            )
            .await?;

        Api::<DNSZone>::namespaced(client, &zone.namespace().as_ref().unwrap())
            .patch_status(
                &zone.name_any(),
                &PatchParams::apply(CONTROLLER_NAME),
                &Patch::Merge(json!({
                    "status": {
                        "hash": hash.to_string(),
                    },
                })),
            )
            .await?;
    }

    Ok(())
}

async fn reconcile_zones(zone: Arc<DNSZone>, ctx: Arc<Data>) -> Result<Action, kube::Error> {
    // Determine the fqdn of the zone
    if zone.spec.name.ends_with(".") {
        set_zone_fqdn(ctx.client.clone(), &zone, &zone.spec.name).await?;
    } else {
        let Some(zone_ref) = zone.spec.zone_ref.as_ref() else {
            warn!("zone {} does not have a fully qualified domain name, nor does it reference a zone.", zone.name_any());
            return Ok(Action::requeue(Duration::from_secs(300)))
        };
        let parent_zone = Api::<DNSZone>::namespaced(
            ctx.client.clone(),
            &zone_ref
                .namespace
                .as_ref()
                .or(zone.namespace().as_ref())
                .cloned()
                .unwrap(),
        )
        .get(&zone_ref.name)
        .await?;

        let Some(parent_fqdn) = parent_zone
            .status
            .as_ref()
            .and_then(|status| status.fqdn.as_ref())
        else {
            info!(
                "parent zone {} missing fqdn, requeuing.",
                parent_zone.name_any()
            );
            return Ok(Action::requeue(Duration::from_secs(5)));
        };

        let fqdn = format!("{}.{}", zone.spec.name, parent_fqdn);

        set_zone_fqdn(ctx.client.clone(), &zone, &fqdn).await?;

        let parent_ref = format!(
            "{}-{}",
            parent_zone.namespace().as_ref().unwrap(),
            parent_zone.name_any()
        );

        set_zone_parent_ref(ctx.client.clone(), &zone, parent_ref).await?;
    };

    rotate_zone_serial(zone, ctx.client.clone()).await?;
    Ok(Action::requeue(Duration::from_secs(30)))
}

async fn reconcile_records(record: Arc<DNSRecord>, ctx: Arc<Data>) -> Result<Action, kube::Error> {
    // Determine the fqdn of the record
    if record.spec.name.ends_with(".") {
        set_record_fqdn(ctx.client.clone(), &record, &record.spec.name).await?;

        // Retrieve all zones with a defined fqdn.
        let mut all_zones: Vec<_> = Api::<DNSZone>::all(ctx.client.clone())
            .list(&ListParams::default())
            .await?
            .into_iter()
            .filter_map(|zone| {
                if let Some(fqdn) = zone
                    .status
                    .as_ref()
                    .and_then(|status| status.fqdn.as_ref().map(|fqdn| fqdn.clone()))
                {
                    Some((fqdn, zone))
                } else {
                    None
                }
            })
            .collect();

        // Sort the zones by *reversed* fqdn in *reverse* order, putting the longer fqdns on top.
        all_zones.sort_by(|a, b| {
            b.0.chars()
                .rev()
                .collect::<Vec<_>>()
                .cmp(&a.0.chars().rev().collect())
        });

        let Some(longest_parent_zone) = all_zones.into_iter().find_map(|(fqdn, zone)| {
             if record.spec.name.ends_with(&fqdn) {
                Some(zone)
            } else {
                None
            }
        }) else {
            warn!("record {} ({}) does not fit into any found dnszone", record.name_any(), &record.spec.name);
            return Ok(Action::requeue(Duration::from_secs(30)))
        };

        // Populate the `dnsetes.pius.dev/parent-zone` annotation
        let parent_ref = format!(
            "{}-{}",
            longest_parent_zone.namespace().as_ref().unwrap(),
            longest_parent_zone.name_any()
        );

        set_record_parent_ref(ctx.client.clone(), &record, parent_ref).await?;
    } else {
        let Some(zone_ref) = record.spec.zone_ref.as_ref() else {
            warn!("record {} does not have a fully qualified domain name, nor does it reference a zone.", record.name_any());
            return Ok(Action::requeue(Duration::from_secs(300)))
        };
        let parent_zone = Api::<DNSZone>::namespaced(
            ctx.client.clone(),
            &zone_ref
                .namespace
                .as_ref()
                .or(record.namespace().as_ref())
                .cloned()
                .unwrap(),
        )
        .get(&zone_ref.name)
        .await?;

        let Some(parent_fqdn) = parent_zone
            .status
            .as_ref()
            .and_then(|status| status.fqdn.as_ref())
        else {
            info!(
                "parent zone {} missing fqdn, requeuing.",
                parent_zone.name_any()
            );
            return Ok(Action::requeue(Duration::from_secs(5)));
        };

        let fqdn = format!("{}.{}", record.spec.name, parent_fqdn);

        set_record_fqdn(ctx.client.clone(), &record, &fqdn).await?;

        // Populate the `dnsetes.pius.dev/parent-zone` annotation
        let parent_ref = format!(
            "{}-{}",
            parent_zone.namespace().as_ref().unwrap(),
            parent_zone.name_any()
        );

        set_record_parent_ref(ctx.client.clone(), &record, parent_ref).await?;
    };

    Ok(Action::requeue(Duration::from_secs(30)))
}

fn zone_error_policy(zone: Arc<DNSZone>, error: &kube::Error, _ctx: Arc<Data>) -> Action {
    error!(
        "zone {} reconciliation encountered error: {error}",
        zone.name_any()
    );
    Action::requeue(Duration::from_secs(60))
}

fn record_error_policy(record: Arc<DNSRecord>, error: &kube::Error, _ctx: Arc<Data>) -> Action {
    error!(
        "record {} reconciliation encountered error: {error}",
        record.name_any()
    );
    Action::requeue(Duration::from_secs(60))
}

pub async fn resolve_fqdns(client: Client) {
    let zones = Api::<DNSZone>::all(client.clone());

    let zone_controller = Controller::new(zones.clone(), watcher::Config::default())
        .watches(
            Api::<DNSZone>::all(client.clone()),
            watcher::Config::default(),
            |zone| {
                let parent = zone.labels().get("dnsetes.pius.dev/parent-zone")?;

                let (namespace, name) = parent.split_once("/")?;

                Some(ObjectRef::new(name).within(namespace))
            },
        )
        .shutdown_on_signal()
        .run(
            reconcile_zones,
            zone_error_policy,
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

    let records = Api::<DNSRecord>::all(client.clone());

    let record_controller = Controller::new(records, watcher::Config::default())
        .watches(
            Api::<DNSZone>::all(client.clone()),
            watcher::Config::default(),
            |zone| {
                let parent = zone.labels().get("dnsetes.pius.dev/parent-zone")?;

                let (namespace, name) = parent.split_once("/")?;

                Some(ObjectRef::new(name).within(namespace))
            },
        )
        .shutdown_on_signal()
        .run(
            reconcile_records,
            record_error_policy,
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

    select! {
        _ = zone_controller => (),
        _ = record_controller => ()
    }
}
