//! Cross-axis sizing and alignment of flex items — CSS Flexible Box
//! §9.4 (cross size determination), §9.5 / §9.6 (`auto` cross margins
//! and the resulting cross offset), and the `aspect-ratio` derivation
//! of the cross size from the resolved main size (CSS Sizing 4 §3.2).

use rdom_core::Dom;

use crate::ext::TuiExt;
use crate::layout::{Direction, MarginValue, Size, clamp_size};
use crate::render::layout_pass::items::Item;
use crate::style::ComputedStyle;

/// The already-resolved main axis, as the cross resolver sees it, and
/// the container's cross-axis `margin-trim`.
#[derive(Debug, Clone, Copy)]
pub(super) struct ResolvedMain {
    /// Resolved main-axis size (for `aspect-ratio`).
    pub(super) size: u16,
    /// Whether the main size was declared `auto` (aspect-ratio needs an
    /// explicit main size).
    pub(super) was_auto: bool,
    /// The container trims the item's cross-start margin (CSS Box 4
    /// §3.2: every item of a single-line container adjoins it, the
    /// items of a multi-line container's first line).
    pub(super) trim_cross_start: bool,
    /// The container trims the item's cross-end margin.
    pub(super) trim_cross_end: bool,
    /// The cross axis runs from its physical end (`AxisFlip::cross`: a
    /// column under `rtl`, `wrap-reverse`): the item's cross-start
    /// margin is its right (bottom) one.
    pub(super) mirror: bool,
}

/// The cross-axis space an item is placed in: its flex line's cross
/// size, and the container's inner cross size, which percentages
/// resolve against (CSS Flexbox §9.4). A single-line container's line is
/// the container's cross size.
#[derive(Debug, Clone, Copy)]
pub(super) struct CrossSpace {
    pub(super) line: u16,
    /// `None` while the container's cross size is the very thing being
    /// measured (its intrinsic size, §9.9): a percentage is then cyclic
    /// and behaves as `auto` (CSS Sizing 3 §5.2.1).
    pub(super) container: Option<u16>,
}

/// An item's resolved cross-axis extent and its offset from its line's
/// cross-axis start.
pub(super) struct CrossPlacement {
    pub(super) size: u16,
    pub(super) offset: i32,
}

/// The item's cross-start and cross-end margins in the flex-relative
/// frame, as signed cells (`auto` → 0) with which of them is `auto`.
struct CrossMargins {
    start: i32,
    end: i32,
    start_auto: bool,
    end_auto: bool,
}

fn cross_margins(
    computed: &ComputedStyle,
    cb_width: u16,
    direction: Direction,
    main: &ResolvedMain,
) -> CrossMargins {
    let m = &computed.margin;
    let (start, end) = match (direction, main.mirror) {
        (Direction::Row, false) => (&m.top, &m.bottom),
        (Direction::Row, true) => (&m.bottom, &m.top),
        (Direction::Column, false) => (&m.left, &m.right),
        (Direction::Column, true) => (&m.right, &m.left),
    };
    const TRIMMED: MarginValue = MarginValue::Cells(0);
    let start = if main.trim_cross_start {
        &TRIMMED
    } else {
        start
    };
    let end = if main.trim_cross_end { &TRIMMED } else { end };
    // Signed: a negative cross margin starts the box before the line's
    // edge and widens a stretched box (Flexbox §9.4).
    let cells = |m: &MarginValue| -> i32 {
        if m.is_auto() {
            0
        } else {
            i32::from(m.resolve(cb_width))
        }
    };
    CrossMargins {
        start: cells(start),
        end: cells(end),
        start_auto: start.is_auto(),
        end_auto: end.is_auto(),
    }
}

