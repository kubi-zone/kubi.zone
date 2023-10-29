use kube::{CustomResource, ResourceExt};
use kubizone_crds::v1alpha1::ZoneRef;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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

    /// Number of zonefile ConfigMaps to keep around.
    pub const HISTORY: u32 = 10;

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

    pub(super) const fn history() -> u32 {
        HISTORY
    }
}

/// Label attached to [`Zone`](kubizone_crds::Zone)s as backreferences
/// to a single downstream [`ZoneFile`] generated from it.
///
/// Used by the controller to trigger reconciliation when upstream
/// zones change.
pub const TARGET_ZONEFILE_LABEL: &str = "kubi.zone/zonefile";

/// A [`ZoneFile`] references an upstream [`Zone`](kubizone_crds::Zone) and (re)builds
/// a configmap of the same name, whenever the zone changes, automatically incrementing
/// serials as necessary.
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema, Hash)]
#[kube(
    group = "zonefile.kubi.zone",
    version = "v1alpha1",
    kind = "ZoneFile",
    namespaced
)]
#[kube(status = "ZoneFileStatus")]
#[kube(printcolumn = r#"{"name":"zone", "jsonPath": ".spec.zoneRef.name", "type": "string"}"#)]
#[kube(printcolumn = r#"{"name":"serial", "jsonPath": ".status.serial", "type": "string"}"#)]
#[kube(printcolumn = r#"{"name":"hash", "jsonPath": ".status.hash", "type": "string"}"#)]
#[kube(printcolumn = r#"{"name":"configmap", "jsonPath": ".status.configMap", "type": "string"}"#)]
#[serde(rename_all = "camelCase")]
pub struct ZoneFileSpec {
    /// Reference to a [`Zone`](kubizone_crds::Zone), optionally in a different namespace.
    pub zone_ref: ZoneRef,

    /// Number of zonefile revisions to keep around in the form of ConfigMaps.
    ///
    /// If more than N ConfigMaps exist, which are descendents of this ZoneFile,
    /// then delete the oldest (lowest revision) ones, until N <= history.
    #[serde(default = "defaults::history")]
    pub history: u32,

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

impl ZoneFile {
    /// Retrieve the [`ZoneFile`]'s `zoneRef`, but populate the `namespace` variable,
    /// if not specified by the zoneref itself.
    pub fn zone_ref(&self) -> ZoneRef {
        ZoneRef {
            name: self.spec.zone_ref.name.clone(),
            namespace: self
                .spec
                .zone_ref
                .namespace
                .as_ref()
                .or(self.namespace().as_ref())
                .cloned(),
        }
    }
}

/// Describes the current state of the [`ZoneFile`], tracks state of
/// the upstream [`Zone`](kubizone_crds::Zone), to determine when the
/// output `ConfigMap` should be re-generated.
#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ZoneFileStatus {
    /// Last observed hash of the upstream [`Zone`](kubizone_crds::Zone)
    ///
    /// Used by the zonefile controller to trigger configmap rebuilds
    /// and zone serial rotation.
    pub hash: Option<String>,

    /// Serial of the latest generated zonefile.
    ///
    /// The zonefile controller will automatically increment this value
    /// whenever the zonefile configmap is rebuilt, in accordance with
    /// [RFC 1912](https://datatracker.ietf.org/doc/html/rfc1912#section-2.2)
    pub serial: Option<u32>,

    /// .metadata.name of the latest generated configmap
    pub config_map: Option<String>,
}
