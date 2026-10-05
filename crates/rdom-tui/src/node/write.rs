//! `TuiNodeMutExt` — the builder-chain setters on `NodeMut`, each a
//! declaration in the element's inline style.

use rdom_core::NodeMut;

use crate::ext::TuiExt;
use crate::layout::{
    Alignment, Border, BorderRadius, BoxSizing, Corners, Direction, FlexDirection, FlexWrap,
    GridAutoFlow, GridLine, GridTemplate, Margin, MarginTrim, Overflow, Padding, Size,
    TextDirection, TrackSize, Visibility, WritingMode,
};
use crate::style::{TuiStyle, Value};

/// Mutation helpers for `TuiExt`-bearing elements. All methods return
/// `&mut self` for chaining; they silently no-op on non-Element nodes
/// (matching how `set_attribute` behaves in rdom-core — errors there,
/// no-ops here to keep builder chains readable).
pub trait TuiNodeMutExt<'a>: crate::sealed::Sealed {
    fn tui_ext_mut(&mut self) -> Option<&mut TuiExt>;

    /// Apply `f` to the element's inline declarations — the one write
    /// path under every setter below. `NodeMut`'s implementation also
    /// reflects the result into the `style` attribute, as a CSSOM write
    /// does (`cssom::inline`, `P7G-SETTER-MUTATION-1`): that attribute
    /// write is the DOM mutation the `App`'s dirty tracker restyles
    /// from, so a setter called from a listener, timer or injected
    /// closure reaches the next frame. The default, for an implementor
    /// with no DOM access, writes the `TuiExt` slot only and marks it
    /// style-dirty; nothing is queued for an `App`'s next cascade.
    fn write_inline_style(&mut self, f: impl FnOnce(&mut TuiStyle)) {
        if let Some(e) = self.tui_ext_mut() {
            f(e.inline_style_mut());
            e.style_dirty = true;
        }
    }

    // Geometry setters write the element's **inline style** (the cascade
    // input) through `write_inline_style`, so the next cascade carries
    // them into `ComputedStyle` — the only thing layout reads. (Before
    // `EXT-LAYOUT-SETTERS-1` these wrote raw `ext` fields that layout
    // ignored.) On a `NodeMut` each call also rewrites the `style`
    // attribute.
    /// Declare `width` inline: the width of the box `box-sizing` names —
    /// the content box by default, the border box under `box-sizing:
    /// border-box` (CSS UI 3 §3.1). A flex weight is kept in `<number
    /// [0,∞]>` ([`Size::validated`]).
    fn set_width(&mut self, w: impl Into<Size>) -> &mut Self {
        let w = w.into().validated();
        self.write_inline_style(|s| s.width = Some(Value::Specified(w)));
        self
    }
    /// Declare `height` inline: the height of the box `box-sizing` names
    /// — the content box by default, the border box under `box-sizing:
    /// border-box` (CSS UI 3 §3.1). A flex weight is kept in `<number
    /// [0,∞]>` ([`Size::validated`]).
    fn set_height(&mut self, h: impl Into<Size>) -> &mut Self {
        let h = h.into().validated();
        self.write_inline_style(|s| s.height = Some(Value::Specified(h)));
        self
    }
    /// Declare `min-width` inline: a `u16` (cells) or a
    /// [`MinSize`](rdom_style::layout::MinSize). Remove the declaration
    /// with `style_mut().remove_property("min-width")`.
    fn set_min_width(&mut self, v: impl Into<rdom_style::layout::MinSize>) -> &mut Self {
        let v = v.into();
        self.write_inline_style(|s| s.min_width = Some(Value::Specified(v)));
        self
    }
    /// Declare `max-width` inline: a `u16` (cells) or a
    /// [`MaxSize`](rdom_style::layout::MaxSize) — `MaxSize::None` for
    /// `none` (CSS Sizing 3 §5.2). Remove the declaration with
    /// `style_mut().remove_property("max-width")`.
    fn set_max_width(&mut self, v: impl Into<rdom_style::layout::MaxSize>) -> &mut Self {
        let v = v.into();
        self.write_inline_style(|s| s.max_width = Some(Value::Specified(v)));
        self
    }
    /// Declare `min-height` inline ([`Self::set_min_width`]).
    fn set_min_height(&mut self, v: impl Into<rdom_style::layout::MinSize>) -> &mut Self {
        let v = v.into();
        self.write_inline_style(|s| s.min_height = Some(Value::Specified(v)));
        self
    }
    /// Declare `max-height` inline ([`Self::set_max_width`]).
    fn set_max_height(&mut self, v: impl Into<rdom_style::layout::MaxSize>) -> &mut Self {
        let v = v.into();
        self.write_inline_style(|s| s.max_height = Some(Value::Specified(v)));
        self
    }
    /// Declare `box-sizing` inline (CSS UI 3 §3.1): which box `width` /
    /// `height` and `min-*` / `max-*` measure.
    fn set_box_sizing(&mut self, b: BoxSizing) -> &mut Self {
        self.write_inline_style(|s| s.box_sizing = Some(Value::Specified(b)));
        self
    }
    /// Declare `direction` inline (CSS Writing Modes 4 §2.1): the inline
    /// base direction. (`set_direction` is `flex-direction`.)
    fn set_text_direction(&mut self, d: TextDirection) -> &mut Self {
        self.write_inline_style(|s| s.text_direction = Some(Value::Specified(d)));
        self
    }
    /// Declare `writing-mode` inline (CSS Writing Modes 4 §3.1).
    fn set_writing_mode(&mut self, m: WritingMode) -> &mut Self {
        self.write_inline_style(|s| s.writing_mode = Some(Value::Specified(m)));
        self
    }
    /// Declare `margin-trim` inline (CSS Box 4 §3).
    fn set_margin_trim(&mut self, t: MarginTrim) -> &mut Self {
        self.write_inline_style(|s| s.margin_trim = Some(Value::Specified(t)));
        self
    }
    /// Declare `flex-direction` inline (`direction` is
    /// [`set_text_direction`](Self::set_text_direction)).
    fn set_direction(&mut self, d: Direction) -> &mut Self {
        self.write_inline_style(|s| {
            s.direction = Some(Value::Specified(d));
            s.flex_reverse = Some(Value::Specified(false));
        });
        self
    }
    /// Declare `flex-direction` inline as one value (CSS Flexbox §5.1):
    /// its axis and whether it is reversed.
    fn set_flex_direction(&mut self, d: FlexDirection) -> &mut Self {
        self.write_inline_style(|s| *s = std::mem::take(s).flex_direction(d));
        self
    }
    /// Declare `flex-wrap` inline (CSS Flexbox §5.2).
    fn set_flex_wrap(&mut self, w: FlexWrap) -> &mut Self {
        self.write_inline_style(|s| s.flex_wrap = Some(Value::Specified(w)));
        self
    }
    /// Declare `order` inline (CSS Flexbox §5.4).
    fn set_order(&mut self, order: i32) -> &mut Self {
        self.write_inline_style(|s| s.order = Some(Value::Specified(order)));
        self
    }
    /// Declare `grid-template-columns` inline (CSS Grid 2 §7.2), through
    /// [`TuiStyle::grid_template_columns`] (an invalid list is refused).
    fn set_grid_template_columns(&mut self, t: impl Into<GridTemplate>) -> &mut Self {
        self.write_inline_style(|s| *s = std::mem::take(s).grid_template_columns(t));
        self
    }
    /// Declare `grid-template-rows` inline (CSS Grid 2 §7.2).
    fn set_grid_template_rows(&mut self, t: impl Into<GridTemplate>) -> &mut Self {
        self.write_inline_style(|s| *s = std::mem::take(s).grid_template_rows(t));
        self
    }
    /// Declare `grid-auto-columns` inline (CSS Grid 2 §7.6), through
    /// [`TuiStyle::grid_auto_columns`] (an empty or invalid list is refused).
    fn set_grid_auto_columns(&mut self, sizes: impl IntoIterator<Item = TrackSize>) -> &mut Self {
        self.write_inline_style(|s| *s = std::mem::take(s).grid_auto_columns(sizes));
        self
    }
    /// Declare `grid-auto-rows` inline (CSS Grid 2 §7.6).
    fn set_grid_auto_rows(&mut self, sizes: impl IntoIterator<Item = TrackSize>) -> &mut Self {
        self.write_inline_style(|s| *s = std::mem::take(s).grid_auto_rows(sizes));
        self
    }
    /// Declare `grid-auto-flow` inline (CSS Grid 2 §7.7).
    fn set_grid_auto_flow(&mut self, flow: GridAutoFlow) -> &mut Self {
        self.write_inline_style(|s| s.grid_auto_flow = Some(Value::Specified(flow)));
        self
    }
    /// Declare `grid-row` inline (CSS Grid 2 §8.4): its start and end
    /// lines, each checked as [`TuiStyle::grid_row_start`] checks it.
    fn set_grid_row(&mut self, start: GridLine, end: GridLine) -> &mut Self {
        self.write_inline_style(|s| *s = std::mem::take(s).grid_row(start, end));
        self
    }
    /// Declare `grid-column` inline (CSS Grid 2 §8.4).
    fn set_grid_column(&mut self, start: GridLine, end: GridLine) -> &mut Self {
        self.write_inline_style(|s| *s = std::mem::take(s).grid_column(start, end));
        self
    }
    /// Declare `visibility` inline (CSS Display 3 §4).
    fn set_visibility(&mut self, v: Visibility) -> &mut Self {
        self.write_inline_style(|s| s.visibility = Some(Value::Specified(v)));
        self
    }
    /// Declare `justify-content` inline (CSS Box Alignment 3 §5.2): a
    /// keyword or an [`Alignment`](crate::layout::Alignment), checked
    /// against the property's grammar as the
    /// [`TuiStyle::justify_content`] builder checks it.
    fn set_justify_content(&mut self, v: impl Into<Alignment>) -> &mut Self {
        let v = v.into();
        self.write_inline_style(|s| *s = std::mem::take(s).justify_content(v));
        self
    }
    /// Declare `align-content` inline (§5.1), checked as
    /// [`Self::set_justify_content`] is.
    fn set_align_content(&mut self, v: impl Into<Alignment>) -> &mut Self {
        let v = v.into();
        self.write_inline_style(|s| *s = std::mem::take(s).align_content(v));
        self
    }
    /// Declare `justify-items` inline (§6.2), checked as
    /// [`Self::set_justify_content`] is.
    fn set_justify_items(&mut self, v: impl Into<Alignment>) -> &mut Self {
        let v = v.into();
        self.write_inline_style(|s| *s = std::mem::take(s).justify_items(v));
        self
    }
    /// Declare `align-items` inline (§6.3), checked as
    /// [`Self::set_justify_content`] is.
    fn set_align_items(&mut self, v: impl Into<Alignment>) -> &mut Self {
        let v = v.into();
        self.write_inline_style(|s| *s = std::mem::take(s).align_items(v));
        self
    }
    /// Declare `justify-self` inline (§6.1), checked as
    /// [`Self::set_justify_content`] is.
    fn set_justify_self(&mut self, v: impl Into<Alignment>) -> &mut Self {
        let v = v.into();
        self.write_inline_style(|s| *s = std::mem::take(s).justify_self(v));
        self
    }
    /// Declare `align-self` inline (§6.1), checked as
    /// [`Self::set_justify_content`] is.
    fn set_align_self(&mut self, v: impl Into<Alignment>) -> &mut Self {
        let v = v.into();
        self.write_inline_style(|s| *s = std::mem::take(s).align_self(v));
        self
    }
    /// Declare the four `padding-*` longhands inline: a [`Padding`] or a
    /// plain `u16` (every side), as the `TuiStyle::padding` builder.
    fn set_padding(&mut self, p: impl Into<Padding>) -> &mut Self {
        let p = p.into();
        self.write_inline_style(|s| *s = std::mem::take(s).padding(p));
        self
    }
    /// Declare the four `margin-*` longhands inline: a [`Margin`] or a
    /// plain `i16` (every side), as the `TuiStyle::margin` builder.
    fn set_margin(&mut self, m: impl Into<Margin>) -> &mut Self {
        let m = m.into();
        self.write_inline_style(|s| *s = std::mem::take(s).margin(m));
        self
    }
    /// Set the inline style's four `border-*-style`s.
    fn set_border(&mut self, b: Border) -> &mut Self {
        self.write_inline_style(|s| s.border_style = b.sides().map(|s| Some(Value::Specified(s))));
        self
    }
    /// Set the inline style's four `border-*-radius`es (CSS Backgrounds 3
    /// §5.1): one [`BorderRadius`] for every corner
    /// (`BorderRadius::cells(1.0)` rounds them all) or a [`Corners`] of
    /// them — the shape [`border_radius`](crate::TuiNodeExt::border_radius)
    /// reads back — as the `TuiStyle::border_radius` builder does.
    fn set_border_radius(&mut self, r: impl Into<Corners<BorderRadius>>) -> &mut Self {
        let corners = r.into();
        self.write_inline_style(|s| {
            s.border_radius = corners.map(|r| Some(Value::Specified(r)));
        });
        self
    }
    fn set_gap(&mut self, g: u16) -> &mut Self {
        self.write_inline_style(|s| {
            s.row_gap = Some(Value::Specified(crate::layout::GapValue::Cells(g)));
            s.column_gap = Some(Value::Specified(crate::layout::GapValue::Cells(g)));
        });
        self
    }
    fn set_overflow(&mut self, o: Overflow) -> &mut Self {
        self.write_inline_style(|s| {
            s.overflow_x = Some(Value::Specified(o));
            s.overflow_y = Some(Value::Specified(o));
        });
        self
    }
    /// Replace the inline declarations wholesale (an empty style clears
    /// them); reflected into `style` like the other setters.
    fn set_inline_style(&mut self, style: TuiStyle) -> &mut Self {
        self.write_inline_style(|s| *s = style);
        self
    }
    fn set_before_content(&mut self, text: impl Into<String>) -> &mut Self {
        if let Some(e) = self.tui_ext_mut() {
            e.before_content = Some(text.into());
        }
        self
    }
    fn clear_before_content(&mut self) -> &mut Self {
        if let Some(e) = self.tui_ext_mut() {
            e.before_content = None;
        }
        self
    }
    fn set_after_content(&mut self, text: impl Into<String>) -> &mut Self {
        if let Some(e) = self.tui_ext_mut() {
            e.after_content = Some(text.into());
        }
        self
    }
    fn clear_after_content(&mut self) -> &mut Self {
        if let Some(e) = self.tui_ext_mut() {
            e.after_content = None;
        }
        self
    }
    /// Write the raw scroll offsets (`TuiExt::scroll_x` / `scroll_y`).
    /// Builder-time setup; at run time prefer
    /// [`TuiAccessorsMut::scroll_to`](crate::TuiAccessorsMut::scroll_to),
    /// which clamps and fires `scroll`. Either way the `App` repaints
    /// on its next frame.
    fn set_scroll(&mut self, x: i32, y: i32) -> &mut Self {
        if let Some(e) = self.tui_ext_mut() {
            e.scroll_x = x;
            e.scroll_y = y;
        }
        self
    }
}

impl<'a> TuiNodeMutExt<'a> for NodeMut<'a, TuiExt> {
    fn tui_ext_mut(&mut self) -> Option<&mut TuiExt> {
        self.ext_mut()
    }

    fn write_inline_style(&mut self, f: impl FnOnce(&mut TuiStyle)) {
        let id = self.id();
        crate::cssom::inline::write_inline_style(self.dom_mut(), id, f);
    }
}
