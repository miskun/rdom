//! The `TuiStyle` builder: chainable setters for every property
//! (`.fg(…)`, `.padding(…)`, `….._important(…)`); the background,
//! border and shadow setters are in `decoration`, the sizing ones
//! (`width` / `height`, `min-*` / `max-*`, `aspect-ratio`) in `sizing`,
//! the margin and padding ones in `spacing`, the flex and box-alignment
//! ones in `flex`.

use super::{ImportantMask, TuiStyle};
#[allow(unused_imports)]
use crate::layout::{
    Border, CaretColor, CaretTextColor, Direction, Display, Overflow, Padding, Sides, Size,
    TextDecoration, UserSelect, WhiteSpace,
};
use crate::{Content, TuiColor, Value};

/// A builder setter and its `!important` twin for one `TuiStyle` field:
/// `setter!("css-name", field, setter, important_setter, MASK, Type)`,
/// with an optional last `$valid` path that keeps a Rust-built value in
/// the property's range. The docs name the CSS property (`css-name`),
/// not the Rust field (`direction` is `flex-direction`).
macro_rules! setter {
    ($css:literal, $field:ident, $setter:ident, $important_setter:ident, $mask:ident, $ty:ty) => {
        setter!($css, $field, $setter, $important_setter, $mask, $ty, std::convert::identity);
    };
    ($css:literal, $field:ident, $setter:ident, $important_setter:ident, $mask:ident, $ty:ty, $valid:path) => {
        #[doc = concat!("Set the `", $css, "` property to `v`. Chainable.")]
        pub fn $setter(mut self, v: $ty) -> Self {
            self.$field = Some(Value::Specified($valid(v)));
            self
        }

        #[doc = concat!("Like `", stringify!($setter), "` but also marks the `", $css, "` declaration `!important`.")]
        pub fn $important_setter(mut self, v: $ty) -> Self {
            self.$field = Some(Value::Specified($valid(v)));
            self.important |= ImportantMask::$mask;
            self
        }
    };
}

mod decoration;
mod flex;
mod grid;
mod sizing;
mod spacing;

