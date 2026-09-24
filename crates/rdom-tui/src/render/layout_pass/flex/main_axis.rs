//! Main-axis sizing of flex items — CSS Flexible Box §9.2 / §9.7.
//!
//! Owns the per-item main-size bookkeeping ([`ChildMain`],
//! [`MainNatural`]), the gathering pass that turns each item's
//! declared size and main-axis margins into a natural size plus the
//! line's consumed space ([`collect_main_axis_items`]), and the §9.7
//! "resolve the flexible lengths" freeze loops for grow and shrink
//! ([`resolve_flexible_lengths`]), including the lazily resolved §4.5
//! `min-*: auto` floor.

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::{Direction, MarginValue, MinSize, Overflow, Size, clamp_size};
use crate::node::TuiNodeExt;
use crate::render::layout_pass::intrinsic::{content_min_size, intrinsic_size};
use crate::style::ComputedStyle;

/// An item's main size before flexible-length resolution.
pub(super) enum MainNatural {
    Fixed(u16),
    Flex(u16),
    Auto(u16),
}

/// Per-item main-axis inputs gathered before distribution.
pub(super) struct ChildMain {
    pub(super) id: NodeId,
    pub(super) main: MainNatural,
    /// Explicit `min: <n>` cell floor, or `None` for "use auto-min
    /// resolved lazily during shrink". `None` covers both an unset
    /// min (CSS spec default for flex items) and an explicit
    /// `min: auto` — they share the same auto resolution path per
    /// CSS Flexbox §4.5.
    pub(super) min: Option<u16>,
    pub(super) max: Option<u16>,
    /// Pre-resolved main-axis start margin. `Cells` carries the
    /// resolved cell count (including `calc()` percent terms folded
    /// against the parent's main-axis cb-width). `Auto` is preserved
    /// because the placement loop resolves auto margins lazily by
    /// distributing leftover free space.
    pub(super) main_start_margin: MarginValue,
    pub(super) main_end_margin: MarginValue,
}

/// The gathered flex line: one [`ChildMain`] per item plus the totals
/// the free-space computation needs.
pub(super) struct MainAxisItems {
    pub(super) items: Vec<ChildMain>,
    /// Main-axis cells already spoken for by non-flex sizes and
    /// non-auto margins. Signed: a negative margin frees main-axis
    /// space (Flexbox §9.7 counts outer sizes; CSS margins may be
    /// negative).
    pub(super) consumed_fixed: i32,
    /// Number of `auto` main-axis margins across the line.
    pub(super) auto_main_count: u32,
}

