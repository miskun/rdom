//! Flex lines — CSS Flexible Box §9.3 (collecting items into lines),
//! §9.7 run per line, and §9.4 steps 8 / 15 (each line's cross size, and
//! `align-content: normal` stretching the lines).
//!
//! A single-line container (`flex-wrap: nowrap`) has one line holding
//! every item, as tall (wide) as the container's inner cross size; a
//! multi-line container (`wrap`, `wrap-reverse`) breaks its items into
//! lines by their outer hypothetical main sizes, each line as tall as its
//! tallest item. The intrinsic sizes of a multi-line container (§9.9) run
//! the same steps ([`lines_cross_size`]).

use std::ops::Range;

use rdom_core::{Dom, NodeId};

use super::item::FlexItem;

use super::align::{CrossFrame, LinePlan, PlanItem};
use super::collapse::SiblingOverlap;
use super::cross::{CrossSpace, ResolvedMain, hypothetical_outer_cross};
use super::distribute::{MainAxisBudget, resolve_flexible_lengths};
use super::main_axis::{ChildMain, MainBudgets, collect_main_axis_items};
use super::placement::AutoMainMargins;
use crate::ext::TuiExt;
use crate::layout::{Direction, FlexWrap, MarginValue};
use crate::node::TuiNodeExt;
use crate::render::layout_pass::margin_trim::FlexTrim;
use crate::style::ComputedStyle;

/// Whether `container` is a multi-line flex container (§5.2).
pub(in crate::render::layout_pass) fn is_multi_line(container: &ComputedStyle) -> bool {
    container.flex_wrap != FlexWrap::NoWrap
}

/// A margin's cells on the main axis; `auto` is 0 (§9.3: it is treated
/// as 0 when the hypothetical main sizes are summed).
fn cells(m: &MarginValue) -> i32 {
    match m {
        MarginValue::Cells(n) => i32::from(*n),
        _ => 0,
    }
}

/// §9.3 step 5: collect consecutive items into a line until the next
/// one's outer hypothetical main size — the `gap` before it counted (CSS
/// Box Alignment 3 §8.1) — would overflow `main`; an item that does not
/// fit an empty line takes one of its own. Every line holds at least
/// one item.
///
/// The hypothetical main size is the flex base size clamped by `min-*`
/// / `max-*`, the §4.5 automatic minimum included (resolved only for an
/// item whose base it can raise — a content-sized base is never below
/// it, nor a specified one — and then once, `ChildMain::auto_min`).
pub(super) fn break_lines(
    dom: &Dom<TuiExt>,
    items: &[ChildMain],
    direction: Direction,
    budgets: MainBudgets,
    gap: u16,
) -> Vec<Range<usize>> {
    let main = i32::from(budgets.main);
    let gap = i32::from(gap);
    let mut lines = Vec::new();
    let mut start = 0;
    let mut used = 0;
    // The line holds an item that is not a strut.
    let mut holds = false;
    for (i, ci) in items.iter().enumerate() {
        // A strut has no main size and no gap beside it (§9.4 step 10): it
        // joins the line it falls in.
        if ci.strut.is_some() {
            continue;
        }
        let outer = i32::from(ci.hypothetical(dom, direction, budgets))
            + cells(&ci.main_start_margin)
            + cells(&ci.main_end_margin);
        if holds && used + gap + outer > main {
            lines.push(start..i);
            start = i;
            used = outer;
        } else if holds {
            used += gap + outer;
        } else {
            used = outer;
        }
        holds = true;
    }
    if start < items.len() {
        lines.push(start..items.len());
    }
    lines
}

/// CSS Box 4 §3.2 in a multi-line container: the main-axis margins of
/// each line's first and last items are trimmed (the container's first
/// and last items were trimmed when the items were gathered).
pub(super) fn trim_line_edges(items: &mut [ChildMain], trim: FlexTrim) {
    if trim.main_start
        && let Some(first) = items.first_mut()
    {
        first.main_start_margin = MarginValue::Cells(0);
    }
    if trim.main_end
        && let Some(last) = items.last_mut()
    {
        last.main_end_margin = MarginValue::Cells(0);
    }
}

/// One line's resolved main axis: each item's main size and the share
/// of the line's leftover free space its `auto` margins take.
pub(super) struct LineMain {
    pub(super) final_main: Vec<u16>,
    pub(super) auto_margins: AutoMainMargins,
    /// The line's leftover free space after its flexible lengths —
    /// negative when its items overflow it; `auto` margins took it when
    /// positive and [`Self::has_auto_margins`].
    pub(super) free: i32,
    pub(super) has_auto_margins: bool,
}

