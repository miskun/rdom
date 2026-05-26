//! `TuiStyle` — author-written style block.
//!
//! Every property field is `Option<Value<T>>`:
//!
//! - `None` = author didn't mention this property.
//! - `Some(Value::Specified(v))` = author wrote `prop: v;`.
//! - `Some(Value::Inherit)` = author wrote `prop: inherit;`.
//! - `Some(Value::Initial)` = author wrote `prop: initial;` / `unset;`.
//!
//! Fields are unified: paint AND layout live here. A stylesheet
//! rule like `tree-item { padding: 1 2; gap: 1; fg: text; }` is
//! one `TuiStyle` with five fields set.
//!
//! `!important` is tracked via a parallel `ImportantMask` bitset. The
//! cascade applies important declarations in a second pass per the CSS
//! spec.

#[cfg(test)]
use crate::Color;
use crate::layout::{
    Border, CaretColor, CaretTextColor, Direction, Display, Overflow, Padding, Size,
    TextDecoration, UserSelect, WhiteSpace,
};
use crate::{Content, TuiColor, Value};

use rdom_core::bitflags_like;

bitflags_like! {
    /// One bit per `TuiStyle` property — flipped when the author wrote
    /// `!important` on that declaration. Kept parallel to the fields
    /// rather than wrapping each in `(Value<T>, bool)` to keep the hot
    /// property accessors cheap.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct ImportantMask(u64) {
        FG         = 1 << 0;
        BG         = 1 << 1;
        BORDER_FG  = 1 << 2;
        BOLD       = 1 << 3;
        // Bits 4, 6, 7 are unused — `text-decoration` (bit 31) is
        // the sole entry point for the underlined / line-through
        // SGR primitives.
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

        // ── Positioning ──
        POSITION    = 1 << 24;
        TOP         = 1 << 25;
        RIGHT       = 1 << 26;
        BOTTOM      = 1 << 27;
        LEFT        = 1 << 28;
        Z_INDEX     = 1 << 29;

        TRANSITIONS = 1 << 30;

        TEXT_DECORATION = 1 << 31;
        OPACITY = 1 << 32;
        ASPECT_RATIO = 1 << 33;
        MARGIN = 1 << 34;
        BORDER_COLLAPSE = 1 << 35;
        CARET_COLOR = 1 << 36;
        CARET_TEXT_COLOR = 1 << 37;
        FLEX_SHRINK = 1 << 38;
        FLOW = 1 << 39;
        SCROLLBAR_GUTTER = 1 << 40;
    }
}

