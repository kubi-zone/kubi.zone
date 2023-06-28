use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod defaults {
    pub const REFRESH: u32 = 7600;
    pub const TTL: u32 = 3600;
    pub const SERIAL: u32 = 1;
    pub const RETRY: u32 = 3600;
    pub const EXPIRE: u32 = 3600;
    pub const NEGATIVE_RESPONSE_CACHE: u32 = 3600;
    pub const CLASS: &str = "IN";

    pub(super) fn refresh() -> u32 {
        REFRESH
    }
    pub(super) fn ttl() -> u32 {
        TTL
    }
    pub(super) fn serial() -> u32 {
        SERIAL
    }
    pub(super) fn retry() -> u32 {
        RETRY
    }
    pub(super) fn expire() -> u32 {
        EXPIRE
    }
    pub(super) fn negative_response_cache() -> u32 {
        NEGATIVE_RESPONSE_CACHE
    }
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
#[kube(printcolumn = r#"{"name":"serial", "jsonPath": ".spec.serial", "type": "integer"}"#)]
#[kube(printcolumn = r#"{"name":"fqdn", "jsonPath": ".status.fqdn", "type": "string"}"#)]
#[kube(
    printcolumn = r#"{"name":"parent", "jsonPath": ".metadata.annotations.dnsetes\\.pius\\.dev/parent-zone", "type": "string"}"#
)]
#[serde(rename_all = "camelCase")]
pub struct DNSZoneSpec {
    pub name: String,
    pub zone_ref: Option<ZoneRef>,
    #[serde(default = "defaults::ttl")]
    pub ttl: u32,
    #[serde(default = "defaults::serial")]
    pub serial: u32,
    #[serde(default = "defaults::refresh")]
    pub refresh: u32,
    #[serde(default = "defaults::retry")]
    pub retry: u32,
    #[serde(default = "defaults::expire")]
    pub expire: u32,
    #[serde(default = "defaults::negative_response_cache")]
    pub negative_response_cache: u32,
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
