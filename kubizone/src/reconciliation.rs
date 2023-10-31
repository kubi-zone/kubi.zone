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
use kubizone_crds::{
    v1alpha1::{Record, Zone},
    PARENT_ZONE_LABEL,
};
use tracing::log::{info, warn};

struct Data {
    client: Client,
}

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
    parent_ref: ZoneRef,
) -> Result<(), kube::Error> {
    if zone.labels().get(PARENT_ZONE_LABEL) != Some(&parent_ref.as_label()) {
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
                            PARENT_ZONE_LABEL: parent_ref.as_label()
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

async fn find_longest_matching_parent_zone(client: Client) {
    // Retrieve all zones with a defined fqdn.
    let mut all_zones: Vec<_> = Api::<Zone>::all(client)
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
}

async fn reconcile_zones(zone: Arc<Zone>, ctx: Arc<Data>) -> Result<Action, kube::Error> {
    if let Some(zone_ref) = zone.spec.zone_ref.as_ref() {
        // Follow the zoneRef to the supposed parent zone, if it exists
        // or requeue later if it does not.
        let Some(parent_zone) = Api::<Zone>::namespaced(
            ctx.client.clone(),
            &zone_ref
                .namespace
                .as_ref()
                .or(zone.namespace().as_ref())
                .cloned()
                .unwrap(),
        )
        .get_opt(&zone_ref.name)
        .await?
        else {
            warn!("zone {} references unknown zone {}", zone, zone_ref);
            return Ok(Action::requeue(Duration::from_secs(30)));
        };

        // If the parent does not have a fully qualified domain name defined
        // yet, we can't check if the delegations provided by it are valid.
        // Postpone the reconcilliation until a later time, when the fqdn
        // has (hopefully) been determined.
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

        // This is only "alleged", since we don't know yet if the referenced
        // zone's delegations allow the adoption.
        let alleged_fqdn = format!("{}.{}", zone.spec.domain_name, parent_fqdn);

        for delegation in &parent_zone.spec.delegations {
            // Unwrap safe: All zones have namespaces.
            if delegation.covers_namespace(zone.namespace().as_deref().unwrap())
                && delegation.validate_zone(&alleged_fqdn)
            {
                set_zone_fqdn(ctx.client.clone(), &zone, &alleged_fqdn).await?;

                set_zone_parent_ref(
                    ctx.client.clone(),
                    &zone,
                    parent_zone.zone_ref(),
                )
                .await?;

                break;
            }
        }
        warn!("parent zone {parent_zone} was found, but its delegations does not allow adoption of {zone} with {alleged_fqdn}");
    } else if zone.spec.domain_name.ends_with('.') {
        // If this zone ends in a dot, it is itself a fully qualified domain name,
        // and no zoneRef is defined, then we must deduce the parent zone through
        // domain traversal.
    }

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
            parent_zone.zone_ref(),
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

        // Fetch all zones from across the cluster and then filter down results to only parent
        // zones which are valid parent zones for this one.
        //
        // This means filtering out parent zones without fqdns, as well as ones which do not
        // have appropriate delegations for our `needle`'s namespace, record type, and suffix.
        let Some(longest_parent_zone) = Api::<Zone>::all(ctx.client.clone())
            .list(&ListParams::default())
            .await?
            .into_iter()
            .filter_map(|zone| zone.validate_record(&record))
            .max_by_key(|zone| zone.fqdn().unwrap().len())
            else {
                warn!(
                    "record {record} ({}) does not fit into any found Zone",
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
