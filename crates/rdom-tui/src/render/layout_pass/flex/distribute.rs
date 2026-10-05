//! "Resolve the flexible lengths" — CSS Flexible Box §9.7: the freeze
//! loops that distribute a line's free space to its items for grow and
//! shrink ([`resolve_flexible_lengths`]), with the lazily resolved §4.5
//! `min-*: auto` floor. The items come from `main_axis`.

use rdom_core::{Dom, NodeId};

use super::main_axis::{ChildMain, MainNatural};
use crate::ext::TuiExt;
use crate::layout::{Direction, Overflow, Size, clamp_size};
use crate::node::TuiNodeExt;
use crate::render::layout_pass::intrinsic::Keywords;
use crate::render::layout_pass::intrinsic::content_min_size;

/// Budget figures the §9.7 freeze loops distribute against.
pub(super) struct MainAxisBudget {
    /// The container's main-axis content extent.
    pub(super) main: u16,
    /// The container's cross-axis content extent (auto-min resolution
    /// measures content against it).
    pub(super) cross: u16,
    /// Free space handed to flex-grow: 0 when `auto` main margins
    /// claim it instead.
    pub(super) flex_remaining: u16,
    /// `main − gaps + overlap savings`: the extent the items' sizes
    /// must fit within before flex-shrink kicks in.
    pub(super) net: i32,
}

