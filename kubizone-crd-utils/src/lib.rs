use std::{path::Path, time::Duration};

use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use kube::{
    api::{DeleteParams, PostParams},
    runtime::{
        conditions,
        wait::{await_condition, Condition},
    },
    Api, CustomResourceExt, Resource,
};

pub fn serialize_crd<C>() -> Result<String, serde_yaml::Error>
where
    C: Resource<DynamicType = ()> + CustomResourceExt,
{
    Ok(format!("---\n{}", serde_yaml::to_string(&C::crd())?))
}

pub fn write_to_path<C>(path: &Path) -> Result<(), std::io::Error>
where
    C: Resource<DynamicType = ()> + CustomResourceExt,
{
    let directory = path.join(C::api_version(&()).as_ref());

    std::fs::create_dir_all(&directory)?;

    std::fs::write(
        directory.join(format!("{name}.yaml", name = C::kind(&()))),
        serialize_crd::<C>().unwrap(),
    )
    .unwrap();

    Ok(())
}

pub async fn destroy_crd<C>(api: Api<CustomResourceDefinition>)
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

pub async fn create_crd<C>(api: Api<CustomResourceDefinition>)
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

pub async fn recreate_crd_destructively<C>(api: Api<CustomResourceDefinition>)
where
    C: Resource<DynamicType = ()> + CustomResourceExt,
{
    destroy_crd::<C>(api.clone()).await;
    create_crd::<C>(api).await
}
