//! `TuiDocAccessors` — document-level read accessors that
//! complement the per-element [`TuiAccessors`](super::TuiAccessors).
//!
//! Wraps the existing [`HitTestExt`](crate::HitTestExt) hit-test
//! pipeline with browser-IDL-shaped names and `NodeRef` returns
//! (rather than the runtime-flavored `NodeId`):
//!
//! - [`Document.elementFromPoint`](https://developer.mozilla.org/docs/Web/API/Document/elementFromPoint)
//!   → [`Self::element_from_point`].
//! - [`Document.elementsFromPoint`](https://developer.mozilla.org/docs/Web/API/Document/elementsFromPoint)
//!   → [`Self::elements_from_point`].
//! - [`Document.caretPositionFromPoint`](https://developer.mozilla.org/docs/Web/API/Document/caretPositionFromPoint)
//!   → [`Self::caret_position_from_point`].
//!
//! ## Coordinate types
//!
//! Spec sigs take `i32` to match the browser IDL (DOM rects are
//! integer cell coordinates for our viewport). Out-of-range
//! values (negative, > `u16::MAX`) miss every element by
//! definition → return `None` / empty `Vec` cleanly.
//!
//! ## Why this lives in `rdom-tui` and not `rdom-core`
//!
//! Hit-testing depends on the painted layout pipeline
//! (`TuiExt::layout`), which is TUI-side. The substrate boundary
//! rule keeps these traits on the side that owns the data.
//!
//! ## `style_sheets`
//!
//! Stylesheets live on [`App`](crate::App), not [`TuiDom`], because
//! they're an App-lifecycle concept. See `App::style_sheets()`.

use rdom_core::NodeRef;

use crate::runtime::hit_test::HitTestExt;
use crate::{TuiDom, TuiExt};

/// Document-level CSSOM-flavored read accessors. Implemented for
/// [`TuiDom`].
pub trait TuiDocAccessors {
    /// [`Document.elementFromPoint(x, y)`] — the deepest element
    /// whose painted area contains the cell `(x, y)`. Returns
    /// `None` if no element covers the point (empty viewport,
    /// negative coordinates, off-screen).
    ///
    /// [`Document.elementFromPoint(x, y)`]:
    ///   https://developer.mozilla.org/docs/Web/API/Document/elementFromPoint
    fn element_from_point(&self, x: i32, y: i32) -> Option<NodeRef<'_, TuiExt>>;

    /// [`Document.elementsFromPoint(x, y)`] — the full ancestor
    /// chain at `(x, y)`, root-most first, deepest last. Single
    /// hit-test walk (not per-layer re-run) — backed by
    /// [`HitTestExt::hit_test_path`].
    ///
    /// Empty when nothing was hit.
    ///
    /// [`Document.elementsFromPoint(x, y)`]:
    ///   https://developer.mozilla.org/docs/Web/API/Document/elementsFromPoint
    fn elements_from_point(&self, x: i32, y: i32) -> Vec<NodeRef<'_, TuiExt>>;

    /// [`Document.caretPositionFromPoint(x, y)`] — text-position
    /// at the screen cell, or `None` if the hit misses every
    /// IFC block / lands in `user-select: none` / falls outside
    /// any text fragment. See [`HitTestExt::position_at`] for
    /// the full miss matrix.
    ///
    /// [`Document.caretPositionFromPoint(x, y)`]:
    ///   https://developer.mozilla.org/docs/Web/API/Document/caretPositionFromPoint
    fn caret_position_from_point(&self, x: i32, y: i32) -> Option<rdom_core::Position>;
}

impl TuiDocAccessors for TuiDom {
    fn element_from_point(&self, x: i32, y: i32) -> Option<NodeRef<'_, TuiExt>> {
        let (x, y) = clamp_cell_coords(x, y)?;
        let id = self.hit_test(x, y)?;
        Some(self.node(id))
    }

    fn elements_from_point(&self, x: i32, y: i32) -> Vec<NodeRef<'_, TuiExt>> {
        let Some((x, y)) = clamp_cell_coords(x, y) else {
            return Vec::new();
        };
        self.hit_test_path(x, y)
            .into_iter()
            .map(|id| self.node(id))
            .collect()
    }

    fn caret_position_from_point(&self, x: i32, y: i32) -> Option<rdom_core::Position> {
        let (x, y) = clamp_cell_coords(x, y)?;
        self.position_at(x, y)
    }
}