/// Resolve an item's cross size and cross offset in its line from its
/// cross margins and its self-alignment (Flexbox §8.3, §9.4 / §9.5).
///
/// The item's outer cross size includes its cross margins, so a
/// stretched item shrinks by their sum and the start margin offsets
/// the box. `auto` cross margins take the free cross space (both
/// auto → centered), per §8.1, whatever `align` says; otherwise `align`
/// places the item (§8.3) — `safe` aligning an overflowing one as
/// cross-start.
///
/// `main` describes the already-resolved main axis; `aspect-ratio`
/// needs an explicit main size to derive from.
#[allow(clippy::too_many_arguments)]
pub(super) fn place_cross(
    dom: &Dom<TuiExt>,
    item: &Item,
    child_computed: &ComputedStyle,
    container_width: u16,
    space: CrossSpace,
    direction: Direction,
    main: ResolvedMain,
    align: super::align::ItemAlign,
) -> CrossPlacement {
    use super::align::CrossAlign;
    let margins = cross_margins(child_computed, container_width, direction, &main);
    let line_avail =
        (i32::from(space.line) - margins.start - margins.end).clamp(0, i32::from(u16::MAX)) as u16;
    // Flexbox §9.5 / §9.4 step 11: only a `stretch` item with no `auto`
    // cross margin is stretched; any other takes its content size.
    let auto_margin = margins.start_auto || margins.end_auto;
    let stretch = !auto_margin && align.align == CrossAlign::Stretch;
    let cross_size = resolve_cross_size(
        dom,
        item,
        child_computed,
        CrossSpace {
            line: line_avail,
            container: space.container,
        },
        container_width,
        direction,
        MainAxisFacts {
            size: main.size,
            was_auto: main.was_auto,
            stretch,
        },
    );
    let cross_free = i32::from(line_avail.saturating_sub(cross_size));
    // Signed: an item larger than its line overflows it.
    let free = i32::from(space.line) - margins.start - margins.end - i32::from(cross_size);
    let cross_offset: i32 = match (margins.start_auto, margins.end_auto) {
        (true, true) => margins.start + cross_free / 2,
        (true, false) => margins.start + cross_free,
        (false, true) => margins.start,
        // Box Alignment §4.4: `safe` aligns an overflowing item as start.
        _ if align.safe && free < 0 => margins.start,
        _ => match align.align {
            CrossAlign::Stretch | CrossAlign::Start => margins.start,
            CrossAlign::End => margins.start + free,
            // Whole cells: the leading space rounded down.
            CrossAlign::Center => margins.start + free.div_euclid(2),
            CrossAlign::At(offset) => offset,
        },
    };
    CrossPlacement {
        size: cross_size,
        offset: cross_offset,
    }
}

/// An item's outer hypothetical cross size (CSS Flexbox §9.4 step 7):
/// its cross size laid out at its used main size with an `auto` cross
/// size as its content size (nothing stretches yet), plus its cross
/// margins — what a multi-line container's line is as large as.
pub(super) fn hypothetical_outer_cross(
    dom: &Dom<TuiExt>,
    item: &Item,
    container_width: u16,
    space: CrossSpace,
    direction: Direction,
    main: ResolvedMain,
) -> u16 {
    let computed = item.computed(dom);
    let margins = cross_margins(&computed, container_width, direction, &main);
    let size = resolve_cross_size(
        dom,
        item,
        &computed,
        space,
        container_width,
        direction,
        MainAxisFacts {
            size: main.size,
            was_auto: main.was_auto,
            stretch: false,
        },
    );
    (i32::from(size) + margins.start + margins.end).clamp(0, i32::from(u16::MAX)) as u16
}

/// A row item's block-axis geometry for baseline alignment (CSS Flexbox
/// §8.3): its physical top and bottom margins, its hypothetical (not
/// stretched) border-box height, and its first and last baseline rows
/// from its border-box top — its first and last content rows, or, with
/// no content rows, a baseline synthesized at its border box's bottom
/// row (CSS Box Alignment 3 §9.1).
#[derive(Debug, Clone, Copy)]
pub(super) struct BaselineBox {
    pub(super) margin_top: i32,
    pub(super) height: u16,
    pub(super) margin_bottom: i32,
    pub(super) first: u16,
    pub(super) last: u16,
}

