//! Extension traits on `NodeRef` / `NodeMut` for presentation-data access.
//!
//! rdom-core's `NodeMut` exposes DOM methods (`set_id`, `add_class`,
//! `append_child`, …). The methods here live in `rdom-tui` and deal with
//! the `TuiExt` fields — sizing, padding, style, scroll, etc. Keeps the
//! builder-chain pattern ergonomic without cluttering `rdom-core` with
//! TUI-specific concepts.
//!
//! ## Why a local trait?
//!
//! rdom-core's `NodeMut<'a, TuiExt>` has no presentation methods. Adding
//! them there would mean polluting the core with TUI-specific API. So
//! we define `TuiNodeExt` / `TuiNodeMutExt` as local extension traits
//! that `TuiNodeRef` / `TuiNodeMut` opt into.
//!
//! A caller does `use rdom_tui::*;` and gets the methods for free:
//!
//! ```
//! # use rdom_tui::*;
//! let mut dom: TuiDom = TuiDom::new();
//! let div = dom.create_element("div");
//! dom.node_mut(div)
//!     .set_width(Size::Fixed(40))
//!     .set_padding(Padding::all(1));
//! ```

use rdom_core::{NodeMut, NodeRef, NodeType};

use crate::ext::TuiExt;
use crate::layout::{Border, Direction, LayoutRect, Overflow, Padding, Size};
use crate::style::{ComputedStyle, TuiStyle, Value};

// ───────────────────────── Read helpers ─────────────────────────────

/// Readonly sugar over `NodeRef<'_, TuiExt>::ext()`. The underlying
/// `ext()` on rdom-core returns `Option<&Ext>` already; this trait adds
/// field-level getters so callers don't have to destructure.
pub trait TuiNodeExt<'a> {
    fn tui_ext(&self) -> Option<&'a TuiExt>;

    // These read the *specified* inline-style value (`None` when the
    // property wasn't set via a node setter or inline style). Layout
    // reads the post-cascade `ComputedStyle`; these are the author-input
    // side, kept symmetric with the `set_*` setters in `TuiNodeMutExt`.
    fn width(&self) -> Option<Size> {
        self.inline_style()
            .and_then(|s| s.width.as_ref())
            .and_then(|v| v.as_specified().cloned())
    }
    fn height(&self) -> Option<Size> {
        self.inline_style()
            .and_then(|s| s.height.as_ref())
            .and_then(|v| v.as_specified().cloned())
    }
    fn direction(&self) -> Option<Direction> {
        self.inline_style()
            .and_then(|s| s.direction.as_ref())
            .and_then(|v| v.as_specified().copied())
    }
    fn padding(&self) -> Option<Padding> {
        self.inline_style()
            .and_then(|s| s.padding.as_ref())
            .and_then(|v| v.as_specified().cloned())
    }
    fn border(&self) -> Option<Border> {
        self.inline_style()
            .and_then(|s| s.border.as_ref())
            .and_then(|v| v.as_specified().copied())
    }
    fn gap(&self) -> Option<u16> {
        self.inline_style()
            .and_then(|s| s.gap.as_ref())
            .and_then(|v| v.as_specified().copied())
    }
    fn overflow(&self) -> Option<Overflow> {
        self.inline_style()
            .and_then(|s| s.overflow_x.as_ref())
            .and_then(|v| v.as_specified().copied())
    }
    fn inline_style(&self) -> Option<&'a TuiStyle> {
        self.tui_ext().map(|e| &e.inline_style)
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
        self.tui_ext().and_then(|e| e.computed.as_ref())
    }

    /// Like `computed`, but returns `ComputedStyle::initial()` when
    /// unset. Use in render code that must not panic pre-cascade.
    fn computed_or_initial(&self) -> ComputedStyle {
        self.computed()
            .cloned()
            .unwrap_or_else(ComputedStyle::initial)
    }

    fn computed_before(&self) -> Option<&'a ComputedStyle> {
        self.tui_ext().and_then(|e| e.computed_before.as_ref())
    }

    fn computed_after(&self) -> Option<&'a ComputedStyle> {
        self.tui_ext().and_then(|e| e.computed_after.as_ref())
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
    /// - `contenteditable="true"` / `""` (HTML boolean shorthand) on
    ///   any element (Phase B).
    /// - `<textarea>` tag (Phase C.4a).
    /// - `<input>` of a text-family `type` (Phase C.4a) — `text`,
    ///   `password`, `email`, `url`, `tel`, `search`, plus the
    ///   default-type input where `type` is missing.
    ///
    /// `disabled` overrides everything (never editable). `readonly`
    /// keeps the element editable for focus / selection routing but
    /// `perform_edit` blocks mutation — see
    /// `runtime::editing::perform`.
    ///
    /// Does NOT walk up the tree — a child element inherits
    /// editability only via `nearest_editable_ancestor`.
    fn is_editable(&self) -> bool;
}