/// Convert browser-IDL `i32` cell coordinates to the `u16`
/// shape `HitTestExt` accepts. Returns `None` when either axis
/// is out of the visible range (negative or > `u16::MAX`),
/// matching browser behavior of returning no element for points
/// outside the viewport.
fn clamp_cell_coords(x: i32, y: i32) -> Option<(u16, u16)> {
    let x = u16::try_from(x).ok()?;
    let y = u16::try_from(y).ok()?;
    Some((x, y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::Size;
    use crate::style::Stylesheet;

    // Build a tiny tree: root → div (10x5 at offset 0,0).
    // Run cascade + layout at a fixed viewport so hit-testing has
    // a realized layout to query. Pattern lifted from
    // `crate::runtime::hit_test::tests`.
    fn dom_with_div_at_origin() -> (TuiDom, rdom_core::NodeId) {
        use crate::LayoutExt;
        use crate::render::Rect;
        use crate::style::{CascadeExt, TuiStyle};
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let div = dom.create_element("div");
        dom.append_child(root, div).unwrap();
        let sheet = Stylesheet::bare().rule_unchecked(
            "div",
            TuiStyle::new()
                .width(Size::Fixed(10))
                .height(Size::Fixed(5)),
        );
        dom.cascade(&sheet);
        dom.layout_dom(Rect::new(0, 0, 80, 24));
        (dom, div)
    }

    #[test]
    fn element_from_point_returns_hit_node_ref() {
        let (dom, div) = dom_with_div_at_origin();
        let el = dom.element_from_point(2, 2).expect("hit");
        assert_eq!(el.id(), div);
    }

    #[test]
    fn element_from_point_returns_none_for_off_screen() {
        let (dom, _div) = dom_with_div_at_origin();
        assert!(dom.element_from_point(200, 200).is_none());
    }

    #[test]
    fn element_from_point_returns_none_for_negative_coords() {
        // Browser: `elementFromPoint(-1, -1)` returns null. Our
        // coordinate clamp rejects negative values up front.
        let (dom, _div) = dom_with_div_at_origin();
        assert!(dom.element_from_point(-1, 5).is_none());
        assert!(dom.element_from_point(5, -1).is_none());
    }

    #[test]
    fn elements_from_point_root_first_deepest_last() {
        let (dom, div) = dom_with_div_at_origin();
        let path: Vec<rdom_core::NodeId> = dom
            .elements_from_point(2, 2)
            .iter()
            .map(|n| n.id())
            .collect();
        // The deepest hit is `div`; the path should include div as
        // its last entry.
        assert!(!path.is_empty());
        assert_eq!(*path.last().unwrap(), div);
    }

    #[test]
    fn elements_from_point_empty_for_off_screen() {
        let (dom, _div) = dom_with_div_at_origin();
        assert!(dom.elements_from_point(500, 500).is_empty());
    }

    #[test]
    fn elements_from_point_empty_for_negative_coords() {
        let (dom, _div) = dom_with_div_at_origin();
        assert!(dom.elements_from_point(-1, 0).is_empty());
        assert!(dom.elements_from_point(0, -1).is_empty());
    }

    #[test]
    fn caret_position_from_point_none_outside_text() {
        // The div has no text; clicking inside it should miss
        // every IFC block → `None`.
        let (dom, _div) = dom_with_div_at_origin();
        assert!(dom.caret_position_from_point(2, 2).is_none());
    }

    #[test]
    fn caret_position_from_point_none_for_negative_coords() {
        let (dom, _div) = dom_with_div_at_origin();
        assert!(dom.caret_position_from_point(-1, 0).is_none());
    }
}
