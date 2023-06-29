use dnsetes_crds::ZoneRef;
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
}

pub const TARGET_ZONEFILE_LABEL: &str = "dnsetes.pius.dev/zonefile";

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema, Hash)]
#[kube(
    group = "dnsetes.pius.dev",
    version = "v1alpha1",
    kind = "ZoneFile",
    namespaced
)]
#[kube(status = "ZoneFileStatus")]
#[kube(printcolumn = r#"{"name":"zone", "jsonPath": ".spec.zone.name", "type": "string"}"#)]
#[kube(printcolumn = r#"{"name":"serial", "jsonPath": ".spec.serial", "type": "string"}"#)]
#[serde(rename_all = "camelCase")]
pub struct ZoneFileSpec {
    pub zone_ref: ZoneRef,
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
pub struct ZoneFileStatus {
    pub hash: Option<String>,
}
