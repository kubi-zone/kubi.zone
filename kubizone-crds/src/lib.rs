use std::{
    fmt::{Debug, Display},
    hash::Hash,
};

use kube::{runtime::reflector::ObjectRef, CustomResource, Resource, ResourceExt};
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

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

mod defaults {
    pub const CLASS: &str = "IN";
    pub(super) fn class() -> String {
        CLASS.to_string()
    }
}

#[derive(
    CustomResource,
    Deserialize,
    Serialize,
    Clone,
    Debug,
    JsonSchema,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[kube(group = "kubi.zone", version = "v1alpha1", kind = "Zone", namespaced)]
#[kube(status = "ZoneStatus")]
#[kube(printcolumn = r#"{"name":"name", "jsonPath": ".spec.name", "type": "string"}"#)]
#[kube(printcolumn = r#"{"name":"fqdn", "jsonPath": ".status.fqdn", "type": "string"}"#)]
#[kube(printcolumn = r#"{"name":"hash", "jsonPath": ".status.hash", "type": "string"}"#)]
#[kube(
    printcolumn = r#"{"name":"parent", "jsonPath": ".metadata.labels.kube\\.zone/parent-zone", "type": "string"}"#
)]
#[serde(rename_all = "camelCase")]
pub struct ZoneSpec {
    pub name: String,
    pub zone_ref: Option<ZoneRef>,
}

impl Zone {
    pub fn zone_ref(&self) -> ZoneRef {
        ZoneRef {
            name: self.name_any(),
            namespace: self.namespace(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ZoneStatus {
    pub fqdn: Option<String>,
    pub hash: Option<String>,
}

#[derive(
    CustomResource,
    Deserialize,
    Serialize,
    Clone,
    Debug,
    JsonSchema,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[kube(group = "kubi.zone", version = "v1alpha1", kind = "Record", namespaced)]
#[kube(status = "RecordStatus")]
#[kube(printcolumn = r#"{"name":"name", "jsonPath": ".spec.name", "type": "string"}"#)]
#[kube(printcolumn = r#"{"name":"class", "jsonPath": ".spec.class", "type": "string"}"#)]
#[kube(printcolumn = r#"{"name":"type", "jsonPath": ".spec.type", "type": "string"}"#)]
#[kube(printcolumn = r#"{"name":"data", "jsonPath": ".spec.rdata", "type": "string"}"#)]
#[kube(printcolumn = r#"{"name":"fqdn", "jsonPath": ".status.fqdn", "type": "string"}"#)]
#[kube(
    printcolumn = r#"{"name":"parent", "jsonPath": ".metadata.labels.kube\\.zone/parent-zone", "type": "string"}"#
)]
#[serde(rename_all = "camelCase")]
pub struct RecordSpec {
    pub name: String,
    pub zone_ref: Option<ZoneRef>,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(default = "defaults::class")]
    pub class: String,
    pub ttl: Option<u32>,
    pub rdata: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct RecordStatus {
    pub fqdn: Option<String>,
}

#[derive(
    Serialize, Deserialize, Clone, Debug, JsonSchema, Hash, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct ZoneRef {
    pub name: String,
    pub namespace: Option<String>,
}

impl Display for ZoneRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(namespace) = &self.namespace {
            write!(f, "{}.{namespace}", self.name)
        } else {
            f.write_str(&self.name)
        }
    }
}
