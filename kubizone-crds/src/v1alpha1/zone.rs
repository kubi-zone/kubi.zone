use std::fmt::Display;

use kube::{core::object::HasSpec, CustomResource, ResourceExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{domain_matches_pattern, Record, ZoneRef};

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
    pub delegations: Vec<Delegation>,
}

impl Zone {
    pub fn zone_ref(&self) -> ZoneRef {
        ZoneRef {
            name: self.name_any(),
            namespace: self.namespace(),
        }
    }

    pub fn fqdn(&self) -> Option<&str> {
        self.status
            .as_ref()
            .and_then(|status| status.fqdn.as_deref())
    }

    /// Validate that the given Record is allowed, given the delegations of this Zone.
    pub fn validate_record(&self, record: &Record) -> bool {
        if !record
            .status
            .as_ref()
            .and_then(|status| status.fqdn.as_ref())
            .is_some_and(|fqdn| fqdn.ends_with(&self.spec.domain_name))
        {
            return false;
        }

        self.spec().delegations.iter().any(|delegation| {
            delegation.covers_namespace(&record.namespace().unwrap_or_default())
                && delegation.validate_record(&record.spec.type_, &record.spec.domain_name)
        })
    }

    /// Validate that the given Zone is allowed by the delgations specified in this Zone.
    pub fn validate_zone(&self, zone: &Zone) -> bool {
        if !zone
            .status
            .as_ref()
            .and_then(|status| status.fqdn.as_ref())
            .is_some_and(|fqdn| fqdn.ends_with(&self.spec.domain_name))
        {
            return false;
        }

        // Cannot be a subdomain of itself
        if self.uid() == zone.uid() {
            return false;
        }

        self.spec().delegations.iter().any(|delegation| {
            delegation.covers_namespace(&zone.namespace().unwrap_or_default())
                && delegation.validate_zone(&zone.spec.domain_name)
        })
    }
}

impl Display for Zone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Unwrap safety: Zones are namespaced and therefore always have a name.
        write!(
            f,
            "{}/{}",
            self.metadata.namespace.as_ref().unwrap(),
            self.name_any()
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ZoneStatus {
    pub fqdn: Option<String>,
    pub hash: Option<String>,
}

#[derive(
    Serialize, Deserialize, Clone, Debug, JsonSchema, Hash, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(rename_all = "camelCase")]
pub struct RecordDelegation {
    /// Pattern which delegated records must match.
    pub pattern: String,

    /// Type of record to allow. Empty list implies *any*.
    #[serde(default)]
    pub record_types: Vec<String>,
}

impl RecordDelegation {
    pub fn validate(&self, record_type: &str, domain: &str) -> bool {
        let record_type = record_type.to_uppercase();

        return domain_matches_pattern(&self.pattern, domain)
            && (self.record_types.is_empty()
                || self
                    .record_types
                    .iter()
                    .any(|delegated_type| delegated_type.to_uppercase() == record_type));
    }
}

#[derive(
    Serialize, Deserialize, Clone, Debug, JsonSchema, Hash, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct Delegation {
    #[serde(default)]
    pub namespaces: Vec<String>,
    #[serde(default)]
    pub zones: Vec<String>,
    #[serde(default)]
    pub records: Vec<RecordDelegation>,
}

impl Delegation {
    /// Check if the given namespace is covered by this Delegation.
    pub fn covers_namespace(&self, namespace: &str) -> bool {
        if self.namespaces.is_empty() {
            return true;
        }

        if self
            .namespaces
            .iter()
            .any(|delegated_namespace| delegated_namespace == namespace)
        {
            return true;
        }

        false
    }

    /// Verify that a (record type, domain) pair matches the delegation
    /// rules of this delegation.
    pub fn validate_record(&self, record_type: &str, domain: &str) -> bool {
        for record_delegation in &self.records {
            if record_delegation.validate(record_type, domain) {
                return true;
            }
        }

        // If no record delegations exist, deny.
        false
    }