impl TuiStyle {
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
    /// Set `border-color` on all four sides. Chainable.
    pub fn border_fg(mut self, color: impl Into<TuiColor>) -> Self {
        self.border_color = Sides::all(Some(Value::Specified(color.into())));
        self
    }
    pub fn border_fg_important(mut self, color: impl Into<TuiColor>) -> Self {
        self.border_color = Sides::all(Some(Value::Specified(color.into())));
        self.important |= ImportantMask::BORDER_TOP_COLOR
            | ImportantMask::BORDER_RIGHT_COLOR
            | ImportantMask::BORDER_BOTTOM_COLOR
            | ImportantMask::BORDER_LEFT_COLOR;
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

    setter!("font-weight", bold, bold, bold_important, BOLD, bool);
    setter!("font-style", italic, italic, italic_important, ITALIC, bool);

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

    /// `.collapse_borders()` — sets `border-collapse: collapse` on
    /// this element. Convenience shortcut over the verbose
    /// `.border_collapse(BorderCollapse::Collapse)`. Chainable.
    pub fn collapse_borders(mut self) -> Self {
        self.border_collapse = Some(Value::Specified(crate::layout::BorderCollapse::Collapse));
        self
    }
    setter!(
        "border-collapse",
        border_collapse,
        border_collapse,
        border_collapse_important,
        BORDER_COLLAPSE,
        crate::layout::BorderCollapse
    );
    setter!(
        "overflow-x",
        overflow_x,
        overflow_x,
        overflow_x_important,
        OVERFLOW_X,
        Overflow
    );
    setter!(
        "overflow-y",
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
    // styles at round-trip boundaries. `display(Display::Inline)`,
    // `display(Display::None)` and `display(Display::Contents)` leave
    // `flow` untouched (`.flow(Flow::Flex).display(Display::Inline)` is
    // `inline flex`).
    pub fn display(mut self, v: Display) -> Self {
        self.display = Some(Value::Specified(v));
        // No `Display` is a list item; `list_item` is the flag beside it.
        self.list_item = Some(Value::Specified(false));
        match v {
            Display::Block | Display::InlineBlock => {
                self.flow = Some(Value::Specified(crate::layout::Flow::Block));
            }
            Display::Inline | Display::None | Display::Contents => {}
        }
        self
    }
    pub fn display_important(mut self, v: Display) -> Self {
        self = self.display(v);
        self.important |= ImportantMask::DISPLAY | ImportantMask::LIST_ITEM;
        self
    }
    /// Set the inner display type — the second half of `display`
    /// (`flex` in `display: flex`, CSS Display 3 §2.2). Chainable.
    pub fn flow(mut self, v: crate::layout::Flow) -> Self {
        self.flow = Some(Value::Specified(v));
        self
    }
    /// Like `flow` but also marks the inner display type `!important`.
    pub fn flow_important(mut self, v: crate::layout::Flow) -> Self {
        self.flow = Some(Value::Specified(v));
        self.important |= ImportantMask::FLOW;
        self
    }
    setter!(
        "counter-reset",
        counter_reset,
        counter_reset,
        counter_reset_important,
        COUNTER_RESET,
        Vec<crate::counters::CounterOp>
    );
    setter!(
        "counter-increment",
        counter_increment,
        counter_increment,
        counter_increment_important,
        COUNTER_INCREMENT,
        Vec<crate::counters::CounterOp>
    );
    setter!(
        "color-scheme",
        color_scheme,
        color_scheme,
        color_scheme_important,
        COLOR_SCHEME,
        crate::color::ColorSchemeList
    );

    setter!(
        "overflow-clip-margin",
        overflow_clip_margin,
        overflow_clip_margin,
        overflow_clip_margin_important,
        OVERFLOW_CLIP_MARGIN,
        crate::layout::OverflowClipMargin
    );
    setter!(
        "text-overflow",
        text_overflow,
        text_overflow,
        text_overflow_important,
        TEXT_OVERFLOW,
        crate::layout::TextOverflow
    );
    setter!(
        "max-lines",
        max_lines,
        max_lines,
        max_lines_important,
        MAX_LINES,
        Option<u32>
    );
    setter!(
        "block-ellipsis",
        block_ellipsis,
        block_ellipsis,
        block_ellipsis_important,
        BLOCK_ELLIPSIS,
        crate::layout::BlockEllipsis
    );
    setter!(
        "continue",
        continue_,
        continue_,
        continue_important,
        CONTINUE,
        crate::layout::Continue
    );
    setter!(
        "-webkit-box-orient",
        webkit_box_orient,
        webkit_box_orient,
        webkit_box_orient_important,
        WEBKIT_BOX_ORIENT,
        crate::layout::BoxOrient
    );
    setter!(
        "scrollbar-gutter",
        scrollbar_gutter,
        scrollbar_gutter,
        scrollbar_gutter_important,
        SCROLLBAR_GUTTER,
        crate::layout::ScrollbarGutter
    );
    setter!(
        "direction",
        text_direction,
        text_direction,
        text_direction_important,
        TEXT_DIRECTION,
        crate::layout::TextDirection
    );
    setter!(
        "writing-mode",
        writing_mode,
        writing_mode,
        writing_mode_important,
        WRITING_MODE,
        crate::layout::WritingMode
    );
    setter!(
        "margin-trim",
        margin_trim,
        margin_trim,
        margin_trim_important,
        MARGIN_TRIM,
        crate::layout::MarginTrim
    );
    setter!(
        "contain-intrinsic-width",
        contain_intrinsic_width,
        contain_intrinsic_width,
        contain_intrinsic_width_important,
        CONTAIN_INTRINSIC_WIDTH,
        crate::layout::ContainIntrinsicSize
    );
    setter!(
        "contain-intrinsic-height",
        contain_intrinsic_height,
        contain_intrinsic_height,
        contain_intrinsic_height_important,
        CONTAIN_INTRINSIC_HEIGHT,
        crate::layout::ContainIntrinsicSize
    );
    setter!(
        "box-sizing",
        box_sizing,
        box_sizing,
        box_sizing_important,
        BOX_SIZING,
        crate::layout::BoxSizing
    );
    setter!(
        "scroll-behavior",
        scroll_behavior,
        scroll_behavior,
        scroll_behavior_important,
        SCROLL_BEHAVIOR,
        crate::layout::ScrollBehavior
    );
    setter!(
        "white-space",
        white_space,
        white_space,
        white_space_important,
        WHITE_SPACE,
        WhiteSpace
    );
    setter!(
        "user-select",
        user_select,
        user_select,
        user_select_important,
        USER_SELECT,
        UserSelect
    );
    setter!(
        "pointer-events",
        pointer_events,
        pointer_events,
        pointer_events_important,
        POINTER_EVENTS,
        crate::layout::PointerEvents
    );
    setter!(
        "visibility",
        visibility,
        visibility,
        visibility_important,
        VISIBILITY,
        crate::layout::Visibility
    );
    setter!(
        "caret-color",
        caret_color,
        caret_color,
        caret_color_important,
        CARET_COLOR,
        CaretColor
    );
    setter!(
        "caret-text-color",
        caret_text_color,
        caret_text_color,
        caret_text_color_important,
        CARET_TEXT_COLOR,
        CaretTextColor
    );
    setter!(
        "text-decoration",
        text_decoration,
        text_decoration,
        text_decoration_important,
        TEXT_DECORATION,
        TextDecoration
    );

    // Content setter.
    setter!(
        "content",
        content,
        content,
        content_important,
        CONTENT,
        Content
    );

    // ── Positioning setters (M2) ─────────────────────────────────────
    setter!(
        "position",
        position,
        position,
        position_important,
        POSITION,
        crate::layout::Position
    );
    setter!("top", top, top, top_important, TOP, crate::layout::Length);
    setter!(
        "right",
        right,
        right,
        right_important,
        RIGHT,
        crate::layout::Length
    );
    setter!(
        "bottom",
        bottom,
        bottom,
        bottom_important,
        BOTTOM,
        crate::layout::Length
    );
    setter!(
        "left",
        left,
        left,
        left_important,
        LEFT,
        crate::layout::Length
    );
    setter!(
        "z-index",
        z_index,
        z_index,
        z_index_important,
        Z_INDEX,
        crate::layout::ZIndex
    );
    setter!(
        "float",
        float,
        float,
        float_important,
        FLOAT,
        crate::layout::Float
    );
    setter!(
        "clear",
        clear,
        clear,
        clear_important,
        CLEAR,
        crate::layout::Clear
    );

    // ── Transitions setters (M3) ─────────────────────────────────────
    // Vec-typed fields can't go through the `setter!` macro (no
    // `Value<T>` wrapping), so we hand-write a thin layer.
    pub fn transition_property(mut self, v: Vec<crate::transition::TransitionProperty>) -> Self {
        self.transition_property = Some(Value::Specified(v));
        self
    }
    pub fn transition_duration(mut self, v: Vec<u32>) -> Self {
        self.transition_duration = Some(Value::Specified(v));
        self
    }
    pub fn transition_timing_function(mut self, v: Vec<crate::transition::TimingFunction>) -> Self {
        self.transition_timing_function = Some(Value::Specified(v));
        self
    }
    pub fn transition_delay(mut self, v: Vec<u32>) -> Self {
        self.transition_delay = Some(Value::Specified(v));
        self
    }
    /// Mark the four transition longhands `!important`, as a
    /// `transition: … !important` declaration does (each longhand has
    /// its own bit; `ImportantMask::TRANSITIONS` is their union).
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
}
