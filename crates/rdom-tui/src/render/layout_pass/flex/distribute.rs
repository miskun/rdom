//! "Resolve the flexible lengths" — CSS Flexible Box §9.7: the freeze
//! loop that distributes a line's free space to its items by
//! `flex-grow`, or takes its overflow from them by the scaled shrink
//! factor ([`resolve_flexible_lengths`]), with the lazily resolved §4.5
//! `min-*: auto` floor. The items, with their flex base sizes, come from
//! `main_axis`.

use rdom_core::Dom;

use super::main_axis::ChildMain;
use crate::ext::TuiExt;
use crate::layout::{Direction, clamp_size};
use crate::render::layout_pass::items::Item;
use crate::render::layout_pass::shares::{Rolling, sums_below_one};

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
///    sizes (bases clamped by min / max, the §4.5 automatic minimum
///    included) leave free space, else `flex-shrink` — also when the
///    line has no room at all.
/// 2. An item is inflexible — frozen at its hypothetical size — when
///    its factor is 0, or when growing its base exceeds its hypothetical
///    size (a `max-*` clamp), or when shrinking it is below it (a
///    `min-*` clamp).
/// 3. Loop: the free space is the net extent less the frozen items'
///    targets and the others' bases (a factor sum below one takes only
///    that fraction of the initial free space); it is shared by
///    `flex-grow`, or, as overflow, by `flex-shrink × inner base`; each
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
    let budgets = super::main_axis::MainBudgets {
        main: budget.main,
        cross: budget.cross,
    };
    // The hypothetical main sizes: the bases clamped by `min-*` / `max-*`,
    // the §4.5 automatic minimum included (`ChildMain::hypothetical`, as
    // line breaking takes them).
    let mut target: Vec<u16> = items
        .iter()
        .map(|ci| ci.hypothetical(dom, direction, budgets))
        .collect();
    let net = budget.net;
    // A line with no room (`net <= 0`) shrinks too: its free space is
    // negative.
    let growing = target.iter().map(|&h| i32::from(h)).sum::<i32>() < net;
    let factor = |ci: &ChildMain| f64::from(if growing { ci.grow } else { ci.shrink });
    // Inflexible items freeze at their hypothetical sizes; the others
    // start from their bases.
    let mut frozen = vec![false; n];
    for (i, ci) in items.iter().enumerate() {
        let h = target[i];
        if factor(ci) <= 0.0 || (growing && ci.base > h) || (!growing && ci.base < h) {
            frozen[i] = true;
        } else {
            target[i] = ci.base;
        }
    }
    let free_now = |target: &[u16], frozen: &[bool]| -> i32 {
        let used: i32 = (0..n)
            .map(|i| i32::from(if frozen[i] { target[i] } else { items[i].base }))
            .sum();
        net - used
    };
    let initial_free = f64::from(free_now(&target, &frozen));
    // An unfrozen item's target clamped by its min / max: the §4.5
    // automatic minimum only where it can bind (a growing target is at
    // or above its base), resolved once per item (`ChildMain::auto_min`)
    // — so clamping again costs no walk, and the loop needs no buffer of
    // clamped targets.
    let clamped = |ci: &ChildMain, t: u16| -> u16 {
        let floor = if growing {
            ci.min_main_above_base(dom, direction, budgets)
        } else {
            Some(ci.min_main(dom, direction, budgets))
        };
        clamp_size(t, floor, ci.max)
    };
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
        // Sum the violations of the unfrozen targets' clamps.
        let total: i64 = (0..n)
            .filter(|&i| !frozen[i])
            .map(|i| i64::from(clamped(&items[i], target[i])) - i64::from(target[i]))
            .sum();
        // Freezing an item changes only its own state, so one pass over
        // the items still unfrozen at its start decides them all.
        for i in 0..n {
            if frozen[i] {
                continue;
            }
            let c = clamped(&items[i], target[i]);
            let v = i64::from(c) - i64::from(target[i]);
            let freeze = match total.signum() {
                0 => true,
                1 => v > 0,
                _ => v < 0,
            };
            if freeze {
                frozen[i] = true;
                target[i] = c;
            }
        }
    }
    target
}

/// Share `free` among the unfrozen items: by `flex-grow` when growing,
/// as overflow by `flex-shrink × inner base` when shrinking (§9.7 step
/// 4.c: the inner flex base size, the content box); each unfrozen target
/// is its base plus (minus) its share, rolling so the shares sum to the
/// whole cells of `free`.
fn distribute(items: &[ChildMain], frozen: &[bool], target: &mut [u16], free: f64, growing: bool) {
    let weight = |ci: &ChildMain| -> f64 {
        if growing {
            f64::from(ci.grow)
        } else {
            f64::from(ci.shrink) * f64::from(ci.inner_base)
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
    let mut shares = Rolling::new(amount, total);
    for (i, ci) in items.iter().enumerate() {
        if frozen[i] {
            continue;
        }
        let share = shares.share(weight(ci)).min(u32::from(u16::MAX)) as u16;
        target[i] = if growing {
            ci.base.saturating_add(share)
        } else {
            ci.base.saturating_sub(share)
        };
    }
}

/// Compute the auto-min floor for `id` along `direction`, per CSS
/// Flexbox §4.5: the item's content-based minimum size
/// (`items::content_based_minimum`), its specified size suggestion
/// resolved against the container's main size. Called from the shrink
/// branch when an item with implicit/auto min is about to be shrunk —
/// eager resolution during the natural-size pass would walk every flex
/// item's subtree every layout (the +47% regression observed in the
/// full-frame benchmark), so we defer until we know the item is
/// actually shrinking.
pub(super) fn resolve_auto_min(
    dom: &Dom<TuiExt>,
    item: &Item,
    direction: Direction,
    main_budget: u16,
    cross_budget: u16,
) -> u16 {
    #[cfg(test)]
    super::cost_tests::AUTO_MINS.with(|c| c.set(c.get() + 1));
    let cb_width = match direction {
        Direction::Row => main_budget,
        Direction::Column => cross_budget,
    };
    crate::render::layout_pass::items::content_based_minimum(
        dom,
        item,
        direction,
        crate::render::layout_pass::items::Suggestion {
            basis: Some(main_budget),
            available: main_budget,
            cross_budget,
            cb_width,
        },
    )
}
