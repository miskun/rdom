//! The session's `report`: the frame as the static HTML inspector
//! (`views::html`), with every element's explanation precomputed — the
//! same answers `explain` gives — so the page needs no server.

use std::collections::HashMap;

use rdom_tui::core_api::NodeId;

use super::{CaptureRequest, Session, SessionError};
use crate::app::CaptureApp;
use crate::capture::{Layer, node_key};
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
    ///
    /// Refused ([`SessionError::InvalidInput`]) when the image would be
    /// past the rasterizer's limits.
    pub fn report(&self, how: Raster<'_>) -> Result<String, SessionError> {
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
        let image = render(grid, how).map_err(|e| SessionError::InvalidInput(e.to_string()))?;
        let profile = how.profile();
        let dom = o.app.dom();
        let ids: HashMap<String, NodeId> = std::iter::once(dom.root())
            .chain(dom.descendants(dom.root()))
            .map(|n| (node_key(n), n))
            .collect();
        let identities = crate::capture::identity::identities(dom);
        let explain = nodes
            .iter()
            .map(|(key, _)| {
                let id = ids[key];
                let e = explain::element(&o.app, id, &[], None, Detail::Summary, &identities);
                let e = serde_json::to_value(e).expect("an explanation serializes");
                (key.to_string(), e)
            })
            .collect();
        let report = Report {
            title: match &o.fixture {
                Some(f) => format!("rdom-capture — {f}"),
                None => "rdom-capture".to_string(),
            },
            meta: &capture.meta,
            profile: &profile.name,
            scale: how.scale(),
            cell: how.cell_px(),
            png: crate::views::base64::base64(&image.png()),
            width: image.width(),
            height: image.height(),
            missing: image.missing().iter().map(char::to_string).collect(),
            palette: (0..=255u8)
                .map(|n| hex(profile.rgb(crate::capture::ColorValue::Ansi(n), false, false)))
                .collect(),
            default_fg: hex(profile.fg),
            default_bg: hex(profile.bg),
            grid,
            owners,
            nodes: nodes.iter().collect(),
            boxes: boxes.iter().collect(),
            explain,
        };
        Ok(page(&report))
    }
}

/// `c` as `#rrggbb`.
fn hex([r, g, b]: crate::profile::Rgb) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}
