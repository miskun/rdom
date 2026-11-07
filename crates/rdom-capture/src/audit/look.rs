//! The look checks (`specs/CAPTURE.md` §Audits, Look).

use std::collections::{BTreeMap, HashMap};

use rdom_tui::capture::PaintPart;
use rdom_tui::core_api::{NodeId, NodeType};
use rdom_tui::{Color, ColorDepth};
use serde_json::json;

use super::color::{apca_lc, delta_e, wcag_ratio};
use super::frame::{Cells, Frame, rects};
use super::{Checked, DesignSystem, Finding, Severity};
use crate::capture::{Attr, ColorValue};
use crate::env::Depth;
use crate::profile::Rgb;

fn hex([r, g, b]: Rgb) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn round(v: f64, places: i32) -> f64 {
    let p = 10f64.powi(places);
    (v * p).round() / p
}

/// Whether `part` is text (an element's or a pseudo-element's).
fn is_text(part: &str) -> bool {
    part == "text" || part.starts_with("pseudo:")
}

/// Block elements and shades: graphics whose background never shows.
fn is_block(glyph: &str) -> bool {
    glyph
        .chars()
        .next()
        .is_some_and(|c| ('\u{2580}'..='\u{259F}').contains(&c))
}

/// Text below the design's contrast ratio (WCAG 2; 4.5 by default), and
/// other glyphs — borders, marks — below 3 (WCAG 1.4.11), as the profile
/// shows them after the color depth. APCA's Lc is reported beside it.
pub(super) fn contrast(f: &Frame<'_>, d: &DesignSystem) -> Checked {
    let mut groups: BTreeMap<(Option<NodeId>, Rgb, Rgb, bool), Cells> = BTreeMap::new();
    for (x, y, c) in f.each() {
        if !c.inked() || is_block(&c.glyph) {
            continue;
        }
        groups
            .entry((c.owner, c.fg, c.bg, is_text(&c.part)))
            .or_default()
            .push((x, y));
    }
    let mut out = Vec::new();
    for ((owner, fg, bg, text), cells) in groups {
        let ratio = wcag_ratio(fg, bg);
        let min = if text { d.min_contrast } else { 3.0 };
        if ratio >= min {
            continue;
        }
        out.push(Finding {
            check: "contrast",
            severity: Severity::Error,
            message: format!(
                "{} {} on {}: {:.2}:1, under {min}:1",
                if text { "text" } else { "a glyph" },
                hex(fg),
                hex(bg),
                ratio
            ),
            nodes: owner.map(|n| f.key(n)).into_iter().collect(),
            cells: rects(&cells),
            inputs: json!({"fg": hex(fg), "bg": hex(bg), "ratio": round(ratio, 2), "apca_lc": round(apca_lc(fg, bg), 1), "cells": cells.len()}),
            threshold: json!({"wcag": min}),
        });
    }
    Ok(out)
}

/// Whether element `id` sits in a scroll container (what it cuts is
/// scrolled, not lost).
fn scrolled(f: &Frame<'_>, id: NodeId) -> bool {
    let dom = f.app.dom();
    let mut cur = dom.node(id).parent_node().map(|p| p.id());
    while let Some(p) = cur {
        if f.boxes.get(&p).is_some_and(|b| b.scroll.is_some()) {
            return true;
        }
        cur = dom.node(p).parent_node().map(|n| n.id());
    }
    false
}

