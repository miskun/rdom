//! Main-axis sizing of flex items — CSS Flexible Box §9.2.
//!
//! Owns the per-item main-size bookkeeping ([`ChildMain`]) and the
//! gathering pass that turns each item's `flex-basis`, main size and
//! main-axis margins into its flex base size and flex factors
//! ([`collect_main_axis_items`]; §9.2 step 3). The §9.7 resolution of
//! the flexible lengths is `distribute`.

use rdom_core::Dom;

use super::item::FlexItem;
use crate::ext::TuiExt;
use crate::layout::{Direction, FlexBasis, MarginValue, Size, clamp_size};
use crate::render::layout_pass::intrinsic::Keywords;
use crate::render::layout_pass::margin_trim::FlexTrim;

/// Per-item main-axis inputs gathered before distribution.
pub(super) struct ChildMain {
    pub(super) item: FlexItem,
    /// The flex base size (§9.2 step 3), a border box in cells.
    pub(super) base: u16,
    /// The inner flex base size: the base less the item's padding and
    /// border on the main axis — what §9.7 scales the shrink factor by.
    pub(super) inner_base: u16,
    /// `flex-grow` (`width: <n>fr`, rdom's grow, when `flex-grow` is 0).
    pub(super) grow: f32,
    /// `flex-shrink`.
    pub(super) shrink: f32,
    /// The base is the item's content size (`content`, or `auto` with an
    /// `auto` main size): its automatic minimum (§4.5) is no larger, so
    /// it cannot raise a size at or above the base.
    pub(super) content_base: bool,
    /// The base is the item's definite main size property (`flex-basis:
    /// auto` with a definite `width` / `height`) — the specified size
    /// suggestion its automatic minimum (§4.5) never exceeds, so it
    /// cannot raise a size at or above the base either.
    pub(super) specified_base: bool,
    /// The §4.5 automatic minimum, resolved on first use and kept for
    /// line breaking and every freeze-loop iteration ([`Self::auto_min`]).
    auto_min: std::cell::Cell<Option<u16>>,
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
    /// A collapsed item's strut size (CSS Flexbox §9.4 step 10): the
    /// cross size of its line laid out uncollapsed — `None` for every
    /// other item. A strut has no main size and the rest of the
    /// algorithm ignores it (no gap, no `justify-content` share, no
    /// baseline) but for its line's cross size, at least this.
    pub(super) strut: Option<u16>,
}

impl ChildMain {
    /// The item's §4.5 automatic minimum along `direction` in a
    /// container of content size `budgets`, resolved once
    /// (`distribute::resolve_auto_min`) — "in all cases […] clamped by
    /// the maximum main size if it's definite".
    pub(super) fn auto_min(
        &self,
        dom: &Dom<TuiExt>,
        direction: Direction,
        budgets: MainBudgets,
    ) -> u16 {
        if let Some(v) = self.auto_min.get() {
            return v;
        }
        let v = super::distribute::resolve_auto_min(
            dom,
            &self.item,
            direction,
            budgets.main,
            budgets.cross,
        );
        let v = self.max.map_or(v, |max| v.min(max));
        self.auto_min.set(Some(v));
        v
    }

    /// Make the item a strut of cross size `size` (§4.4, §9.4 step 10):
    /// no main size, flex factors, min / max or main-axis margins.
    pub(super) fn make_strut(&mut self, size: u16) {
        self.base = 0;
        self.inner_base = 0;
        self.grow = 0.0;
        self.shrink = 0.0;
        self.content_base = false;
        self.specified_base = false;
        self.auto_min = std::cell::Cell::new(None);
        self.main_auto = false;
        self.min = Some(0);
        self.max = Some(0);
        self.main_start_margin = MarginValue::Cells(0);
        self.main_end_margin = MarginValue::Cells(0);
        self.strut = Some(size);
    }

    /// The item's min main size: its `min-*`, else its automatic
    /// minimum.
    pub(super) fn min_main(
        &self,
        dom: &Dom<TuiExt>,
        direction: Direction,
        budgets: MainBudgets,
    ) -> u16 {
        self.min
            .unwrap_or_else(|| self.auto_min(dom, direction, budgets))
    }

    /// The item's min main size as it bears on a size at or above its
    /// base: `None` where the automatic minimum cannot raise such a size
    /// — a content-sized or specified base is never below it (and it is
    /// clamped by the `max-*` the size is clamped by too) — so it is not
    /// resolved there.
    pub(super) fn min_main_above_base(
        &self,
        dom: &Dom<TuiExt>,
        direction: Direction,
        budgets: MainBudgets,
    ) -> Option<u16> {
        match self.min {
            Some(m) => Some(m),
            None if self.content_base || self.specified_base => None,
            None => Some(self.auto_min(dom, direction, budgets)),
        }
    }

    /// The hypothetical main size (§9.3, §9.7 step 1): the base clamped
    /// by the min and max main sizes, the automatic minimum included.
    pub(super) fn hypothetical(
        &self,
        dom: &Dom<TuiExt>,
        direction: Direction,
        budgets: MainBudgets,
    ) -> u16 {
        clamp_size(
            self.base,
            self.min_main_above_base(dom, direction, budgets),
            self.max,
        )
    }
}

/// The container's content size on the main and cross axes.
#[derive(Debug, Clone, Copy)]
pub(super) struct MainBudgets {
    pub(super) main: u16,
    pub(super) cross: u16,
}

