//! Cross-axis sizing and alignment of flex items — CSS Flexible Box
//! §9.4 (cross size determination), §9.5 / §9.6 (`auto` cross margins
//! and the resulting cross offset), and the `aspect-ratio` derivation
//! of the cross size from the resolved main size (CSS Sizing 4 §3.2).

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::{AspectRatio, Direction, Display, MarginValue, MinSize, Size, clamp_size};
use crate::render::layout_pass::intrinsic::intrinsic_size;
use crate::style::ComputedStyle;

/// The already-resolved main axis, as the cross resolver sees it.
pub(super) struct ResolvedMain {
    /// Resolved main-axis size (for `aspect-ratio`).
    pub(super) size: u16,
    /// Whether the main size was declared `auto` (aspect-ratio needs an
    /// explicit main size).
    pub(super) was_auto: bool,
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
/// an `aspect-ratio: w/h` value. `Row` direction: cross is height, so
/// `height = width * h / w`. `Column` direction: cross is width, so
/// `width = height * w / h`. Half-to-even rounding to integer cells.
fn aspect_cross_from_main(main: u16, ratio: AspectRatio, direction: Direction) -> u16 {
    let r = ratio.as_f32();
    let cross_f = match direction {
        Direction::Row => (main as f32) / r,
        Direction::Column => (main as f32) * r,
    };
    if cross_f.is_finite() {
        cross_f.max(0.0).round_ties_even() as u16
    } else {
        0
    }
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
        Direction::Row => (&computed.height, computed.min_height, computed.max_height),
        Direction::Column => (&computed.width, computed.min_width, computed.max_width),
    };
    let cross_dir = match direction {
        Direction::Row => Direction::Column,
        Direction::Column => Direction::Row,
    };
    let natural = match cross_size {
        Size::Fixed(n) => *n,
        Size::Flex(_) => container_cross,
        Size::Percent(p) => {
            // Cross-axis percent resolves against the container's
            // cross-axis dimension.
            Size::percent_of(container_cross as i32, *p).clamp(0, u16::MAX as i32) as u16
        }
        Size::Calc(expr) => {
            let v = expr.resolve(&rdom_style::calc::ResolveCtx::new(container_cross as i32));
            v.max(0).min(u16::MAX as i32) as u16
        }
        Size::Auto => {
            if let Some(ratio) = computed.aspect_ratio
                && !main_was_auto
                && main_size > 0
            {
                aspect_cross_from_main(main_size, ratio, direction)
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
    // Cross axis intentionally does NOT carry an auto-min content
    // floor. CSS Flexbox §4.5's content-based min applies to the
    // MAIN axis only, where a flex distribution can drive an item
    // below its content size. The cross axis in rdom has no shrink
    // distribution — items either stretch to fill the container
    // (`Size::Auto` / `Flex`), take a declared value (`Fixed` /
    // `Percent` / `Calc`), or honor an explicit `min-*: auto` opt-
    // in. There is no path that would silently collapse cross-axis
    // sizes, so no floor is needed. Adding one would force items
    // to GROW past their natural cross size — which would break
    // IFC wrap (a narrow column container's wider-content child
    // would balloon to its content width, defeating the wrap).
    let min = match min_raw {
        None => None,
        Some(MinSize::Cells(n)) => Some(n),
        Some(MinSize::Auto) => Some(intrinsic_size(
            dom,
            child_id,
            cross_dir,
            container_cross,
            container_width,
        )),
    };
    clamp_size(natural, min, max)
}
