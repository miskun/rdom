//! Cross-axis sizing and alignment of flex items — CSS Flexible Box
//! §9.4 (cross size determination), §9.5 / §9.6 (`auto` cross margins
//! and the resulting cross offset), and the `aspect-ratio` derivation
//! of the cross size from the resolved main size (CSS Sizing 4 §3.2).

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::{AspectRatio, Direction, Display, MarginValue, Size, clamp_size};
use crate::render::layout_pass::block::nearest_block_ancestor_height_is_definite;
use crate::render::layout_pass::box_sizing::Sizer;
use crate::render::layout_pass::intrinsic::{Keywords, intrinsic_size};
use crate::style::ComputedStyle;

/// The already-resolved main axis, as the cross resolver sees it, and
/// the container's cross-axis `margin-trim`.
pub(super) struct ResolvedMain {
    /// Resolved main-axis size (for `aspect-ratio`).
    pub(super) size: u16,
    /// Whether the main size was declared `auto` (aspect-ratio needs an
    /// explicit main size).
    pub(super) was_auto: bool,
    /// The container trims the item's cross-start margin (CSS Box 4
    /// §3.2: every item of a single-line container adjoins it).
    pub(super) trim_cross_start: bool,
    /// The container trims the item's cross-end margin.
    pub(super) trim_cross_end: bool,
}

/// An item's resolved cross-axis extent and its offset from the
/// container's cross-axis start.
pub(super) struct CrossPlacement {
    pub(super) size: u16,
    pub(super) offset: i32,
}

/// Resolve an item's cross size and cross offset from its cross
/// margins (Flexbox §9.4 / §9.5).
///
/// The item's outer cross size includes its cross margins, so a
/// stretched item shrinks by their sum and the start margin offsets
/// the box. `auto` cross margins take the free cross space (both
/// auto → centered), per §9.5.
///
/// `main` describes the already-resolved main axis; `aspect-ratio`
/// needs an explicit main size to derive from.
pub(super) fn place_cross(
    dom: &Dom<TuiExt>,
    child_id: NodeId,
    child_computed: &ComputedStyle,
    container_width: u16,
    cross_budget: u16,
    direction: Direction,
    main: ResolvedMain,
) -> CrossPlacement {
    let cb_width = container_width;
    let (cross_start_m, cross_end_m) = match direction {
        Direction::Row => (&child_computed.margin.top, &child_computed.margin.bottom),
        Direction::Column => (&child_computed.margin.left, &child_computed.margin.right),
    };
    const TRIMMED: MarginValue = MarginValue::Cells(0);
    let cross_start_m = if main.trim_cross_start {
        &TRIMMED
    } else {
        cross_start_m
    };
    let cross_end_m = if main.trim_cross_end {
        &TRIMMED
    } else {
        cross_end_m
    };
    // Signed: a negative cross margin starts the box before the
    // container's edge and widens a stretched box (Flexbox §9.4).
    let cross_cells = |m: &MarginValue| -> i32 {
        if m.is_auto() {
            0
        } else {
            i32::from(m.resolve(cb_width))
        }
    };
    let cross_start_cells = cross_cells(cross_start_m);
    let cross_end_cells = cross_cells(cross_end_m);
    let cross_avail = (i32::from(cross_budget) - cross_start_cells - cross_end_cells)
        .clamp(0, i32::from(u16::MAX)) as u16;
    // Flexbox §9.5: an item with an `auto` cross margin is not
    // stretched — it takes its content size and the margins absorb
    // the free space.
    let stretch = !(cross_start_m.is_auto() || cross_end_m.is_auto());
    let cross_size = resolve_cross_size(
        dom,
        child_id,
        child_computed,
        cross_avail,
        container_width,
        direction,
        MainAxisFacts {
            size: main.size,
            was_auto: main.was_auto,
            stretch,
        },
    );
    let cross_free = i32::from(cross_avail.saturating_sub(cross_size));
    let cross_offset: i32 = match (cross_start_m.is_auto(), cross_end_m.is_auto()) {
        (true, true) => cross_start_cells + cross_free / 2,
        (true, false) => cross_start_cells + cross_free,
        _ => cross_start_cells,
    };
    CrossPlacement {
        size: cross_size,
        offset: cross_offset,
    }
}

