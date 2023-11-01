use std::fmt::Display;

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
    Default,
    JsonSchema,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[kube(group = "kubi.zone", version = "v1alpha1", kind = "Record", namespaced)]
#[kube(status = "RecordStatus")]
#[kube(printcolumn = r#"{"name":"domain name", "jsonPath": ".spec.domainName", "type": "string"}"#)]
#[kube(printcolumn = r#"{"name":"class", "jsonPath": ".spec.class", "type": "string"}"#)]
#[kube(printcolumn = r#"{"name":"type", "jsonPath": ".spec.type", "type": "string"}"#)]
#[kube(printcolumn = r#"{"name":"data", "jsonPath": ".spec.rdata", "type": "string"}"#)]
#[kube(printcolumn = r#"{"name":"fqdn", "jsonPath": ".status.fqdn", "type": "string"}"#)]
#[kube(
    printcolumn = r#"{"name":"parent", "jsonPath": ".metadata.labels.kubi\\.zone/parent-zone", "type": "string"}"#
)]
#[serde(rename_all = "camelCase")]
pub struct RecordSpec {
    pub domain_name: String,
    pub zone_ref: Option<ZoneRef>,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(default = "super::defaults::class")]
    pub class: String,
    pub ttl: Option<u32>,
    pub rdata: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct RecordStatus {
    pub fqdn: Option<String>,
}

impl Record {
    pub fn fqdn(&self) -> Option<&str> {
        self.status
            .as_ref()
            .and_then(|status| status.fqdn.as_deref())
    }
}

impl Display for Record {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Unwrap safety: Records are namespaced and therefore always have a name.
        write!(
            f,
            "{}/{}",
            self.metadata.namespace.as_ref().unwrap(),
            self.name_any()
        )
    }
}