/// Gather each item's flex base size, factors, min and max and its
/// main-axis margins, one [`ChildMain`] per item (each line sums its
/// own margins, `lines::resolve_line_main`).
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
    children: &[FlexItem],
    direction: Direction,
    budgets: MainBudgets,
    trim: FlexTrim,
    mirror: bool,
) -> Vec<ChildMain> {
    let MainBudgets {
        main: main_budget,
        cross: cross_budget,
    } = budgets;
    let mut child_info: Vec<ChildMain> = Vec::with_capacity(children.len());
    // The basis `min-*` / `max-*` percentages resolve against: the
    // container's main size — for a column, its height, which is
    // indefinite when it is `auto` (CSS 2.1 §10.7: then a `max-height`
    // percentage is `none` and a `min-height` one 0, as in block flow).
    let main_basis = match direction {
        Direction::Row => Some(main_budget),
        Direction::Column => children
            .first()
            .is_none_or(|c| c.height_basis_is_definite(dom))
            .then_some(main_budget),
    };

    for (i, item) in children.iter().enumerate() {
        let c = item.computed(dom);
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
        let kw = item.keywords(dom, &c, direction, cross_budget, main_cb_w);
        // `min-*` / `max-*` percentages resolve against the container's
        // main size, as the main size's own do (CSS Sizing 3 §5.2).
        let max = kw.max(max, main_basis, main_budget);
        // TABLE-COLSYNC-1: a table cell's *used* column width — computed by
        // `size_columns` from the column's author widths + content and stored
        // on the cell's ext (layout output, NOT author `inline_style`) —
        // overrides the normal main-size resolution so every cell in the
        // column lines up. A width drives the Row main axis only.
        let used_column_width = match direction {
            Direction::Row => item
                .node()
                .and_then(|id| dom.node(id).ext().and_then(|e| e.table_used_width)),
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
        let content = || item.intrinsic_size(dom, direction, cross_budget, main_cb_w);
        let main_auto = matches!(main_size, Size::Auto | Size::Intrinsic(_));
        let mut grow = c.flex_grow;
        let mut specified_base = false;
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
            // §9.2 step 3.B: a used flex basis of `content` with a preferred
            // aspect ratio and a definite cross size is the cross size
            // through the ratio.
            let from_ratio = || aspect_base(dom, item, &c, direction, budgets, main_cb_w);
            match (basis_size, main_size) {
                // `content`: the max-content size, whatever the main
                // size property says (§9.2 step 3.E).
                (Some(Size::Auto), _) => from_ratio().map_or_else(
                    || {
                        (
                            item.content_extreme(dom, direction, cross_budget, main_cb_w, true),
                            true,
                        )
                    },
                    |b| (b, false),
                ),
                (Some(b), _) => match used_size(&b, main_basis) {
                    Some(cells) => (cells, false),
                    None => from_ratio().map_or_else(|| (content(), true), |b| (b, false)),
                },
                (None, Size::Flex(w)) => {
                    if grow <= 0.0 {
                        grow = *w;
                    }
                    (0, false)
                }
                (None, size) => match used_size(size, Some(main_budget)) {
                    Some(cells) => {
                        specified_base = true;
                        (cells, false)
                    }
                    None => from_ratio().map_or_else(|| (content(), true), |b| (b, false)),
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
        // **Lazy resolution.** `min` carries only an explicit floor; the
        // automatic minimum is resolved on first use and cached
        // (`ChildMain::auto_min`), and only where it can bind: a
        // content-sized base is at least the content size suggestion and
        // a specified base is the specified size suggestion, so neither
        // needs it for its hypothetical size or for growing — the common
        // case. A base it can raise (`flex: 1` — a basis of 0 under an
        // `auto` width) takes it into its hypothetical main size, as
        // §9.7 step 1 requires; shrinking takes it everywhere.
        //
        // Profile evidence: eager resolution added +47% to the
        // full-frame benchmark
        // (`benches/runtime.rs::bench_full_frame`).
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
            item: item.clone(),
            base,
            inner_base: base.saturating_sub(kw.sizer().chrome()),
            grow,
            shrink: c.flex_shrink,
            content_base,
            specified_base,
            auto_min: std::cell::Cell::new(None),
            main_auto,
            min,
            max,
            main_start_margin: resolve_margin(main_start_m),
            main_end_margin: resolve_margin(main_end_m),
            strut: None,
        });
    }

    child_info
}

/// §9.2 step 3.B: the flex base size of an item with a preferred aspect
/// ratio, a used flex basis of `content` and a definite cross size — a
/// length, or a percentage of a definite container cross size — is that
/// cross size through the ratio (CSS Sizing 4 §5.1, the ratio sizing the
/// box `box-sizing` names). `None` when it does not apply.
fn aspect_base(
    dom: &Dom<TuiExt>,
    item: &FlexItem,
    c: &crate::style::ComputedStyle,
    direction: Direction,
    budgets: MainBudgets,
    cb_width: u16,
) -> Option<u16> {
    let FlexItem::Element(id) = item else {
        return None;
    };
    let ratio = c.aspect_ratio?;
    let (cross_dir, cross_size) = match direction {
        Direction::Row => (Direction::Column, &c.height),
        Direction::Column => (Direction::Row, &c.width),
    };
    // An intrinsic keyword or `auto` is no definite cross size.
    if matches!(cross_size, Size::Auto | Size::Intrinsic(_) | Size::Flex(_)) {
        return None;
    }
    let basis = match direction {
        Direction::Row => item.height_basis_is_definite(dom).then_some(budgets.cross),
        Direction::Column => Some(budgets.cross),
    };
    let cross = Keywords::new(dom, *id, c, cross_dir, budgets.main, cb_width).size(
        cross_size,
        basis,
        budgets.cross,
    )?;
    super::cross::aspect_cross_from_main(cross, ratio, cross_dir, c, cb_width)
}
