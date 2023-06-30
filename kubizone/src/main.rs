use std::path::PathBuf;

use clap::{command, Parser, Subcommand};
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use kube::{
    api::{DeleteParams, PostParams},
    Api, Client, CustomResourceExt,
};
use kubizone_crds::{Record, Zone};
use tracing::log::warn;

mod reconciliation;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    PrintCrds,
    DumpCrds { path: PathBuf },
    Reconcile { danger_recreate_crds: bool },
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    match args.command {
        Command::PrintCrds => {
            println!("{}", kubizone_crd_utils::serialize_crd::<Zone>().unwrap());
            println!("{}", kubizone_crd_utils::serialize_crd::<Record>().unwrap());
        }
        Command::DumpCrds { path } => {
            kubizone_crd_utils::write_to_path::<Zone>(&path).unwrap();
            kubizone_crd_utils::write_to_path::<Record>(&path).unwrap();
        }
        Command::Reconcile {
            danger_recreate_crds,
        } => {
            let client = Client::try_default().await.unwrap();

            if danger_recreate_crds {
                warn!("flag --danger-recreate-crds set, deleting Zone and Record CRDs from cluster, and recreating. This will delete all existing Records and Zones!");
                let api: Api<CustomResourceDefinition> = Api::all(client.clone());
                kubizone_crd_utils::recreate_crd_destructively::<Zone>(api.clone()).await;
                kubizone_crd_utils::recreate_crd_destructively::<Record>(api.clone()).await;
            }

            reconciliation::reconcile(client).await;
        }
    }

    let client = Client::try_default().await.unwrap();

    let crds: Api<CustomResourceDefinition> = Api::all(client.clone());

    crds.delete(Record::crd_name(), &DeleteParams::default())
        .await
        .ok();
    crds.delete(Zone::crd_name(), &DeleteParams::default())
        .await
        .ok();

    crds.create(&PostParams::default(), &Record::crd())
        .await
        .unwrap();
    crds.create(&PostParams::default(), &Zone::crd())
        .await
        .unwrap();

    reconciliation::reconcile(client).await;
}