/// Resolve each child's main-axis final size with min/max.
///
/// CSS Flexible Box §9.7 "resolve the flexible lengths": distribute
/// the free space among the unfrozen flex items; any item whose
/// share violates its min/max is *frozen* at the clamped size and
/// the loop runs again over the survivors with the leftover budget,
/// until no clamp fires. A single pass with a per-item clamp (the
/// previous shape) left the clamped remainder unallocated — visible
/// as a gap — or, on the shrink side, as overflow past the container.
///
/// Distribution inside a pass is rolling (Bresenham-style) so the
/// integer-division remainder is never dropped: two `Flex(1)`
/// children over 31 cells get 15 + 16, not 15 + 15.
pub(super) fn resolve_flexible_lengths(
    dom: &Dom<TuiExt>,
    child_info: &[ChildMain],
    direction: Direction,
    budget: MainAxisBudget,
) -> Vec<u16> {
    let mut final_main: Vec<u16> = child_info
        .iter()
        .map(|ci| match ci.main {
            MainNatural::Fixed(n) | MainNatural::Auto(n) => clamp_size(n, ci.min, ci.max),
            MainNatural::Flex(_) => 0,
        })
        .collect();
    distribute_grow(child_info, &mut final_main, budget.flex_remaining);
    distribute_shrink(dom, child_info, &mut final_main, direction, &budget);
    final_main
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

/// The grow half of §9.7: split `flex_remaining` across `Flex(w)`
/// items by weight, freezing any item its min/max clamps. Weights are
/// `<number>`s; when the unfrozen items' weights sum to less than one
/// they share only that fraction of the initial free space (§9.7 step
/// 4.b), the rest staying free.
fn distribute_grow(child_info: &[ChildMain], final_main: &mut [u16], flex_remaining: u16) {
    let mut frozen: Vec<bool> = child_info
        .iter()
        .map(|ci| !matches!(ci.main, MainNatural::Flex(_)))
        .collect();
    let initial = f64::from(flex_remaining);
    let mut budget = initial;
    loop {
        let weight: f64 = child_info
            .iter()
            .zip(frozen.iter())
            .filter(|(_, f)| !**f)
            .map(|(ci, _)| match ci.main {
                MainNatural::Flex(w) => f64::from(w),
                _ => 0.0,
            })
            .sum();
        if weight <= 0.0 {
            break;
        }
        // Every share in a pass is computed from the pass-start
        // budget; the budget consumed by items frozen in this pass is
        // subtracted only after the pass (a mid-pass subtraction made
        // later items' shares shrink and falsely froze them at their
        // floors).
        let pass_budget = if sums_below_one(weight) {
            budget.min(initial * weight)
        } else {
            budget
        };
        let mut frozen_this_pass = 0.0;
        let mut accumulated_weight = 0.0;
        let mut accumulated: u32 = 0;
        let mut clamped_any = false;
        for (i, ci) in child_info.iter().enumerate() {
            if frozen[i] {
                continue;
            }
            let MainNatural::Flex(w) = ci.main else {
                continue;
            };
            accumulated_weight += f64::from(w);
            // Rolling (Bresenham) targets: the running total is floored
            // once, so no cell of the remainder is dropped between
            // items.
            let target = floor_cells(pass_budget * accumulated_weight / weight);
            let share = target.saturating_sub(accumulated).min(u32::from(u16::MAX)) as u16;
            accumulated = target;
            let clamped = clamp_size(share, ci.min, ci.max);
            final_main[i] = clamped;
            if clamped != share {
                // Freeze at the clamped size; its budget is spoken for.
                frozen[i] = true;
                frozen_this_pass += f64::from(clamped);
                clamped_any = true;
            }
        }
        if !clamped_any {
            break;
        }
        budget = (budget - frozen_this_pass).max(0.0);
    }
}

/// The shrink half of §9.7.
///
/// When the sum of children's declared sizes (+ gaps − overlap)
/// exceeds the main-axis budget, CSS distributes the overflow
/// proportional to `flex_shrink * basis` across shrinkable
/// children. Default `flex_shrink: 1` makes overflow gracefully
/// shrink-to-fit instead of clipping past the parent's edge —
/// the behavior every CSS author expects from
/// `height: 100% on a flex child` (the showcase chrome case).
///
/// Same §9.7 freeze loop as grow: an item that would shrink below
/// its min (explicit `min-width` or the auto-min of §4.5) is frozen
/// at the floor and the remaining overflow is redistributed over
/// the others. Bresenham accumulation keeps each pass exact.
fn distribute_shrink(
    dom: &Dom<TuiExt>,
    child_info: &[ChildMain],
    final_main: &mut [u16],
    direction: Direction,
    budget: &MainAxisBudget,
) {
    let net_budget = budget.net;
    if net_budget <= 0 {
        return;
    }
    let shrink_of = |ci: &ChildMain| -> f64 {
        dom.node(ci.id)
            .computed()
            .map_or(1.0, |c| f64::from(c.flex_shrink))
    };
    // Basis = the size before any shrinking in this loop.
    let basis: Vec<u16> = final_main.to_vec();
    let mut frozen: Vec<bool> = child_info.iter().map(|ci| shrink_of(ci) <= 0.0).collect();
    let mut floors: Vec<Option<u16>> = vec![None; child_info.len()];
    let initial_total: i32 = final_main.iter().map(|&n| i32::from(n)).sum();
    let initial_overflow = f64::from((initial_total - net_budget).max(0));
    loop {
        let total: i32 = final_main.iter().map(|&n| i32::from(n)).sum();
        if total <= net_budget {
            break;
        }
        let unfrozen = || child_info.iter().enumerate().filter(|(i, _)| !frozen[*i]);
        // §9.7 step 4.b: shrink factors summing below one take only
        // that fraction of the initial overflow.
        let factor_sum: f64 = unfrozen().map(|(_, ci)| shrink_of(ci)).sum();
        let mut overflow = f64::from(total - net_budget);
        if sums_below_one(factor_sum) {
            overflow = overflow.min(initial_overflow * factor_sum);
        }
        // §9.7 step 4.c: shared in proportion to the scaled shrink
        // factor, `flex-shrink × flex base size`.
        let divisor: f64 = unfrozen()
            .map(|(i, ci)| f64::from(basis[i]) * shrink_of(ci))
            .sum();
        if divisor <= 0.0 {
            break; // nothing left that can shrink
        }
        let mut accumulated_basis = 0.0;
        let mut accumulated_shrink: u32 = 0;
        let mut clamped_any = false;
        for (i, ci) in child_info.iter().enumerate() {
            if frozen[i] {
                continue;
            }
            accumulated_basis += f64::from(basis[i]) * shrink_of(ci);
            let target_total_shrink = floor_cells(accumulated_basis * overflow / divisor);
            let my_shrink = target_total_shrink.saturating_sub(accumulated_shrink);
            let my_shrink = my_shrink.min(u32::from(u16::MAX)) as u16;
            accumulated_shrink = target_total_shrink;
            // Honor min clamp — child can't shrink below its
            // `min-width` / `min-height`. Explicit `Cells(n)` is
            // stored in `ci.min`; the auto-min (implicit or
            // explicit `Auto`) is resolved lazily here per CSS
            // Flexbox §4.5 and cached per item.
            let floor = *floors[i].get_or_insert_with(|| {
                ci.min.unwrap_or_else(|| {
                    resolve_auto_min(dom, ci.id, direction, budget.main, budget.cross)
                })
            });
            let wanted = final_main[i].saturating_sub(my_shrink);
            if wanted < floor {
                final_main[i] = floor;
                frozen[i] = true;
                clamped_any = true;
            } else {
                final_main[i] = wanted;
            }
        }
        if !clamped_any {
            break;
        }
        // A clamp fired: the unfrozen items were shrunk against a
        // stale overflow figure. Restore them to their basis and
        // redistribute the recomputed overflow on the next pass.
        for i in 0..final_main.len() {
            if !frozen[i] {
                final_main[i] = basis[i];
            }
        }
    }
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
