use futures::StreamExt;
use k8s_openapi::serde_json::json;
use std::{sync::Arc, time::Duration};

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
const PARENT_ZONE_ANNOTATION: &str = "dnsetes.pius.dev/parent-zone";

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
    }
    Ok(())
}
async fn set_zone_parent_ref(
    client: Client,
    zone: &Arc<DNSZone>,
    parent_ref: String,
) -> Result<(), kube::Error> {
    info!(
        "updating zone {}'s {PARENT_ZONE_ANNOTATION} to {parent_ref}",
        zone.name_any()
    );
    Api::<DNSZone>::namespaced(client, &zone.metadata.namespace.as_ref().unwrap())
        .patch_metadata(
            &zone.name_any(),
            &PatchParams::apply(CONTROLLER_NAME),
            &Patch::Merge(json!({
                "metadata": {
                    "annotations": {
                        PARENT_ZONE_ANNOTATION: parent_ref
                    },
                }
            })),
        )
        .await?;

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
    }
    Ok(())
}

async fn set_record_parent_ref(
    client: Client,
    record: &Arc<DNSRecord>,
    parent_ref: String,
) -> Result<(), kube::Error> {
    info!(
        "updating record {}'s {PARENT_ZONE_ANNOTATION} to {parent_ref}",
        record.name_any()
    );
    Api::<DNSRecord>::namespaced(client, &record.metadata.namespace.as_ref().unwrap())
        .patch_metadata(
            &record.name_any(),
            &PatchParams::apply(CONTROLLER_NAME),
            &Patch::Merge(json!({
                "metadata": {
                    "annotations": {
                        PARENT_ZONE_ANNOTATION: parent_ref
                    },
                }
            })),
        )
        .await?;

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
            "{}/{}",
            parent_zone.namespace().as_ref().unwrap(),
            parent_zone.name_any()
        );

        set_zone_parent_ref(ctx.client.clone(), &zone, parent_ref).await?;
    };

    Ok(Action::requeue(Duration::from_secs(30)))
}

async fn reconcile_records(record: Arc<DNSRecord>, ctx: Arc<Data>) -> Result<Action, kube::Error> {
    // Determine the fqdn of the record
    if record.spec.name.ends_with(".") {
        set_record_fqdn(ctx.client.clone(), &record, &record.spec.name).await?;

        // Try to deduce the parent zone using just the name.
        let all_zones: Vec<DNSZone> = Api::<DNSZone>::all(ctx.client.clone())
            .list(&ListParams::default())
            .await?
            .into_iter()            
            .collect();


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
            "{}/{}",
            parent_zone.namespace().as_ref().unwrap(),
            parent_zone.name_any()
        );

        set_record_parent_ref(ctx.client.clone(), &record, parent_ref).await?;
    };

    Ok(Action::requeue(Duration::from_secs(30)))
}

fn error_policy(_object: Arc<DNSZone>, _error: &kube::Error, _ctx: Arc<Data>) -> Action {
    Action::requeue(Duration::from_secs(60))
}

pub async fn resolve_fqdns(client: Client) {
    let zones = Api::<DNSZone>::all(client.clone());

    let zone_controller = Controller::new(zones.clone(), watcher::Config::default())
        .watches(
            Api::<DNSZone>::all(client.clone()),
            watcher::Config::default(),
            |zone| {
                let parent = zone.annotations().get("dnsetes.pius.dev/parent-zone")?;

                let (namespace, name) = parent.split_once("/")?;

                Some(ObjectRef::new(name).within(namespace))
            },
        )
        .shutdown_on_signal()
        .run(
            reconcile_zones,
            error_policy,
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
                let parent = zone.annotations().get("dnsetes.pius.dev/parent-zone")?;

                let (namespace, name) = parent.split_once("/")?;

                Some(ObjectRef::new(name).within(namespace))
            },
        )
        .shutdown_on_signal()
        .run(
            reconcile_records,
            error_policy,
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
