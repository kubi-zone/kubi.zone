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

impl Display for ZoneRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(namespace) = &self.namespace {
            write!(f, "{}.{namespace}", self.name)
        } else {
            f.write_str(&self.name)
        }
    }
}

pub mod defaults {
    pub const CLASS: &str = "IN";
    pub(super) fn class() -> String {
        CLASS.to_string()
    }
}
