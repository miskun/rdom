//! Flex layout — CSS Flexible Box Layout 1, with CSS Box Alignment 3.
//!
//! Given a container's `content_layout` and its flex items (§4: its
//! in-flow elements, pseudo-elements and anonymous items wrapping its
//! runs of text), computes each item's main and cross size and position and
//! recursively lays it out: the flex base sizes (§9.2, `flex-basis`, the
//! main size property, rdom's `width: <n>fr`), the lines (§9.3,
//! `flex-wrap`), the flexible lengths per line (§9.7, `flex-grow` /
//! `flex-shrink`, clamped by `min-*` / `max-*` and the §4.5 automatic
//! minimum), `auto` margins and `justify-content` (§8.1 / §8.2), the
//! lines' cross sizes and `align-content` (§9.4, §8.4), and each item's
//! cross size and `align-self` (§9.4 step 11, §8.3).
//!
//! ## Module layout
//!
//! - `mod.rs` — [`layout_flex_container`] (the container's items) and
//!   [`layout_flex_children`], the orchestrator that threads the flex
//!   lines through the pieces below in spec order. Which formatting
//!   context an element's children get is `layout_pass::dispatch`.
//! - [`main_axis`] — per-item main-size gathering (`ChildMain`, §9.2).
//! - [`distribute`] — the §9.7 grow / shrink freeze loop and the lazy
//!   §4.5 auto-min floor.
//! - [`lines`] — §9.3 line breaking, per-line §9.7, line cross sizes,
//!   `align-content: normal`, and a multi-line container's intrinsic
//!   cross size (§9.9).
//! - [`content`] — `justify-content` and `align-content`'s positions
//!   and distributions, in whole cells.
//! - [`align`] — `align-items` / `align-self`, baseline groups.
//! - [`cross`] — §9.4 cross-size determination, `aspect-ratio`, `auto`
//!   cross margins, the item's cross offset.
//! - [`placement`] — §9.5 main-axis placement: auto main margins,
//!   gaps, cursor advance, and the `layout_node` recursion per item.
//! - [`collapse`] — the flex-specific `border-collapse: collapse`
//!   rules: parent-edge inset and one-cell sibling overlap.
//!
//! The flex items themselves (§4: elements, pseudo-elements and
//! anonymous items wrapping the runs of text, and their measurement)
//! are `layout_pass::items`.

mod align;
mod collapse;
mod content;
#[cfg(test)]
mod cost_tests;
mod cross;
mod distribute;
mod lines;
mod main_axis;
mod placement;
mod strut;

use rdom_core::{Dom, NodeId};

use crate::ext::{AnonymousIfc, TuiExt};
use crate::layout::{Direction, LayoutRect};
use crate::style::ComputedStyle;

use super::items::{self, Item};
use super::margin_trim::FlexTrim;
use collapse::SiblingOverlap;
use cross::CrossSpace;
pub(in crate::render::layout_pass) use lines::{is_multi_line, lines_cross_size};
use main_axis::{MainBudgets, collect_main_axis_items};
use placement::{FlexLine, place_items};

/// `visibility: collapse` on a flex item (Flexbox §4.4): it is laid out
/// as a strut — no main size, its cross size kept — and drawn as
/// `hidden`. The document root's children are flex items of rdom's
/// viewport column only as a layout device (DIVERGENCES), as a
/// browser's `<body>` children are blocks: there `collapse` is
/// `hidden` (CSS Display 3 §4).
pub(in crate::render::layout_pass) fn is_collapsed(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    use crate::node::TuiNodeExt;
    dom.node(id)
        .computed()
        .is_some_and(|c| c.visibility == crate::layout::Visibility::Collapse)
        && crate::render::box_tree::box_parent(dom, id).is_some_and(|p| {
            let parent = dom.node(p);
            parent.node_type() == rdom_core::NodeType::Element
                && parent
                    .computed()
                    .is_some_and(|c| c.flow == crate::layout::Flow::Flex)
        })
}

