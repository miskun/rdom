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
/// from the parent's computed style.
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
    // white_space + user_select inherit; display does not.
    working.white_space = parent.white_space;
    working.user_select = parent.user_select;
    // border-collapse inherits per CSS spec (M5.5a). The default
    // `Separate` propagates from root → children unless an ancestor
    // sets `collapse`, in which case every descendant inherits it
    // until something explicitly resets to `separate`.
    working.border_collapse = parent.border_collapse;
    // Inherit custom-property map by Rc::clone (cheap).
    working.vars = parent.vars.clone();
}

/// True iff any layout-affecting computed property differs between
/// `a` and `b`. Keeps in sync with `LAYOUT_MASK` — if you add a new
/// layout-affecting property to that mask, add the field-comparison
/// here.
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
        || a.direction != b.direction
        || a.overflow_x != b.overflow_x
        || a.overflow_y != b.overflow_y
        || a.display != b.display
        || a.white_space != b.white_space
}
