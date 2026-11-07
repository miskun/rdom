//! The tokens check (`specs/CAPTURE.md` §Audits): colors the frame shows
//! that are not the design system's — declared (`DesignSystem::colors`)
//! or, when none are, the custom properties on the root element that hold
//! a color — each with the nearest token.

use std::collections::BTreeMap;

use rdom_tui::core_api::{NodeId, NodeType};
use serde_json::json;

use super::color::delta_e;
use super::frame::{Cells, Frame, rects};
use super::{Checked, DesignSystem, Finding, Severity};
use crate::capture::ColorValue;
use crate::profile::Rgb;

fn hex([r, g, b]: Rgb) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// `value` as the color the profile shows.
fn shown(f: &Frame<'_>, value: &str, glyph: bool) -> Option<Rgb> {
    let c = rdom_style::parse_color(value)?;
    Some(f.profile.rgb(ColorValue::from(c), glyph, false))
}

/// The color tokens, by name, and where they came from.
fn tokens(f: &Frame<'_>, d: &DesignSystem) -> (Vec<(String, Rgb)>, &'static str) {
    if !d.colors.is_empty() {
        let list = d
            .colors
            .iter()
            .filter_map(|(n, v)| Some((n.clone(), shown(f, v, false)?)))
            .collect();
        return (list, "declared");
    }
    let dom = f.app.dom();
    let root: Option<NodeId> = dom
        .node(dom.root())
        .children()
        .map(|c| c.id())
        .find(|&c| dom.node(c).node_type() == NodeType::Element);
    // `:root` custom properties live in the sheet's root variables and
    // reach every element's computed `vars`: read the root element's.
    use rdom_tui::node::TuiNodeExt;
    let mut list: Vec<(String, Rgb)> = root
        .and_then(|r| dom.node(r).computed().map(|c| c.vars.clone()))
        .map(|vars| {
            vars.iter()
                .filter_map(|(n, v)| {
                    let name = if n.starts_with("--") {
                        n.clone()
                    } else {
                        format!("--{n}")
                    };
                    Some((name, shown(f, v.as_str(), false)?))
                })
                .collect()
        })
        .unwrap_or_default();
    list.sort();
    (list, "the root element's custom properties")
}

/// Colors shown that no token has.
pub(super) fn tokens_check(f: &Frame<'_>, d: &DesignSystem) -> Checked {
    let (tokens, from) = tokens(f, d);
    if tokens.is_empty() {
        return Err(
            "no color tokens: declare DesignSystem::colors, or custom properties holding colors on the root element"
                .into(),
        );
    }
    let mut off: BTreeMap<Rgb, (Cells, Vec<NodeId>)> = BTreeMap::new();
    for (x, y, c) in f.each() {
        let mut note = |rgb: Rgb| {
            if tokens.iter().any(|(_, t)| *t == rgb) {
                return;
            }
            let e = off.entry(rgb).or_default();
            e.0.push((x, y));
            if let Some(o) = c.owner
                && !e.1.contains(&o)
                && e.1.len() < 8
            {
                e.1.push(o);
            }
        };
        if c.inked() && c.shown.0 != ColorValue::Default {
            note(c.fg);
        }
        if c.shown.1 != ColorValue::Default {
            note(c.bg);
        }
    }
    Ok(off
        .into_iter()
        .map(|(rgb, (cells, nodes))| {
            let (name, near) = tokens
                .iter()
                .map(|(n, t)| (n, delta_e(rgb, *t)))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .expect("tokens is not empty");
            Finding {
                check: "tokens",
                severity: Severity::Warning,
                message: format!("{} is no token ({from}); the nearest is {name} (ΔE {near:.3})", hex(rgb)),
                nodes: nodes.iter().map(|n| f.key(*n)).collect(),
                cells: rects(&cells),
                inputs: json!({"color": hex(rgb), "nearest": name, "delta_e": (near * 1000.0).round() / 1000.0, "cells": cells.len()}),
                threshold: json!({"tokens": tokens.len()}),
            }
        })
        .collect())
}