/// §9.7 and §9.5 step 12 for one line: resolve the flexible lengths of
/// `items` (the line's items) against the line's
/// extent — the main size less the gaps and the non-`auto` margins, plus
/// the cells sibling overlap reclaims — then split what is left over
/// the `auto` main-axis margins (a remainder cell to each of the first).
pub(super) fn resolve_line_main(
    dom: &Dom<TuiExt>,
    items: &[ChildMain],
    direction: Direction,
    budgets: MainBudgets,
    gap: u16,
    overlap: &SiblingOverlap,
) -> LineMain {
    // Struts take no gap (§9.4 step 10).
    let spaced = items.iter().filter(|ci| ci.strut.is_none()).count();
    let gap_total = gap.saturating_mul((spaced as u16).saturating_sub(1));
    let margins: i32 = items
        .iter()
        .map(|ci| cells(&ci.main_start_margin) + cells(&ci.main_end_margin))
        .sum();
    let auto_count: u32 = items
        .iter()
        .map(|ci| {
            u32::from(ci.main_start_margin.is_auto()) + u32::from(ci.main_end_margin.is_auto())
        })
        .sum();
    let net = i32::from(budgets.main) - i32::from(gap_total)
        + i32::from(
            overlap.savings(
                dom,
                items
                    .iter()
                    .filter(|ci| ci.strut.is_none())
                    .map(|ci| &ci.item),
            ),
        )
        - margins;
    let final_main = resolve_flexible_lengths(
        dom,
        items,
        direction,
        MainAxisBudget {
            main: budgets.main,
            cross: budgets.cross,
            net,
        },
    );
    let used: i32 = final_main.iter().map(|&n| i32::from(n)).sum();
    let free = net - used;
    let remaining = free.clamp(0, i32::from(u16::MAX)) as u32;
    LineMain {
        free,
        has_auto_margins: auto_count > 0,
        final_main,
        auto_margins: AutoMainMargins {
            share: remaining.checked_div(auto_count).unwrap_or(0) as u16,
            remainder: remaining.checked_rem(auto_count).unwrap_or(0),
        },
    }
}

/// §9.4 step 15, `align-content: normal` (which behaves as `stretch` in a
/// flex container, CSS Box Alignment 3 §5.3): when the lines and the
/// gaps between them leave free cross space, each line grows by an equal
/// share — whole cells, a remainder cell to each of the first lines.
/// Negative free space leaves the lines as they are.
pub(super) fn stretch_lines(lines: &mut [u16], cross: u16, gap: u16) {
    let n = lines.len() as u32;
    if n == 0 {
        return;
    }
    let used: u32 = lines.iter().map(|&l| u32::from(l)).sum::<u32>() + u32::from(gap) * (n - 1);
    let Some(free) = u32::from(cross).checked_sub(used) else {
        return;
    };
    for (k, line) in lines.iter_mut().enumerate() {
        let extra = free / n + u32::from((k as u32) < free % n);
        *line = line.saturating_add(extra.min(u32::from(u16::MAX)) as u16);
    }
}

/// How one line's items are measured and placed on the cross axis.
#[derive(Debug, Clone, Copy)]
pub(super) struct LineFrame {
    pub(super) direction: Direction,
    pub(super) flip: super::AxisFlip,
    /// The line's cross-axis `margin-trim`.
    pub(super) trim: FlexTrim,
    /// The cross space the items measure against.
    pub(super) space: CrossSpace,
    /// The basis of the items' margin and padding percentages.
    pub(super) cb_width: u16,
}

/// What the cross resolver knows of an item at its used main size.
pub(super) fn resolved_main(ci: &ChildMain, size: u16, frame: &LineFrame) -> ResolvedMain {
    ResolvedMain {
        size,
        was_auto: ci.main_auto,
        trim_cross_start: frame.trim.cross_start,
        trim_cross_end: frame.trim.cross_end,
        mirror: frame.flip.cross,
    }
}

