use dnsetes_zonefile_crds::ZoneFile;
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use kube::{
    api::{DeleteParams, PostParams},
    Api, Client, CustomResourceExt,
};

mod reconciliation;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await.unwrap();

    let crds: Api<CustomResourceDefinition> = Api::all(client.clone());

    crds.delete(ZoneFile::crd_name(), &DeleteParams::default())
        .await
        .ok();

    crds.create(&PostParams::default(), &ZoneFile::crd())
        .await
        .unwrap();

    reconciliation::reconcile(client).await;
}