/// Gather per-child (Size, min, max, is_flex) tuples for the main
/// axis, together with the line's consumed space and auto-margin
/// count.
pub(super) fn collect_main_axis_items(
    dom: &Dom<TuiExt>,
    children: &[NodeId],
    direction: Direction,
    main_budget: u16,
    cross_budget: u16,
) -> MainAxisItems {
    let mut child_info: Vec<ChildMain> = Vec::with_capacity(children.len());
    let mut consumed_fixed: i32 = 0;
    let mut auto_main_count: u32 = 0;

    for &child in children {
        let c = dom
            .node(child)
            .computed_rc()
            .unwrap_or_else(|| std::rc::Rc::new(ComputedStyle::initial()));
        let (main_size, min_raw, max) = match direction {
            Direction::Row => (c.width.clone(), c.min_width, c.max_width),
            Direction::Column => (c.height.clone(), c.min_height, c.max_height),
        };
        // TABLE-COLSYNC-1: a table cell's *used* column width — computed by
        // `size_columns` from the column's author widths + content and stored
        // on the cell's ext (layout output, NOT author `inline_style`) —
        // overrides the normal main-size resolution so every cell in the
        // column lines up. A width drives the Row main axis only.
        let main_size = match (
            direction,
            dom.node(child).ext().and_then(|e| e.table_used_width),
        ) {
            (Direction::Row, Some(w)) => Size::Fixed(w),
            _ => main_size,
        };

        // Main-axis margins (M5.3b). Cells contribute to consumed
        // space; Auto absorbs remaining free space after flex
        // distribution (CSS rule).
        let (main_start_m, main_end_m) = match direction {
            Direction::Row => (c.margin.left.clone(), c.margin.right.clone()),
            Direction::Column => (c.margin.top.clone(), c.margin.bottom.clone()),
        };
        // Resolve margin values (including Calc-with-percent) against
        // the parent's main-axis budget — CSS 2.1 §8.3 always uses
        // width, so `main_budget` is correct for Row main, and we
        // use `cross_budget` for Column main (= parent's width).
        let main_cb_w = match direction {
            Direction::Row => main_budget,
            Direction::Column => cross_budget,
        };
        let margin_consumed = |m: &MarginValue| -> i32 {
            if m.is_auto() {
                0
            } else {
                i32::from(m.resolve(main_cb_w))
            }
        };
        consumed_fixed += margin_consumed(&main_start_m) + margin_consumed(&main_end_m);
        if matches!(main_start_m, MarginValue::Auto) {
            auto_main_count += 1;
        }
        if matches!(main_end_m, MarginValue::Auto) {
            auto_main_count += 1;
        }

        let natural = match &main_size {
            Size::Fixed(n) => MainNatural::Fixed(*n),
            Size::Flex(w) => MainNatural::Flex(*w),
            Size::Percent(p) => {
                // Percent resolves against the parent's main-axis
                // content area at layout time. Treated as a fixed
                // cell value once resolved — does NOT participate
                // in flex weight distribution.
                let resolved =
                    Size::percent_of(main_budget as i32, *p).clamp(0, u16::MAX as i32) as u16;
                MainNatural::Fixed(resolved)
            }
            Size::Calc(expr) => {
                // Calc resolves against the same axis basis as
                // Percent — parent's main-axis content dimension.
                let v = expr.resolve(&rdom_style::calc::ResolveCtx::new(main_budget as i32));
                let resolved = v.max(0).min(u16::MAX as i32) as u16;
                MainNatural::Fixed(resolved)
            }
            Size::Auto => {
                // The container's inner width is definite here, so the
                // item's percent padding / margins resolve against it.
                let intrinsic = intrinsic_size(dom, child, direction, cross_budget, main_cb_w);
                MainNatural::Auto(intrinsic)
            }
        };

        // Resolve `min-width: auto` / `min-height: auto` → intrinsic
        // content size, per CSS Flexbox §4.5. Flex items default to
        // `min-*: auto` even when the author writes nothing —
        // that's the CSS contract. Without this floor, a flex
        // container that overflows would silently shrink its items
        // to zero cells (the M5-MIN-CONTENT-1 substrate bug).
        //
        // **Lazy resolution.** The auto-min only matters during
        // shrink (`total > net_budget`). For the first `clamp_size`
        // pass below, auto-min is mathematically ≤ natural for
        // every Size variant (Flex items have specified_cap = 0;
        // Fixed/Percent/Calc items have natural = specified_cap;
        // Auto items have natural = intrinsic ≥ content-min). So
        // we skip the content walk here — `min` carries only the
        // explicit `Cells(n)` floor for the first pass; the shrink
        // branch resolves Auto on demand for items it actually
        // shrinks.
        //
        // Authors that want strict zero shrink set `min-*: 0`
        // explicitly. Authors that want content-protection on a
        // grow item write the basis explicitly (`flex: 0 1 auto`
        // / `width: auto`) so the specified suggestion is
        // unbounded.
        //
        // Profile evidence: eager resolution added +47% to the
        // full-frame benchmark
        // (`benches/runtime.rs::bench_full_frame`); shrink-only
        // resolution recovers the cost for the non-overflowing
        // case (the common case).
        //
        // v1 approximates CSS min-content with intrinsic natural
        // size; strict min-content (longest-word width with wrap)
        // is a future polish tracked as `M5-MIN-CONTENT-2`.
        let min = match min_raw {
            None => None,
            Some(MinSize::Cells(n)) => Some(n),
            Some(MinSize::Auto) => None,
        };

        if let MainNatural::Fixed(n) | MainNatural::Auto(n) = natural {
            consumed_fixed += i32::from(n);
        }

        // Pre-resolve `Calc` margins to `Cells` here so the placement
        // loop can match on `Cells | Auto` exhaustively. Calc
        // percent resolves against the parent's main-axis width
        // (CSS 2.1 §8.3) — `main_cb_w` computed above.
        let resolve_margin = |m: MarginValue| -> MarginValue {
            match m {
                MarginValue::Auto => MarginValue::Auto,
                MarginValue::Cells(n) => MarginValue::Cells(n),
                MarginValue::Calc(_) => MarginValue::Cells(m.resolve(main_cb_w)),
            }
        };
        child_info.push(ChildMain {
            id: child,
            main: natural,
            min,
            max,
            main_start_margin: resolve_margin(main_start_m),
            main_end_margin: resolve_margin(main_end_m),
        });
    }

    MainAxisItems {
        items: child_info,
        consumed_fixed,
        auto_main_count,
    }
}

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