/// Text-family `<input>` `type` values per HTML living standard.
/// `<input>` without `type` defaults to `text`. Toggle types
/// (checkbox, radio) and form-action types (submit, reset,
/// button, hidden, image, file, color) are not text-editable.
///
/// `number` is included: it edits as text but with a numeric
/// `beforeinput` filter installed by `runtime::builtins::number`.
fn is_text_input_type(ty: Option<&str>) -> bool {
    matches!(
        ty,
        None | Some("text")
            | Some("password")
            | Some("email")
            | Some("url")
            | Some("tel")
            | Some("search")
            | Some("number")
    )
}

impl<'a> TuiNodeExt<'a> for NodeRef<'a, TuiExt> {
    fn tui_ext(&self) -> Option<&'a TuiExt> {
        self.ext()
    }

    fn is_editable(&self) -> bool {
        if self.has_attribute("disabled") {
            return false;
        }
        if matches!(
            self.get_attribute("contenteditable"),
            Some("true") | Some("")
        ) {
            return true;
        }
        match self.tag_name() {
            Some("textarea") => true,
            Some("input") => is_text_input_type(self.get_attribute("type")),
            _ => false,
        }
    }
}

/// Walk up from `node_id` (inclusive) to the nearest element with
/// `contenteditable="true"`. Returns the editable ancestor's id, or
/// `None` when neither `node_id` nor any ancestor is editable.
/// Used by runtime paths that need to route an edit or caret action
/// to the enclosing editable scope.
pub fn nearest_editable_ancestor(
    dom: &rdom_core::Dom<TuiExt>,
    node_id: rdom_core::NodeId,
) -> Option<rdom_core::NodeId> {
    let mut cur = Some(node_id);
    while let Some(id) = cur {
        if dom.node(id).is_editable() {
            return Some(id);
        }
        cur = dom.node(id).parent_node().map(|p| p.id());
    }
    None
}

