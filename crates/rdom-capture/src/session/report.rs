//! The session's `report`: the frame as the static HTML inspector
//! (`views::html`), with every element's explanation precomputed — the
//! same answers `explain` gives — so the page needs no server.

use std::collections::HashMap;

use rdom_tui::core_api::NodeId;

use super::{CaptureRequest, Session, SessionError};
use crate::app::CaptureApp;
use crate::capture::{Layer, node_key};
use crate::profile::Profile;
use crate::query::Detail;
use crate::query::explain;
use crate::raster::{Raster, render};
use crate::views::html::{Report, page};

impl<A: CaptureApp> Session<A> {
    /// The current frame as one self-contained HTML page: the image drawn
    /// under `profile` at `scale`, and an inspector over it — each cell's
    /// colors and the element whose paint it shows, that element's boxes,
    /// and how each of its styles came to be. The same frame gives the
    /// same bytes.
    pub fn report(&self, profile: &Profile, scale: u32) -> Result<String, SessionError> {
        let capture = self.capture(&CaptureRequest::with_layers([
            Layer::Grid,
            Layer::Owners,
            Layer::Boxes,
            Layer::Nodes,
        ]))?;
        let o = self.open.as_ref().ok_or(SessionError::NotOpen)?;
        let (grid, owners) = (
            capture.grid.as_ref().expect("asked for"),
            capture.owners.as_ref().expect("asked for"),
        );
        let (nodes, boxes) = (
            capture.nodes.as_ref().expect("asked for"),
            capture.boxes.as_ref().expect("asked for"),
        );
        let scale = scale.max(1);
        let image = render(grid, Raster::new(profile, scale));
        let dom = o.app.dom();
        let ids: HashMap<String, NodeId> = std::iter::once(dom.root())
            .chain(dom.descendants(dom.root()))
            .map(|n| (node_key(n), n))
            .collect();
        let explain = nodes
            .iter()
            .filter_map(|(key, _)| {
                let id = *ids.get(key)?;
                let e = explain::element(&o.app, id, &[], None, Detail::Summary);
                Some((key.to_string(), serde_json::to_value(e).ok()?))
            })
            .collect();
        let report = Report {
            title: match &o.fixture {
                Some(f) => format!("rdom-capture — {f}"),
                None => "rdom-capture".to_string(),
            },
            meta: &capture.meta,
            profile: &profile.name,
            scale,
            cell: [
                u32::from(profile.cell[0]) * scale,
                u32::from(profile.cell[1]) * scale,
            ],
            png: crate::views::base64::base64(&image.png()),
            width: image.width,
            height: image.height,
            missing: image.missing.iter().map(char::to_string).collect(),
            grid,
            owners,
            nodes: nodes.iter().collect(),
            boxes: boxes.iter().collect(),
            explain,
        };
        Ok(page(&report))
    }
}