/// Compute the cross-axis cell count from the main-axis cell count and
/// an `aspect-ratio` value (CSS Sizing 4 §5.1). `Row` direction: cross
/// is height, so `height = width * h / w`. `Column` direction: cross is
/// width, so `width = height * w / h`. The ratio sizes the border box
/// (rdom's box-sizing box), or the content box for `auto && <ratio>` —
/// the main size's padding and border come off first and the cross
/// size's are added back. Half-to-even rounding to integer cells.
/// `None` for a degenerate ratio, which behaves as `auto`.
fn aspect_cross_from_main(
    main: u16,
    ratio: AspectRatio,
    direction: Direction,
    computed: &ComputedStyle,
    cb_width: u16,
) -> Option<u16> {
    let r = ratio.value()?;
    // (main-axis, cross-axis) padding + border, for the content box.
    let main_sizer = Sizer::along(computed, direction, cb_width);
    let (main_edges, cross_edges) = if ratio.auto() || main_sizer.is_content_box() {
        let horizontal = Sizer::horizontal(computed, cb_width).chrome();
        let vertical = Sizer::vertical(computed, cb_width).chrome();
        match direction {
            Direction::Row => (horizontal, vertical),
            Direction::Column => (vertical, horizontal),
        }
    } else {
        (0, 0)
    };
    let main = f32::from(main.saturating_sub(main_edges));
    let cross_f = match direction {
        Direction::Row => main / r,
        Direction::Column => main * r,
    };
    let cross = if cross_f.is_finite() {
        cross_f.max(0.0).round_ties_even().min(f32::from(u16::MAX)) as u16
    } else {
        0
    };
    Some(cross.saturating_add(cross_edges))
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
///   - Else if `display: inline-block` → intrinsic content size on the
///     cross axis.
///   - Else → stretch to fill the cross budget.
///
/// Then clamps by `min` / `max`.
fn resolve_cross_size(
    dom: &Dom<TuiExt>,
    child_id: NodeId,
    computed: &ComputedStyle,
    container_cross: u16,
    container_width: u16,
    direction: Direction,
    main: MainAxisFacts,
) -> u16 {
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
        Direction::Row => {
            nearest_block_ancestor_height_is_definite(dom, child_id).then_some(container_cross)
        }
        Direction::Column => Some(container_cross),
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
        Direction::Row => container_cross,
    };
    let kw = Keywords::new(
        dom,
        child_id,
        computed,
        cross_dir,
        measure_budget,
        container_width,
    );
    let sizer = kw.sizer();
    let max = kw.max(max, basis, container_cross);
    // A cross-axis percentage or `calc()` resolves against the
    // container's cross-axis dimension. Only `auto` stretches (Flexbox
    // §9.4): a keyword is its content size.
    let natural = match (
        cross_size,
        kw.size(cross_size, Some(container_cross), container_cross),
    ) {
        (_, Some(cells)) => cells,
        (Size::Flex(_), _) => container_cross,
        _ => {
            if let Some(cross) = computed
                .aspect_ratio
                .filter(|_| !main_was_auto && main_size > 0)
                .and_then(|ratio| {
                    aspect_cross_from_main(main_size, ratio, direction, computed, container_width)
                })
            {
                cross
            } else if computed.display == Display::InlineBlock {
                // Cross-axis intrinsic measurement. `intrinsic_size`'s
                // `direction` argument means "measure along this axis";
                // we want the axis perpendicular to the parent's flex
                // direction. The `cross_budget` argument passed to
                // `intrinsic_size` is for IFC wrap; for the inline-
                // block's own cross-axis sizing we pass the container
                // cross size — a conservative budget that's correct
                // for non-IFC inline-blocks (the common case).
                intrinsic_size(dom, child_id, cross_dir, container_cross, container_width)
            } else if stretch {
                container_cross
            } else {
                // `auto` cross margin: content size, not stretch.
                intrinsic_size(dom, child_id, cross_dir, container_cross, container_width)
            }
        }
    };
    // `min-*: auto` — the initial value — is 0 on the cross axis: the
    // automatic minimum size of CSS Flexbox §4.5 is a main-axis rule
    // (a flex distribution can drive an item below its content there),
    // and elsewhere `auto` resolves to 0 (CSS Sizing 3 §5.2). The cross
    // size comes from the declared size, a stretch, or the content.
    let min = kw.min(min_raw, basis, container_cross);
    sizer.floor(clamp_size(natural, min, max))
}
