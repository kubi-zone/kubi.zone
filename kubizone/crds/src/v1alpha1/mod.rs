mod dnsrecord;
mod zone;

use std::fmt::Display;

pub use dnsrecord::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
pub use zone::*;

#[derive(
    Serialize, Deserialize, Clone, Debug, JsonSchema, Hash, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct ZoneRef {
    pub name: String,
    pub namespace: Option<String>,
}

impl ZoneRef {
    pub fn as_label(&self) -> String {
        if let Some(namespace) = &self.namespace {
            format!("{}.{namespace}", self.name)
        } else {
            self.name.clone()
        }
    }
}

impl Display for ZoneRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(namespace) = &self.namespace {
            write!(f, "{namespace}/{}", self.name)
        } else {
            f.write_str(&self.name)
        }
    }
}

/// Authority on whether a domain matches a domain pattern.
pub fn domain_matches_pattern(pattern: &str, domain: &str) -> bool {
    let pattern_segments: Vec<_> = pattern.split('.').rev().collect();
    let domain_segments: Vec<_> = domain.split('.').rev().collect();

    // If domain and pattern contain an unequal number of segments, and the first
    // segment of the pattern is not a plain wildcard, then this pattern cannot match.
    if pattern_segments.len() != domain_segments.len() && pattern_segments.last() != Some(&"*") {
        return false;
    }

    for (pattern, domain) in pattern_segments.into_iter().zip(domain_segments) {
        if pattern == domain {
            continue;
        }

        if let Some((head, tail)) = pattern.split_once('*') {
            return domain.starts_with(head) && domain.ends_with(tail);
        }

        return false;
    }

    true
}

pub mod defaults {
    pub const CLASS: &str = "IN";
    pub(super) fn class() -> String {
        CLASS.to_string()
    }
}

#[cfg(test)]
mod tests {
    use crate::v1alpha1::domain_matches_pattern;

    #[test]
    fn pattern_matching() {
        // Should match on exact equivalence.
        assert!(domain_matches_pattern("www.example.org", "www.example.org"));

        // Should match on simple wildcard substitution.
        assert!(domain_matches_pattern("*.example.org", "www.example.org"));

        // Should match arbitrary prefixes and segments, if first segment is plain
        // wildcard.
        assert!(domain_matches_pattern(
            "*.example.org",
            "www.test.example.org"
        ));

        // Should NOT match arbitrary prefixes and segments, if first segment
        // is *made up of* wildcard and other values.
        assert!(!domain_matches_pattern(
            "env-*.example.org",
            "www.env-dev.example.org"
        ));

        // Should match if first segment is plain wildcard, and higher segments
        // match, but are partial wildcards.
        assert!(domain_matches_pattern(
            "*.env-*.example.org",
            "www.env-dev.example.org"
        ));

        // Should NOT match subdomains of explicit paths without wildcards.
        assert!(!domain_matches_pattern("example.org", "www.example.org"));

        // Should match on non-prefix wildcard.
        assert!(domain_matches_pattern(
            "www.*.example.org",
            "www.test.example.org"
        ));

        // Should not match on secondary subdivision.
        assert!(!domain_matches_pattern(
            "www.*.example.org",
            "www.subdomain.test.example.org"
        ));
    }
}
