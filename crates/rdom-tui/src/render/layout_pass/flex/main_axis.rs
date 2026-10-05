//! Main-axis sizing of flex items — CSS Flexible Box §9.2.
//!
//! Owns the per-item main-size bookkeeping ([`ChildMain`]) and the
//! gathering pass that turns each item's `flex-basis`, main size and
//! main-axis margins into its flex base size and flex factors
//! ([`collect_main_axis_items`]; §9.2 step 3). The §9.7 resolution of
//! the flexible lengths is `distribute`.

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::{Direction, FlexBasis, MarginValue, Size};
use crate::node::TuiNodeExt;
use crate::render::layout_pass::block::nearest_block_ancestor_height_is_definite;
use crate::render::layout_pass::intrinsic::{Keywords, content_max_size, intrinsic_size};
use crate::render::layout_pass::margin_trim::FlexTrim;
use crate::style::ComputedStyle;

/// Per-item main-axis inputs gathered before distribution.
pub(super) struct ChildMain {
    pub(super) id: NodeId,
    /// The flex base size (§9.2 step 3), a border box in cells.
    pub(super) base: u16,
    /// `flex-grow` (`width: <n>fr`, rdom's grow, when `flex-grow` is 0).
    pub(super) grow: f32,
    /// `flex-shrink`.
    pub(super) shrink: f32,
    /// The base is the item's content size (`content`, or `auto` with an
    /// `auto` main size): its automatic minimum (§4.5) is no larger, so
    /// growing cannot violate it.
    pub(super) content_base: bool,
    /// The main size property is `auto` (the cross pass then derives no
    /// size from `aspect-ratio`).
    pub(super) main_auto: bool,
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

/// The container's content size on the main and cross axes.
pub(super) struct MainBudgets {
    pub(super) main: u16,
    pub(super) cross: u16,
}

/// The gathered flex line: one [`ChildMain`] per item plus the totals
/// the free-space computation needs.
pub(super) struct MainAxisItems {
    pub(super) items: Vec<ChildMain>,
    /// The items' non-`auto` main-axis margins. Signed: a negative
    /// margin frees main-axis space (Flexbox §9.7 counts outer sizes;
    /// CSS margins may be negative).
    pub(super) margins: i32,
    /// Number of `auto` main-axis margins across the line.
    pub(super) auto_main_count: u32,
}

/// Gather each item's flex base size, factors, min and max for the main
/// axis, together with the line's margins and auto-margin count.
///
/// `trim` is the container's `margin-trim` (CSS Box 4 §3.2): a trimmed
/// main-start (main-end) edge zeroes the first (last) item's margin
/// there.
///
/// `mirror`: the main axis runs from its physical end (`AxisFlip::main`:
/// a row under `rtl` or `row-reverse`, a `column-reverse`) — the item's
/// main-start margin is its right (bottom) one.
pub(super) fn collect_main_axis_items(
    dom: &Dom<TuiExt>,
    children: &[NodeId],
    direction: Direction,
    budgets: MainBudgets,
    trim: FlexTrim,
    mirror: bool,
) -> MainAxisItems {
    let MainBudgets {
        main: main_budget,
        cross: cross_budget,
    } = budgets;
    let mut child_info: Vec<ChildMain> = Vec::with_capacity(children.len());
    let mut margins: i32 = 0;
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

    for (i, &child) in children.iter().enumerate() {
        // Flexbox §4.4: a collapsed item is a strut — zero main size and
        // no main-axis margins; the cross pass keeps its cross size, which
        // holds the line's.
        if super::is_collapsed(dom, child) {
            child_info.push(ChildMain {
                id: child,
                base: 0,
                grow: 0.0,
                shrink: 0.0,
                content_base: false,
                main_auto: false,
                min: Some(0),
                max: Some(0),
                main_start_margin: MarginValue::Cells(0),
                main_end_margin: MarginValue::Cells(0),
            });
            continue;
        }
        let c = dom
            .node(child)
            .computed_rc()
            .unwrap_or_else(|| std::rc::Rc::new(ComputedStyle::initial()));
        let (main_size, min_raw, max) = match direction {
            Direction::Row => (&c.width, &c.min_width, &c.max_width),
            Direction::Column => (&c.height, &c.min_height, &c.max_height),
        };
        // Margin and padding percentages resolve against the parent's
        // width — CSS 2.1 §8.3 always uses width, so `main_budget` for
        // a Row main axis and `cross_budget` (the parent's width) for a
        // Column one.
        let main_cb_w = match direction {
            Direction::Row => main_budget,
            Direction::Column => cross_budget,
        };
        // Declared main sizes measure the box `box-sizing` names (CSS UI
        // 3 §3.1), or are intrinsic keywords measured from the content
        // (CSS Sizing 3 §3.1, `fit-content` against the container's main
        // size); `kw` turns each into the border box the line distributes.
        let kw = Keywords::new(dom, child, &c, direction, cross_budget, main_cb_w);
        // `min-*` / `max-*` percentages resolve against the container's
        // main size, as the main size's own do (CSS Sizing 3 §5.2).
        let max = kw.max(max, main_basis, main_budget);
        // TABLE-COLSYNC-1: a table cell's *used* column width — computed by
        // `size_columns` from the column's author widths + content and stored
        // on the cell's ext (layout output, NOT author `inline_style`) —
        // overrides the normal main-size resolution so every cell in the
        // column lines up. A width drives the Row main axis only.
        let used_column_width = match direction {
            Direction::Row => dom.node(child).ext().and_then(|e| e.table_used_width),
            Direction::Column => None,
        };

        // Main-axis margins (M5.3b). Cells contribute to consumed
        // space; Auto absorbs remaining free space after flex
        // distribution (CSS rule).
        let (main_start_m, main_end_m) = match direction {
            Direction::Row if mirror => (c.margin.right.clone(), c.margin.left.clone()),
            Direction::Row => (c.margin.left.clone(), c.margin.right.clone()),
            Direction::Column if mirror => (c.margin.bottom.clone(), c.margin.top.clone()),
            Direction::Column => (c.margin.top.clone(), c.margin.bottom.clone()),
        };
        let main_start_m = if trim.main_start && i == 0 {
            MarginValue::Cells(0)
        } else {
            main_start_m
        };
        let main_end_m = if trim.main_end && i + 1 == children.len() {
            MarginValue::Cells(0)
        } else {
            main_end_m
        };
        let margin_consumed = |m: &MarginValue| -> i32 {
            if m.is_auto() {
                0
            } else {
                i32::from(m.resolve(main_cb_w))
            }
        };
        margins += margin_consumed(&main_start_m) + margin_consumed(&main_end_m);
        if matches!(main_start_m, MarginValue::Auto) {
            auto_main_count += 1;
        }
        if matches!(main_end_m, MarginValue::Auto) {
            auto_main_count += 1;
        }

        // CSS Flexbox §9.2 step 3, the flex base size. A definite
        // `flex-basis` is it (measuring the box `box-sizing` names, as
        // `width` does; a percentage against the container's inner main
        // size, and as `content` when that is indefinite); `auto` takes
        // the main size property; `content`, or `auto` with an `auto`
        // main size, the item's content size. `width: <n>fr` (rdom) is
        // a basis of 0 growing by `n`. A table cell's used column width
        // (TABLE-COLSYNC-1) is its base and does not grow. The result
        // is a fixed cell value — percentages resolve here, not in the
        // distribution.
        let used_size = |size: &Size, basis: Option<u16>| match (direction, size) {
            // A keyword height is the content height, as `auto` is (CSS
            // Sizing 3 §3.1).
            (Direction::Column, Size::Intrinsic(_)) => None,
            _ => kw.size(size, basis, main_budget),
        };
        let content = || intrinsic_size(dom, child, direction, cross_budget, main_cb_w);
        let main_auto = matches!(main_size, Size::Auto | Size::Intrinsic(_));
        let mut grow = c.flex_grow;
        let (base, content_base) = if let Some(w) = used_column_width {
            grow = 0.0;
            (w, false)
        } else {
            let basis_size = match &c.flex_basis {
                FlexBasis::Auto => None,
                FlexBasis::Content => Some(Size::Auto),
                FlexBasis::Cells(n) => Some(Size::Fixed(*n)),
                FlexBasis::Calc(e) => Some(Size::Calc(e.clone())),
                FlexBasis::Intrinsic(k) => Some(Size::Intrinsic(k.clone())),
            };
            match (basis_size, main_size) {
                // `content`: the max-content size, whatever the main
                // size property says (§9.2 step 3.E).
                (Some(Size::Auto), _) => (
                    content_max_size(dom, child, direction, cross_budget, main_cb_w),
                    true,
                ),
                (Some(b), _) => match used_size(&b, main_basis) {
                    Some(cells) => (cells, false),
                    None => (content(), true),
                },
                (None, Size::Flex(w)) => {
                    if grow <= 0.0 {
                        grow = *w;
                    }
                    (0, false)
                }
                (None, size) => match used_size(size, Some(main_budget)) {
                    Some(cells) => (cells, false),
                    None => (content(), true),
                },
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
        let min = kw.min(min_raw, main_basis, main_budget);

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
            base,
            grow,
            shrink: c.flex_shrink,
            content_base,
            main_auto,
            min,
            max,
            main_start_margin: resolve_margin(main_start_m),
            main_end_margin: resolve_margin(main_end_m),
        });
    }

    MainAxisItems {
        items: child_info,
        margins,
        auto_main_count,
    }
}