    /// Verify that a domain matches the zone delegation
    /// rules of this delegation.
    pub fn validate_zone(&self, domain: &str) -> bool {
        for zone_delegation in &self.zones {
            if domain_matches_pattern(zone_delegation, domain) {
                return true;
            }
        }

        // If no zone delegations exist, deny.
        false
    }
}

#[cfg(test)]
mod tests {
    use kube::core::ObjectMeta;

    use crate::v1alpha1::{Record, RecordSpec, RecordStatus};

    use super::{Delegation, RecordDelegation, Zone, ZoneSpec};

    #[test]
    fn test_record_delegation() {
        let zone = Zone {
            spec: ZoneSpec {
                domain_name: String::from("example.org."),
                zone_ref: None,
                delegations: vec![Delegation {
                    namespaces: vec![String::from("default")],
                    zones: vec![],
                    records: vec![RecordDelegation {
                        pattern: String::from("*.example.org."),
                        record_types: vec![],
                    }],
                }],
            },
            status: None,
            metadata: kube::core::ObjectMeta::default(),
        };

        // Record in delegated namespace should be allowed.
        assert!(zone.validate_record(&Record {
            metadata: ObjectMeta {
                namespace: Some(String::from("default")),
                ..Default::default()
            },
            spec: RecordSpec {
                domain_name: String::from("www.example.org."),
                zone_ref: None,
                type_: String::from("A"),
                class: String::from("IN"),
                ttl: None,
                rdata: String::from("192.168.0.1")
            },
            status: Some(RecordStatus {
                fqdn: Some(String::from("www.example.org."))
            })
        }));

        // Record in non-delegated namespace should fail.
        assert!(!zone.validate_record(&Record {
            metadata: ObjectMeta {
                namespace: Some(String::from("not-default")),
                ..Default::default()
            },
            spec: RecordSpec {
                domain_name: String::from("www.example.org."),
                zone_ref: None,
                type_: String::from("A"),
                class: String::from("IN"),
                ttl: None,
                rdata: String::from("192.168.0.1")
            },
            status: None
        }));

        // Record in delegated namespace, with invalid super-domain should fail.
        assert!(!zone.validate_record(&Record {
            metadata: ObjectMeta {
                namespace: Some(String::from("default")),
                ..Default::default()
            },
            spec: RecordSpec {
                domain_name: String::from("www.test.com."),
                zone_ref: None,
                type_: String::from("A"),
                class: String::from("IN"),
                ttl: None,
                rdata: String::from("192.168.0.1")
            },
            status: None
        }))
    }

    #[test]
    fn test_record_type_limit() {
        let zone = Zone {
            spec: ZoneSpec {
                domain_name: String::from("example.org."),
                zone_ref: None,
                delegations: vec![Delegation {
                    namespaces: vec![String::from("default")],
                    zones: vec![],
                    records: vec![RecordDelegation {
                        pattern: String::from("example.org."),
                        record_types: vec![String::from("MX")],
                    }],
                }],
            },
            status: None,
            metadata: kube::core::ObjectMeta::default(),
        };

        // Record in delegated namespace with delegated record type
        // (MX) should be allowed.
        assert!(zone.validate_record(&Record {
            metadata: ObjectMeta {
                namespace: Some(String::from("default")),
                ..Default::default()
            },
            spec: RecordSpec {
                domain_name: String::from("example.org."),
                zone_ref: None,
                type_: String::from("MX"),
                class: String::from("IN"),
                ttl: None,
                rdata: String::from("10 mail1.example.org.")
            },
            status: Some(RecordStatus {
                fqdn: Some(String::from("example.org."))
            })
        }));

        // Record in delegated namespace with non-delegated record type
        // (A) should not be allowed.
        assert!(!zone.validate_record(&Record {
            metadata: ObjectMeta {
                namespace: Some(String::from("default")),
                ..Default::default()
            },
            spec: RecordSpec {
                domain_name: String::from("example.org."),
                zone_ref: None,
                type_: String::from("A"),
                class: String::from("IN"),
                ttl: None,
                rdata: String::from("192.168.0.1")
            },
            status: None
        }));
    }
}