/// Author-written style block. Build with the fluent setters; feed
/// into a `Stylesheet` via `rule(...)` or assign to
/// `TuiExt::inline_style` via `TuiNodeMutExt::set_inline_style(...)`.
// `Eq` is intentionally omitted: `opacity: Option<Value<f32>>` blocks
// it (f32 is only PartialEq). We never use TuiStyle as a hashmap key
// or in a `HashSet`, so PartialEq suffices for equality testing.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TuiStyle {
    // ── Paint ─────────────────────────────────────────────────────────
    pub fg: Option<Value<TuiColor>>,
    pub bg: Option<Value<TuiColor>>,
    pub border_fg: Option<Value<TuiColor>>,
    pub bold: Option<Value<bool>>,
    pub italic: Option<Value<bool>>,
    /// CSS `opacity`: 0.0–1.0 (clamped at cascade time). The
    /// `.opacity(f)` / `.opacity_important(f)` setters clamp at
    /// the call site. Paint alpha-blends fg / bg / border-fg
    /// against the resolved parent bg. Truecolor-only — opacity
    /// only blends `Color::Rgb` values; `Color::Reset` opacity
    /// is a no-op (the terminal default bg is unknowable, so
    /// blending isn't well-defined). Does NOT inherit per CSS
    /// spec; default `1.0`.
    pub opacity: Option<Value<f32>>,
    /// `aspect-ratio: <w> / <h>`. When set and one axis (width/height)
    /// resolves explicitly while the other is `auto`, the auto axis
    /// is computed as `explicit / ratio` (width-from-height) or
    /// `explicit * ratio` (height-from-width), rounded half-to-even
    /// to integer cells. When both axes are explicit, the ratio is
    /// ignored (CSS rule).
    pub aspect_ratio: Option<Value<crate::layout::AspectRatio>>,

    // ── Layout ────────────────────────────────────────────────────────
    pub width: Option<Value<Size>>,
    pub height: Option<Value<Size>>,
    pub min_width: Option<Value<crate::layout::MinSize>>,
    pub max_width: Option<Value<u16>>,
    pub min_height: Option<Value<crate::layout::MinSize>>,
    pub max_height: Option<Value<u16>>,
    pub padding: Option<Value<Padding>>,
    pub margin: Option<Value<crate::layout::Margin>>,
    pub gap: Option<Value<u16>>,
    /// CSS `flex-shrink`. Default `1` per CSS spec — when total
    /// declared flex-item sizes exceed the parent's main axis,
    /// items shrink proportional to `flex_shrink * basis`. `0`
    /// opts out of shrinking (the item keeps its declared size
    /// and overflows). Larger values shrink more aggressively.
    pub flex_shrink: Option<Value<u16>>,
    pub border: Option<Value<Border>>,
    /// `border-collapse: separate | collapse`. CSS-faithful name but
    /// rdom extends the property's scope from `<table>` only to any
    /// flex container. See `crate::layout::BorderCollapse` for the
    /// divergence rationale.
    pub border_collapse: Option<Value<crate::layout::BorderCollapse>>,
    pub direction: Option<Value<Direction>>,
    /// Per-axis overflow. Set via the `.overflow(v)` shorthand
    /// (writes both axes) or `.overflow_x(v)` / `.overflow_y(v)`
    /// longhands.
    pub overflow_x: Option<Value<Overflow>>,
    pub overflow_y: Option<Value<Overflow>>,
    /// `scrollbar-gutter: auto | stable`. Gates the layout pass's
    /// gutter reservation for scrollable elements. Default `Auto`.
    pub scrollbar_gutter: Option<Value<crate::layout::ScrollbarGutter>>,

    // ── Inline formatting ────────────────────────────────────────────
    /// Outer display. Set by `display: <kw>` keywords. The companion
    /// `flow` field captures the inner display (block vs flex layout
    /// of THIS element's children). Both are written by the `display`
    /// property parser: `display: flex` sets `display = Some(Block)`
    /// AND `flow = Some(Flex)`; `display: block` sets `display =
    /// Some(Block)` + `flow = Some(Block)`; etc.
    pub display: Option<Value<Display>>,
    /// Inner display — how this element lays out its children.
    /// Written alongside `display` by the same parser. See [`Flow`]
    /// for the mapping table.
    pub flow: Option<Value<crate::layout::Flow>>,
    pub white_space: Option<Value<WhiteSpace>>,
    pub user_select: Option<Value<UserSelect>>,
    /// CSS `caret-color`. `Auto` (default) paints the caret cell
    /// with bg = underlying-cell fg. `Transparent` suppresses paint.
    /// `Color(c)` uses `c` as the caret bg. Inherits per CSS spec.
    pub caret_color: Option<Value<CaretColor>>,
    /// rdom-extension `caret-text-color`: glyph color of the caret
    /// cell. `Auto` (default) uses the underlying cell's bg. Pairs
    /// with `caret-color`. Inherits.
    pub caret_text_color: Option<Value<CaretTextColor>>,
    /// CSS `text-decoration` (line subset). Maps to the
    /// `UNDERLINED` / `CROSSED_OUT` modifier bits at cascade time.
    /// `None` clears both. Non-inheriting per CSS spec.
    pub text_decoration: Option<Value<TextDecoration>>,

    // ── Pseudo-element content ───────────────────────────────────────
    pub content: Option<Value<Content>>,

    // ── Positioning (M2) ─────────────────────────────────────────────
    pub position: Option<Value<crate::layout::Position>>,
    pub top: Option<Value<crate::layout::Length>>,
    pub right: Option<Value<crate::layout::Length>>,
    pub bottom: Option<Value<crate::layout::Length>>,
    pub left: Option<Value<crate::layout::Length>>,
    pub z_index: Option<Value<crate::layout::ZIndex>>,

    // ── Transitions (M3) ─────────────────────────────────────────────
    /// `transition-property` longhand. Each entry covers one
    /// CSS property (or `all` / `none`) at the matching index in
    /// the duration / timing / delay lists.
    pub transition_property: Option<Vec<crate::transition::TransitionProperty>>,
    /// `transition-duration` longhand, in milliseconds.
    pub transition_duration: Option<Vec<u32>>,
    /// `transition-timing-function` longhand.
    pub transition_timing_function: Option<Vec<crate::transition::TimingFunction>>,
    /// `transition-delay` longhand, in milliseconds.
    pub transition_delay: Option<Vec<u32>>,

    // ── `!important` bits ─────────────────────────────────────────────
    pub important: ImportantMask,
}

