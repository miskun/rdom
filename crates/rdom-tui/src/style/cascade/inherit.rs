//! Inheritance + layout-change diff helpers used by the cascade walk.
//!
//! `inherit_inheritable_from` seeds the working `ComputedStyle` with
//! its parent's inheritable bits before any rule application.
//!
//! `layout_differs` answers "did any layout-affecting property change
//! from the previous cascade?" — used by the walk to set
//! `layout_dirty` on the element.

use crate::style::{ComputedStyle, Modifier};

/// Copy the inherited subset of properties from `parent` into
/// `working`. Called at the start of every element's cascade to seed
/// from the parent's computed style. The set is
/// `rdom_style::property_dispatch::inherits` — the cascade test
/// `cascade_inherits_exactly_the_style_crates_inherited_set` probes every
/// property against that table (`STYLE-INHERITS-TWO-SOURCES-1`).
pub(super) fn inherit_inheritable_from(working: &mut ComputedStyle, parent: &ComputedStyle) {
    working.fg = parent.fg;
    // Modifiers: copy only the inheriting bits (bold / italic).
    // Pre-T8 also inherited DIM (gone, replaced by `color: gray`).
    // Pre-T10 also inherited UNDERLINED via the modifier mask, which
    // diverged from CSS spec (`text-decoration` is non-inheriting).
    // T10 dropped the public `.underline()` setter and lifted
    // UNDERLINED out of this mask — children of an element with
    // `text-decoration: underline` now render without an underline,
    // matching the web platform.
    let inherit_mods = Modifier::BOLD | Modifier::ITALIC;
    working.modifiers = parent.modifiers & inherit_mods;
    // white_space inherits; display does not. Neither does
    // `user-select` (CSS UI 4 §6.1): its *used* value of `auto` depends
    // on the parent's used value, resolved in `style::user_select`.
    working.white_space = parent.white_space;
    working.pointer_events = parent.pointer_events;
    // CSS UI 4 §7.1: `caret-color` inherits; rdom's `caret-text-color`
    // mirrors it.
    working.caret_color = parent.caret_color.clone();
    working.caret_text_color = parent.caret_text_color.clone();
    // `border-collapse` does NOT inherit in rdom — documented
    // divergence (BORDER-MODEL-1). Containers that want their direct
    // children to participate in collapse declare it themselves;
    // demos and downstream consumer subtrees never inherit the
    // chrome's choice implicitly. `working.border_collapse` keeps
    // its initialized default (`Separate`); explicit author
    // declarations are applied later in `apply_border_collapse`.
    // Inherit custom-property map by Rc::clone (cheap).
    working.vars = parent.vars.clone();
}

/// True iff any layout-affecting computed property differs between
/// `a` and `b`. When a new layout-affecting property lands, add its
/// field comparison here (the cascade test `layout_differs_covers_…`
/// probes the geometry fields).
pub(super) fn layout_differs(a: &ComputedStyle, b: &ComputedStyle) -> bool {
    a.width != b.width
        || a.height != b.height
        || a.min_width != b.min_width
        || a.max_width != b.max_width
        || a.min_height != b.min_height
        || a.max_height != b.max_height
        || a.aspect_ratio != b.aspect_ratio
        || a.padding != b.padding
        || a.margin != b.margin
        || a.gap != b.gap
        || a.flex_shrink != b.flex_shrink
        || a.border != b.border
        || a.border_collapse != b.border_collapse
        || a.border_collapse_declared != b.border_collapse_declared
        || a.direction != b.direction
        || a.overflow_x != b.overflow_x
        || a.overflow_y != b.overflow_y
        || a.display != b.display
        || a.white_space != b.white_space
        // Positioning: the box's placement, its containing-block role,
        // and stacking all feed layout / paint order.
        || a.position != b.position
        || a.top != b.top
        || a.right != b.right
        || a.bottom != b.bottom
        || a.left != b.left
        || a.z_index != b.z_index
        || a.flow != b.flow
        || a.scrollbar_gutter != b.scrollbar_gutter
}
