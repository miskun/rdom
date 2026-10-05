//! "Resolve the flexible lengths" — CSS Flexible Box §9.7: the freeze
//! loop that distributes a line's free space to its items by
//! `flex-grow`, or takes its overflow from them by the scaled shrink
//! factor ([`resolve_flexible_lengths`]), with the lazily resolved §4.5
//! `min-*: auto` floor. The items, with their flex base sizes, come from
//! `main_axis`.

use rdom_core::{Dom, NodeId};

use super::main_axis::ChildMain;
use crate::ext::TuiExt;
use crate::layout::{Direction, Overflow, Size, clamp_size};
use crate::node::TuiNodeExt;
use crate::render::layout_pass::intrinsic::Keywords;
use crate::render::layout_pass::intrinsic::content_min_size;

/// Budget figures the §9.7 loop distributes against.
pub(super) struct MainAxisBudget {
    /// The container's main-axis content extent.
    pub(super) main: u16,
    /// The container's cross-axis content extent (auto-min resolution
    /// measures content against it).
    pub(super) cross: u16,
    /// `main − gaps + overlap savings − the items' non-auto margins`:
    /// the extent the items' outer sizes share (§9.7 "inner main size").
    pub(super) net: i32,
}

/// Resolve each item's final main size (CSS Flexbox §9.7).
///
/// 1. The flex factor: `flex-grow` when the outer hypothetical main
///    sizes (bases clamped by min / max) leave free space, else
///    `flex-shrink`.
/// 2. An item is inflexible — frozen at its hypothetical size — when
///    its factor is 0, or when growing its base exceeds its hypothetical
///    size (a `max-*` clamp), or when shrinking it is below it (a
///    `min-*` clamp).
/// 3. Loop: the free space is the net extent less the frozen items'
///    targets and the others' bases (a factor sum below one takes only
///    that fraction of the initial free space); it is shared by
///    `flex-grow`, or, as overflow, by `flex-shrink × base`; each
///    unfrozen target is clamped by its min / max (the §4.5 automatic
///    minimum resolved only for an item whose clamp could fire); the
///    total violation decides who freezes — all of them when zero, the
///    min-clamped ones when positive, the max-clamped ones when
///    negative — and the loop runs again until every item is frozen.
///
/// Shares are whole cells, distributed rolling (Bresenham) so no cell
/// of the free space is dropped: two growing items over 31 cells get
/// 15 + 16.
pub(super) fn resolve_flexible_lengths(
    dom: &Dom<TuiExt>,
    items: &[ChildMain],
    direction: Direction,
    budget: MainAxisBudget,
) -> Vec<u16> {
    let n = items.len();
    let hypothetical: Vec<u16> = items
        .iter()
        .map(|ci| clamp_size(ci.base, ci.min, ci.max))
        .collect();
    let net = budget.net;
    let growing = hypothetical.iter().map(|&h| i32::from(h)).sum::<i32>() < net;
    // A container with no room shrinks nothing (its items keep their
    // hypothetical sizes and overflow).
    if !growing && net <= 0 {
        return hypothetical;
    }
    let factor = |ci: &ChildMain| f64::from(if growing { ci.grow } else { ci.shrink });
    let mut target: Vec<u16> = items.iter().map(|ci| ci.base).collect();
    let mut frozen = vec![false; n];
    for (i, ci) in items.iter().enumerate() {
        let h = hypothetical[i];
        if factor(ci) <= 0.0 || (growing && ci.base > h) || (!growing && ci.base < h) {
            frozen[i] = true;
            target[i] = h;
        }
    }
    let free_now = |target: &[u16], frozen: &[bool]| -> i32 {
        let used: i32 = (0..n)
            .map(|i| i32::from(if frozen[i] { target[i] } else { items[i].base }))
            .sum();
        net - used
    };
    let initial_free = f64::from(free_now(&target, &frozen));
    // The automatic minimum (§4.5), resolved at most once per item.
    let mut auto_min: Vec<Option<u16>> = vec![None; n];
    while frozen.iter().any(|f| !f) {
        let factor_sum: f64 = (0..n)
            .filter(|&i| !frozen[i])
            .map(|i| factor(&items[i]))
            .sum();
        let mut free = f64::from(free_now(&target, &frozen));
        if sums_below_one(factor_sum) {
            let scaled = initial_free * factor_sum;
            if scaled.abs() < free.abs() {
                free = scaled;
            }
        }
        distribute(items, &frozen, &mut target, free, growing);
        // Clamp each unfrozen target and sum the violations.
        let mut clamped: Vec<u16> = target.clone();
        let mut total: i64 = 0;
        for i in (0..n).filter(|&i| !frozen[i]) {
            let ci = &items[i];
            let floor = match ci.min {
                Some(m) => Some(m),
                // Growing from a content-sized base cannot fall below
                // the automatic minimum, which is no larger.
                None if growing && ci.content_base => None,
                None => Some(*auto_min[i].get_or_insert_with(|| {
                    resolve_auto_min(dom, ci.id, direction, budget.main, budget.cross)
                })),
            };
            clamped[i] = clamp_size(target[i], floor, ci.max);
            total += i64::from(clamped[i]) - i64::from(target[i]);
        }
        let unfrozen: Vec<usize> = (0..n).filter(|&i| !frozen[i]).collect();
        for i in unfrozen {
            let v = i64::from(clamped[i]) - i64::from(target[i]);
            let freeze = match total.signum() {
                0 => true,
                1 => v > 0,
                _ => v < 0,
            };
            if freeze {
                frozen[i] = true;
                target[i] = clamped[i];
            }
        }
    }
    target
}

