//! A diff of two moments (`specs/CAPTURE.md` §Session protocol, `diff`):
//! the cells that show something else, and the elements — matched across
//! the two by identity (`capture::identity`), never by node key — that
//! appeared, went, moved or resized, or whose declared styles changed.
//! Every match says which rule made it.

use std::collections::HashMap;

use serde::Serialize;

use crate::capture::{Attr, By, ColorValue, Grid};

/// What a diff compares of one moment: its grid, and its elements by
/// identity. Taken by [`Session::snapshot`](crate::Session::snapshot).
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub(crate) time_ms: u64,
    pub(crate) grid_hash: String,
    pub(crate) grid: Grid,
    /// In document order.
    pub(crate) elements: Vec<Element>,
}

impl Snapshot {
    /// The moment on the session's virtual clock, in milliseconds.
    pub fn time_ms(&self) -> u64 {
        self.time_ms
    }

    /// The grid hash of the moment.
    pub fn grid_hash(&self) -> &str {
        &self.grid_hash
    }
}

/// One element of a snapshot.
#[derive(Debug, Clone)]
pub(crate) struct Element {
    pub key: String,
    pub by: By,
    /// Its node key in its own capture.
    pub node: String,
    pub path: String,
    pub border: Option<[i32; 4]>,
    /// Its declared styles: each property's winning value, by name.
    pub styles: Vec<(String, String)>,
}

/// How two moments differ.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct Diff {
    /// The two moments' grid hashes; equal hashes, no change.
    pub from: String,
    pub to: String,
    /// How many cells show something else (glyph, colors, attributes).
    pub cells_changed: u32,
    /// The smallest rectangle holding them, `[x, y, width, height]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounds: Option<[u16; 4]>,
    /// The viewport size before and after, when it changed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resized: Option<[[u16; 2]; 2]>,
    /// Per changed row, the span from its first to its last changed cell.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<RowChange>,
    /// Elements only the second moment has.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub added: Vec<DiffNode>,
    /// Elements only the first moment has.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub removed: Vec<DiffNode>,
    /// Matched elements whose border box moved or resized.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub boxes: Vec<BoxChange>,
    /// Matched elements' declared styles that changed.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub styles: Vec<StyleChange>,
    /// How the elements in both were matched.
    pub matched: Matched,
}

/// A changed span of one row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct RowChange {
    pub y: u16,
    /// The span's first column.
    pub x: u16,
    /// Its glyphs before and after (a blank cell is a space).
    pub before: String,
    pub after: String,
}

/// An element, by identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct DiffNode {
    pub key: String,
    pub by: By,
    /// Its node key in the moment it is in.
    pub node: String,
    pub path: String,
}

/// A matched element's border box, before and after (`None`: no box).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct BoxChange {
    pub key: String,
    pub by: By,
    pub before: Option<[i32; 4]>,
    pub after: Option<[i32; 4]>,
}

/// A matched element's declared value of one property, before and after
/// (`None`: not declared).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct StyleChange {
    pub key: String,
    pub property: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

/// How many elements were matched by each rule.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct Matched {
    pub id: u32,
    pub data_key: u32,
    pub fingerprint: u32,
}

/// One cell as it shows: glyph, colors as sent, attributes, underline.
type Shown<'a> = (
    &'a str,
    ColorValue,
    ColorValue,
    &'a [Attr],
    Option<ColorValue>,
);

/// Every cell of `grid`, row by row.
fn cells(grid: &Grid) -> Vec<Vec<Shown<'_>>> {
    grid.rows
        .iter()
        .map(|runs| {
            runs.iter()
                .flat_map(|r| {
                    let (fg, bg) = r.shown_colors();
                    r.cells()
                        .into_iter()
                        .map(move |g| (g, fg, bg, r.mods.as_slice(), r.ul))
                })
                .collect()
        })
        .collect()
}