/// Text cut without an ellipsis (an element shows fewer of its own text's
/// cells than the text needs, outside every scroll container), and text in
/// a zero-size box.
pub(super) fn truncation(f: &Frame<'_>) -> Checked {
    let dom = f.app.dom();
    // Each element's own text: the non-blank cells it needs.
    let mut needed: HashMap<NodeId, usize> = HashMap::new();
    for id in dom.descendants(dom.root()) {
        if dom.node(id).node_type() != NodeType::Text {
            continue;
        }
        let Some(parent) = dom.node(id).parent_node().map(|p| p.id()) else {
            continue;
        };
        let text = dom.node(id).node_value().unwrap_or_default();
        let w: usize = text
            .chars()
            .filter(|c| !c.is_whitespace())
            .map(|c| unicode_width::UnicodeWidthChar::width(c).unwrap_or(0))
            .sum();
        *needed.entry(parent).or_default() += w;
    }
    let mut shown: HashMap<NodeId, (usize, bool, Cells)> = HashMap::new();
    for (x, y, c) in f.each() {
        if let Some(o) = c.owner
            && c.part == "text"
            && c.inked()
        {
            let e = shown.entry(o).or_default();
            e.0 += unicode_width::UnicodeWidthStr::width(c.glyph.as_str()).max(1);
            e.2.push((x, y));
        }
    }
    let mut out = Vec::new();
    let mut ids: Vec<_> = needed.into_iter().filter(|(_, w)| *w > 0).collect();
    ids.sort();
    for (id, need) in ids {
        let Some(b) = f.boxes.get(&id) else {
            continue;
        };
        if b.border[2] <= 0 || b.border[3] <= 0 {
            out.push(Finding {
                check: "truncation",
                severity: Severity::Warning,
                message: "text in a zero-size box: none of it shows".into(),
                nodes: vec![f.key(id)],
                cells: Vec::new(),
                inputs: json!({"box": b.border, "text_cells": need}),
                threshold: serde_json::Value::Null,
            });
            continue;
        }
        let (got, _, cells) = shown.remove(&id).unwrap_or_default();
        // An ellipsis anywhere in its box — whichever part of the paint
        // drew it — says the cut is meant.
        let [bx, by, bw, bh] = b.border;
        let ellipsis = (by.max(0)..by + bh).any(|y| {
            f.cells.get(y as usize).is_some_and(|row| {
                (bx.max(0)..bx + bw).any(|x| row.get(x as usize).is_some_and(|c| c.glyph == "…"))
            })
        });
        if got == 0 || got >= need || ellipsis || scrolled(f, id) {
            continue;
        }
        out.push(Finding {
            check: "truncation",
            severity: Severity::Warning,
            message: format!("{got} of {need} text cells show, cut without an ellipsis"),
            nodes: vec![f.key(id)],
            cells: rects(&cells),
            inputs: json!({"shown": got, "needed": need}),
            threshold: serde_json::Value::Null,
        });
    }
    Ok(out)
}

/// Text painted and then covered by another element's paint.
pub(super) fn occlusion(f: &Frame<'_>) -> Checked {
    let area = f.app.terminal().buffer().area;
    let trace = f.app.trace_paint(area);
    let mut covered: BTreeMap<(NodeId, Option<NodeId>), Cells> = BTreeMap::new();
    for y in 0..area.height {
        for x in 0..area.width {
            let Some(top) = trace.painter_at(x, y) else {
                continue;
            };
            let text_by = trace
                .ops_at(x, y)
                .filter(|op| op.part == PaintPart::Text && !op.cell.symbol().trim().is_empty())
                .filter_map(|op| op.node)
                .find(|n| Some(*n) != top.node);
            if let Some(a) = text_by {
                covered.entry((a, top.node)).or_default().push((x, y));
            }
        }
    }
    Ok(covered
        .into_iter()
        .map(|((a, b), cells)| Finding {
            check: "occlusion",
            severity: Severity::Warning,
            message: format!(
                "{} text cells painted and then covered{}",
                cells.len(),
                b.map(|b| format!(" by {}", f.key(b))).unwrap_or_default()
            ),
            nodes: std::iter::once(f.key(a))
                .chain(b.map(|b| f.key(b)))
                .collect(),
            cells: rects(&cells),
            inputs: json!({"cells": cells.len()}),
            threshold: serde_json::Value::Null,
        })
        .collect())
}

