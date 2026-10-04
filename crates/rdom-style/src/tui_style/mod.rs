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
#[cfg(test)]
use crate::layout::Border;
use crate::layout::{
    CaretColor, CaretTextColor, Direction, Display, Overflow, Padding, Sides, Size, TextDecoration,
    UserSelect, WhiteSpace,
};
use crate::{Content, TuiColor, Value};

use rdom_core::bitflags_like;

bitflags_like! {
    /// One bit per `TuiStyle` property — flipped when the author wrote
    /// `!important` on that declaration. Kept parallel to the fields
    /// rather than wrapping each in `(Value<T>, bool)` to keep the hot
    /// property accessors cheap.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct ImportantMask(u128) {
        FG         = 1 << 0;
        BG         = 1 << 1;
        // Bit 2 was `BORDER_FG`, replaced by the per-side `BORDER_*_COLOR`.
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
        // Bit 16 was `BORDER`, replaced by the per-side `BORDER_*_STYLE`.
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
        POINTER_EVENTS = 1 << 39;
        SCROLLBAR_GUTTER = 1 << 40;
        FLOW = 1 << 41;
        COUNTER_RESET = 1 << 42;
        COUNTER_INCREMENT = 1 << 43;
        SCROLL_BEHAVIOR = 1 << 44;
        FLEX_BASIS = 1 << 45;
        COLOR_SCHEME = 1 << 46;
        // ── Backgrounds (CSS Backgrounds 3 §3) ──
        BACKGROUND_IMAGE = 1 << 47;
        BACKGROUND_POSITION = 1 << 48;
        BACKGROUND_SIZE = 1 << 49;
        BACKGROUND_REPEAT = 1 << 50;
        BACKGROUND_ATTACHMENT = 1 << 51;
        BACKGROUND_ORIGIN = 1 << 52;
        BACKGROUND_CLIP = 1 << 53;
        // ── Borders (CSS Backgrounds 3 §4), one bit per longhand ──
        BORDER_TOP_COLOR = 1 << 54;
        BORDER_RIGHT_COLOR = 1 << 55;
        BORDER_BOTTOM_COLOR = 1 << 56;
        BORDER_LEFT_COLOR = 1 << 57;
        BORDER_TOP_WIDTH = 1 << 58;
        BORDER_RIGHT_WIDTH = 1 << 59;
        BORDER_BOTTOM_WIDTH = 1 << 60;
        BORDER_LEFT_WIDTH = 1 << 61;
        BORDER_TOP_STYLE = 1 << 62;
        BORDER_RIGHT_STYLE = 1 << 63;
        BORDER_BOTTOM_STYLE = 1 << 64;
        BORDER_LEFT_STYLE = 1 << 65;
        BORDER_TOP_LEFT_RADIUS = 1 << 66;
        BORDER_TOP_RIGHT_RADIUS = 1 << 67;
        BORDER_BOTTOM_RIGHT_RADIUS = 1 << 68;
        BORDER_BOTTOM_LEFT_RADIUS = 1 << 69;
        BOX_SHADOW = 1 << 70;
        BORDER_SPACING = 1 << 71;
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
    /// `border-top-color` … `border-left-color` (CSS Backgrounds 3
    /// §4.1), one longhand per side; initial `currentcolor`.
    pub border_color: Sides<Option<Value<TuiColor>>>,
    /// `background-image` (CSS Backgrounds 3 §3.3), one entry per
    /// layer, each `none`, `url("…")` or a gradient as CSS text. Inert:
    /// rdom draws no images (DIVERGENCES §1).
    pub background_image: Option<Value<Vec<String>>>,
    /// `background-position` (§3.6), per layer, as CSS text. Inert.
    pub background_position: Option<Value<Vec<String>>>,
    /// `background-size` (§3.9), per layer, as CSS text. Inert.
    pub background_size: Option<Value<Vec<String>>>,
    /// `background-repeat` (§3.4), per layer. Inert.
    pub background_repeat: Option<Value<Vec<crate::layout::BackgroundRepeat>>>,
    /// `background-attachment` (§3.5), per layer. Inert.
    pub background_attachment: Option<Value<Vec<crate::layout::BackgroundAttachment>>>,
    /// `background-origin` (§3.7), per layer. Inert.
    pub background_origin: Option<Value<Vec<crate::layout::VisualBox>>>,
    /// `background-clip` (§3.8), per layer.
    pub background_clip: Option<Value<Vec<crate::layout::VisualBox>>>,
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
    pub aspect_ratio: Option<Value<Option<crate::layout::AspectRatio>>>,

    // ── Layout ────────────────────────────────────────────────────────
    pub width: Option<Value<Size>>,
    pub height: Option<Value<Size>>,
    pub min_width: Option<Value<crate::layout::MinSize>>,
    pub max_width: Option<Value<crate::layout::MaxSize>>,
    pub min_height: Option<Value<crate::layout::MinSize>>,
    pub max_height: Option<Value<crate::layout::MaxSize>>,
    pub padding: Option<Value<Padding>>,
    pub margin: Option<Value<crate::layout::Margin>>,
    pub gap: Option<Value<crate::layout::GapValue>>,
    /// CSS `flex-shrink`. Default `1` per CSS spec — when total
    /// declared flex-item sizes exceed the parent's main axis,
    /// items shrink proportional to `flex_shrink * basis`. `0`
    /// opts out of shrinking (the item keeps its declared size
    /// and overflows). Larger values shrink more aggressively.
    pub flex_shrink: Option<Value<f32>>,
    /// `flex-basis`, set by the `flex` shorthand (CSS Flexbox §7.2).
    pub flex_basis: Option<Value<crate::layout::FlexBasis>>,
    /// `border-top-style` … `border-left-style` (CSS Backgrounds 3
    /// §4.2), one longhand per side; initial `none`.
    pub border_style: Sides<Option<Value<crate::layout::BorderStyle>>>,
    /// `border-top-width` … `border-left-width` (CSS Backgrounds 3
    /// §4.3), one longhand per side; initial `medium`.
    pub border_width: Sides<Option<Value<crate::layout::BorderWidth>>>,
    /// `border-top-left-radius` … `border-bottom-left-radius` (CSS
    /// Backgrounds 3 §5.1), one longhand per corner; initial `0`.
    pub border_radius: crate::layout::Corners<Option<Value<crate::layout::BorderRadius>>>,
    /// `box-shadow` (CSS Backgrounds 3 §6.1): the shadows front to back;
    /// `none` is the empty list.
    pub box_shadow: Option<Value<Vec<crate::layout::BoxShadow>>>,
    /// `border-spacing` (CSS 2.1 §17.6.1). Inherited.
    pub border_spacing: Option<Value<crate::layout::BorderSpacing>>,
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
    /// `scroll-behavior: auto | smooth` (CSSOM View §12.1). Whether a
    /// programmatic scroll of this container animates. Default `Auto`.
    pub scroll_behavior: Option<Value<crate::layout::ScrollBehavior>>,

    // ── Inline formatting ────────────────────────────────────────────
    /// Outer display. Set by `display: <kw>` keywords. The companion
    /// `flow` field captures the inner display (block vs flex layout
    /// of THIS element's children). Both are written by the `display`
    /// property parser: `display: flex` sets `display = Some(Block)`
    /// AND `flow = Some(Flex)`; `display: block` sets `display =
    /// Some(Block)` + `flow = Some(Block)`; etc.
    pub display: Option<Value<Display>>,
    /// Inner display — how this element lays out its children.
    /// Written alongside `display` by the same parser. See [`Flow`](crate::layout::Flow)
    /// for the mapping table.
    pub flow: Option<Value<crate::layout::Flow>>,
    pub white_space: Option<Value<WhiteSpace>>,
    pub user_select: Option<Value<UserSelect>>,
    /// CSS `pointer-events` (`auto` | `none`). Inherited.
    pub pointer_events: Option<Value<crate::layout::PointerEvents>>,
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
    pub transition_property: Option<Value<Vec<crate::transition::TransitionProperty>>>,
    /// `transition-duration` longhand, in milliseconds.
    pub transition_duration: Option<Value<Vec<u32>>>,
    /// `transition-timing-function` longhand.
    pub transition_timing_function: Option<Value<Vec<crate::transition::TimingFunction>>>,
    /// `transition-delay` longhand, in milliseconds.
    pub transition_delay: Option<Value<Vec<u32>>>,

    // ── Counters (CSS Lists 3 §3.1) ──────────────────────────────────
    pub counter_reset: Option<Value<Vec<crate::counters::CounterOp>>>,
    pub counter_increment: Option<Value<Vec<crate::counters::CounterOp>>>,

    // ── Color adjustment (CSS Color Adjust 1) ────────────────────────
    /// `color-scheme` (§2): the color schemes the element supports,
    /// which pick `light-dark()`'s color. Inherits.
    pub color_scheme: Option<Value<crate::color::ColorSchemeList>>,

    // ── Custom properties (CSS Variables 1) ──────────────────────────
    /// `--name: value` declarations, in source order, names without
    /// the `--`. The cascade folds them into the element's inherited
    /// variable map before any `var()` consumer resolves
    /// (`CSS-VARS-SCOPE-1`). Importance is per declaration here, not
    /// in `important`, because the set of names is open.
    pub custom_properties: Vec<CustomDeclaration>,

    // ── `var()` (CSS Variables 1 §3) ──────────────────────────────────
    /// Declarations whose value holds `var()`, kept as tokens for the
    /// cascade to substitute per element, and the block's declarations
    /// after the first of them (replayed in order). Empty for a style
    /// without `var()` (`crate::var`).
    pub pending: Vec<crate::var::PendingDeclaration>,

    // ── `!important` bits ─────────────────────────────────────────────
    pub important: ImportantMask,
}