macro_rules! setter {
    ($field:ident, $setter:ident, $important_setter:ident, $mask:ident, $ty:ty) => {
        #[doc = concat!("Set the `", stringify!($field), "` property to `v`. Chainable.")]
        pub fn $setter(mut self, v: $ty) -> Self {
            self.$field = Some(Value::Specified(v));
            self
        }

        #[doc = concat!("Like `", stringify!($setter), "` but also marks the declaration `!important`.")]
        pub fn $important_setter(mut self, v: $ty) -> Self {
            self.$field = Some(Value::Specified(v));
            self.important |= ImportantMask::$mask;
            self
        }
    };
}

impl TuiStyle {
    pub fn new() -> Self {
        Self::default()
    }

    // Paint color setters — accept both `Color::Rgb(255, 0, 0)` and
    // `TuiColor::var("accent")` via `impl Into<TuiColor>`.
    pub fn fg(mut self, color: impl Into<TuiColor>) -> Self {
        self.fg = Some(Value::Specified(color.into()));
        self
    }
    pub fn fg_important(mut self, color: impl Into<TuiColor>) -> Self {
        self.fg = Some(Value::Specified(color.into()));
        self.important |= ImportantMask::FG;
        self
    }
    pub fn bg(mut self, color: impl Into<TuiColor>) -> Self {
        self.bg = Some(Value::Specified(color.into()));
        self
    }
    pub fn bg_important(mut self, color: impl Into<TuiColor>) -> Self {
        self.bg = Some(Value::Specified(color.into()));
        self.important |= ImportantMask::BG;
        self
    }
    pub fn border_fg(mut self, color: impl Into<TuiColor>) -> Self {
        self.border_fg = Some(Value::Specified(color.into()));
        self
    }
    pub fn border_fg_important(mut self, color: impl Into<TuiColor>) -> Self {
        self.border_fg = Some(Value::Specified(color.into()));
        self.important |= ImportantMask::BORDER_FG;
        self
    }

    // Convenience `var(--…)` helpers so callers don't need to spell
    // `TuiColor::var("name")` — `.fg_var("accent")` reads like CSS.
    pub fn fg_var(self, name: impl Into<String>) -> Self {
        self.fg(TuiColor::var(name))
    }
    pub fn bg_var(self, name: impl Into<String>) -> Self {
        self.bg(TuiColor::var(name))
    }
    pub fn border_fg_var(self, name: impl Into<String>) -> Self {
        self.border_fg(TuiColor::var(name))
    }

    setter!(bold, bold, bold_important, BOLD, bool);
    setter!(italic, italic, italic_important, ITALIC, bool);

    /// CSS `opacity`. Clamped to `[0.0, 1.0]` at the call site
    /// (out-of-range inputs are silently saturated). Custom
    /// setter rather than `setter!` because `Eq` doesn't impl
    /// for `f32` (which `Value<T>` requires for `TuiStyle`'s
    /// derived `PartialEq` / `Eq` — see `Value::Specified`
    /// where the actual value is held).
    pub fn opacity(mut self, v: f32) -> Self {
        self.opacity = Some(Value::Specified(v.clamp(0.0, 1.0)));
        self
    }
    /// `opacity` with `!important`. Same clamp.
    pub fn opacity_important(mut self, v: f32) -> Self {
        self.opacity = Some(Value::Specified(v.clamp(0.0, 1.0)));
        self.important |= ImportantMask::OPACITY;
        self
    }