/// Siblings stacked one above the other whose left or right edges miss a
/// shared column by one cell.
pub(super) fn alignment(f: &Frame<'_>) -> Checked {
    let dom = f.app.dom();
    let mut out = Vec::new();
    for parent in std::iter::once(dom.root()).chain(dom.descendants(dom.root())) {
        let kids: Vec<NodeId> = dom.node(parent).children().map(|c| c.id()).collect();
        // A paragraph's inline boxes wrap; only block-like stacks count.
        let texty = kids.iter().any(|&k| {
            dom.node(k).node_type() == NodeType::Text
                && !dom
                    .node(k)
                    .node_value()
                    .unwrap_or_default()
                    .trim()
                    .is_empty()
        });
        if texty {
            continue;
        }
        let boxed: Vec<(NodeId, [i32; 4])> = kids
            .iter()
            .filter_map(|&k| Some((k, f.boxes.get(&k)?.border)))
            .filter(|(_, b)| b[2] > 0 && b[3] > 0)
            .collect();
        for (i, (a, ra)) in boxed.iter().enumerate() {
            for (b, rb) in &boxed[i + 1..] {
                let stacked = ra[1] + ra[3] <= rb[1] || rb[1] + rb[3] <= ra[1];
                if !stacked {
                    continue;
                }
                for (edge, ea, eb) in [
                    ("left", ra[0], rb[0]),
                    ("right", ra[0] + ra[2], rb[0] + rb[2]),
                ] {
                    if (ea - eb).abs() == 1 {
                        out.push(Finding {
                            check: "alignment",
                            severity: Severity::Info,
                            message: format!("{edge} edges one column apart ({ea} and {eb})"),
                            nodes: vec![f.key(*a), f.key(*b)],
                            cells: Vec::new(),
                            inputs: json!({"edge": edge, "a": ra, "b": rb}),
                            threshold: json!({"columns": 1}),
                        });
                    }
                }
            }
        }
    }
    out.truncate(32);
    Ok(out)
}

/// Near-duplicate colors: shown colors closer than ΔE_ok 0.02, yet not
/// the same.
pub(super) fn palette(f: &Frame<'_>) -> Checked {
    let mut uses: BTreeMap<Rgb, (usize, Vec<NodeId>)> = BTreeMap::new();
    let mut note = |c: Rgb, owner: Option<NodeId>| {
        let e = uses.entry(c).or_default();
        e.0 += 1;
        if let Some(o) = owner
            && e.1.len() < 4
            && !e.1.contains(&o)
        {
            e.1.push(o);
        }
    };
    for (_, _, c) in f.each() {
        if c.inked() {
            note(c.fg, c.owner);
        }
        note(c.bg, c.owner);
    }
    let colors: Vec<(&Rgb, &(usize, Vec<NodeId>))> = uses.iter().take(256).collect();
    let mut out = Vec::new();
    for (i, (a, (na, oa))) in colors.iter().enumerate() {
        for (b, (nb, ob)) in &colors[i + 1..] {
            let de = delta_e(**a, **b);
            if de < 0.02 {
                out.push(Finding {
                    check: "palette",
                    severity: Severity::Warning,
                    message: format!("{} and {} are near-duplicates (ΔE {:.3})", hex(**a), hex(**b), de),
                    nodes: oa.iter().chain(ob.iter()).map(|n| f.key(*n)).collect(),
                    cells: Vec::new(),
                    inputs: json!({"a": hex(**a), "b": hex(**b), "delta_e": round(de, 4), "uses": [na, nb]}),
                    threshold: json!({"delta_e": 0.02}),
                });
            }
        }
    }
    Ok(out)
}

/// `c` as the runtime's color.
fn runtime_color(c: ColorValue) -> Color {
    match c {
        ColorValue::Default => Color::Reset,
        ColorValue::Rgb(r, g, b) => Color::Rgb(r, g, b),
        ColorValue::Rgba(r, g, b, a) => Color::Rgba(r, g, b, a),
        ColorValue::Ansi(n) => Color::Indexed(n),
    }
}

/// Text that stays readable at the environment's color depth but vanishes
/// at a lower one (its colors collapse together).
pub(super) fn depth_loss(f: &Frame<'_>) -> Checked {
    let lower: &[(ColorDepth, &str)] = match f.env.color_depth {
        Depth::TrueColor => &[(ColorDepth::Ansi256, "256"), (ColorDepth::Ansi16, "16")],
        Depth::Ansi256 => &[(ColorDepth::Ansi16, "16")],
        _ => {
            return Err(
                "the environment already shows 16 colors or none: nothing lower to lose to".into(),
            );
        }
    };
    let mut lost: BTreeMap<(Option<NodeId>, &str, Rgb, Rgb), Cells> = BTreeMap::new();
    for (x, y, c) in f.each() {
        if !c.inked() || wcag_ratio(c.fg, c.bg) < 1.5 {
            continue;
        }
        let bold = c.mods.contains(&Attr::Bold);
        for (depth, name) in lower {
            let q = |v: ColorValue| ColorValue::from(depth.quantize(runtime_color(v)));
            let fg = f.profile.rgb(q(c.painted.0), true, bold);
            let bg = f.profile.rgb(q(c.painted.1), false, false);
            if wcag_ratio(fg, bg) < 1.2 {
                lost.entry((c.owner, name, fg, bg))
                    .or_default()
                    .push((x, y));
                break;
            }
        }
    }
    Ok(lost
        .into_iter()
        .map(|((owner, depth, fg, bg), cells)| Finding {
            check: "depth-loss",
            severity: Severity::Warning,
            message: format!(
                "{} glyphs vanish at {depth} colors: they become {} on {}",
                cells.len(),
                hex(fg),
                hex(bg)
            ),
            nodes: owner.map(|n| f.key(n)).into_iter().collect(),
            cells: rects(&cells),
            inputs: json!({"depth": depth, "fg": hex(fg), "bg": hex(bg)}),
            threshold: json!({"wcag": 1.2}),
        })
        .collect())
}

