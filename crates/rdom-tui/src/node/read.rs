//! `TuiNodeExt` — read access to an element's `TuiExt` fields.

use rdom_core::NodeRef;

use crate::ext::TuiExt;
use crate::layout::{
    Border, BorderRadius, BoxSizing, Corners, Direction, LayoutRect, MarginTrim, Overflow, Padding,
    Size, TextDirection, WritingMode,
};
use crate::style::{ComputedStyle, TuiStyle};

use super::tree::is_text_input;

/// Readonly sugar over `NodeRef<'_, TuiExt>::ext()`. The underlying
/// `ext()` on rdom-core returns `Option<&Ext>` already; this trait adds
/// field-level getters so callers don't have to destructure.
pub trait TuiNodeExt<'a>: crate::sealed::Sealed {
    fn tui_ext(&self) -> Option<&'a TuiExt>;

    // These read the *specified* inline-style value (`None` when the
    // property wasn't set via a node setter or inline style). Layout
    // reads the post-cascade `ComputedStyle`; these are the author-input
    // side, kept symmetric with the `set_*` setters in `TuiNodeMutExt`.
    /// The inline style's `width` — of the box `box-sizing` names (CSS
    /// UI 3 §3.1) — when set.
    fn width(&self) -> Option<Size> {
        self.inline_style()
            .and_then(|s| s.width.as_ref())
            .and_then(|v| v.as_specified().cloned())
    }
    /// The inline style's `height` — of the box `box-sizing` names (CSS
    /// UI 3 §3.1) — when set.
    fn height(&self) -> Option<Size> {
        self.inline_style()
            .and_then(|s| s.height.as_ref())
            .and_then(|v| v.as_specified().cloned())
    }
    /// The inline style's `box-sizing` (CSS UI 3 §3.1), when set.
    fn box_sizing(&self) -> Option<BoxSizing> {
        self.inline_style()
            .and_then(|s| s.box_sizing.as_ref())
            .and_then(|v| v.as_specified().copied())
    }
    /// The inline style's `direction` (CSS Writing Modes 4 §2.1), when
    /// set. ([`direction`](Self::direction) is `flex-direction`.)
    fn text_direction(&self) -> Option<TextDirection> {
        self.inline_style()
            .and_then(|s| s.text_direction.as_ref())
            .and_then(|v| v.as_specified().copied())
    }
    /// The inline style's `writing-mode` (CSS Writing Modes 4 §3.1), when
    /// set.
    fn writing_mode(&self) -> Option<WritingMode> {
        self.inline_style()
            .and_then(|s| s.writing_mode.as_ref())
            .and_then(|v| v.as_specified().copied())
    }
    /// The inline style's `margin-trim` (CSS Box 4 §3), when set.
    fn margin_trim(&self) -> Option<MarginTrim> {
        self.inline_style()
            .and_then(|s| s.margin_trim.as_ref())
            .and_then(|v| v.as_specified().copied())
    }
    /// The inline style's `flex-direction`, when set.
    fn direction(&self) -> Option<Direction> {
        self.inline_style()
            .and_then(|s| s.direction.as_ref())
            .and_then(|v| v.as_specified().copied())
    }
    /// The inline style's four `padding-*`s, when every one is set.
    fn padding(&self) -> Option<Padding> {
        let sides = &self.inline_style()?.padding;
        let [top, right, bottom, left] = sides.each().map(|s| s.as_ref()?.as_specified().cloned());
        Some(Padding {
            top: top?,
            right: right?,
            bottom: bottom?,
            left: left?,
        })
    }
    /// The inline style's four `border-*-style`s, when every one is set.
    fn border(&self) -> Option<Border> {
        let sides = &self.inline_style()?.border_style;
        let [top, right, bottom, left] = sides.each().map(|s| s.as_ref()?.as_specified().copied());
        Some(Border::new(top?, right?, bottom?, left?))
    }
    /// The inline style's four `border-*-radius`es, when every one is set.
    fn border_radius(&self) -> Option<Corners<BorderRadius>> {
        let corners = &self.inline_style()?.border_radius;
        let [tl, tr, br, bl] = corners.each().map(|r| r.as_ref()?.as_specified().cloned());
        Some(Corners::new(tl?, tr?, br?, bl?))
    }
    /// The inline style's `gap` in cells, when `row-gap` and
    /// `column-gap` are both set to the same whole cells.
    fn gap(&self) -> Option<u16> {
        let style = self.inline_style()?;
        let cells = |g: &Option<crate::style::Value<crate::layout::GapValue>>| {
            g.as_ref()
                .and_then(|v| v.as_specified())
                .and_then(|g| g.as_cells())
        };
        let row = cells(&style.row_gap)?;
        (cells(&style.column_gap)? == row).then_some(row)
    }
    fn overflow(&self) -> Option<Overflow> {
        self.inline_style()
            .and_then(|s| s.overflow_x.as_ref())
            .and_then(|v| v.as_specified().copied())
    }
    fn inline_style(&self) -> Option<&'a TuiStyle> {
        self.tui_ext().map(TuiExt::inline_style_or_empty)
    }
    fn layout_rect(&self) -> Option<LayoutRect> {
        self.tui_ext().map(|e| e.layout)
    }
    fn content_layout_rect(&self) -> Option<LayoutRect> {
        self.tui_ext().map(|e| e.content_layout)
    }

    /// The post-cascade computed style for this element. `None` until
    /// the cascade has run at least once. Prefer `computed_or_initial`
    /// for code paths that need a concrete value unconditionally.
    fn computed(&self) -> Option<&'a ComputedStyle> {
        self.tui_ext().and_then(|e| e.computed.as_deref())
    }

    /// The computed style as a shared handle — an `Rc` clone instead of
    /// a deep copy, for the layout and paint paths that need an owned
    /// value while they mutate the arena. `None` until the cascade ran.
    fn computed_rc(&self) -> Option<std::rc::Rc<ComputedStyle>> {
        self.tui_ext().and_then(|e| e.computed.clone())
    }

    /// Like `computed`, but returns `ComputedStyle::initial()` when
    /// unset. Use in render code that must not panic pre-cascade.
    fn computed_or_initial(&self) -> ComputedStyle {
        self.computed()
            .cloned()
            .unwrap_or_else(ComputedStyle::initial)
    }

    fn computed_before(&self) -> Option<&'a ComputedStyle> {
        self.tui_ext().and_then(|e| e.computed_before.as_deref())
    }

    fn computed_after(&self) -> Option<&'a ComputedStyle> {
        self.tui_ext().and_then(|e| e.computed_after.as_deref())
    }

    /// `true` when the cascade needs to re-run on this element's subtree.
    fn is_style_dirty(&self) -> bool {
        self.tui_ext().is_some_and(|e| e.style_dirty)
    }

    /// `true` when layout needs to re-run on this element.
    fn is_layout_dirty(&self) -> bool {
        self.tui_ext().is_some_and(|e| e.layout_dirty)
    }

    /// `true` when this element opts into text editing.
    ///
    /// Three sources of editability:
    /// - an editing host: `contenteditable` in the true (`true` / `""`)
    ///   or plaintext-only state, keywords ASCII case-insensitive (HTML
    ///   §6.8.1, `Dom::content_editable_state`). rdom's editing is
    ///   plain text in both states (Enter inserts `\n`, paste inserts
    ///   text), so `plaintext-only` behaves exactly like `true`.
    /// - `<textarea>` tag (Phase C.4a).
    /// - `<input>` of a text-family `type` (Phase C.4a) — `text`,
    ///   `password`, `email`, `url`, `tel`, `search`, plus the
    ///   default-type input where `type` is missing.
    ///
    /// Being actually disabled — own `disabled` or inside a
    /// `<fieldset disabled>` (`Dom::is_actually_disabled`) — overrides
    /// everything (never editable). `readonly`
    /// keeps the element editable for focus / selection routing but
    /// `perform_edit` blocks mutation — see
    /// `runtime::editing::perform`.
    ///
    /// Does NOT walk up the tree — a child element inherits
    /// editability only via `nearest_editable_ancestor`.
    fn is_editable(&self) -> bool;
}

impl<'a> TuiNodeExt<'a> for NodeRef<'a, TuiExt> {
    fn tui_ext(&self) -> Option<&'a TuiExt> {
        self.ext()
    }

    fn is_editable(&self) -> bool {
        if self.dom().is_actually_disabled(self.id()) {
            return false;
        }
        if self
            .dom()
            .content_editable_state(self.id())
            .is_some_and(rdom_core::ContentEditableState::is_editing_host)
        {
            return true;
        }
        match self.tag_name() {
            Some("textarea") => true,
            Some("input") => is_text_input(self.dom(), self.id()),
            _ => false,
        }
    }
}
