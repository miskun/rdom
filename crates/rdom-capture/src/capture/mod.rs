//! The `Capture` contract (`specs/CAPTURE.md` §The `Capture` contract):
//! one versioned JSON document per moment of an app, the single source
//! every view, diff and query reads.
//!
//! A capture is layered. `meta` is always present and so is the grid
//! hash; the grid itself and the other layers are included when asked
//! ([`Layer`]).

mod color;
mod grid;
mod hash;

pub use color::ColorValue;
pub use grid::{Attr, Grid, Run};
pub use hash::{fnv1a64, grid_hash};

use serde::{Deserialize, Serialize};

use crate::env::Environment;

/// The schema identifier every capture carries.
pub const SCHEMA: &str = "rdom-capture/1";

/// One moment of an app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Capture {
    /// Always [`SCHEMA`].
    pub schema: String,
    pub meta: Meta,
    /// What every cell shows, when [`Layer::Grid`] was asked for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid: Option<Grid>,
}

impl Capture {
    /// The capture as compact JSON, its canonical form: the same capture
    /// always gives the same bytes.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("a capture always serializes")
    }

    /// The capture as indented JSON, for files a person reads.
    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).expect("a capture always serializes")
    }
}

/// What a capture was taken under.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Meta {
    /// The `rdom-capture` version that took it.
    pub rdom_capture: String,
    /// The virtual clock, in milliseconds since the session opened.
    pub time_ms: u64,
    /// The environment the app ran in.
    pub env: Environment,
    /// The fixture the app was opened with.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixture: Option<String>,
    /// The grid's hash ([`grid_hash`]), always present.
    pub grid_hash: String,
}

/// A layer a capture can include.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Layer {
    /// What every cell shows.
    Grid,
}
