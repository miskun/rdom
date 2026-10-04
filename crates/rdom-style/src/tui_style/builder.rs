//! The `TuiStyle` builder: chainable setters for every property
//! (`.fg(…)`, `.padding(…)`, `….._important(…)`).

use super::{ImportantMask, TuiStyle};
#[allow(unused_imports)]
use crate::layout::{
    Border, CaretColor, CaretTextColor, Direction, Display, Overflow, Padding, Size,
    TextDecoration, UserSelect, WhiteSpace,
};
use crate::{Content, TuiColor, Value};

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
    /// Set the `max-width` property. Accepts a `u16` (cells) or a
    /// [`MaxSize`](crate::layout::MaxSize). Chainable.
    pub fn max_width(mut self, v: impl Into<crate::layout::MaxSize>) -> Self {
        self.max_width = Some(Value::Specified(v.into()));
        self
    }
    /// Like `max_width` but marks the declaration `!important`.
    pub fn max_width_important(mut self, v: impl Into<crate::layout::MaxSize>) -> Self {
        self.max_width = Some(Value::Specified(v.into()));
        self.important |= ImportantMask::MAX_WIDTH;
        self
    }
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
    /// Set the `max-height` property. Accepts a `u16` (cells) or a
    /// [`MaxSize`](crate::layout::MaxSize). Chainable.
    pub fn max_height(mut self, v: impl Into<crate::layout::MaxSize>) -> Self {
        self.max_height = Some(Value::Specified(v.into()));
        self
    }
    /// Like `max_height` but marks the declaration `!important`.
    pub fn max_height_important(mut self, v: impl Into<crate::layout::MaxSize>) -> Self {
        self.max_height = Some(Value::Specified(v.into()));
        self.important |= ImportantMask::MAX_HEIGHT;
        self
    }
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
    /// Set `gap`: whole cells (`gap(2)`) or a `calc()` / percentage
    /// (`gap(GapValue::from(CalcExpr::Percent(10.0)))`), resolved at
    /// layout. Chainable.
    pub fn gap(mut self, v: impl Into<crate::layout::GapValue>) -> Self {
        self.gap = Some(Value::Specified(v.into()));
        self
    }

    /// Like `gap` but also marks the declaration `!important`.
    pub fn gap_important(mut self, v: impl Into<crate::layout::GapValue>) -> Self {
        self.gap = Some(Value::Specified(v.into()));
        self.important |= ImportantMask::GAP;
        self
    }
    setter!(
        flex_shrink,
        flex_shrink,
        flex_shrink_important,
        FLEX_SHRINK,
        f32
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
        counter_reset,
        counter_reset,
        counter_reset_important,
        COUNTER_RESET,
        Vec<crate::counters::CounterOp>
    );
    setter!(
        counter_increment,
        counter_increment,
        counter_increment_important,
        COUNTER_INCREMENT,
        Vec<crate::counters::CounterOp>
    );

    /// `display: flex` — outer [`Display::Block`] + inner
    /// [`Flow::Flex`](crate::layout::Flow::Flex). Mirrors the CSS
    /// `display: flex` keyword.
    ///
    /// Prefer this over `.display(Display::Block).flow(Flow::Flex)`:
    /// `.display(...)` *resets* `flow` (e.g. `Display::Block` forces
    /// `Flow::Block`), so `.flow(Flex).display(Block)` would silently
    /// clobber the flex flow. `.flex()` sets both in the right order.
    pub fn flex(mut self) -> Self {
        self.display = Some(Value::Specified(Display::Block));
        self.flow = Some(Value::Specified(crate::layout::Flow::Flex));
        self
    }

    /// `display: flex; flex-direction: row`.
    pub fn flex_row(self) -> Self {
        self.flex().direction(crate::layout::Direction::Row)
    }

    /// `display: flex; flex-direction: column`.
    pub fn flex_column(self) -> Self {
        self.flex().direction(crate::layout::Direction::Column)
    }

    /// `display: inline-flex` — outer [`Display::Inline`] + inner
    /// [`Flow::Flex`](crate::layout::Flow::Flex).
    pub fn inline_flex(mut self) -> Self {
        self.display = Some(Value::Specified(Display::Inline));
        self.flow = Some(Value::Specified(crate::layout::Flow::Flex));
        self
    }

    setter!(
        scrollbar_gutter,
        scrollbar_gutter,
        scrollbar_gutter_important,
        SCROLLBAR_GUTTER,
        crate::layout::ScrollbarGutter
    );
    setter!(
        scroll_behavior,
        scroll_behavior,
        scroll_behavior_important,
        SCROLL_BEHAVIOR,
        crate::layout::ScrollBehavior
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
        pointer_events,
        pointer_events,
        pointer_events_important,
        POINTER_EVENTS,
        crate::layout::PointerEvents
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
}
