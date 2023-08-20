use kube::{CustomResource, ResourceExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::ZoneRef;

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
#[kube(printcolumn = r#"{"name":"domain name", "jsonPath": ".spec.domainName", "type": "string"}"#)]
#[kube(printcolumn = r#"{"name":"fqdn", "jsonPath": ".status.fqdn", "type": "string"}"#)]
#[kube(printcolumn = r#"{"name":"hash", "jsonPath": ".status.hash", "type": "string"}"#)]
#[kube(
    printcolumn = r#"{"name":"parent", "jsonPath": ".metadata.labels.kubi\\.zone/parent-zone", "type": "string"}"#
)]
#[serde(rename_all = "camelCase")]
pub struct ZoneSpec {
    pub domain_name: String,
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