/// One custom-property declaration (`--name: value [!important]`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CustomDeclaration {
    /// Name without the leading `--`.
    pub name: String,
    /// The value as written (custom properties are untyped), tokenized
    /// once here.
    pub value: crate::CustomValue,
    pub important: bool,
}

impl TuiStyle {
    pub fn new() -> Self {
        Self::default()
    }

    /// This block restricted to the properties that apply to
    /// `::first-line` — and so to `::placeholder` (CSS Pseudo-Elements 4
    /// §2.1.1, §4.3) — among those rdom has: `color`, `background-color`,
    /// the font properties (`font-weight` / `font-style`),
    /// `text-decoration`, `opacity`, and custom properties. Everything
    /// else is dropped with its `!important` bit.
    pub fn first_line_subset(&self) -> Self {
        let keep = ImportantMask::FG
            | ImportantMask::BG
            | ImportantMask::BOLD
            | ImportantMask::ITALIC
            | ImportantMask::TEXT_DECORATION
            | ImportantMask::OPACITY;
        Self {
            fg: self.fg.clone(),
            bg: self.bg.clone(),
            bold: self.bold,
            italic: self.italic,
            text_decoration: self.text_decoration,
            opacity: self.opacity,
            custom_properties: self.custom_properties.clone(),
            important: self.important & keep,
            ..Self::default()
        }
    }

