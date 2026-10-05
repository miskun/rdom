//! `TuiNodeMutExt` — the builder-chain setters on `NodeMut`, each a
//! declaration in the element's inline style.

use rdom_core::NodeMut;

use crate::ext::TuiExt;
use crate::layout::{Border, BorderRadius, BoxSizing, Corners, Direction, Overflow, Padding, Size};
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
    /// Declare `width` inline. A flex weight is kept in `<number [0,∞]>`
    /// ([`Size::validated`]).
    fn set_width(&mut self, w: impl Into<Size>) -> &mut Self {
        let w = w.into().validated();
        self.write_inline_style(|s| s.width = Some(Value::Specified(w)));
        self
    }
    /// Declare `height` inline. A flex weight is kept in `<number
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
    fn set_direction(&mut self, d: Direction) -> &mut Self {
        self.write_inline_style(|s| s.direction = Some(Value::Specified(d)));
        self
    }
    fn set_padding(&mut self, p: Padding) -> &mut Self {
        self.write_inline_style(|s| s.padding = Some(Value::Specified(p)));
        self
    }
    /// Set the inline style's four `border-*-style`s.
    fn set_border(&mut self, b: Border) -> &mut Self {
        self.write_inline_style(|s| s.border_style = b.sides().map(|s| Some(Value::Specified(s))));
        self
    }
    /// Set the inline style's four `border-*-radius`es (CSS Backgrounds 3
    /// §5.1; `BorderRadius::cells(1.0)` rounds every corner), as the
    /// `TuiStyle::border_radius` builder does.
    fn set_border_radius(&mut self, r: BorderRadius) -> &mut Self {
        self.write_inline_style(|s| s.border_radius = Corners::all(Some(Value::Specified(r))));
        self
    }
    fn set_gap(&mut self, g: u16) -> &mut Self {
        self.write_inline_style(|s| {
            s.gap = Some(Value::Specified(crate::layout::GapValue::Cells(g)))
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
    fn set_scroll(&mut self, x: usize, y: usize) -> &mut Self {
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
