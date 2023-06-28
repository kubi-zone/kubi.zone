use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "dnsetes.pius.dev",
    version = "v1alpha1",
    kind = "DNSZone",
    namespaced
)]
#[kube(printcolumn = r#"{"name":"zone", "jsonPath": ".spec.name", "type": "string"}"#)]
#[kube(printcolumn = r#"{"name":"serial", "jsonPath": ".spec.serial", "type": "integer"}"#)]
#[serde(rename_all = "camelCase")]
pub struct DNSZoneSpec {
    pub name: String,
    pub serial: Option<u32>,
    pub refresh: Option<u32>,
    pub retry: Option<u32>,
    pub expire: Option<u32>,
    pub negative_response_cache: Option<u32>
}
