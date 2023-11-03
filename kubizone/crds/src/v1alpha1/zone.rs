use std::fmt::Display;

use kube::{core::object::HasSpec, CustomResource, ResourceExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::*;

use super::{domain_matches_pattern, Record, ZoneRef};

pub mod defaults {

    pub const REFRESH: u32 = 86400;
    /// Service addresses might change often, so we use a low
    /// Time-to-Live to increase cache responsiveness.
    pub const TTL: u32 = 360;

    /// Recommendation for small and stable zones[^1]: 7200 seconds (2 hours).
    ///
    /// [^1]: <https://www.ripe.net/publications/docs/ripe-203>
    pub const RETRY: u32 = 7200;

    /// Recommendation for small and stable zones[^1]: 3600000 seconds (1000 hours).
    ///
    /// [^1]: <https://www.ripe.net/publications/docs/ripe-203>
    pub const EXPIRE: u32 = 3600000;

    /// Recommendation for small and stable zones[^1]: 172800 seconds (2 days),
    /// but we select a much lower value to increase cache responsiveness
    /// and reduce failed lookups to records still being provisioned.
    ///
    /// [^1]: <https://www.ripe.net/publications/docs/ripe-203>
    pub const NEGATIVE_RESPONSE_CACHE: u32 = 360;

    // The functions below are only there for use with `serde(default)`.
    pub(super) const fn refresh() -> u32 {
        REFRESH
    }
    pub(super) const fn ttl() -> u32 {
        TTL
    }
    pub(super) const fn retry() -> u32 {
        RETRY
    }

    pub(super) const fn expire() -> u32 {
        EXPIRE
    }

    pub(super) const fn negative_response_cache() -> u32 {
        NEGATIVE_RESPONSE_CACHE
    }
}

#[derive(
    Default,
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
#[kube(printcolumn = r#"{"name":"serial", "jsonPath": ".status.serial", "type": "string"}"#)]
#[kube(
    printcolumn = r#"{"name":"parent", "jsonPath": ".metadata.labels.kubi\\.zone/parent-zone", "type": "string"}"#
)]
#[serde(rename_all = "camelCase")]
pub struct ZoneSpec {
    pub domain_name: String,

    /// Optional reference to a parent zone which this zone is a sub-zone of.
    ///
    /// Zones must have *either* a zoneRef, or end in a '.', making it a fully
    /// qualified domain name. It cannot have both.
    pub zone_ref: Option<ZoneRef>,

    /// List of namespaced records and zones which are allowed to "insert"
    /// themselves into this zone. See the [`Delegation`] type for more information.
    pub delegations: Vec<Delegation>,

    /// Time-to-Live. Represents how long (in seconds) recursive resolvers should
    /// keep this record in their cache.
    #[serde(default = "defaults::ttl")]
    pub ttl: u32,

    /// Number of seconds after which secondary name servers should
    /// query the master for the SOA record, to detect zone changes.
    ///
    /// Recommendation for small and stable zones[^1]: 86400 seconds (24 hours).
    ///
    /// [^1]: <https://www.ripe.net/publications/docs/ripe-203>
    #[serde(default = "defaults::refresh")]
    pub refresh: u32,

    /// Number of seconds after which secondary name servers should
    /// retry to request the serial number from the master if the
    /// master does not respond.
    ///
    /// It must be less than Refresh.
    ///
    /// Recommendation for small and stable zones[^1]: 7200 seconds (2 hours).
    ///
    /// [^1]: <https://www.ripe.net/publications/docs/ripe-203>
    #[serde(default = "defaults::retry")]
    pub retry: u32,

    /// Number of seconds after which secondary name servers should
    /// stop answering request for this zone if the master does not respond.
    ///
    /// This value must be bigger than the sum of Refresh and Retry.
    ///
    /// Recommendation for small and stable zones[^1]: 3600000 seconds (1000 hours)
    ///
    /// [^1]: <https://www.ripe.net/publications/docs/ripe-203>
    #[serde(default = "defaults::expire")]
    pub expire: u32,

    /// Used in calculating the time to live for purposes of negative caching.
    /// Authoritative name servers take the smaller of the SOA TTL and this value
    /// to send as the SOA TTL in negative responses.
    ///
    /// Resolvers use the resulting SOA TTL to understand for how long they
    /// are allowed to cache a negative response.
    ///
    /// Recommendation for small and stable zones[^1] 172800 seconds (2 days)
    ///
    /// [^1]: <https://www.ripe.net/publications/docs/ripe-203>
    #[serde(default = "defaults::negative_response_cache")]
    pub negative_response_cache: u32,
}

impl Zone {
    /// Produce a zoneRef pointing to this zone
    pub fn zone_ref(&self) -> ZoneRef {
        ZoneRef {
            name: self.name_any(),
            namespace: self.namespace(),
        }
    }

    /// Fetch the computed FQDN from this zone, if one has been set.
    pub fn fqdn(&self) -> Option<&str> {
        self.status
            .as_ref()
            .and_then(|status| status.fqdn.as_deref())
    }

    pub fn hash(&self) -> Option<&str> {
        self.status
            .as_ref()
            .and_then(|status| status.hash.as_deref())
    }

    pub fn serial(&self) -> Option<u32> {
        self.status.as_ref().and_then(|status| status.serial)
    }

