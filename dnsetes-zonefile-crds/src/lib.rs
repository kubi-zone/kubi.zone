use dnsetes_crds::ZoneRef;
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema, Hash)]
#[kube(
    group = "dnsetes.pius.dev",
    version = "v1alpha1",
    kind = "ZoneFile",
    namespaced
)]
#[kube(status = "ZoneFileStatus")]
#[kube(printcolumn = r#"{"name":"zone", "jsonPath": ".spec.zone.name", "type": "string"}"#)]
#[serde(rename_all = "camelCase")]
pub struct ZoneFileSpec {
    pub zone: ZoneRef,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct ZoneFileStatus {
    pub last_serial: Option<String>,
}