impl BaselineBox {
    /// Rows from its margin-box top to its first baseline row.
    pub(super) fn above_first(&self) -> i32 {
        self.margin_top + i32::from(self.first)
    }
    /// Rows from its margin-box top to its last baseline row.
    pub(super) fn above_last(&self) -> i32 {
        self.margin_top + i32::from(self.last)
    }
    /// Its margin box's height.
    pub(super) fn outer(&self) -> i32 {
        self.margin_top + i32::from(self.height) + self.margin_bottom
    }
}

/// Measure `item` (a row item of used width `main.size`) for baseline
/// alignment.
pub(super) fn baseline_box(
    dom: &Dom<TuiExt>,
    item: &Item,
    container_width: u16,
    space: CrossSpace,
    main: ResolvedMain,
) -> BaselineBox {
    let computed = item.computed(dom);
    let margins = cross_margins(&computed, container_width, Direction::Row, &main);
    let (margin_top, margin_bottom) = if main.mirror {
        (margins.end, margins.start)
    } else {
        (margins.start, margins.end)
    };
    let height = resolve_cross_size(
        dom,
        item,
        &computed,
        space,
        container_width,
        Direction::Row,
        MainAxisFacts {
            size: main.size,
            was_auto: main.was_auto,
            stretch: false,
        },
    );
    let synthesized = height.saturating_sub(1);
    let rows = match item {
        Item::Element(id) => crate::render::inline::vertical::content_rows(
            dom,
            *id,
            &computed,
            main.size,
            container_width,
        ),
        Item::Anonymous(anon) => anon.content_rows(dom, main.size, container_width),
    };
    let (first, last) = rows.unwrap_or((synthesized, synthesized));
    // CSS Box Alignment 3 §9.1: "for legacy reasons" a scroll container's
    // last baselines are its block-end margin edge — its scrollbar gutter
    // and clipped content aside (as an inline block's in its line,
    // `inline::vertical::atom_rows`).
    let scrolls = computed.overflow_x != crate::layout::Overflow::Visible
        || computed.overflow_y != crate::layout::Overflow::Visible;
    let last = if scrolls {
        (i32::from(height) + margin_bottom - 1).clamp(0, i32::from(u16::MAX)) as u16
    } else {
        last
    };
    BaselineBox {
        margin_top,
        height,
        margin_bottom,
        first,
        last,
    }
}

/// An inline-level child of the document root. A flex item's `display`
/// is blockified at computed-value time (CSS Display 3 §2.7, the
/// cascade's `blockify`), so an atomic inline here is a child of the
/// document root, which rdom lays out in its viewport column only as a
/// layout device standing in for a browser's `<body>` — whose children
/// are not flex items: an inline block sits in a line at its content
/// width, so it is not stretched.
fn hugs_as_inline_level(computed: &ComputedStyle) -> bool {
    crate::render::box_tree::is_atomic_inline(computed)
}

/// What the cross-axis resolver needs to know about the main axis and
/// the item's margins.
struct MainAxisFacts {
    /// Resolved main-axis size (for `aspect-ratio`).
    size: u16,
    /// Whether the main size was declared `auto` (aspect-ratio needs an
    /// explicit main size).
    was_auto: bool,
    /// `false` when a cross margin is `auto` — the item is not stretched
    /// and takes its content size (Flexbox §9.5).
    stretch: bool,
}

