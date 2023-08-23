use std::{sync::Arc, time::Duration};

use kube::{
    runtime::{controller::Action, watcher, Controller},
    Api, Client, ResourceExt,
};
use kubizone_crds::{v1alpha1::Zone, PARENT_ZONE_LABEL};
use tracing::log::*;

struct Data {
    client: Client,
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
                    parent_zone.zone_ref().to_string(),
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
            parent_zone.zone_ref().to_string(),
        )
        .await?;
    };

    update_zone_hash(zone, ctx.client.clone()).await?;
    Ok(Action::requeue(Duration::from_secs(30)))
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

async fn set_r

fn zone_error_policy(zone: Arc<Zone>, error: &kube::Error, _ctx: Arc<Data>) -> Action {
    error!(
        "zone {} reconciliation encountered error: {error}",
        zone.name_any()
    );
    Action::requeue(Duration::from_secs(60))
}