/// Meaning carried only by an attribute the profile does not draw: an
/// element's cells set apart from their neighbours by nothing else.
pub(super) fn profile(f: &Frame<'_>) -> Checked {
    let p = f.profile;
    let lacks = |a: &Attr| match a {
        Attr::Italic => !p.italic,
        Attr::Strikethrough => !p.strikethrough,
        Attr::Overline => !p.overline,
        _ => false,
    };
    let mut groups: BTreeMap<(Option<NodeId>, String), Cells> = BTreeMap::new();
    for (y, row) in f.cells.iter().enumerate() {
        for (x, c) in row.iter().enumerate() {
            if !c.inked() {
                continue;
            }
            let Some(attr) = c.mods.iter().find(|a| lacks(a)) else {
                continue;
            };
            // Its nearest inked neighbours on the row outside the element,
            // one each way (a word's neighbours are spaces first).
            let outside = |n: &&super::frame::Cell| n.owner != c.owner && n.inked();
            let left = row[..x].iter().rev().find(outside);
            let right = row[x + 1..].iter().find(outside);
            let near: Vec<&super::frame::Cell> = left.into_iter().chain(right).collect();
            let alike = near
                .iter()
                .all(|n| n.fg == c.fg && n.bg == c.bg && !n.mods.contains(attr));
            if alike && !near.is_empty() {
                groups
                    .entry((c.owner, format!("{attr:?}").to_lowercase()))
                    .or_default()
                    .push((x as u16, y as u16));
            }
        }
    }
    Ok(groups
        .into_iter()
        .map(|((owner, attr), cells)| Finding {
            check: "profile",
            severity: Severity::Warning,
            message: format!(
                "set apart only by {attr}, which {} does not draw: there it looks like its neighbours",
                p.name
            ),
            nodes: owner.map(|n| f.key(n)).into_iter().collect(),
            cells: rects(&cells),
            inputs: json!({"attribute": attr, "profile": p.name}),
            threshold: serde_json::Value::Null,
        })
        .collect())
}

/// Padding off the declared spacing scale.
pub(super) fn rhythm(f: &Frame<'_>, d: &DesignSystem) -> Checked {
    if d.spacing.is_empty() {
        return Err(
            "no spacing scale declared (DesignSystem::spacing): rhythm is held to one".into(),
        );
    }
    let mut off: BTreeMap<i32, Vec<NodeId>> = BTreeMap::new();
    let mut ids: Vec<_> = f.boxes.keys().copied().collect();
    ids.sort();
    for id in ids {
        let b = &f.boxes[&id];
        let Some(c) = b.content else {
            continue;
        };
        let p = b.padding;
        for inset in [
            c[0] - p[0],
            c[1] - p[1],
            (p[0] + p[2]) - (c[0] + c[2]),
            (p[1] + p[3]) - (c[1] + c[3]),
        ] {
            if inset > 0 && !d.spacing.iter().any(|s| i32::from(*s) == inset) {
                let e = off.entry(inset).or_default();
                if !e.contains(&id) {
                    e.push(id);
                }
            }
        }
    }
    Ok(off
        .into_iter()
        .map(|(v, nodes)| Finding {
            check: "rhythm",
            severity: Severity::Info,
            message: format!(
                "padding of {v} cells is off the spacing scale, on {} elements",
                nodes.len()
            ),
            nodes: nodes.iter().take(8).map(|n| f.key(*n)).collect(),
            cells: Vec::new(),
            inputs: json!({"padding": v}),
            threshold: json!({"scale": d.spacing}),
        })
        .collect())
}
