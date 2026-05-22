//! `PropMask` bitfield + predefined subsets (`INHERITS_MASK`,
//! `LAYOUT_MASK`).
//!
//! Data, not code — adding a new inheritable or layout-affecting
//! property is a one-line bit-set change here, not a code-path
//! rewrite in the cascade walk.

use rdom_core::bitflags_like;

bitflags_like! {
    /// One bit per `TuiStyle` property. Used to drive cascade
    /// operations (inheritance decisions, layout-vs-paint
    /// partitioning) without hardcoding the property list in each
    /// call site.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct PropMask(u32) {
        FG         = 1 << 0;
        BG         = 1 << 1;
        BORDER_FG  = 1 << 2;
        BOLD       = 1 << 3;
        // Bits 4 (DIM), 6 (UNDERLINE), and 7 (REVERSED) are unused.
        // DIM was dropped when `.dim()` was removed; UNDERLINE was
        // dropped when `.underline()` was replaced by `text-decoration`;
        // REVERSED was dropped when the caret switched to explicit
        // fg/bg paint. The gaps stay so the remaining bit values
        // don't shift.
        ITALIC     = 1 << 5;
        WIDTH      = 1 << 8;
        HEIGHT     = 1 << 9;
        MIN_WIDTH  = 1 << 10;
        MAX_WIDTH  = 1 << 11;
        MIN_HEIGHT = 1 << 12;
        MAX_HEIGHT = 1 << 13;
        PADDING    = 1 << 14;
        GAP        = 1 << 15;
        BORDER     = 1 << 16;
        DIRECTION  = 1 << 17;
        OVERFLOW_X = 1 << 18;
        CONTENT    = 1 << 19;
        DISPLAY     = 1 << 20;
        WHITE_SPACE = 1 << 21;
        USER_SELECT = 1 << 22;
        OVERFLOW_Y  = 1 << 23;
        FLEX_SHRINK = 1 << 24;
    }
}

/// Properties that inherit from parent by default (CSS-style). Data,
/// not code — modifying inheritance is a one-line change.
pub const INHERITS_MASK: PropMask = PropMask(
    PropMask::FG.bits()
        | PropMask::BOLD.bits()
        | PropMask::ITALIC.bits()
        | PropMask::WHITE_SPACE.bits()
        | PropMask::USER_SELECT.bits(),
);

/// Properties that affect layout geometry. Changing one of these
/// triggers `layout_dirty = true` on the element.
pub const LAYOUT_MASK: PropMask = PropMask(
    PropMask::WIDTH.bits()
        | PropMask::HEIGHT.bits()
        | PropMask::MIN_WIDTH.bits()
        | PropMask::MAX_WIDTH.bits()
        | PropMask::MIN_HEIGHT.bits()
        | PropMask::MAX_HEIGHT.bits()
        | PropMask::PADDING.bits()
        | PropMask::GAP.bits()
        | PropMask::BORDER.bits()
        | PropMask::DIRECTION.bits()
        | PropMask::OVERFLOW_X.bits()
        | PropMask::OVERFLOW_Y.bits()
        | PropMask::DISPLAY.bits()
        | PropMask::WHITE_SPACE.bits()
        | PropMask::FLEX_SHRINK.bits(),
);