    /// Validate that the given Record is allowed, given the delegations of this Zone.
    pub fn validate_record(&self, record: &Record) -> bool {
        let Some(parent_fqdn) = self.fqdn() else {
            trace!("parent zone {self} has no fqdn, and can therefore not validate record");
            return false;
        };

        let Some(record_fqdn) = record.fqdn() else {
            trace!("record {record} has no fqdn, and can therefore not be validated");
            return false;
        };

        if !record
            .fqdn()
            .is_some_and(|fqdn| fqdn.ends_with(parent_fqdn))
        {
            trace!("record {record_fqdn} is not a subdomain of {parent_fqdn}");
            return false;
        }

        if self.spec().delegations.iter().any(|delegation| {
            delegation.covers_namespace(&record.namespace().unwrap_or_default())
                && delegation.validate_record(
                    parent_fqdn,
                    &record.spec.type_,
                    &record.spec.domain_name,
                )
        }) {
            debug!("zone {parent_fqdn} allows delegation to record {record_fqdn}");
            true
        } else {
            trace!("zone {parent_fqdn} forbid delegation to record {record_fqdn}");
            false
        }
    }

    /// Validate that the given Zone is allowed by the delgations specified in this Zone.
    pub fn validate_zone(&self, zone: &Zone) -> bool {
        let Some(parent_fqdn) = self.fqdn() else {
            trace!("zone {self}'s fqdn is not defined.");
            return false;
        };

        let Some(zone_fqdn) = zone.fqdn() else {
            trace!("zone {self}'s fqdn is not defined.");
            return false;
        };

        if zone_fqdn.ends_with(parent_fqdn) {
            trace!("zone {} is not a subdomain of {}", zone_fqdn, parent_fqdn);
            return false;
        }

        // Cannot be a subdomain of itself
        if self.uid() == zone.uid() {
            return false;
        }

        self.spec().delegations.iter().any(|delegation| {
            delegation.covers_namespace(&zone.namespace().unwrap_or_default())
                && delegation.validate_zone(parent_fqdn, &zone.spec.domain_name)
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

#[derive(Default, Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ZoneStatus {
    #[serde(default)]
    pub entries: Vec<ZoneEntry>,

    /// Zones fully qualified domain name.
    ///
    /// If the `.spec.domainName` is already fully qualified, these are identical.
    ///
    /// If instead the Zone uses a `.spec.zoneRef` to indicate its parent,
    /// this will be the concatenated version of this zone's `.spec.domainName`
    /// and the parent's `.status.fqdn`
    #[serde(default)]
    pub fqdn: Option<String>,

    /// Hash value of all relevant zone entries.
    #[serde(default)]
    pub hash: Option<String>,

    /// Serial of the latest generated zonefile.
    ///
    /// The controller will automatically increment this value
    /// whenever the zone changes, in accordance with
    /// [RFC 1912](https://datatracker.ietf.org/doc/html/rfc1912#section-2.2)
    #[serde(default)]
    pub serial: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema, Hash)]
#[serde(rename_all = "camelCase")]
pub struct ZoneEntry {
    pub fqdn: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub class: String,
    pub ttl: u32,
    pub rdata: String,
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
    pub types: Vec<String>,
}

impl RecordDelegation {
    pub fn validate(&self, zone_fqdn: &str, record_type: &str, domain: &str) -> bool {
        let record_type = record_type.to_uppercase();

        return domain_matches_pattern(&self.pattern.replace('@', zone_fqdn), domain)
            && (self.types.is_empty()
                || self
                    .types
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

        trace!("delegation {self:?} does not cover {namespace}");
        false
    }

    /// Verify that a (record type, domain) pair matches the delegation
    /// rules of this delegation.
    pub fn validate_record(&self, zone_fqdn: &str, record_type: &str, domain: &str) -> bool {
        for record_delegation in &self.records {
            if record_delegation.validate(zone_fqdn, record_type, domain) {
                return true;
            }
        }

        // If no record delegations exist, deny.
        false
    }

    /// Verify that a domain matches the zone delegation
    /// rules of this delegation.
    pub fn validate_zone(&self, parent_fqdn: &str, domain: &str) -> bool {
        for zone_delegation in &self.zones {
            if domain_matches_pattern(&zone_delegation.replace('@', parent_fqdn), domain) {
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

    use crate::v1alpha1::{Record, RecordSpec, RecordStatus, ZoneStatus};

    use super::{Delegation, RecordDelegation, Zone, ZoneSpec};

    #[test]
    fn test_record_delegation() {
        tracing_subscriber::fmt::init();

        let zone = Zone {
            spec: ZoneSpec {
                domain_name: String::from("example.org."),
                zone_ref: None,
                delegations: vec![Delegation {
                    namespaces: vec![String::from("default")],
                    zones: vec![],
                    records: vec![RecordDelegation {
                        pattern: String::from("*.example.org."),
                        types: vec![],
                    }],
                }],
                ..Default::default()
            },
            status: Some(ZoneStatus {
                fqdn: Some(String::from("example.org.")),
                ..Default::default()
            }),
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
                        types: vec![String::from("MX")],
                    }],
                }],
                ..Default::default()
            },
            status: Some(ZoneStatus {
                fqdn: Some(String::from("example.org.")),
                ..Default::default()
            }),
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
