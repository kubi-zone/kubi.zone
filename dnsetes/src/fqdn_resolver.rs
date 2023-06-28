use futures::StreamExt;
use k8s_openapi::serde_json::json;
use std::{sync::Arc, time::Duration};

use dnsetes_crds::DNSZone;
use kube::{
    api::{Patch, PatchParams},
    runtime::{controller::Action, reflector::ObjectRef, watcher, Controller},
    Api, Client, ResourceExt,
};
use tracing::log::*;

struct Data {
    client: Client,
}

async fn set_zone_fqdn(client: Client, zone: &DNSZone, fqdn: &str) -> Result<(), kube::Error> {
    if !zone
        .status
        .as_ref()
        .and_then(|status| status.fqdn.as_ref())
        .is_some_and(|current_fqdn| current_fqdn == &fqdn)
    {
        info!("updating fqdn for {} to {}", zone.name_any(), fqdn);
        Api::<DNSZone>::namespaced(client, &zone.metadata.namespace.as_ref().unwrap())
            .patch_status(
                &zone.name_any(),
                &PatchParams::apply("dnsetes.pius.dev/zone-resolver"),
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

async fn reconcile_zones(zone: Arc<DNSZone>, ctx: Arc<Data>) -> Result<Action, kube::Error> {
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

        if let Some(parent_fqdn) = parent_zone
            .status
            .as_ref()
            .and_then(|status| status.fqdn.as_ref())
        {
            let fqdn = format!("{}.{}", zone.spec.name, parent_fqdn);

            set_zone_fqdn(ctx.client.clone(), &zone, &fqdn).await?;
        } else {
            info!(
                "parent zone {} missing fqdn, requeuing..",
                parent_zone.name_any()
            );
            return Ok(Action::requeue(Duration::from_secs(5)));
        }
    }
    Ok(Action::requeue(Duration::from_secs(30)))
}

fn error_policy(_object: Arc<DNSZone>, _error: &kube::Error, _ctx: Arc<Data>) -> Action {
    Action::requeue(Duration::from_secs(60))
}

pub async fn resolve_fqdns(client: Client) {
    let zones = Api::<DNSZone>::all(client.clone());

    Controller::new(zones, watcher::Config::default())
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
        })
        .await;
}
