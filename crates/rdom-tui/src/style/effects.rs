//! What an element's transform, filter and compositing properties do to
//! its box (CSS Transforms 1 §2–§3, Transforms 2 §6): the one answer
//! layout (the translation, the containing block of positioned
//! descendants), paint and hit-testing (the stacking context) ask.
//!
//! A cell grid draws a transform's translation alone, in whole cells: the
//! box is laid out where it is in flow, then it and its subtree move by
//! the offset (`layout_pass::layout_node`, as a relative offset moves
//! them), so its paint, its hit-testing and its scroll container's
//! scrollable overflow follow, and the boxes around it do not (§3). Every
//! other transform function is inert; any transform other than `none`
//! still makes a stacking context and a containing block (§2).

use crate::layout::{Display, LayoutRect, TablePart};
use crate::style::ComputedStyle;

/// Whether a box styled `c` is transformable (CSS Transforms 1 §3): a
/// block-level or atomic inline-level box, or a table row, row group,
/// cell or caption — not a non-atomic inline box, a table column or
/// column group, nor an element without a box.
pub(crate) fn transformable(c: &ComputedStyle) -> bool {
    match c.display {
        Display::None | Display::Contents => false,
        Display::Inline => !c.flow.is_block_flow(),
        Display::TablePart(TablePart::Column | TablePart::ColumnGroup) => false,
        _ => true,
    }
}

/// Whether the box styled `c` is transformed: a transformable box whose
/// `transform`, `translate`, `rotate` or `scale` is not `none` — even an
/// inert or a zero one (§2).
pub(crate) fn transformed(c: &ComputedStyle) -> bool {
    c.effects.is_transformed() && transformable(c)
}

/// Whether a box styled `c` can take a graphical effect rdom draws by
/// layering its paint — a filter, a blend: a box that is not a non-atomic
/// inline box, whose paint is its block's lines (DIVERGENCES §2).
pub(crate) fn layerable(c: &ComputedStyle) -> bool {
    let boxless = matches!(c.display, Display::None | Display::Contents);
    let inline_box = c.display == Display::Inline && c.flow.is_block_flow();
    !boxless && !inline_box
}

/// Whether the box styled `c` is filtered: its `filter` is not `none`
/// (Filter Effects 1 §5) — `blur()` and `url()` too, which draw nothing.
pub(crate) fn filtered(c: &ComputedStyle) -> bool {
    !c.effects.filter.is_none() && layerable(c)
}

/// Whether the box styled `c` filters its backdrop (Filter Effects 2 §3).
pub(crate) fn backdrop_filtered(c: &ComputedStyle) -> bool {
    !c.effects.backdrop_filter.is_none() && layerable(c)
}

/// Whether the box styled `c` blends with its backdrop: its
/// `mix-blend-mode` is not `normal` (Compositing 1 §3.2).
pub(crate) fn blends(c: &ComputedStyle) -> bool {
    c.effects.mix_blend_mode != rdom_style::layout::BlendMode::Normal && layerable(c)
}

/// Whether the box styled `c` is isolated: `isolation: isolate` (§5.2).
pub(crate) fn isolates(c: &ComputedStyle) -> bool {
    c.effects.isolation == rdom_style::layout::Isolation::Isolate && layerable(c)
}

/// Whether the box styled `c` has a `clip-path` (CSS Masking 1 §5.1) —
/// `url()` and `path()` too, which clip nothing.
pub(crate) fn clip_pathed(c: &ComputedStyle) -> bool {
    c.effects.clip_path != rdom_style::layout::ClipPath::None && layerable(c)
}

/// Whether the element styled `c` establishes a stacking context through
/// its graphical effects: a transform (Transforms 1 §2), a filter or a
/// backdrop filter (Filter Effects 1 §5, 2 §3), blending or isolation
/// (Compositing 1 §3.2, §5.2), a clip path (Masking 1 §5.1).
pub(crate) fn makes_stacking_context(c: &ComputedStyle) -> bool {
    transformed(c)
        || filtered(c)
        || backdrop_filtered(c)
        || blends(c)
        || isolates(c)
        || clip_pathed(c)
}

/// Whether the element styled `c` is the containing block of its
/// absolutely and fixed positioned descendants through its graphical
/// effects: a transform (Transforms 1 §2), a filter or a backdrop filter
/// (Filter Effects 1 §5, 2 §3).
pub(crate) fn contains_positioned(c: &ComputedStyle) -> bool {
    transformed(c) || filtered(c) || backdrop_filtered(c)
}

/// The whole-cell offset the transforms of a box styled `c` move it by,
/// its border box `border` and content box `content` laid out: the exact
/// sum of its `translate` and its `transform`'s translate functions
/// against the reference box (`transform-box`, §7 — the border box, or
/// the content box for `content-box` / `fill-box`), rounded once to
/// whole cells, ties to even (DIVERGENCES §2), within ±`u16::MAX`.
/// `(0, 0)` for a box that is not transformed.
pub(crate) fn translation(
    c: &ComputedStyle,
    border: LayoutRect,
    content: LayoutRect,
) -> (i32, i32) {
    if !transformed(c) {
        return (0, 0);
    }
    let reference = if c.effects.transform_box.is_content_box() {
        content
    } else {
        border
    };
    let (x, y) = c
        .effects
        .translation(i32::from(reference.width), i32::from(reference.height));
    // No grid is larger than `u16::MAX` cells: a farther offset moves the
    // box as far out of sight, and stays clear of the `i32` geometry.
    let max = i32::from(u16::MAX);
    let cells = |v: f64| rdom_style::calc::to_cells(v).clamp(-max, max);
    (cells(x), cells(y))
}