/// Lay out the flex container `id`'s items (CSS Flexbox §4: its in-flow
/// elements, pseudo-elements and an anonymous item per run of text)
/// inside `container`, its content box. Returns the anonymous items'
/// boxes.
pub(super) fn layout_flex_container(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    container: LayoutRect,
    computed: &ComputedStyle,
) -> Vec<AnonymousIfc> {
    // Filter out children that don't participate in normal flow:
    //
    // - `display: none` — invisible to layout and paint.
    // - `position: absolute` / `position: fixed` (M2) — removed
    //   from flow so the parent's flex distribution doesn't see
    //   them. Their final layout rect is filled in by phase-2
    //   placement after this pass returns.
    //
    // Their `LayoutRect` stays at the default zero from
    // `TuiExt::default` until something writes to it.
    //
    // CSS Flexbox §4: the items are the in-flow children — elements, the
    // pseudo-elements, and an anonymous item per run of text.
    let mut children = items::items_of(dom, id);
    // CSS Flexbox §5.4: the items are laid out in order-modified
    // document order.
    items::sort_by_order(dom, &mut children);
    // `D-M2-2`: a positioned child's static position in a flex
    // container is the content box's start — Flexbox §4.1 places it as
    // the sole item; `justify-content` / `align-items` are not applied
    // (DIVERGENCES). Both scroll offsets apply, as they do to the
    // in-flow items.
    let (static_x, static_y) = dom.node(id).ext().map_or((container.x, container.y), |e| {
        (container.x - e.scroll_x, container.y - e.scroll_y)
    });
    for n in super::positioning::out_of_flow_positioned_children(dom, id) {
        super::positioning::record_static_position(dom, n, static_x, static_y);
    }
    layout_flex_children(dom, &children, container, computed)
}