/// Share `free` among the unfrozen items: by `flex-grow` when growing,
/// as overflow by `flex-shrink × base` when shrinking; each unfrozen
/// target is its base plus (minus) its share, rolling so the shares sum
/// to the whole cells of `free`.
fn distribute(items: &[ChildMain], frozen: &[bool], target: &mut [u16], free: f64, growing: bool) {
    let weight = |ci: &ChildMain| -> f64 {
        if growing {
            f64::from(ci.grow)
        } else {
            f64::from(ci.shrink) * f64::from(ci.base)
        }
    };
    let total: f64 = items
        .iter()
        .zip(frozen)
        .filter(|(_, f)| !**f)
        .map(|(ci, _)| weight(ci))
        .sum();
    let amount = if growing {
        free.max(0.0)
    } else {
        (-free).max(0.0)
    };
    let mut accumulated_weight = 0.0;
    let mut accumulated: u32 = 0;
    for (i, ci) in items.iter().enumerate() {
        if frozen[i] {
            continue;
        }
        let share = if total > 0.0 {
            accumulated_weight += weight(ci);
            let to = floor_cells(amount * accumulated_weight / total);
            let share = to.saturating_sub(accumulated).min(u32::from(u16::MAX)) as u16;
            accumulated = to;
            share
        } else {
            0
        };
        target[i] = if growing {
            ci.base.saturating_add(share)
        } else {
            ci.base.saturating_sub(share)
        };
    }
}

/// Relative tolerance for arithmetic on flex factors.
///
/// Factors are CSS `<number>`s parsed into `f32` (24-bit mantissa), so
/// a decimal factor carries a relative error of up to 2⁻²⁴ and a sum or
/// product of them a few times that: `0.1 + 0.2 + 0.7` is
/// 0.99999999255 once widened to `f64`, and `80 × (0.2 + 0.7)` is
/// 71.9999999. Comparisons against one and the floors that turn shares
/// into cells treat values within this tolerance as equal.
///
/// Why a tolerance and not an `f32` sum: summing in `f32` only moves
/// the rounding (it happens to give exactly 1.0 for `0.1 + 0.2 + 0.7`,
/// but `10 × 0.1` gives 1.0000001), and the floors still see products
/// a hair below an integer. The tolerance cannot misfire on a real
/// fraction: four `f32` epsilons (≈ 4.8e-7) of the largest main size
/// (`u16::MAX` cells) is 0.03 of a cell, and a factor sum within it of
/// one is one at the precision an `f32` factor holds.
const FACTOR_TOLERANCE: f64 = 4.0 * f32::EPSILON as f64;

/// §9.7 step 4.b's "sum of the flex factors is less than one", with
/// the factors' `f32` rounding forgiven ([`FACTOR_TOLERANCE`]).
fn sums_below_one(sum: f64) -> bool {
    sum < 1.0 - FACTOR_TOLERANCE
}

/// Floor a rolling share target to whole cells, forgiving the factors'
/// `f32` rounding ([`FACTOR_TOLERANCE`]) so `71.9999999` is 72.
fn floor_cells(x: f64) -> u32 {
    (x + x.abs() * FACTOR_TOLERANCE + 1e-9)
        .floor()
        .clamp(0.0, f64::from(u32::MAX)) as u32
}

/// Compute the auto-min floor for `id` along `direction`, per CSS
/// Flexbox §4.5. Called from the shrink branch when an item with
/// implicit/auto min is about to be shrunk — eager resolution
/// during the natural-size pass would walk every flex item's
/// subtree every layout (the +47% regression observed in the
/// full-frame benchmark), so we defer until we know the item is
/// actually shrinking.
fn resolve_auto_min(
    dom: &Dom<TuiExt>,
    id: NodeId,
    direction: Direction,
    main_budget: u16,
    cross_budget: u16,
) -> u16 {
    let computed = match dom.node(id).computed() {
        Some(c) => c.clone(),
        None => return 0,
    };
    let main_size = match direction {
        Direction::Row => &computed.width,
        Direction::Column => &computed.height,
    };
    let overflow_on_axis = match direction {
        Direction::Row => computed.overflow_x,
        Direction::Column => computed.overflow_y,
    };
    let cb_width = match direction {
        Direction::Row => main_budget,
        Direction::Column => cross_budget,
    };
    // Whatever the suggestion, the content box is never negative: the
    // floor is at least the item's padding and border on the axis (CSS
    // Flexbox §9.7 clamps the target main size to the content box's 0).
    let kw = Keywords::new(dom, id, &computed, direction, cross_budget, cb_width);
    let sizer = kw.sizer();
    // CSS §4.5 exception: non-visible overflow drops the floor to 0
    // — items inside a scroll container are allowed to be sized
    // below their content.
    if overflow_on_axis != Overflow::Visible {
        return sizer.chrome();
    }
    // Specified size suggestion per spec: the declared main size, as
    // the border box `box-sizing` makes of it (CSS UI 3 §3.1).
    let specified_cap: Option<u16> = match main_size {
        Size::Flex(_) => Some(0),
        // A keyword height is the automatic size: no cap (CSS Sizing 3
        // §3.1); a keyword width is its content size.
        Size::Intrinsic(_) if direction == Direction::Column => None,
        definite => kw.size(definite, Some(main_budget), main_budget),
    };
    // `flex: N` (basis 0%) trivially has specified=0, so auto-min
    // = min(content, 0) = 0. Skip the content walk.
    if matches!(specified_cap, Some(0)) {
        return sizer.chrome();
    }
    let content = content_min_size(dom, id, direction, cross_budget, cb_width);
    sizer.floor(match specified_cap {
        Some(cap) => content.min(cap),
        None => content,
    })
}
