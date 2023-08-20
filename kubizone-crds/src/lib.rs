use std::{fmt::Debug, hash::Hash};

pub mod v1alpha1;

use kube::{runtime::reflector::ObjectRef, Resource, ResourceExt};
use serde::de::DeserializeOwned;

pub const PARENT_ZONE_LABEL: &str = "kubi.zone/parent-zone";

pub fn watch_reference<Parent, K>(label: &'static str) -> impl Fn(K) -> Option<ObjectRef<Parent>>
where
    K: ResourceExt,
    Parent: Clone + Resource + DeserializeOwned + Debug + Send + 'static,
    Parent::DynamicType: Default + Debug + Clone + Eq + Hash,
{
    |object| {
        let parent = object.labels().get(label)?;

        let (name, namespace) = parent.split_once('.')?;

        Some(ObjectRef::new(name).within(namespace))
    }
}