/// Lay out a flex container's items: `children` (its flex items, in
/// order-modified document order) inside `container`, driven by
/// `parent`'s `flex-direction`, `flex-wrap`, gaps and `border-collapse`.
/// Returns the anonymous items' boxes, in placement order.
///
/// Runs the CSS Flexible Box algorithm in spec order — gather the flex
/// base sizes (§9.2), collect the items into lines (§9.3, `lines`),
/// resolve each line's flexible lengths and `auto` margins (§9.7 /
/// §9.5), size the lines on the cross axis (§9.4 steps 7–8 and 15),
/// then place each line's items along the main axis and size them on
/// the cross axis (§9.4–§9.6) — with the per-concern math in the sibling
/// modules.
pub(super) fn layout_flex_children(
    dom: &mut Dom<TuiExt>,
    children: &[Item],
    container: LayoutRect,
    parent: &ComputedStyle,
) -> Vec<AnonymousIfc> {
    let mut anonymous = Vec::new();
    if children.is_empty() {
        return anonymous;
    }

    let direction = parent.direction;
    let cross_direction = match direction {
        Direction::Row => Direction::Column,
        Direction::Column => Direction::Row,
    };
    // CSS Box Alignment 3 §8.1: the gutters between items on a line, and
    // between the lines.
    let gap = super::resolve_gap(parent, container, direction);
    let line_gap = super::resolve_gap(parent, container, cross_direction);

    let container = collapse::inset_container_for_children(dom, children, parent, container);

    // The container's inner size on the main and cross axes.
    let (main_budget, cross_budget) = match direction {
        Direction::Row => (container.width, container.height),
        Direction::Column => (container.height, container.width),
    };
    let budgets = MainBudgets {
        main: main_budget,
        cross: cross_budget,
    };

    let trim = FlexTrim::of(parent, direction);
    // CSS Writing Modes 4 §2.1: under `rtl` the inline axis — a row's
    // main axis, a column's cross axis — runs right to left; CSS Flexbox
    // §5.1: `row-reverse` / `column-reverse` swap the main axis's start
    // and end (so a `row-reverse` under `rtl` runs left to right), and
    // §5.2: `wrap-reverse` swaps the cross axis's. A mirrored axis is
    // laid out in a mirrored frame (the item's physical end margin its
    // start one) and flipped back across the container.
    let flip = AxisFlip::of(parent, direction);
    let mut items = collect_main_axis_items(dom, children, direction, budgets, trim, flip.main);

    // §9.3: a single-line container's one line holds every item.
    let multi_line = lines::is_multi_line(parent);
    // §9.4 step 10: collapsed items become struts of their line's size.
    strut::make_struts(
        dom,
        parent,
        &mut items,
        budgets,
        gap,
        lines::LineFrame {
            direction,
            flip,
            trim,
            space: CrossSpace {
                line: cross_budget,
                container: Some(cross_budget),
            },
            cb_width: container.width,
        },
        multi_line,
    );
    let line_ranges = if multi_line {
        lines::break_lines(dom, &items, direction, budgets, gap)
    } else {
        std::iter::once(0..items.len()).collect()
    };
    let overlap = SiblingOverlap::new(parent, gap, direction);

    // §9.7 per line, then each line's cross size: a single line is the
    // container's inner cross size (§9.4 step 8); a multi-line
    // container's lines are as large as their largest outer
    // hypothetical cross size, stretched by `align-content: normal`
    // (§9.4 step 15).
    let last_line = line_ranges.len() - 1;
    let line_trim = |k: usize| FlexTrim {
        cross_start: trim.cross_start && (!multi_line || k == 0),
        cross_end: trim.cross_end && (!multi_line || k == last_line),
        ..trim
    };
    let mut resolved = Vec::with_capacity(line_ranges.len());
    let mut line_cross = Vec::with_capacity(line_ranges.len());
    for (k, range) in line_ranges.iter().enumerate() {
        if multi_line {
            lines::trim_line_edges(&mut items[range.clone()], trim);
        }
        let line = lines::resolve_line_main(
            dom,
            &items[range.clone()],
            direction,
            budgets,
            gap,
            &overlap,
        );
        let (tallest, plan) = lines::line_cross_size(
            dom,
            parent,
            &items[range.clone()],
            &line.final_main,
            lines::LineFrame {
                direction,
                flip,
                trim: line_trim(k),
                space: CrossSpace {
                    line: cross_budget,
                    container: Some(cross_budget),
                },
                cb_width: container.width,
            },
            multi_line,
        );
        line_cross.push(if multi_line { tallest } else { cross_budget });
        resolved.push((line, plan));
    }
    // §9.4 step 15: `align-content` (a multi-line container only, §8.4)
    // stretches the lines, or places its free cross space around them.
    let mut line_lead = vec![0; line_cross.len()];
    if multi_line {
        let n = line_cross.len();
        let used: i32 = line_cross.iter().map(|&l| i32::from(l)).sum::<i32>()
            + i32::from(line_gap) * (n as i32 - 1);
        match content::align_content_offsets(
            parent,
            direction,
            flip.cross,
            i32::from(cross_budget) - used,
            n,
        ) {
            Some(lead) => line_lead = lead,
            None => lines::stretch_lines(&mut line_cross, cross_budget, line_gap),
        }
    }

    let mut line_offset: i32 = 0;
    for (k, (range, (line, plan))) in line_ranges.iter().zip(resolved).enumerate() {
        line_offset += line_lead[k];
        // §8.3: each item's cross alignment, its baselines placed now the
        // line's cross size is known.
        let align = plan.resolve(line_cross[k], flip.cross);
        // §8.2: `justify-content` places the line's leftover free space.
        // §9.4 step 10: struts take no share — the line's other items are
        // the ones it places.
        let line_items = &items[range.clone()];
        let spaced = line_items.iter().filter(|ci| ci.strut.is_none()).count();
        let mut shares = content::justify_offsets(
            parent,
            direction,
            flip.main,
            line.free,
            spaced,
            line.has_auto_margins,
        )
        .into_iter();
        let justify: Vec<i32> = line_items
            .iter()
            .map(|ci| match ci.strut {
                Some(_) => 0,
                None => shares.next().unwrap_or(0),
            })
            .collect();
        place_items(
            dom,
            FlexLine {
                items: &items[range.clone()],
                final_main: &line.final_main,
                justify: &justify,
                align: &align,
                container,
                direction,
                gap,
                space: CrossSpace {
                    line: line_cross[k],
                    container: Some(cross_budget),
                },
                line_offset,
                auto_margins: line.auto_margins,
                overlap,
                trim: line_trim(k),
                flip,
            },
            &mut anonymous,
        );
        line_offset += i32::from(line_cross[k]) + i32::from(line_gap);
    }
    anonymous
}

/// Which of a flex container's axes run from their physical end edge
/// — right to left, or bottom to top.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct AxisFlip {
    /// The main axis: a row's under `rtl` XOR `row-reverse`, a column's
    /// under `column-reverse`.
    pub(super) main: bool,
    /// The cross axis: a column's under `rtl` XOR `wrap-reverse`, a
    /// row's under `wrap-reverse` (CSS Flexbox §5.2: cross-start and
    /// cross-end swap; a row's cross axis, the block axis, otherwise runs
    /// top to bottom).
    pub(super) cross: bool,
}

impl AxisFlip {
    pub(super) fn of(container: &ComputedStyle, direction: Direction) -> Self {
        let rtl = super::margin_trim::inline_reversed(container);
        let reverse = container.flex_reverse;
        let wrap_reverse = container.flex_wrap == crate::layout::FlexWrap::WrapReverse;
        match direction {
            Direction::Row => Self {
                main: rtl != reverse,
                cross: wrap_reverse,
            },
            Direction::Column => Self {
                main: reverse,
                cross: rtl != wrap_reverse,
            },
        }
    }
}
