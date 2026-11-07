//! The look checks about layout: truncation, occlusion, alignment, rhythm.

use std::collections::{BTreeMap, HashMap};

use rdom_tui::capture::PaintPart;
use rdom_tui::core_api::{NodeId, NodeType};
use serde_json::json;

use crate::audit::frame::{Cells, Frame, rects};
use crate::audit::{Check, Checked, DesignSystem, Finding, Ran, Severity};

/// Whether what element `id` cuts is a scroll away: it or an ancestor
/// clips with `overflow: scroll` or `auto`, which a user can scroll
/// (`hidden` clips for good, though scripts may scroll it).
fn scrolled(f: &Frame<'_>, id: NodeId) -> bool {
    use rdom_style::layout::Overflow;
    use rdom_tui::node::TuiNodeExt;
    let dom = f.app.dom();
    let mut cur = Some(id);
    while let Some(p) = cur {
        let scrolls = dom.node(p).computed().is_some_and(|c| {
            [c.overflow_x, c.overflow_y]
                .iter()
                .any(|o| matches!(o, Overflow::Scroll | Overflow::Auto))
        });
        if scrolls {
            return true;
        }
        cur = dom.node(p).parent_node().map(|n| n.id());
    }
    false
}

/// Text cut without an ellipsis (an element shows fewer of its own text's
/// cells than the text needs, unless it or an ancestor scrolls), and text
/// in a zero-size box.
pub(in crate::audit) fn truncation(f: &Frame<'_>) -> Checked {
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
                check: Check::Truncation,
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
            check: Check::Truncation,
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
pub(in crate::audit) fn occlusion(f: &Frame<'_>) -> Checked {
    let area = f.app.terminal().buffer().area;
    let trace = f.app.trace_paint(area);
    let mut covered: BTreeMap<(NodeId, Option<NodeId>), Cells> = BTreeMap::new();
    for y in 0..area.height {
        for x in 0..area.width {
            let Some(top) = trace.painter_at(x, y) else {
                continue;
            };
            // Overlays that belong to text — its caret, its ellipsis, its
            // selection or highlight — and paint by the text's own
            // ancestors (a container's ellipsis over a child's text)
            // cover nothing.
            if matches!(
                top.part,
                PaintPart::Caret
                    | PaintPart::TextOverflow
                    | PaintPart::Selection
                    | PaintPart::Highlight
            ) {
                continue;
            }
            let text_by = trace
                .ops_at(x, y)
                .filter(|op| op.part == PaintPart::Text && !op.cell.symbol().trim().is_empty())
                .filter_map(|op| op.node)
                .find(|n| Some(*n) != top.node && !top.node.is_some_and(|t| ancestor_of(f, t, *n)));
            if let Some(a) = text_by {
                covered.entry((a, top.node)).or_default().push((x, y));
            }
        }
    }
    let dom = f.app.dom();
    Ok(covered
        .into_iter()
        .map(|((a, b), cells)| Finding {
            check: Check::Occlusion,
            // A top-layer cover (a dialog, a popover, a picker) is meant to.
            severity: if b.is_some_and(|b| top_layer(dom, b)) {
                Severity::Info
            } else {
                Severity::Warning
            },
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

/// Whether `a` is an ancestor of `b`.
fn ancestor_of(f: &Frame<'_>, a: NodeId, b: NodeId) -> bool {
    let dom = f.app.dom();
    let mut cur = dom.node(b).parent_node().map(|p| p.id());
    while let Some(p) = cur {
        if p == a {
            return true;
        }
        cur = dom.node(p).parent_node().map(|n| n.id());
    }
    false
}

/// Whether `id` or an ancestor is in the top layer.
fn top_layer(dom: &rdom_tui::TuiDom, id: NodeId) -> bool {
    let mut cur = Some(id);
    while let Some(p) = cur {
        if dom.is_in_top_layer(p) {
            return true;
        }
        cur = dom.node(p).parent_node().map(|n| n.id());
    }
    false
}

/// Siblings stacked one above the other whose left or right edges miss a
/// shared column by one cell.
pub(in crate::audit) fn alignment(f: &Frame<'_>) -> Result<Ran, String> {
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
                            check: Check::Alignment,
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
    let total = out.len();
    out.truncate(32);
    Ok(Ran {
        partial: (total > 32).then(|| format!("{total} misalignments; the first 32 are listed")),
        findings: out,
    })
}

/// Padding off the declared spacing scale.
pub(in crate::audit) fn rhythm(f: &Frame<'_>, d: &DesignSystem) -> Result<Ran, String> {
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
    let findings: Vec<Finding> = off
        .into_iter()
        .map(|(v, nodes)| Finding {
            check: Check::Rhythm,
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
        .collect();
    Ok(Ran::from(findings))
}