/// Cell `x` of row `r`, if the row has one.
fn at<'g>(r: Option<&Vec<Shown<'g>>>, x: u16) -> Option<Shown<'g>> {
    r.and_then(|r| r.get(usize::from(x)).copied())
}

/// How `a` and `b` differ.
pub fn diff(a: &Snapshot, b: &Snapshot) -> Diff {
    let mut out = Diff {
        from: a.grid_hash.clone(),
        to: b.grid_hash.clone(),
        ..Diff::default()
    };
    // Cells: the larger extent of the two, a cell only one has changed.
    let (ca, cb) = (cells(&a.grid), cells(&b.grid));
    if (a.grid.width, a.grid.height) != (b.grid.width, b.grid.height) {
        out.resized = Some([[a.grid.width, a.grid.height], [b.grid.width, b.grid.height]]);
    }
    let (w, h) = (
        a.grid.width.max(b.grid.width),
        a.grid.height.max(b.grid.height),
    );
    let (mut x0, mut y0, mut x1, mut y1) = (u16::MAX, u16::MAX, 0, 0);
    for y in 0..h {
        let (ra, rb) = (ca.get(usize::from(y)), cb.get(usize::from(y)));
        let changed: Vec<u16> = (0..w).filter(|&x| at(ra, x) != at(rb, x)).collect();
        let (Some(&first), Some(&last)) = (changed.first(), changed.last()) else {
            continue;
        };
        out.cells_changed += changed.len() as u32;
        (x0, y0, x1, y1) = (x0.min(first), y0.min(y), x1.max(last), y1.max(y));
        let text = |r| -> String {
            (first..=last)
                .map(|x| at(r, x).map_or(" ", |c| c.0))
                .collect()
        };
        out.rows.push(RowChange {
            y,
            x: first,
            before: text(ra),
            after: text(rb),
        });
    }
    if out.cells_changed > 0 {
        out.bounds = Some([x0, y0, x1 - x0 + 1, y1 - y0 + 1]);
    }
    // Elements, by identity.
    let by_key: HashMap<&str, &Element> = b.elements.iter().map(|e| (e.key.as_str(), e)).collect();
    let in_a: HashMap<&str, &Element> = a.elements.iter().map(|e| (e.key.as_str(), e)).collect();
    let node_ref = |e: &Element| DiffNode {
        key: e.key.clone(),
        by: e.by,
        node: e.node.clone(),
        path: e.path.clone(),
    };
    for ea in &a.elements {
        let Some(eb) = by_key.get(ea.key.as_str()) else {
            out.removed.push(node_ref(ea));
            continue;
        };
        match ea.by {
            By::Id => out.matched.id += 1,
            By::DataKey => out.matched.data_key += 1,
            _ => out.matched.fingerprint += 1,
        }
        if ea.border != eb.border {
            out.boxes.push(BoxChange {
                key: ea.key.clone(),
                by: ea.by,
                before: ea.border,
                after: eb.border,
            });
        }
        let (sa, sb): (HashMap<&str, &str>, HashMap<&str, &str>) = (
            ea.styles
                .iter()
                .map(|(p, v)| (p.as_str(), v.as_str()))
                .collect(),
            eb.styles
                .iter()
                .map(|(p, v)| (p.as_str(), v.as_str()))
                .collect(),
        );
        let mut props: Vec<&str> = sa.keys().chain(sb.keys()).copied().collect();
        props.sort_unstable();
        props.dedup();
        for p in props {
            let (before, after) = (sa.get(p), sb.get(p));
            if before != after {
                out.styles.push(StyleChange {
                    key: ea.key.clone(),
                    property: p.to_string(),
                    before: before.map(|v| (*v).to_string()),
                    after: after.map(|v| (*v).to_string()),
                });
            }
        }
    }
    out.added = b
        .elements
        .iter()
        .filter(|e| !in_a.contains_key(e.key.as_str()))
        .map(node_ref)
        .collect();
    out
}
