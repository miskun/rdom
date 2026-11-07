//! `query`: reverse lookups over the current frame — which elements a
//! selector matches, which cells and elements use a color, what is painted
//! in a rectangle, where a text is on screen.

use rdom_tui::core_api::NodeId;
use rdom_tui::render::TestBackend;
use rdom_tui::{App, Rect};
use serde::Serialize;

use crate::capture::{ColorValue, Grid, Owners, node_key, nodes};

/// One element a query found.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Found {
    pub node: String,
    pub path: String,
    /// Its border box.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rect: Option<[i32; 4]>,
    /// What it was found by: `fg`, `bg`, the part of its paint, the text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    /// How many cells.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cells: Option<u32>,
    /// The first matching cell, `[x, y]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<[u16; 2]>,
}

fn found(app: &App<TestBackend>, id: NodeId) -> Found {
    let dom = app.dom();
    Found {
        node: node_key(id),
        path: nodes::node_info(dom, id).path,
        rect: rdom_tui::capture::box_model(dom, id).map(|b| {
            let r = b.border;
            [r.x, r.y, i32::from(r.width), i32::from(r.height)]
        }),
        by: None,
        cells: None,
        at: None,
    }
}

/// The elements `selector` matches, in document order.
pub(crate) fn by_selector(app: &App<TestBackend>, selector: &str) -> Result<Vec<Found>, String> {
    let dom = app.dom();
    let ids = dom
        .query_selector_all_in(dom.root(), selector)
        .map_err(|e| format!("invalid selector {selector:?}: {e:?}"))?;
    Ok(ids.into_iter().map(|id| found(app, id)).collect())
}

/// Each cell of `grid` with its owner run, row by row.
fn cells<'a>(
    grid: &'a Grid,
    owners: &'a Owners,
) -> impl Iterator<
    Item = (
        u16,
        u16,
        &'a str,
        ColorValue,
        ColorValue,
        Option<&'a str>,
        &'a str,
    ),
> {
    grid.rows
        .iter()
        .zip(&owners.rows)
        .enumerate()
        .flat_map(|(y, (runs, owned))| {
            runs.iter().flat_map(move |run| {
                let (fg, bg) = run.shown_colors();
                run.cells().into_iter().enumerate().map(move |(i, glyph)| {
                    let x = run.x + i as u16;
                    let owner = owned.iter().find(|o| o.x <= x && x < o.x + o.w);
                    (
                        x,
                        y as u16,
                        glyph,
                        fg,
                        bg,
                        owner.and_then(|o| o.node.as_deref()),
                        owner.map_or("none", |o| o.part.as_str()),
                    )
                })
            })
        })
}

/// Tally `(node, by)` hits with their first cell.
fn tally(
    hits: impl Iterator<Item = (String, String, u16, u16)>,
) -> Vec<(String, String, u32, [u16; 2])> {
    let mut out: Vec<(String, String, u32, [u16; 2])> = Vec::new();
    for (node, by, x, y) in hits {
        match out.iter_mut().find(|(n, b, _, _)| *n == node && *b == by) {
            Some(e) => e.2 += 1,
            None => out.push((node, by, 1, [x, y])),
        }
    }
    out
}

fn resolve(app: &App<TestBackend>, key: &str) -> Option<NodeId> {
    let dom = app.dom();
    dom.descendants(dom.root()).find(|&n| node_key(n) == key)
}

fn results(app: &App<TestBackend>, tallied: Vec<(String, String, u32, [u16; 2])>) -> Vec<Found> {
    tallied
        .into_iter()
        .map(|(node, by, cells, at)| {
            let mut f = match resolve(app, &node) {
                Some(id) => found(app, id),
                None => Found {
                    node: node.clone(),
                    path: String::new(),
                    rect: None,
                    by: None,
                    cells: None,
                    at: None,
                },
            };
            f.by = Some(by);
            f.cells = Some(cells);
            f.at = Some(at);
            f
        })
        .collect()
}

/// The elements that paint `color` as a glyph or background color (as
/// shown, after the color depth), with how many cells.
pub(crate) fn by_color(
    app: &App<TestBackend>,
    grid: &Grid,
    owners: &Owners,
    color: ColorValue,
) -> Vec<Found> {
    let hits = cells(grid, owners).flat_map(move |(x, y, glyph, fg, bg, node, _)| {
        let node = node.unwrap_or("").to_string();
        let mut v = Vec::new();
        if fg == color && glyph.trim() != "" {
            v.push((node.clone(), "fg".to_string(), x, y));
        }
        if bg == color {
            v.push((node, "bg".to_string(), x, y));
        }
        v
    });
    results(app, tally(hits))
}

/// What is painted inside `rect`: each element and part, with its cells.
pub(crate) fn in_rect(
    app: &App<TestBackend>,
    grid: &Grid,
    owners: &Owners,
    rect: Rect,
) -> Vec<Found> {
    let hits = cells(grid, owners)
        .filter(move |(x, y, ..)| rect.contains(*x, *y))
        .filter_map(|(x, y, _, _, _, node, part)| {
            node.map(|n| (n.to_string(), part.to_string(), x, y))
        });
    results(app, tally(hits))
}

/// Where `text` is on screen: each row it starts in, with the element that
/// painted its first cell.
pub(crate) fn by_text(
    app: &App<TestBackend>,
    grid: &Grid,
    owners: &Owners,
    text: &str,
) -> Vec<Found> {
    let mut out = Vec::new();
    if text.is_empty() {
        return out;
    }
    for y in 0..grid.height {
        let row: Vec<&str> = grid.rows[usize::from(y)]
            .iter()
            .flat_map(|r| r.cells())
            .collect();
        let joined: String = row.concat();
        let mut from = 0;
        while let Some(pos) = joined[from..].find(text) {
            let byte = from + pos;
            // The cell the match starts in.
            let mut acc = 0;
            let x = row
                .iter()
                .position(|g| {
                    let start = acc;
                    acc += g.len();
                    start >= byte
                })
                .unwrap_or(0) as u16;
            let owner = owners.rows[usize::from(y)]
                .iter()
                .find(|o| o.x <= x && x < o.x + o.w);
            let mut f = match owner
                .and_then(|o| o.node.as_deref())
                .and_then(|k| resolve(app, k))
            {
                Some(id) => found(app, id),
                None => Found {
                    node: String::new(),
                    path: String::new(),
                    rect: None,
                    by: None,
                    cells: None,
                    at: None,
                },
            };
            f.by = owner.map(|o| o.part.clone());
            f.at = Some([x, y]);
            out.push(f);
            from = byte + text.len().max(1);
        }
    }
    out
}
