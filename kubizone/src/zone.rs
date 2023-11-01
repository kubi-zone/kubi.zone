use std::{
    collections::{hash_map::DefaultHasher, BTreeSet},
    hash::{Hash, Hasher},
    sync::Arc,
    time::Duration,
};

use futures::StreamExt;
use k8s_openapi::serde_json::json;
use kube::{
    api::{ListParams, Patch, PatchParams},
    runtime::{controller::Action, watcher, Controller},
    Api, Client, ResourceExt,
};
use kubizone_crds::{
    v1alpha1::{Record, Zone, ZoneRef},
    PARENT_ZONE_LABEL,
};

use tracing::log::*;

struct Data {
    client: Client,
}

pub const CONTROLLER_NAME: &str = "kubi.zone/zone-resolver";

pub async fn controller(client: Client) {
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

    zone_controller.await;
}

async fn reconcile_zones(zone: Arc<Zone>, ctx: Arc<Data>) -> Result<Action, kube::Error> {
    match (
        zone.spec.zone_ref.as_ref(),
        zone.spec.domain_name.ends_with('.'),
    ) {
        (Some(zone_ref), false) => {
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
                warn!("zone {zone} references unknown zone {zone_ref}");
                return Ok(Action::requeue(Duration::from_secs(30)));
            };

            // If the parent does not have a fully qualified domain name defined
            // yet, we can't check if the delegations provided by it are valid.
            // Postpone the reconcilliation until a later time, when the fqdn
            // has (hopefully) been determined.
            let Some(parent_fqdn) = parent_zone.fqdn() else {
                info!(
                    "parent zone {} missing fqdn, requeuing.",
                    parent_zone.name_any()
                );
                return Ok(Action::requeue(Duration::from_secs(5)));
            };

            // This is only "alleged", since we don't know yet if the referenced
            // zone's delegations allow the adoption.
            let alleged_fqdn = format!("{}.{}", zone.spec.domain_name, parent_fqdn);

            if parent_zone.spec.delegations.iter().any(|delegation| {
                delegation.covers_namespace(zone.namespace().as_deref().unwrap())
                    && delegation.validate_zone(&alleged_fqdn)
            }) {
                set_zone_fqdn(ctx.client.clone(), &zone, &alleged_fqdn).await?;
                set_zone_parent_ref(ctx.client.clone(), &zone, parent_zone.zone_ref()).await?;
            } else {
                warn!("parent zone {parent_zone} was found, but its delegations does not allow adoption of {zone} with {alleged_fqdn}");
                return Ok(Action::requeue(Duration::from_secs(300)));
            }
        }
        (None, true) => {
            set_zone_fqdn(ctx.client.clone(), &zone, &zone.spec.domain_name).await?;

            // Fetch all zones from across the cluster and then filter down results to only parent
            // zones which are valid parent zones for this one.
            //
            // This means filtering out parent zones without fqdns, as well as ones which do not
            // have appropriate delegations for our `zone`'s namespace and suffix.
            if let Some(longest_parent_zone) = Api::<Zone>::all(ctx.client.clone())
                .list(&ListParams::default())
                .await?
                .into_iter()
                .filter(|parent| parent.validate_zone(&zone))
                .max_by_key(|parent| parent.fqdn().unwrap().len())
            {
                set_zone_parent_ref(ctx.client.clone(), &zone, longest_parent_zone.zone_ref())
                    .await?;
            } else {
                warn!(
                    "zone {} ({}) does not fit into any found parent Zone",
                    zone.name_any(),
                    &zone.spec.domain_name
                );
            };
        }
        (Some(zone_ref), true) => {
            warn!("zone {zone}'s has both a fully qualified domain_name ({}) and a zoneRef({zone_ref}). It cannot have both.", zone.spec.domain_name);
            return Ok(Action::requeue(Duration::from_secs(300)));
        }
        (None, false) => {
            warn!("{zone} has neither zoneRef nor a fully qualified domainName, making it impossible to deduce its parent zone.");
            return Ok(Action::requeue(Duration::from_secs(300)));
        }
    }

    update_zone_hash(zone, ctx.client.clone()).await?;
    Ok(Action::requeue(Duration::from_secs(300)))
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

async fn update_zone_hash(zone: Arc<Zone>, client: Client) -> Result<(), kube::Error> {
    let new_hash = {
        // Reference to this zone, which other zones and records
        // will use to refer to it by.
        let zone_ref = ListParams::default().labels(&format!(
            "{PARENT_ZONE_LABEL}={}",
            zone.zone_ref().as_label()
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

fn zone_error_policy(zone: Arc<Zone>, error: &kube::Error, _ctx: Arc<Data>) -> Action {
    error!(
        "zone {} reconciliation encountered error: {error}",
        zone.name_any()
    );
    Action::requeue(Duration::from_secs(60))
}
