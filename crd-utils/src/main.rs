use std::{path::PathBuf, process::exit, time::Duration};

use clap::{Parser, Subcommand};
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use kube::{
    api::{DeleteParams, PostParams},
    runtime::{
        conditions,
        wait::{await_condition, Condition},
    },
    Api, Client, CustomResourceExt, Resource,
};
use kubizone_crds::v1alpha1::{Record, Zone};
use zonefile_crds::ZoneFile;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Dump,
    Recreate {
        #[clap(long)]
        yes_im_sure_i_want_to_delete_all_resources: bool,
    },
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    match args.command {
        Command::Dump => {
            write_to_path::<Zone>().unwrap();
            write_to_path::<Record>().unwrap();
            write_to_path::<ZoneFile>().unwrap();
        }
        Command::Recreate {
            yes_im_sure_i_want_to_delete_all_resources,
        } => {
            if !yes_im_sure_i_want_to_delete_all_resources {
                eprintln!("Recreating all custom resource definitions will automatically delete");
                eprintln!("**ALL** zones, records and zonefiles across the ENTIRE CLUSTER!\n");
                eprintln!("If you are ABSOLUTELY SURE THAT IS WHAT YOU WANT TO DO, then you");
                eprintln!("must set the --yes-im-sure-i-want-to-delete-all-resources flag");
                exit(1)
            }

            let client = Client::try_default().await.unwrap();
            let api: Api<CustomResourceDefinition> = Api::all(client);

            recreate_crd_destructively::<Zone>(api.clone()).await;
            recreate_crd_destructively::<Record>(api.clone()).await;
            recreate_crd_destructively::<ZoneFile>(api.clone()).await;
        }
    }
}

fn serialize_crd<C>() -> Result<String, serde_yaml::Error>
where
    C: Resource<DynamicType = ()> + CustomResourceExt,
{
    Ok(format!("---\n{}", serde_yaml::to_string(&C::crd())?))
}

fn write_to_path<C>() -> Result<(), std::io::Error>
where
    C: Resource<DynamicType = ()> + CustomResourceExt,
{
    let directory = PathBuf::from("crds").join(C::api_version(&()).as_ref());

    std::fs::create_dir_all(&directory)?;

    std::fs::write(
        directory.join(format!("{name}.yaml", name = C::kind(&()))),
        serialize_crd::<C>().unwrap(),
    )
    .unwrap();

    Ok(())
}

async fn destroy_crd<C>(api: Api<CustomResourceDefinition>)
where
    C: Resource<DynamicType = ()> + CustomResourceExt,
{
    api.delete(C::crd_name(), &DeleteParams::default())
        .await
        .ok();

    tokio::time::timeout(
        Duration::from_secs(30),
        await_condition(api, C::crd_name(), conditions::is_crd_established().not()),
    )
    .await
    .unwrap()
    .unwrap();
}

async fn create_crd<C>(api: Api<CustomResourceDefinition>)
where
    C: Resource<DynamicType = ()> + CustomResourceExt,
{
    api.create(&PostParams::default(), &C::crd()).await.unwrap();

    tokio::time::timeout(
        Duration::from_secs(30),
        await_condition(api, C::crd_name(), conditions::is_crd_established()),
    )
    .await
    .unwrap()
    .unwrap();
}

async fn recreate_crd_destructively<C>(api: Api<CustomResourceDefinition>)
where
    C: Resource<DynamicType = ()> + CustomResourceExt,
{
    destroy_crd::<C>(api.clone()).await;
    create_crd::<C>(api).await
}