    // Paint color setters — accept both `Color::Rgb(255, 0, 0)` and
    // `TuiColor::var("accent")` via `impl Into<TuiColor>`.

    /// True when no field is set. Empty `TuiStyle` is the `Default`.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Declare the custom property `--name` (with or without the
    /// dashes) as `value`. Chainable.
    pub fn custom_property(mut self, name: &str, value: &str) -> Self {
        self.set_custom_property(name, value, false);
        self
    }

    /// Declare or replace `--name`; a later declaration of the same
    /// name wins, as in a CSS block.
    pub fn set_custom_property(&mut self, name: &str, value: &str, important: bool) {
        let name = name.strip_prefix("--").unwrap_or(name);
        let value = crate::CustomValue::new(value);
        if let Some(d) = self.custom_properties.iter_mut().find(|d| d.name == name) {
            d.value = value;
            d.important = important;
        } else {
            self.custom_properties.push(CustomDeclaration {
                name: name.to_string(),
                value,
                important,
            });
        }
    }

    /// The declared value of `--name`, if any.
    pub fn custom_property_value(&self, name: &str) -> Option<&str> {
        let name = name.strip_prefix("--").unwrap_or(name);
        self.custom_properties
            .iter()
            .find(|d| d.name == name)
            .map(|d| d.value.as_str())
    }

    /// Drop `--name`; `true` if it was declared.
    pub fn remove_custom_property(&mut self, name: &str) -> bool {
        let name = name.strip_prefix("--").unwrap_or(name);
        let before = self.custom_properties.len();
        self.custom_properties.retain(|d| d.name != name);
        self.custom_properties.len() != before
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
        n += self
            .border_color
            .each()
            .iter()
            .filter(|c| c.is_some())
            .count();
        n += self
            .border_width
            .each()
            .iter()
            .filter(|w| w.is_some())
            .count();
        n += [
            self.background_image.is_some(),
            self.background_position.is_some(),
            self.background_size.is_some(),
            self.background_repeat.is_some(),
            self.background_attachment.is_some(),
            self.background_origin.is_some(),
            self.background_clip.is_some(),
        ]
        .iter()
        .filter(|set| **set)
        .count();
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
        n += self
            .border_style
            .each()
            .iter()
            .filter(|s| s.is_some())
            .count();
        n += self
            .border_radius
            .each()
            .iter()
            .filter(|r| r.is_some())
            .count();
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
        if self.scroll_behavior.is_some() {
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
        if self.pointer_events.is_some() {
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
        if self.color_scheme.is_some() {
            n += 1
        }
        n += self.custom_properties.len();
        n += self.pending.iter().filter(|d| d.has_substitution).count();
        n
    }
}

mod builder;
#[cfg(test)]
mod tests;