    /// `aspect-ratio: <w> / <h>`. Both must be positive — non-positive
    /// arguments panic. Use the CSS parser if your numerator or
    /// denominator are author-provided.
    pub fn aspect_ratio(mut self, w: u16, h: u16) -> Self {
        let ratio = crate::layout::AspectRatio::new(w, h)
            .expect("aspect_ratio: numerator and denominator must be positive");
        self.aspect_ratio = Some(Value::Specified(ratio));
        self
    }
    /// `aspect-ratio` with `!important`.
    pub fn aspect_ratio_important(mut self, w: u16, h: u16) -> Self {
        let ratio = crate::layout::AspectRatio::new(w, h)
            .expect("aspect_ratio: numerator and denominator must be positive");
        self.aspect_ratio = Some(Value::Specified(ratio));
        self.important |= ImportantMask::ASPECT_RATIO;
        self
    }

    // Layout setters.
    setter!(width, width, width_important, WIDTH, Size);
    setter!(height, height, height_important, HEIGHT, Size);
    /// Set the `min-width` property. Accepts a `u16` (cells) or
    /// `MinSize::Auto`. Chainable.
    pub fn min_width(mut self, v: impl Into<crate::layout::MinSize>) -> Self {
        self.min_width = Some(Value::Specified(v.into()));
        self
    }
    /// Like `min_width` but marks the declaration `!important`.
    pub fn min_width_important(mut self, v: impl Into<crate::layout::MinSize>) -> Self {
        self.min_width = Some(Value::Specified(v.into()));
        self.important |= ImportantMask::MIN_WIDTH;
        self
    }
    setter!(max_width, max_width, max_width_important, MAX_WIDTH, u16);
    /// Set the `min-height` property. Accepts a `u16` (cells) or
    /// `MinSize::Auto`. Chainable.
    pub fn min_height(mut self, v: impl Into<crate::layout::MinSize>) -> Self {
        self.min_height = Some(Value::Specified(v.into()));
        self
    }
    /// Like `min_height` but marks the declaration `!important`.
    pub fn min_height_important(mut self, v: impl Into<crate::layout::MinSize>) -> Self {
        self.min_height = Some(Value::Specified(v.into()));
        self.important |= ImportantMask::MIN_HEIGHT;
        self
    }
    setter!(
        max_height,
        max_height,
        max_height_important,
        MAX_HEIGHT,
        u16
    );
    setter!(padding, padding, padding_important, PADDING, Padding);
    /// Set the `margin` property. Accepts a `Margin` struct or a
    /// plain `i16` (via `From<i16> for Margin` — applies `n` cells
    /// on all four sides). Chainable.
    pub fn margin(mut self, v: impl Into<crate::layout::Margin>) -> Self {
        self.margin = Some(Value::Specified(v.into()));
        self
    }
    /// Like `margin` but marks the declaration `!important`.
    pub fn margin_important(mut self, v: impl Into<crate::layout::Margin>) -> Self {
        self.margin = Some(Value::Specified(v.into()));
        self.important |= ImportantMask::MARGIN;
        self
    }
    setter!(gap, gap, gap_important, GAP, u16);
    setter!(
        flex_shrink,
        flex_shrink,
        flex_shrink_important,
        FLEX_SHRINK,
        u16
    );
    setter!(border, border, border_important, BORDER, Border);
    /// `.collapse_borders()` — sets `border-collapse: collapse` on
    /// this element. Convenience shortcut over the verbose
    /// `.border_collapse(BorderCollapse::Collapse)`. Chainable.
    pub fn collapse_borders(mut self) -> Self {
        self.border_collapse = Some(Value::Specified(crate::layout::BorderCollapse::Collapse));
        self
    }
    setter!(
        border_collapse,
        border_collapse,
        border_collapse_important,
        BORDER_COLLAPSE,
        crate::layout::BorderCollapse
    );
    setter!(
        direction,
        direction,
        direction_important,
        DIRECTION,
        Direction
    );
    setter!(
        overflow_x,
        overflow_x,
        overflow_x_important,
        OVERFLOW_X,
        Overflow
    );
    setter!(
        overflow_y,
        overflow_y,
        overflow_y_important,
        OVERFLOW_Y,
        Overflow
    );

