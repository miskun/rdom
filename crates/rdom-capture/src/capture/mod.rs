//! The `Capture` contract (`specs/CAPTURE.md` §The `Capture` contract):
//! one versioned JSON document per moment of an app, the single source
//! every view, diff and query reads.
//!
//! A capture is layered. `meta` is always present and so is the grid
//! hash; the grid itself and the other layers are included when asked
//! ([`Layer`]). Whenever a layer names elements, `nodes` says what each
//! one is.

mod color;
mod grid;
mod hash;
pub(crate) mod identity;
mod node_map;
pub(crate) mod nodes;
pub(crate) mod owners;

pub use color::ColorValue;
pub(crate) use grid::split_cells as grid_cells;
pub use grid::{Attr, Grid, Run};
pub use hash::{fnv1a64, grid_hash};
pub use identity::By;
pub use node_map::NodeMap;
pub use nodes::{Boxes, NodeInfo, Scroll, node_key};
pub use owners::{OwnerRun, Owners};

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
    /// Which element's paint decided each cell ([`Layer::Owners`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owners: Option<Owners>,
    /// Every element's boxes, by node ([`Layer::Boxes`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boxes: Option<NodeMap<Boxes>>,
    /// What each element the other layers name is, by node — every
    /// element with [`Layer::Nodes`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nodes: Option<NodeMap<NodeInfo>>,
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
    /// The `rdom-tui` version that painted it.
    pub rdom_tui: String,
    /// The virtual clock, in milliseconds since the session opened.
    pub time_ms: u64,
    /// The environment the app ran in.
    pub env: Environment,
    /// The fixture the app was opened with.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixture: Option<String>,
    /// The grid's hash ([`grid_hash`]), always present.
    pub grid_hash: String,
    /// What the capture could not do exactly, each said plainly (a traced
    /// repaint that differs from the frame).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

/// A layer a capture can include.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Layer {
    /// What every cell shows.
    Grid,
    /// Which element's paint decided each cell (a traced repaint).
    Owners,
    /// Every element's boxes.
    Boxes,
    /// Every element, not only the ones the other layers name.
    Nodes,
}
