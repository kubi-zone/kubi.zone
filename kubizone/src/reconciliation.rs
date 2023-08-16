use futures::StreamExt;
use k8s_openapi::serde_json::json;
use std::{
    collections::{hash_map::DefaultHasher, BTreeSet},
    hash::{Hash, Hasher},
    sync::Arc,
    time::Duration,
};
use tokio::select;

use kube::{
    api::{ListParams, Patch, PatchParams},
    runtime::{controller::Action, watcher, Controller},
    Api, Client, ResourceExt,
};
use kubizone_crds::{Record, Zone, PARENT_ZONE_LABEL};
use tracing::log::*;

struct Data {
    client: Client,
}

pub const CONTROLLER_NAME: &str = "kubi.zone/zone-resolver";

async fn set_zone_fqdn(client: Client, zone: &Zone, fqdn: &str) -> Result<(), kube::Error> {
    if zone
        .status
        .as_ref()
        .and_then(|status| status.fqdn.as_deref())
        != Some(fqdn)
    {
        info!("updating fqdn for zone {} to {}", zone.name_any(), fqdn);
        Api::<Zone>::namespaced(client, zone.namespace().as_ref().unwrap())
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
    zone: &Arc<Zone>,
    parent_ref: String,
) -> Result<(), kube::Error> {
    if zone.labels().get(PARENT_ZONE_LABEL) != Some(&parent_ref) {
        info!(
            "updating zone {}'s {PARENT_ZONE_LABEL} to {parent_ref}",
            zone.name_any()
        );

        Api::<Zone>::namespaced(client, zone.namespace().as_ref().unwrap())
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

async fn set_record_fqdn(client: Client, record: &Record, fqdn: &str) -> Result<(), kube::Error> {
    if record
        .status
        .as_ref()
        .and_then(|status| status.fqdn.as_deref())
        != Some(fqdn)
    {
        info!("updating fqdn for record {} to {}", record.name_any(), fqdn);
        Api::<Record>::namespaced(client, record.namespace().as_ref().unwrap())
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
    record: &Arc<Record>,
    parent_ref: String,
) -> Result<(), kube::Error> {
    if record.labels().get(PARENT_ZONE_LABEL) != Some(&parent_ref) {
        info!(
            "updating record {}'s {PARENT_ZONE_LABEL} to {parent_ref}",
            record.name_any()
        );
        Api::<Record>::namespaced(client, record.namespace().as_ref().unwrap())
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

async fn update_zone_hash(zone: Arc<Zone>, client: Client) -> Result<(), kube::Error> {
    let new_hash = {
        // Reference to this zone, which other zones and records
        // will use to refer to it by.
        let zone_ref = ListParams::default().labels(&format!(
            "{PARENT_ZONE_LABEL}={}",
            zone.zone_ref().to_string()
        ));

        // Get a hash of the collective child zones and records and use
        // as the basis for detecting change.
        let child_zones: BTreeSet<_> = Api::<Zone>::all(client.clone())
            .list(&zone_ref)
            .await?
            .into_iter()
            .map(|zone| zone.spec)
            .collect();

        let child_records: BTreeSet<_> = Api::<Record>::all(client.clone())
            .list(&zone_ref)
            .await?
            .into_iter()
            .map(|record| record.spec)
            .collect();

        let mut hasher = DefaultHasher::new();
        (child_zones, child_records).hash(&mut hasher);

        hasher.finish().to_string()
    };

    let current_hash = zone.status.as_ref().and_then(|status| status.hash.as_ref());

    if current_hash != Some(&new_hash) {
        info!(
            "zone {}'s hash changed (before: {current_hash:?}, now: {new_hash})",
            zone.name_any()
        );

        Api::<Zone>::namespaced(client, zone.namespace().as_ref().unwrap())
            .patch_status(
                &zone.name_any(),
                &PatchParams::apply(CONTROLLER_NAME),
                &Patch::Merge(json!({
                    "status": {
                        "hash": new_hash,
                    },
                })),
            )
            .await?;
    }

    Ok(())
}

async fn reconcile_zones(zone: Arc<Zone>, ctx: Arc<Data>) -> Result<Action, kube::Error> {
    // Determine the fqdn of the zone
    if zone.spec.domain_name.ends_with('.') {
        set_zone_fqdn(ctx.client.clone(), &zone, &zone.spec.domain_name).await?;
    } else {
        let Some(zone_ref) = zone.spec.zone_ref.as_ref() else {
            warn!("zone {} does not have a fully qualified domain name, nor does it reference a zone.", zone.name_any());
            return Ok(Action::requeue(Duration::from_secs(300)));
        };
        let parent_zone = Api::<Zone>::namespaced(
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

        let fqdn = format!("{}.{}", zone.spec.domain_name, parent_fqdn);

        set_zone_fqdn(ctx.client.clone(), &zone, &fqdn).await?;

        set_zone_parent_ref(
            ctx.client.clone(),
            &zone,
            parent_zone.zone_ref().to_string(),
        )
        .await?;
    };

    update_zone_hash(zone, ctx.client.clone()).await?;
    Ok(Action::requeue(Duration::from_secs(30)))
}

async fn reconcile_records(record: Arc<Record>, ctx: Arc<Data>) -> Result<Action, kube::Error> {
    // Determine the fqdn of the record
    if record.spec.domain_name.ends_with('.') {
        set_record_fqdn(ctx.client.clone(), &record, &record.spec.domain_name).await?;

        // Retrieve all zones with a defined fqdn.
        let mut all_zones: Vec<_> = Api::<Zone>::all(ctx.client.clone())
            .list(&ListParams::default())
            .await?
            .into_iter()
            .filter_map(|zone| {
                zone.status
                    .as_ref()
                    .and_then(|status| status.fqdn.as_ref().cloned())
                    .map(|fqdn| (fqdn, zone))
            })
            .collect();

        // Sort the zones by *reversed* fqdn in *reverse* order, putting the longer fqdns on top.
        //
        // Reversing the fqdns sorts by domain suffix.
        //
        // Reversing the order puts the longer domains first, letting us use `Iterator::find`
        // to get the longest matching suffix.
        all_zones.sort_by(|(a, _), (b, _)| {
            b.chars()
                .rev()
                .collect::<Vec<_>>()
                .cmp(&a.chars().rev().collect())
        });

        // Find the longest parent zone which is a suffix of our fqdn.
        let Some(longest_parent_zone) = all_zones.into_iter().find_map(|(fqdn, zone)| {
            if record.spec.domain_name.ends_with(&fqdn) {
                Some(zone)
            } else {
                None
            }
        }) else {
            warn!(
                "record {} ({}) does not fit into any found Zone",
                record.name_any(),
                &record.spec.domain_name
            );
            return Ok(Action::requeue(Duration::from_secs(30)));
        };

        set_record_parent_ref(
            ctx.client.clone(),
            &record,
            longest_parent_zone.zone_ref().to_string(),
        )
        .await?;
    } else {
        let Some(zone_ref) = record.spec.zone_ref.as_ref() else {
            warn!("record {} does not have a fully qualified domain name, nor does it reference a zone.", record.name_any());
            return Ok(Action::requeue(Duration::from_secs(300)));
        };
        let parent_zone = Api::<Zone>::namespaced(
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

        let fqdn = format!("{}.{}", record.spec.domain_name, parent_fqdn);

        set_record_fqdn(ctx.client.clone(), &record, &fqdn).await?;
        set_record_parent_ref(
            ctx.client.clone(),
            &record,
            parent_zone.zone_ref().to_string(),
        )
        .await?;
    };

    Ok(Action::requeue(Duration::from_secs(30)))
}

fn zone_error_policy(zone: Arc<Zone>, error: &kube::Error, _ctx: Arc<Data>) -> Action {
    error!(
        "zone {} reconciliation encountered error: {error}",
        zone.name_any()
    );
    Action::requeue(Duration::from_secs(60))
}

fn record_error_policy(record: Arc<Record>, error: &kube::Error, _ctx: Arc<Data>) -> Action {
    error!(
        "record {} reconciliation encountered error: {error}",
        record.name_any()
    );
    Action::requeue(Duration::from_secs(60))
}

pub async fn reconcile(client: Client) {
    let zones = Api::<Zone>::all(client.clone());

    let zone_controller = Controller::new(zones.clone(), watcher::Config::default())
        .watches(
            Api::<Zone>::all(client.clone()),
            watcher::Config::default(),
            kubizone_crds::watch_reference(PARENT_ZONE_LABEL),
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

    let records = Api::<Record>::all(client.clone());

    let record_controller = Controller::new(records, watcher::Config::default())
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