    /// CSS-shorthand: set `overflow-x` and `overflow-y` to the same
    /// value. Equivalent to `.overflow_x(v).overflow_y(v)`.
    pub fn overflow(mut self, v: Overflow) -> Self {
        self.overflow_x = Some(Value::Specified(v));
        self.overflow_y = Some(Value::Specified(v));
        self
    }

    /// `!important` variant of the `overflow` shorthand. Sets both
    /// longhand `!important` bits.
    pub fn overflow_important(mut self, v: Overflow) -> Self {
        self.overflow_x = Some(Value::Specified(v));
        self.overflow_y = Some(Value::Specified(v));
        self.important |= ImportantMask::OVERFLOW_X;
        self.important |= ImportantMask::OVERFLOW_Y;
        self
    }

    // `display` carries the outer formatting value AND drives the
    // inner `flow` value per the CSS3 Display Module mapping. The
    // setter mirrors what the `display: <kw>` parser does: writing
    // `display(Display::Block)` also sets `flow = Flow::Block`,
    // `display(Display::InlineBlock)` sets `flow = Flow::Block`,
    // etc. Without this, builder-built styles (used in tests, the
    // UA stylesheet, and inline API) would diverge from CSS-parsed
    // styles at round-trip boundaries. `display(Display::Inline)`
    // and `display(Display::None)` leave `flow` untouched (no inner
    // formatting context to declare).
    pub fn display(mut self, v: Display) -> Self {
        self.display = Some(Value::Specified(v));
        match v {
            Display::Block | Display::InlineBlock => {
                self.flow = Some(Value::Specified(crate::layout::Flow::Block));
            }
            Display::Inline | Display::None => {}
        }
        self
    }
    pub fn display_important(mut self, v: Display) -> Self {
        self = self.display(v);
        self.important |= ImportantMask::DISPLAY;
        self
    }
    setter!(flow, flow, flow_important, FLOW, crate::layout::Flow);
    setter!(
        scrollbar_gutter,
        scrollbar_gutter,
        scrollbar_gutter_important,
        SCROLLBAR_GUTTER,
        crate::layout::ScrollbarGutter
    );
    setter!(
        white_space,
        white_space,
        white_space_important,
        WHITE_SPACE,
        WhiteSpace
    );
    setter!(
        user_select,
        user_select,
        user_select_important,
        USER_SELECT,
        UserSelect
    );
    setter!(
        caret_color,
        caret_color,
        caret_color_important,
        CARET_COLOR,
        CaretColor
    );
    setter!(
        caret_text_color,
        caret_text_color,
        caret_text_color_important,
        CARET_TEXT_COLOR,
        CaretTextColor
    );
    setter!(
        text_decoration,
        text_decoration,
        text_decoration_important,
        TEXT_DECORATION,
        TextDecoration
    );

    // Content setter.
    setter!(content, content, content_important, CONTENT, Content);

    // ── Positioning setters (M2) ─────────────────────────────────────
    setter!(
        position,
        position,
        position_important,
        POSITION,
        crate::layout::Position
    );
    setter!(top, top, top_important, TOP, crate::layout::Length);
    setter!(right, right, right_important, RIGHT, crate::layout::Length);
    setter!(
        bottom,
        bottom,
        bottom_important,
        BOTTOM,
        crate::layout::Length
    );
    setter!(left, left, left_important, LEFT, crate::layout::Length);
    setter!(
        z_index,
        z_index,
        z_index_important,
        Z_INDEX,
        crate::layout::ZIndex
    );