/// A multi-line container's line cross size (§9.4 step 8): the largest
/// outer hypothetical cross size of its items — or the extent of its
/// baseline-aligned items, when larger — with the line's alignment plan
/// (`align::LinePlan`), which places its items once the line's final
/// size is known. A single-line container's line is its cross size, so
/// it needs only the plan (`measure: false`, the size is then 0): the
/// hypothetical sizes are measured for multi-line containers alone.
pub(super) fn line_cross_size(
    dom: &Dom<TuiExt>,
    container: &ComputedStyle,
    items: &[ChildMain],
    final_main: &[u16],
    frame: LineFrame,
    measure: bool,
) -> (u16, LinePlan) {
    let plan_items: Vec<PlanItem> = items
        .iter()
        .zip(final_main)
        .map(|(ci, &size)| PlanItem {
            item: ci.item.clone(),
            main: resolved_main(ci, size, &frame),
            strut: ci.strut,
        })
        .collect();
    let plan = LinePlan::new(
        dom,
        container,
        CrossFrame {
            direction: frame.direction,
            flipped: frame.flip.cross,
            rtl: crate::render::layout_pass::margin_trim::inline_reversed(container),
        },
        &plan_items,
        frame.space,
        frame.cb_width,
    );
    if !measure {
        return (0, plan);
    }
    let tallest = plan_items
        .iter()
        .map(|p| {
            // A strut holds its line at its strut size (§9.4 step 10).
            p.strut.unwrap_or_else(|| {
                hypothetical_outer_cross(
                    dom,
                    &p.item,
                    frame.cb_width,
                    frame.space,
                    frame.direction,
                    p.main,
                )
            })
        })
        .max()
        .unwrap_or(0);
    (tallest.max(plan.baseline_extent()), plan)
}

/// The intrinsic cross size of the flex container `id` (CSS Flexbox
/// §9.9.2): its items broken into lines at `main` (one line when it is
/// single-line, §9.3; the
/// inner main size they wrap at), each line's items flexed, each line as
/// large as its largest outer hypothetical cross size — measured with
/// the item's used main size — and the lines `cross_gap` apart.
/// `cross` is the perpendicular extent the items measure against (a
/// column's items' text wraps to it); `cb_width` is the basis of the
/// items' margin and padding percentages. The container's cross size
/// is what is being measured, so a cross-axis percentage is cyclic and
/// behaves as `auto`.
pub(in crate::render::layout_pass) fn lines_cross_size(
    dom: &Dom<TuiExt>,
    id: NodeId,
    children: &[FlexItem],
    main: u16,
    cross: u16,
    cb_width: u16,
) -> u16 {
    let Some(container) = dom.node(id).computed_rc() else {
        return 0;
    };
    let direction = container.direction;
    let cross_dir = match direction {
        Direction::Row => Direction::Column,
        Direction::Column => Direction::Row,
    };
    // Intrinsic sizing has no container size: percentage gaps are 0.
    let gap = crate::render::layout_pass::gap_along(&container, direction).resolve(0);
    let cross_gap = crate::render::layout_pass::gap_along(&container, cross_dir).resolve(0);
    let trim = FlexTrim::of(&container, direction);
    let flip = super::AxisFlip::of(&container, direction);
    let budgets = MainBudgets { main, cross };
    let mut items = collect_main_axis_items(dom, children, direction, budgets, trim, flip.main);
    // §9.4 step 10: collapsed items become struts of their line's size,
    // measured — the container's cross size is what is being found.
    super::strut::make_struts(
        dom,
        &container,
        &mut items,
        budgets,
        gap,
        LineFrame {
            direction,
            flip,
            trim,
            space: CrossSpace {
                line: cross,
                container: None,
            },
            cb_width,
        },
        is_multi_line(&container),
    );
    // §9.3: a single-line container's one line holds every item.
    let lines = if is_multi_line(&container) {
        break_lines(dom, &items, direction, budgets, gap)
    } else {
        std::iter::once(0..items.len()).collect()
    };
    let overlap = SiblingOverlap::new(&container, gap, direction);
    let last = lines.len().saturating_sub(1);
    let mut total: u32 = 0;
    for (k, range) in lines.iter().enumerate() {
        trim_line_edges(&mut items[range.clone()], trim);
        let line = resolve_line_main(
            dom,
            &items[range.clone()],
            direction,
            budgets,
            gap,
            &overlap,
        );
        let line_trim = FlexTrim {
            cross_start: trim.cross_start && k == 0,
            cross_end: trim.cross_end && k == last,
            ..trim
        };
        let (tallest, _) = line_cross_size(
            dom,
            &container,
            &items[range.clone()],
            &line.final_main,
            LineFrame {
                direction,
                flip,
                trim: line_trim,
                space: CrossSpace {
                    line: cross,
                    container: None,
                },
                cb_width,
            },
            true,
        );
        total += u32::from(tallest);
        if k > 0 {
            total += u32::from(cross_gap);
        }
    }
    total.min(u32::from(u16::MAX)) as u16
}