/// Compute the cross-axis size for a child given the container cross
/// budget, the parent's flex direction, and the child's resolved main
/// size. Rules:
///
/// - `Fixed(n)` → `n` (explicit wins).
/// - `Flex(_)` → stretch to fill the cross budget (explicit grow).
/// - `Auto` →
///   - If `aspect-ratio` is set AND the child's main axis was *not*
///     `Auto`, compute cross from main via the ratio (CSS Sizing 4
///     §3.2). Half-to-even rounding to integer cells.
///   - Else, stretched → fill the line; not stretched → its content
///     size at its used main size. A flex item is blockified (CSS
///     Display 3 §2.7, at computed-value time); an inline block among
///     the document root's children, which are not flex items, hugs its
///     content, as in a browser's `<body>`.
///
/// Then clamps by `min` / `max`.
fn resolve_cross_size(
    dom: &Dom<TuiExt>,
    item: &Item,
    computed: &ComputedStyle,
    space: CrossSpace,
    container_width: u16,
    direction: Direction,
    main: MainAxisFacts,
) -> u16 {
    // Percentages resolve against the container's inner cross size; a
    // stretched item fills its line (the space left by its margins).
    let container_cross = space.container;
    let line = space.line;
    // What an `auto`-like size measures against: the container's cross
    // size, or the line's while that is unknown.
    let available = container_cross.unwrap_or(line);
    let MainAxisFacts {
        size: main_size,
        was_auto: main_was_auto,
        stretch,
    } = main;
    let (cross_size, min_raw, max) = match direction {
        Direction::Row => (&computed.height, &computed.min_height, &computed.max_height),
        Direction::Column => (&computed.width, &computed.min_width, &computed.max_width),
    };
    // `min-*` / `max-*` percentages resolve against the container's
    // cross size, as the cross size's own do (CSS Sizing 3 §5.2) — for a
    // row, its height, indefinite when `auto` (CSS 2.1 §10.7: then a
    // `max-height` percentage is `none` and a `min-height` one 0).
    let basis = match direction {
        Direction::Row => container_cross.filter(|_| item.height_basis_is_definite(dom)),
        Direction::Column => container_cross,
    };
    let cross_dir = match direction {
        Direction::Row => Direction::Column,
        Direction::Column => Direction::Row,
    };
    // Declared cross sizes measure the box `box-sizing` names (CSS UI 3
    // §3.1), or are intrinsic keywords measured from the content (CSS
    // Sizing 3 §3.1; a height at the item's resolved width); padding
    // percentages resolve against the container's width.
    let measure_budget = match cross_dir {
        Direction::Column => main_size,
        Direction::Row => available,
    };
    let kw = item.keywords(dom, computed, cross_dir, measure_budget, container_width);
    let sizer = kw.sizer();
    let max = kw.max(max, basis, available);
    // A cross-axis percentage or `calc()` resolves against the
    // container's cross-axis dimension. Only `auto` stretches (Flexbox
    // §9.4): a keyword is its content size.
    let natural = match (cross_size, kw.size(cross_size, container_cross, available)) {
        (_, Some(cells)) => cells,
        (Size::Flex(_), _) if stretch => line,
        _ => {
            if let Some(cross) = computed
                .aspect_ratio
                .filter(|_| !main_was_auto && main_size > 0)
                .and_then(|ratio| {
                    crate::render::layout_pass::box_sizing::aspect_cross_from_main(
                        main_size,
                        ratio,
                        direction,
                        computed,
                        container_width,
                    )
                })
            {
                cross
            } else if stretch && !hugs_as_inline_level(computed) {
                // A flex item is blockified (CSS Display 3 §2.7), so it
                // stretches as a block does.
                line
            } else {
                // Not stretched (an `auto` cross margin, or the
                // hypothetical cross size): its content size, measured
                // at its used main size.
                item.intrinsic_size(dom, cross_dir, measure_budget, container_width)
            }
        }
    };
    // `min-*: auto` — the initial value — is 0 on the cross axis: the
    // automatic minimum size of CSS Flexbox §4.5 is a main-axis rule
    // (a flex distribution can drive an item below its content there),
    // and elsewhere `auto` resolves to 0 (CSS Sizing 3 §5.2). The cross
    // size comes from the declared size, a stretch, or the content.
    let min = kw.min(min_raw, basis, available);
    sizer.floor(clamp_size(natural, min, max))
}