    // ── Transitions setters (M3) ─────────────────────────────────────
    // Vec-typed fields can't go through the `setter!` macro (no
    // `Value<T>` wrapping), so we hand-write a thin layer. All four
    // longhand fields share the `TRANSITIONS` important bit.
    pub fn transition_property(mut self, v: Vec<crate::transition::TransitionProperty>) -> Self {
        self.transition_property = Some(v);
        self
    }
    pub fn transition_duration(mut self, v: Vec<u32>) -> Self {
        self.transition_duration = Some(v);
        self
    }
    pub fn transition_timing_function(mut self, v: Vec<crate::transition::TimingFunction>) -> Self {
        self.transition_timing_function = Some(v);
        self
    }
    pub fn transition_delay(mut self, v: Vec<u32>) -> Self {
        self.transition_delay = Some(v);
        self
    }
    /// Mark the transition longhands as `!important`. All four
    /// longhands share the `TRANSITIONS` mask bit (matches the
    /// CSS spec — `!important` applies to the whole shorthand
    /// declaration).
    pub fn transitions_important(mut self) -> Self {
        self.important |= ImportantMask::TRANSITIONS;
        self
    }

    /// Convenience: set `fg: inherit;` without having to spell `Value::Inherit`.
    pub fn fg_inherit(mut self) -> Self {
        self.fg = Some(Value::Inherit);
        self
    }

    /// Convenience: set `fg: initial;` without having to spell `Value::Initial`.
    pub fn fg_initial(mut self) -> Self {
        self.fg = Some(Value::Initial);
        self
    }

