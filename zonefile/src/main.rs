use std::path::PathBuf;

use clap::{command, Parser, Subcommand};
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use kube::{Api, Client};
use tracing::log::*;
use zonefile_crds::ZoneFile;

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
    DumpCrds {
        path: PathBuf,
    },
    DangerRecreateCrds,
    Reconcile {
        #[clap(long)]
        danger_recreate_crds: bool,
    },
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    match args.command {
        Command::PrintCrds => {
            println!("{}", crd_utils::serialize_crd::<ZoneFile>().unwrap());
        }
        Command::DumpCrds { path } => {
            crd_utils::write_to_path::<ZoneFile>(&path).unwrap();
        }
        Command::DangerRecreateCrds => {
            let client = Client::try_default().await.unwrap();

            warn!("action danger-recreate-crds chosen, deleting ZoneFile CRDs from cluster, and recreating. This will delete all existing ZoneFiles!");
            let api: Api<CustomResourceDefinition> = Api::all(client.clone());
            crd_utils::recreate_crd_destructively::<ZoneFile>(api).await;
        }
        Command::Reconcile {
            danger_recreate_crds,
        } => {
            let client = Client::try_default().await.unwrap();

            if danger_recreate_crds {
                warn!("flag --danger-recreate-crds set, deleting ZoneFile CRDs from cluster, and recreating. This will delete all existing ZoneFiles!");
                let api: Api<CustomResourceDefinition> = Api::all(client.clone());
                crd_utils::recreate_crd_destructively::<ZoneFile>(api).await;
            }

            reconciliation::reconcile(client).await;
        }
    }
}