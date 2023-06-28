use dnsetes_crds::{DNSRecord, DNSZone};
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use kube::{
    api::{DeleteParams, PostParams},
    Api, Client, CustomResourceExt,
};

mod fqdn_resolver;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await.unwrap();

    let crds: Api<CustomResourceDefinition> = Api::all(client.clone());

    crds.delete(DNSRecord::crd_name(), &DeleteParams::default())
        .await
        .unwrap();
    crds.delete(DNSZone::crd_name(), &DeleteParams::default())
        .await
        .unwrap();

    crds.create(&PostParams::default(), &DNSRecord::crd())
        .await
        .unwrap();
    crds.create(&PostParams::default(), &DNSZone::crd())
        .await
        .unwrap();

    fqdn_resolver::resolve_fqdns(client).await;
}