    /// True when no field is set. Empty `TuiStyle` is the `Default`.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Count how many fields are `Some(..)`. Used by the cascade +
    /// devtools to show how "heavy" a rule is.
    pub fn declared_count(&self) -> usize {
        let mut n = 0;
        if self.fg.is_some() {
            n += 1
        }
        if self.bg.is_some() {
            n += 1
        }
        if self.border_fg.is_some() {
            n += 1
        }
        if self.bold.is_some() {
            n += 1
        }
        if self.italic.is_some() {
            n += 1
        }
        if self.width.is_some() {
            n += 1
        }
        if self.height.is_some() {
            n += 1
        }
        if self.min_width.is_some() {
            n += 1
        }
        if self.max_width.is_some() {
            n += 1
        }
        if self.min_height.is_some() {
            n += 1
        }
        if self.max_height.is_some() {
            n += 1
        }
        if self.padding.is_some() {
            n += 1
        }
        if self.gap.is_some() {
            n += 1
        }
        if self.border.is_some() {
            n += 1
        }
        if self.direction.is_some() {
            n += 1
        }
        if self.overflow_x.is_some() {
            n += 1
        }
        if self.overflow_y.is_some() {
            n += 1
        }
        if self.scrollbar_gutter.is_some() {
            n += 1
        }
        if self.display.is_some() {
            n += 1
        }
        if self.flow.is_some() {
            n += 1
        }
        if self.white_space.is_some() {
            n += 1
        }
        if self.user_select.is_some() {
            n += 1
        }
        if self.caret_color.is_some() {
            n += 1
        }
        if self.caret_text_color.is_some() {
            n += 1
        }
        if self.text_decoration.is_some() {
            n += 1
        }
        if self.content.is_some() {
            n += 1
        }
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_empty() {
        assert!(TuiStyle::default().is_empty());
        assert_eq!(TuiStyle::default().declared_count(), 0);
    }

    #[test]
    fn builder_sets_specified() {
        let s = TuiStyle::new()
            .fg(Color::Rgb(255, 0, 0))
            .bg(Color::Rgb(0, 0, 0))
            .bold(true);
        assert_eq!(
            s.fg,
            Some(Value::Specified(TuiColor::Literal(Color::Rgb(255, 0, 0))))
        );
        assert_eq!(
            s.bg,
            Some(Value::Specified(TuiColor::Literal(Color::Rgb(0, 0, 0))))
        );
        assert_eq!(s.bold, Some(Value::Specified(true)));
        assert_eq!(s.declared_count(), 3);
    }

    #[test]
    fn builder_important_variant_sets_flag() {
        let s = TuiStyle::new().fg_important(Color::Rgb(255, 0, 0));
        assert_eq!(
            s.fg,
            Some(Value::Specified(TuiColor::Literal(Color::Rgb(255, 0, 0))))
        );
        assert!(s.important.contains(ImportantMask::FG));
        assert!(!s.important.contains(ImportantMask::BG));
    }

    #[test]
    fn fg_inherit_sets_inherit_variant() {
        let s = TuiStyle::new().fg_inherit();
        assert_eq!(s.fg, Some(Value::Inherit));
    }

    #[test]
    fn fg_initial_sets_initial_variant() {
        let s = TuiStyle::new().fg_initial();
        assert_eq!(s.fg, Some(Value::Initial));
    }

    #[test]
    fn unified_layout_fields_settable() {
        let s = TuiStyle::new()
            .width(Size::Fixed(40))
            .padding(Padding::all(2))
            .gap(1)
            .direction(Direction::Row)
            .border(Border::single())
            .overflow(Overflow::Hidden);

        assert_eq!(s.width, Some(Value::Specified(Size::Fixed(40))));
        assert_eq!(s.padding, Some(Value::Specified(Padding::all(2))));
        assert_eq!(s.gap, Some(Value::Specified(1)));
        assert_eq!(s.direction, Some(Value::Specified(Direction::Row)));
        assert_eq!(s.border, Some(Value::Specified(Border::single())));
        // `overflow` shorthand writes both longhands.
        assert_eq!(s.overflow_x, Some(Value::Specified(Overflow::Hidden)));
        assert_eq!(s.overflow_y, Some(Value::Specified(Overflow::Hidden)));
        // 5 properties above + 2 axes of overflow = 7.
        assert_eq!(s.declared_count(), 7);
    }

    #[test]
    fn min_max_layout_setters() {
        use crate::layout::MinSize;
        let s = TuiStyle::new()
            .min_width(10)
            .max_width(100)
            .min_height(5)
            .max_height(50);
        assert_eq!(s.min_width, Some(Value::Specified(MinSize::Cells(10))));
        assert_eq!(s.max_width, Some(Value::Specified(100)));
        assert_eq!(s.min_height, Some(Value::Specified(MinSize::Cells(5))));
        assert_eq!(s.max_height, Some(Value::Specified(50)));
    }

    #[test]
    fn min_width_setter_accepts_auto_keyword_via_enum() {
        use crate::layout::MinSize;
        let s = TuiStyle::new().min_width(MinSize::Auto);
        assert_eq!(s.min_width, Some(Value::Specified(MinSize::Auto)));
    }

    #[test]
    fn content_setter_accepts_content_enum() {
        let s = TuiStyle::new().content(Content::Str("→".into()));
        assert_eq!(s.content, Some(Value::Specified(Content::Str("→".into()))));
    }

    #[test]
    fn content_var_and_concat() {
        let s = TuiStyle::new().content(Content::Concat(vec![
            Content::Str("▾ ".into()),
            Content::Var("label".into()),
        ]));
        match s.content.unwrap() {
            Value::Specified(Content::Concat(parts)) => {
                assert_eq!(parts.len(), 2);
            }
            _ => panic!("expected concat"),
        }
    }

    #[test]
    fn important_mask_bits_isolated() {
        let s = TuiStyle::new()
            .fg(Color::Rgb(255, 0, 0))
            .bold_important(true);
        assert!(!s.important.contains(ImportantMask::FG));
        assert!(s.important.contains(ImportantMask::BOLD));
    }

    #[test]
    fn is_empty_false_after_any_set() {
        assert!(!TuiStyle::new().fg(Color::Rgb(255, 0, 0)).is_empty());
        assert!(!TuiStyle::new().padding(Padding::all(1)).is_empty());
        assert!(!TuiStyle::new().content(Content::Str("x".into())).is_empty());
    }

    #[test]
    fn clone_preserves_important_bits() {
        let s = TuiStyle::new().fg_important(Color::Rgb(255, 0, 0));
        let c = s.clone();
        assert!(c.important.contains(ImportantMask::FG));
    }

    #[test]
    fn important_mask_bit_ops() {
        let m = ImportantMask::FG | ImportantMask::BG;
        assert!(m.contains(ImportantMask::FG));
        assert!(m.contains(ImportantMask::BG));
        assert!(!m.contains(ImportantMask::BOLD));
    }

    #[test]
    fn every_property_has_a_setter() {
        // Smoke test: call every setter. If one was missed it will fail
        // to compile or miss a field below.
        let s = TuiStyle::new()
            .fg(Color::Rgb(255, 0, 0))
            .bg(Color::Rgb(0, 0, 0))
            .border_fg(Color::Rgb(255, 255, 255))
            .bold(true)
            .italic(true)
            .width(Size::Fixed(1))
            .height(Size::Fixed(1))
            .min_width(1)
            .max_width(1)
            .min_height(1)
            .max_height(1)
            .padding(Padding::all(1))
            .gap(1)
            .border(Border::single())
            .direction(Direction::Row)
            .overflow(Overflow::Hidden)
            .content(Content::Str("x".into()));
        // The `overflow` shorthand counts as 2 (writes both axes).
        assert_eq!(s.declared_count(), 18);
    }

    #[test]
    fn fg_var_sets_var_reference() {
        let s = TuiStyle::new().fg_var("accent");
        match &s.fg {
            Some(Value::Specified(TuiColor::Var { name, fallback })) => {
                assert_eq!(name, "accent");
                assert!(fallback.is_none());
            }
            _ => panic!("expected var(--accent)"),
        }
    }

    #[test]
    fn fg_accepts_literal_and_var_via_into() {
        // Both forms compile and produce the right variant.
        let a = TuiStyle::new().fg(Color::Rgb(255, 0, 0));
        let b = TuiStyle::new().fg(TuiColor::var("accent"));
        assert!(matches!(a.fg, Some(Value::Specified(TuiColor::Literal(_)))));
        assert!(matches!(b.fg, Some(Value::Specified(TuiColor::Var { .. }))));
    }

    #[test]
    fn every_property_has_important_setter() {
        let s = TuiStyle::new()
            .fg_important(Color::Rgb(255, 0, 0))
            .bg_important(Color::Rgb(0, 0, 0))
            .border_fg_important(Color::Rgb(255, 255, 255))
            .bold_important(true)
            .italic_important(true)
            .width_important(Size::Fixed(1))
            .height_important(Size::Fixed(1))
            .min_width_important(1)
            .max_width_important(1)
            .min_height_important(1)
            .max_height_important(1)
            .padding_important(Padding::all(1))
            .margin_important(crate::layout::Margin::all_cells(1))
            .gap_important(1)
            .flex_shrink_important(1)
            .border_important(Border::single())
            .border_collapse_important(crate::layout::BorderCollapse::Collapse)
            .direction_important(Direction::Row)
            .overflow_important(Overflow::Hidden)
            .display_important(Display::Inline)
            .flow_important(crate::layout::Flow::Block)
            .scrollbar_gutter_important(crate::layout::ScrollbarGutter::Stable)
            .white_space_important(WhiteSpace::Pre)
            .user_select_important(UserSelect::None)
            .caret_color_important(CaretColor::Transparent)
            .caret_text_color_important(CaretTextColor::Auto)
            .text_decoration_important(TextDecoration::Underline)
            .opacity_important(0.5)
            .aspect_ratio_important(16, 9)
            .content_important(Content::Str("x".into()))
            .position_important(crate::layout::Position::Absolute)
            .top_important(crate::layout::Length::Cells(1))
            .right_important(crate::layout::Length::Cells(1))
            .bottom_important(crate::layout::Length::Cells(1))
            .left_important(crate::layout::Length::Cells(1))
            .z_index_important(crate::layout::ZIndex::Value(1))
            .transitions_important();
        assert_eq!(s.important, ImportantMask::all());
    }
}
