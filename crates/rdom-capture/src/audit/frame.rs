//! What every check reads of a frame, computed once: each cell as it is
//! painted and as the profile shows it, the element whose paint it is,
//! every element's boxes and identity.

use std::collections::HashMap;

use rdom_tui::App;
use rdom_tui::core_api::NodeId;
use rdom_tui::render::TestBackend;

use crate::app::CaptureApp;
use crate::capture::identity::{Identity, identities};
use crate::capture::{Attr, Boxes, ColorValue, Layer, node_key, nodes};
use crate::env::Environment;
use crate::profile::{Profile, Rgb};
use crate::session::{CaptureRequest, Session, SessionError};

/// One cell.
#[derive(Debug, Clone)]
pub(crate) struct Cell {
    /// The glyph (`" "` blank, `""` a wide glyph's trailing cell).
    pub glyph: String,
    /// The colors rdom painted, and the ones the terminal is sent.
    pub painted: (ColorValue, ColorValue),
    pub shown: (ColorValue, ColorValue),
    /// As the profile shows them.
    pub fg: Rgb,
    pub bg: Rgb,
    pub mods: Vec<Attr>,
    /// The element whose paint the cell shows, and which part.
    pub owner: Option<NodeId>,
    pub part: String,
}

impl Cell {
    /// Whether it shows a glyph (not blank, not hidden).
    pub fn inked(&self) -> bool {
        !self.glyph.trim().is_empty() && !self.mods.contains(&Attr::Hidden)
    }
}

pub(crate) struct Frame<'a> {
    pub app: &'a App<TestBackend>,
    pub profile: &'a Profile,
    pub env: Environment,
    /// Row by row.
    pub cells: Vec<Vec<Cell>>,
    pub boxes: HashMap<NodeId, Boxes>,
    pub ids: HashMap<NodeId, Identity>,
}

impl<'a> Frame<'a> {
    pub fn of<A: CaptureApp>(
        session: &'a Session<A>,
        profile: &'a Profile,
    ) -> Result<Frame<'a>, SessionError> {
        let capture =
            session.capture(&CaptureRequest::with_layers([Layer::Grid, Layer::Owners]))?;
        let app = session.app()?;
        let dom = app.dom();
        let by_key: HashMap<String, NodeId> = std::iter::once(dom.root())
            .chain(dom.descendants(dom.root()))
            .map(|n| (node_key(n), n))
            .collect();
        let grid = capture.grid.expect("asked for");
        let owners = capture.owners.expect("asked for");
        let cells = grid
            .rows
            .iter()
            .zip(&owners.rows)
            .map(|(runs, owner_runs)| {
                let mut row: Vec<Cell> = runs
                    .iter()
                    .flat_map(|r| {
                        let shown = r.shown_colors();
                        let bold = r.mods.contains(&Attr::Bold);
                        r.cells().into_iter().map(move |g| Cell {
                            glyph: g.to_string(),
                            painted: (r.fg, r.bg),
                            shown,
                            fg: profile.rgb(shown.0, true, bold),
                            bg: profile.rgb(shown.1, false, false),
                            mods: r.mods.clone(),
                            owner: None,
                            part: String::new(),
                        })
                    })
                    .collect();
                for o in owner_runs {
                    for x in o.x..o.x + o.w {
                        if let Some(c) = row.get_mut(usize::from(x)) {
                            c.owner = o.node.as_ref().and_then(|k| by_key.get(k).copied());
                            c.part = o.part.clone();
                        }
                    }
                }
                row
            })
            .collect();
        Ok(Frame {
            app,
            profile,
            env: session.environment()?.clone(),
            cells,
            boxes: nodes::boxes(dom).into_iter().collect(),
            ids: identities(dom),
        })
    }

    /// Element `id`'s identity.
    pub fn key(&self, id: NodeId) -> String {
        self.ids
            .get(&id)
            .map_or_else(|| node_key(id), |i| i.key.clone())
    }

    /// Every cell with its position.
    pub fn each(&self) -> impl Iterator<Item = (u16, u16, &Cell)> {
        self.cells.iter().enumerate().flat_map(|(y, row)| {
            row.iter()
                .enumerate()
                .map(move |(x, c)| (x as u16, y as u16, c))
        })
    }
}

/// Cell positions, `(x, y)`.
pub(crate) type Cells = Vec<(u16, u16)>;

/// `cells` as rectangles, a row's runs of neighbours merged; at most 16.
pub(crate) fn rects(cells: &[(u16, u16)]) -> Vec<[u16; 4]> {
    let mut sorted = cells.to_vec();
    sorted.sort_by_key(|&(x, y)| (y, x));
    let mut out: Vec<[u16; 4]> = Vec::new();
    for (x, y) in sorted {
        match out.last_mut() {
            Some(r) if r[1] == y && r[0] + r[2] == x => r[2] += 1,
            _ => out.push([x, y, 1, 1]),
        }
    }
    out.truncate(16);
    out
}
