use std::fmt::Display;

use kube::{CustomResource, ResourceExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const PARENT_ZONE_LABEL: &str = "dnsetes.pius.dev/parent-zone";

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
#[kube(
    group = "dnsetes.pius.dev",
    version = "v1alpha1",
    kind = "DNSZone",
    namespaced
)]
#[kube(status = "DNSZoneStatus")]
#[kube(printcolumn = r#"{"name":"name", "jsonPath": ".spec.name", "type": "string"}"#)]
#[kube(printcolumn = r#"{"name":"fqdn", "jsonPath": ".status.fqdn", "type": "string"}"#)]
#[kube(printcolumn = r#"{"name":"hash", "jsonPath": ".status.hash", "type": "integer"}"#)]
#[kube(
    printcolumn = r#"{"name":"parent", "jsonPath": ".metadata.annotations.dnsetes\\.pius\\.dev/parent-zone", "type": "string"}"#
)]
#[serde(rename_all = "camelCase")]
pub struct DNSZoneSpec {
    pub name: String,
    pub zone_ref: Option<ZoneRef>,
}

impl DNSZone {
    pub fn zone_ref(&self) -> ZoneRef {
        ZoneRef {
            name: self.name_any(),
            namespace: self.namespace(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DNSZoneStatus {
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
#[kube(
    group = "dnsetes.pius.dev",
    version = "v1alpha1",
    kind = "DNSRecord",
    namespaced
)]
#[kube(status = "DNSRecordStatus")]
#[kube(printcolumn = r#"{"name":"name", "jsonPath": ".spec.name", "type": "string"}"#)]
#[kube(printcolumn = r#"{"name":"class", "jsonPath": ".spec.class", "type": "string"}"#)]
#[kube(printcolumn = r#"{"name":"type", "jsonPath": ".spec.type", "type": "string"}"#)]
#[kube(printcolumn = r#"{"name":"data", "jsonPath": ".spec.rdata", "type": "string"}"#)]
#[kube(printcolumn = r#"{"name":"fqdn", "jsonPath": ".status.fqdn", "type": "string"}"#)]
#[kube(
    printcolumn = r#"{"name":"parent", "jsonPath": ".metadata.annotations.dnsetes\\.pius\\.dev/parent-zone", "type": "string"}"#
)]
#[serde(rename_all = "camelCase")]
pub struct DNSRecordSpec {
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
pub struct DNSRecordStatus {
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
            write!(f, "{namespace}.{}", self.name)
        } else {
            f.write_str(&self.name)
        }
    }
}

#[cfg(test)]
mod tests {
    use kube::CustomResourceExt;

    use crate::{DNSRecord, DNSZone};

    #[test]
    fn dump_crds() {
        println!("---{}", serde_yaml::to_string(&DNSZone::crd()).unwrap());
        println!("---{}", serde_yaml::to_string(&DNSRecord::crd()).unwrap());
    }
}