/// True when `node` is a descendant of `ancestor`, or `node ==
/// ancestor`. Used by user-select / focus / selection clamp logic
/// to decide whether a candidate target is inside a host's subtree.
pub fn is_descendant_or_self(
    dom: &rdom_core::Dom<TuiExt>,
    node: rdom_core::NodeId,
    ancestor: rdom_core::NodeId,
) -> bool {
    let mut cur = Some(node);
    while let Some(n) = cur {
        if n == ancestor {
            return true;
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    }
    false
}

/// First text-node descendant of `root` (inclusive) in document
/// order. Returns `None` when no text node exists in the subtree.
/// Used by selection-extension logic that needs the host's
/// leading text position.
pub fn first_text_descendant(
    dom: &rdom_core::Dom<TuiExt>,
    root: rdom_core::NodeId,
) -> Option<rdom_core::NodeId> {
    if dom.node(root).node_type() == NodeType::Text {
        return Some(root);
    }
    for child in dom.node(root).child_nodes() {
        if let Some(found) = first_text_descendant(dom, child.id()) {
            return Some(found);
        }
    }
    None
}

/// Last text-node descendant of `root` (inclusive) in document
/// order. Returns `None` when no text node exists in the subtree.
/// Used by selection-extension logic that needs the host's
/// trailing text position.
pub fn last_text_descendant(
    dom: &rdom_core::Dom<TuiExt>,
    root: rdom_core::NodeId,
) -> Option<rdom_core::NodeId> {
    if dom.node(root).node_type() == NodeType::Text {
        return Some(root);
    }
    let kids: Vec<rdom_core::NodeId> = dom.node(root).child_nodes().map(|c| c.id()).collect();
    for child in kids.into_iter().rev() {
        if let Some(found) = last_text_descendant(dom, child) {
            return Some(found);
        }
    }
    None
}

/// Byte length of a text node's data. Returns `0` for non-text
/// nodes (use as a position-clamp helper, not a node-type check).
pub fn text_len(dom: &rdom_core::Dom<TuiExt>, id: rdom_core::NodeId) -> usize {
    dom.node(id).node_value().map(|s| s.len()).unwrap_or(0)
}

/// Replace all children of `id` with a single text node holding
/// `text`. The canonical "set this element's text content" mutation
/// — used by `<input>` / `<textarea>` value seeding + the
/// `set_value` / form-control setters in `TuiAccessorsMut`. Lives
/// here (substrate-neutral) so the value-installation logic lives
/// in one place rather than being re-implemented per call site.
///
/// Returns `Err` if any of the underlying `remove_child` /
/// `append_child` calls fail. Callers that prefer the
/// generally-forgiving builder-chain style (e.g. the public
/// `runtime::builtins::input::set_value`) ignore the result with
/// `let _ = …` at the call.
pub(crate) fn install_text_content(
    dom: &mut rdom_core::Dom<TuiExt>,
    id: rdom_core::NodeId,
    text: &str,
) -> crate::Result<()> {
    let existing: Vec<rdom_core::NodeId> = dom.node(id).child_nodes().map(|c| c.id()).collect();
    for child in existing {
        dom.remove_child(id, child)?;
    }
    let text_node = dom.create_text_node(text);
    dom.append_child(id, text_node)?;
    Ok(())
}

// ───────────────────────── Write helpers ────────────────────────────

/// Mutation helpers for `TuiExt`-bearing elements. All methods return
/// `&mut self` for chaining; they silently no-op on non-Element nodes
/// (matching how `set_attribute` behaves in rdom-core — errors there,
/// no-ops here to keep builder chains readable).
pub trait TuiNodeMutExt<'a> {
    fn tui_ext_mut(&mut self) -> Option<&mut TuiExt>;

    // Geometry setters write the element's **inline style** (the cascade
    // input) and mark it style-dirty, so the next cascade carries them
    // into `ComputedStyle` — the only thing layout reads. (Before
    // `EXT-LAYOUT-SETTERS-1` these wrote raw `ext` fields that layout
    // ignored, so the setters silently did nothing for layout.)
    fn set_width(&mut self, w: Size) -> &mut Self {
        if let Some(e) = self.tui_ext_mut() {
            e.inline_style.width = Some(Value::Specified(w));
            e.style_dirty = true;
        }
        self
    }
    fn set_height(&mut self, h: Size) -> &mut Self {
        if let Some(e) = self.tui_ext_mut() {
            e.inline_style.height = Some(Value::Specified(h));
            e.style_dirty = true;
        }
        self
    }
    fn set_min_width(&mut self, v: Option<rdom_style::layout::MinSize>) -> &mut Self {
        if let Some(e) = self.tui_ext_mut() {
            e.inline_style.min_width = v.map(Value::Specified);
            e.style_dirty = true;
        }
        self
    }
    fn set_max_width(&mut self, v: Option<u16>) -> &mut Self {
        if let Some(e) = self.tui_ext_mut() {
            e.inline_style.max_width = v.map(Value::Specified);
            e.style_dirty = true;
        }
        self
    }
    fn set_min_height(&mut self, v: Option<rdom_style::layout::MinSize>) -> &mut Self {
        if let Some(e) = self.tui_ext_mut() {
            e.inline_style.min_height = v.map(Value::Specified);
            e.style_dirty = true;
        }
        self
    }
    fn set_max_height(&mut self, v: Option<u16>) -> &mut Self {
        if let Some(e) = self.tui_ext_mut() {
            e.inline_style.max_height = v.map(Value::Specified);
            e.style_dirty = true;
        }
        self
    }
    fn set_direction(&mut self, d: Direction) -> &mut Self {
        if let Some(e) = self.tui_ext_mut() {
            e.inline_style.direction = Some(Value::Specified(d));
            e.style_dirty = true;
        }
        self
    }
    fn set_padding(&mut self, p: Padding) -> &mut Self {
        if let Some(e) = self.tui_ext_mut() {
            e.inline_style.padding = Some(Value::Specified(p));
            e.style_dirty = true;
        }
        self
    }
    fn set_border(&mut self, b: Border) -> &mut Self {
        if let Some(e) = self.tui_ext_mut() {
            e.inline_style.border = Some(Value::Specified(b));
            e.style_dirty = true;
        }
        self
    }
    fn set_gap(&mut self, g: u16) -> &mut Self {
        if let Some(e) = self.tui_ext_mut() {
            e.inline_style.gap = Some(Value::Specified(g));
            e.style_dirty = true;
        }
        self
    }
    fn set_overflow(&mut self, o: Overflow) -> &mut Self {
        if let Some(e) = self.tui_ext_mut() {
            e.inline_style.overflow_x = Some(Value::Specified(o));
            e.inline_style.overflow_y = Some(Value::Specified(o));
            e.style_dirty = true;
        }
        self
    }
    fn set_inline_style(&mut self, s: TuiStyle) -> &mut Self {
        if let Some(e) = self.tui_ext_mut() {
            e.inline_style = s;
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TuiDom;
    use crate::style::Color;

    #[test]
    fn builder_chain_sets_fields() {
        let mut dom: TuiDom = TuiDom::new();
        let div = dom.create_element("div");
        dom.node_mut(div)
            .set_width(Size::Fixed(40))
            .set_height(Size::Flex(1))
            .set_padding(Padding::symmetric(2, 1))
            .set_border(Border::single())
            .set_gap(1)
            .set_overflow(Overflow::Hidden)
            .set_direction(Direction::Row);

        let n = dom.node(div);
        assert_eq!(n.width(), Some(Size::Fixed(40)));
        assert_eq!(n.height(), Some(Size::Flex(1)));
        assert_eq!(n.padding(), Some(Padding::symmetric(2, 1)));
        assert_eq!(n.border(), Some(Border::single()));
        assert_eq!(n.gap(), Some(1));
        assert_eq!(n.overflow(), Some(Overflow::Hidden));
        assert_eq!(n.direction(), Some(Direction::Row));
    }

    #[test]
    fn min_max_constraints() {
        use rdom_style::layout::MinSize;
        let mut dom: TuiDom = TuiDom::new();
        let div = dom.create_element("div");
        dom.node_mut(div)
            .set_min_width(Some(MinSize::Cells(10)))
            .set_max_width(Some(100))
            .set_min_height(Some(MinSize::Cells(5)))
            .set_max_height(Some(50));
        let e = dom.node(div).tui_ext().unwrap();
        use crate::style::Value;
        assert_eq!(
            e.inline_style.min_width,
            Some(Value::Specified(MinSize::Cells(10)))
        );
        assert_eq!(e.inline_style.max_width, Some(Value::Specified(100)));
        assert_eq!(
            e.inline_style.min_height,
            Some(Value::Specified(MinSize::Cells(5)))
        );
        assert_eq!(e.inline_style.max_height, Some(Value::Specified(50)));
    }

    #[test]
    fn inline_style_settable() {
        let mut dom: TuiDom = TuiDom::new();
        let div = dom.create_element("div");
        dom.node_mut(div)
            .set_inline_style(TuiStyle::new().fg(Color::Rgb(255, 0, 0)).bold(true));

        let n = dom.node(div);
        let s = n.inline_style().unwrap();
        use crate::style::{TuiColor, Value};
        assert_eq!(
            s.fg,
            Some(Value::Specified(TuiColor::Literal(Color::Rgb(255, 0, 0))))
        );
        assert_eq!(s.bold, Some(Value::Specified(true)));
    }

    #[test]
    fn before_after_content_setters() {
        let mut dom: TuiDom = TuiDom::new();
        let div = dom.create_element("div");
        dom.node_mut(div)
            .set_before_content("▾ ")
            .set_after_content(" ←");
        let e = dom.node(div).tui_ext().unwrap();
        assert_eq!(e.before_content.as_deref(), Some("▾ "));
        assert_eq!(e.after_content.as_deref(), Some(" ←"));

        dom.node_mut(div).clear_before_content();
        assert!(dom.node(div).ext().unwrap().before_content.is_none());
    }

    #[test]
    fn setters_noop_on_non_element() {
        let mut dom: TuiDom = TuiDom::new();
        let t = dom.create_text_node("hi");
        // No panic, no-op:
        dom.node_mut(t)
            .set_width(Size::Fixed(5))
            .set_border(Border::single());
        assert!(dom.node(t).tui_ext().is_none());
    }

    #[test]
    fn scroll_setter() {
        let mut dom: TuiDom = TuiDom::new();
        let div = dom.create_element("div");
        dom.node_mut(div).set_scroll(12, 34);
        let e = dom.node(div).tui_ext().unwrap();
        assert_eq!(e.scroll_x, 12);
        assert_eq!(e.scroll_y, 34);
    }
}
