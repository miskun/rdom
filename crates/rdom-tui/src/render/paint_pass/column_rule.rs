//! Column rules (CSS Multi-column 1 §4): a line in the middle of the gap
//! between two adjacent column boxes that both hold content, as tall as
//! the columns, drawn just above the multi-column container's border and
//! below its content. In cells: one column of border glyphs — the rule's
//! `column-rule-style`, its weight by `column-rule-width` as a border's —
//! written as border contributions, so the joiner welds it into the
//! container's own border where it meets it (`┬`, `┴`).

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::LayoutRect;
use crate::render::buffer::{BorderContribution, BorderSide, DIR_N, DIR_S};
use crate::render::{Buffer, Rect};
use crate::style::ComputedStyle;
use rdom_style::layout::CornerStyle;

/// Draw `id`'s column rules into `clip`: `computed` its style, `priority`
/// its border contributions' (BORDER-MODEL-1), `outer` its border box.
pub(super) fn paint(
    dom: &Dom<TuiExt>,
    id: NodeId,
    buf: &mut Buffer,
    computed: &ComputedStyle,
    outer: LayoutRect,
    clip: Rect,
    priority: u64,
) {
    let m = &computed.multicol;
    let Some(weight) = m.column_rule_width.weight() else {
        return;
    };
    if m.column_rule_style.is_none() || m.column_rule_style.is_hidden() {
        return;
    }
    let Some(ext) = dom.node(id).ext() else {
        return;
    };
    let sets = ext.column_sets();
    if sets.is_empty() {
        return;
    }
    // §4.1: `currentcolor` and the rest, under the element's used scheme.
    let scheme = computed
        .color_scheme
        .used(crate::style::CascadeExt::color_scheme(dom));
    let cx = crate::ColorContext::new(computed.fg).with_scheme(scheme);
    let fg = m
        .column_rule_color
        .resolve(&computed.vars, &cx)
        .unwrap_or(computed.fg);
    if fg.alpha() == 0 {
        return;
    }
    let content = ext.content_layout;
    let origin = (content.x - ext.scroll_x, content.y - ext.scroll_y);
    // The container's border rows a rule can meet: its top and bottom
    // edges, when the content touches them.
    let (border_top, border_bottom) = (
        (!computed.border.top.is_none()).then_some(outer.y),
        (!computed.border.bottom.is_none()).then_some(outer.y + i32::from(outer.height) - 1),
    );
    let contribution = BorderContribution {
        style: m.column_rule_style,
        fg,
        weight,
        priority,
        corner_style: CornerStyle::Square,
        side: BorderSide::Left,
    };
    let add = |buf: &mut Buffer, x: i32, y: i32, dir: usize| {
        let (Ok(xu), Ok(yu)) = (u16::try_from(x), u16::try_from(y)) else {
            return;
        };
        if clip.contains(xu, yu) && buf.area.contains(xu, yu) {
            buf.add_border_dir(xu, yu, dir, contribution);
        }
    };
    for set in sets {
        let mut columns: Vec<_> = set.columns.iter().collect();
        columns.sort_by_key(|c| c.x);
        let (top, height) = (origin.1 + set.top, i32::from(set.height));
        if height == 0 {
            continue;
        }
        let bottom = top + height - 1;
        for pair in columns.windows(2) {
            let (left, right) = (pair[0], pair[1]);
            if !(left.filled && right.filled) {
                continue;
            }
            let end = left.x + i32::from(left.width);
            let gap = (right.x - end).max(0);
            let x = origin.0 + end + (gap - 1).max(0) / 2;
            // Every rule cell is a full vertical line (a cell's glyph is
            // its own directions'), one row long too.
            for y in top..=bottom {
                add(buf, x, y, DIR_N);
                add(buf, x, y, DIR_S);
            }
            if border_top == Some(top - 1) {
                add(buf, x, top - 1, DIR_S);
            }
            if border_bottom == Some(bottom + 1) {
                add(buf, x, bottom + 1, DIR_N);
            }
        }
    }
}
