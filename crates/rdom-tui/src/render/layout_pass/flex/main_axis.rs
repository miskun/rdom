//! Main-axis sizing of flex items — CSS Flexible Box §9.2.
//!
//! Owns the per-item main-size bookkeeping ([`ChildMain`],
//! [`MainNatural`]) and the gathering pass that turns each item's
//! declared size and main-axis margins into a natural size plus the
//! line's consumed space ([`collect_main_axis_items`]). The §9.7
//! distribution of the free space is `distribute`.

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::{Direction, MarginValue, MinSize, Size};
use crate::node::TuiNodeExt;
use crate::render::layout_pass::block::nearest_block_ancestor_height_is_definite;
use crate::render::layout_pass::intrinsic::intrinsic_size;
use crate::style::ComputedStyle;

/// An item's main size before flexible-length resolution.
pub(super) enum MainNatural {
    Fixed(u16),
    Flex(f32),
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
    // The basis `min-*` / `max-*` percentages resolve against: the
    // container's main size — for a column, its height, which is
    // indefinite when it is `auto` (CSS 2.1 §10.7: then a `max-height`
    // percentage is `none` and a `min-height` one 0, as in block flow).
    let main_basis = match direction {
        Direction::Row => Some(main_budget),
        Direction::Column => children
            .first()
            .is_none_or(|&c| nearest_block_ancestor_height_is_definite(dom, c))
            .then_some(main_budget),
    };

    for &child in children {
        let c = dom
            .node(child)
            .computed_rc()
            .unwrap_or_else(|| std::rc::Rc::new(ComputedStyle::initial()));
        let (main_size, min_raw, max) = match direction {
            Direction::Row => (c.width.clone(), &c.min_width, &c.max_width),
            Direction::Column => (c.height.clone(), &c.min_height, &c.max_height),
        };
        // `min-*` / `max-*` percentages resolve against the container's
        // main size, as the main size's own do (CSS Sizing 3 §5.2).
        let max = max.cells(main_basis);
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

        // A percentage or `calc()` resolves against the parent's
        // main-axis content area at layout time, and is a fixed cell
        // value once resolved — it does NOT take part in flex weight
        // distribution.
        let natural = match (&main_size, main_size.cells(Some(main_budget))) {
            (Size::Flex(w), _) => MainNatural::Flex(*w),
            (_, Some(cells)) => MainNatural::Fixed(cells),
            _ => {
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
            None | Some(MinSize::Auto) => None,
            Some(m) => m.cells(main_basis),
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
