//! The look checks about color: contrast, palette, depth-loss, profile.

use std::collections::BTreeMap;

use rdom_tui::core_api::NodeId;
use rdom_tui::{Color, ColorDepth};
use serde_json::json;

use super::{hex, is_block, is_text, round};
use crate::audit::color::{apca_lc, delta_e, wcag_ratio};
use crate::audit::frame::{Cells, Frame, rects};
use crate::audit::{Check, Checked, DesignSystem, Finding, Ran, Severity};
use crate::capture::{Attr, ColorValue};
use crate::env::Depth;
use crate::profile::Rgb;

/// Text below the design's contrast ratio (WCAG 2; 4.5 by default), and
/// other glyphs — borders, marks — below 3 (WCAG 1.4.11), as the profile
/// shows them after the color depth. APCA's Lc is reported beside it.
pub(in crate::audit) fn contrast(f: &Frame<'_>, d: &DesignSystem) -> Checked {
    let mut groups: BTreeMap<(Option<NodeId>, Rgb, Rgb, bool), Cells> = BTreeMap::new();
    let dom = f.app.dom();
    // WCAG 1.4.3 exempts text of an inactive control.
    let disabled = |id: NodeId| dom.matches(id, ":disabled").unwrap_or(false);
    for (x, y, c) in f.each() {
        if !c.inked() || is_block(&c.glyph) || c.owner.is_some_and(disabled) {
            continue;
        }
        groups
            .entry((c.owner, c.fg, c.bg, is_text(&c.part, &c.glyph)))
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
            check: Check::Contrast,
            // Text is WCAG's requirement; graphics a recommendation here.
            severity: if text { Severity::Error } else { Severity::Warning },
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

/// Near-duplicate colors: shown colors closer than ΔE_ok 0.02, yet not
/// the same.
pub(in crate::audit) fn palette(f: &Frame<'_>) -> Result<Ran, String> {
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
    // The most used colors first: past 256 (a gradient), the rest are
    // left out, and said so.
    let mut colors: Vec<(&Rgb, &(usize, Vec<NodeId>))> = uses.iter().collect();
    colors.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(a.0.cmp(b.0)));
    let total = colors.len();
    colors.truncate(256);
    let mut out = Vec::new();
    for (i, (a, (na, oa))) in colors.iter().enumerate() {
        for (b, (nb, ob)) in &colors[i + 1..] {
            let de = delta_e(**a, **b);
            if de < 0.02 {
                out.push(Finding {
                    check: Check::Palette,
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
    Ok(Ran {
        findings: out,
        partial: (total > 256).then(|| format!("{total} colors; the 256 most used were compared")),
    })
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
pub(in crate::audit) fn depth_loss(f: &Frame<'_>) -> Checked {
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
            check: Check::DepthLoss,
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
pub(in crate::audit) fn profile(f: &Frame<'_>) -> Checked {
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
            let outside = |n: &&crate::audit::frame::Cell| n.owner != c.owner && n.inked();
            let left = row[..x].iter().rev().find(outside);
            let right = row[x + 1..].iter().find(outside);
            let near: Vec<&crate::audit::frame::Cell> = left.into_iter().chain(right).collect();
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
            check: Check::Profile,
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