/// The grow half of §9.7: split `flex_remaining` across `Flex(w)`
/// items by weight, freezing any item its min/max clamps.
fn distribute_grow(child_info: &[ChildMain], final_main: &mut [u16], flex_remaining: u16) {
    let mut frozen: Vec<bool> = child_info
        .iter()
        .map(|ci| !matches!(ci.main, MainNatural::Flex(_)))
        .collect();
    let mut budget: u32 = flex_remaining as u32;
    loop {
        let weight: u32 = child_info
            .iter()
            .zip(frozen.iter())
            .filter(|(_, f)| !**f)
            .map(|(ci, _)| match ci.main {
                MainNatural::Flex(w) => w as u32,
                _ => 0,
            })
            .sum();
        if weight == 0 {
            break;
        }
        // Every share in a pass is computed from the pass-start
        // budget; the budget consumed by items frozen in this pass is
        // subtracted only after the pass (a mid-pass subtraction made
        // later items' shares shrink and falsely froze them at their
        // floors).
        let pass_budget = budget;
        let mut frozen_this_pass: u32 = 0;
        let mut accumulated_weight: u32 = 0;
        let mut accumulated: u32 = 0;
        let mut clamped_any = false;
        for (i, ci) in child_info.iter().enumerate() {
            if frozen[i] {
                continue;
            }
            let MainNatural::Flex(w) = ci.main else {
                continue;
            };
            accumulated_weight = accumulated_weight.saturating_add(w as u32);
            let target = pass_budget
                .saturating_mul(accumulated_weight)
                .checked_div(weight)
                .unwrap_or(0);
            let share = target.saturating_sub(accumulated).min(u16::MAX as u32) as u16;
            accumulated = target;
            let clamped = clamp_size(share, ci.min, ci.max);
            final_main[i] = clamped;
            if clamped != share {
                // Freeze at the clamped size; its budget is spoken for.
                frozen[i] = true;
                frozen_this_pass = frozen_this_pass.saturating_add(clamped as u32);
                clamped_any = true;
            }
        }
        if !clamped_any {
            break;
        }
        budget = budget.saturating_sub(frozen_this_pass);
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
    let shrink_of = |ci: &ChildMain| -> u32 {
        dom.node(ci.id)
            .computed()
            .map(|c| c.flex_shrink as u32)
            .unwrap_or(1)
    };
    // Basis = the size before any shrinking in this loop.
    let basis: Vec<u16> = final_main.to_vec();
    let mut frozen: Vec<bool> = child_info.iter().map(|ci| shrink_of(ci) == 0).collect();
    let mut floors: Vec<Option<u16>> = vec![None; child_info.len()];
    loop {
        let total: i32 = final_main.iter().map(|&n| n as i32).sum();
        if total <= net_budget {
            break;
        }
        let overflow = (total - net_budget) as u32;
        let divisor: u32 = child_info
            .iter()
            .enumerate()
            .filter(|(i, _)| !frozen[*i])
            .map(|(i, ci)| (basis[i] as u32) * shrink_of(ci))
            .sum();
        let Some(divisor) = std::num::NonZeroU32::new(divisor) else {
            break; // nothing left that can shrink
        };
        let mut accumulated_basis: u32 = 0;
        let mut accumulated_shrink: u32 = 0;
        let mut clamped_any = false;
        for (i, ci) in child_info.iter().enumerate() {
            if frozen[i] {
                continue;
            }
            accumulated_basis += (basis[i] as u32) * shrink_of(ci);
            let target_total_shrink =
                ((accumulated_basis as u64 * overflow as u64) / divisor.get() as u64) as u32;
            let my_shrink = target_total_shrink.saturating_sub(accumulated_shrink) as u16;
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
    // CSS §4.5 exception: non-visible overflow drops the floor to 0
    // — items inside a scroll container are allowed to be sized
    // below their content.
    if overflow_on_axis != Overflow::Visible {
        return 0;
    }
    // Specified size suggestion per spec.
    let specified_cap: Option<u16> = match main_size {
        Size::Fixed(n) => Some(*n),
        Size::Percent(p) => {
            Some(Size::percent_of(main_budget as i32, *p).clamp(0, u16::MAX as i32) as u16)
        }
        Size::Calc(expr) => {
            let v = expr.resolve(&rdom_style::calc::ResolveCtx::new(main_budget as i32));
            Some(v.max(0).min(u16::MAX as i32) as u16)
        }
        Size::Flex(_) => Some(0),
        Size::Auto => None,
    };
    // `flex: N` (basis 0%) trivially has specified=0, so auto-min
    // = min(content, 0) = 0. Skip the content walk.
    if matches!(specified_cap, Some(0)) {
        return 0;
    }
    let cb_width = match direction {
        Direction::Row => main_budget,
        Direction::Column => cross_budget,
    };
    let content = content_min_size(dom, id, direction, cross_budget, cb_width);
    match specified_cap {
        Some(cap) => content.min(cap),
        None => content,
    }
}
