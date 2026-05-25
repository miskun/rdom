//! End-to-end CSSOM → cascade integration.
//!
//! The unit tests in `cssom/declaration.rs`, `cssom/observer.rs`,
//! and `style/cascade/tests.rs` each exercise one link of the
//! production chain in isolation:
//!
//! ```text
//!   StyleDeclarationMut::set_property
//!     └─ writes TuiExt::inline_style          ← typed path
//!     └─ writes style="..." attribute         ← under CSSOM_REENTRY guard
//!         └─ Mutation::AttributeChanged       ← fired by Dom
//!             ├─ InlineStyleObserver          ← self-suppresses on reentry
//!             └─ DirtyTracker                 ← marks node style_dirty,
//!                                               pushes to roots list
//! Dom::cascade_subtrees(&sheet, roots)
//!     └─ reads TuiExt::inline_style → writes TuiExt::computed
//! ```
//!
//! This file asserts the *whole* chain — the tripwire D-M4-5
//! called for. If any link breaks silently (reentry guard fails
//! to suppress, observer clobbers the typed write, DirtyTracker
//! misses the attribute mutation, cascade ignores inline_style),
//! one of these tests goes red.

use rdom_style::{Color, TuiColor, Value};
use rdom_tui::{CascadeExt, DirtyTracker, Stylesheet, TuiAccessorsMut, TuiDom, TuiNodeExt};

fn dom_with_div() -> (TuiDom, rdom_core::NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    (dom, div)
}

fn computed_fg(dom: &TuiDom, id: rdom_core::NodeId) -> Color {
    dom.node(id)
        .tui_ext()
        .and_then(|e| e.computed.as_ref())
        .map(|c| c.fg)
        .unwrap_or(Color::Reset)
}

#[test]
fn set_property_then_cascade_writes_computed_fg() {
    // Minimal typed path: CSSOM setter → full cascade → computed.
    // No DirtyTracker / subtree cascade — just the "did the
    // typed inline_style write survive the round trip into
    // computed" question.
    let (mut dom, div) = dom_with_div();

    dom.node_mut(div)
        .style_mut()
        .expect("element has style")
        .set_property("color", "red")
        .unwrap();

    let sheet = Stylesheet::new();
    dom.cascade(&sheet);

    assert_eq!(
        computed_fg(&dom, div),
        Color::Rgb(255, 0, 0),
        "set_property('color','red') must land in computed.fg after cascade",
    );
}

#[test]
fn set_property_dirties_tracker_for_subtree_cascade() {
    // The production path. `App::build` wires a DirtyTracker;
    // each tick takes the dirty roots and calls cascade_subtrees.
    // CSSOM writes have to flow through that path or runtime apps
    // would silently miss style changes.
    let (mut dom, div) = dom_with_div();
    let tracker = DirtyTracker::install(&mut dom);

    // Sanity: no work before the write.
    assert!(tracker.roots_snapshot().is_empty());

    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .set_property("color", "red")
        .unwrap();

    let roots = tracker.take_roots();
    assert!(
        roots.contains(&div),
        "CSSOM setProperty must dirty the target node via the style=\"\" attribute write \
         (got roots = {roots:?})",
    );

    let sheet = Stylesheet::new();
    dom.cascade_subtrees(&sheet, &roots);

    assert_eq!(
        computed_fg(&dom, div),
        Color::Rgb(255, 0, 0),
        "cascade_subtrees with tracker-supplied roots must update computed.fg",
    );
}

#[test]
fn remove_property_then_cascade_clears_computed_fg() {
    // The reverse path. set_property then remove_property must
    // restore the cascade default — same chain, reentry guard +
    // tracker still in play.
    let (mut dom, div) = dom_with_div();
    let tracker = DirtyTracker::install(&mut dom);

    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .set_property("color", "red")
        .unwrap();
    let after_set = tracker.take_roots();
    dom.cascade_subtrees(&Stylesheet::new(), &after_set);
    assert_eq!(computed_fg(&dom, div), Color::Rgb(255, 0, 0));

    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .remove_property("color")
        .unwrap();
    let after_remove = tracker.take_roots();
    assert!(
        after_remove.contains(&div),
        "removeProperty must dirty the node so the next cascade tick clears computed.fg",
    );
    dom.cascade_subtrees(&Stylesheet::new(), &after_remove);

    assert_eq!(
        computed_fg(&dom, div),
        Color::Reset,
        "after remove_property + cascade, fg returns to the cascade initial (Color::Reset)",
    );
}

#[test]
fn set_css_text_then_cascade_writes_computed_fg() {
    // The bulk-write path. cssText = "color: red" parses through
    // rdom_css::parse_inline, writes the typed style, fires the
    // attribute mutation under the reentry guard.
    let (mut dom, div) = dom_with_div();
    let tracker = DirtyTracker::install(&mut dom);

    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .set_css_text("color: red")
        .unwrap();

    let roots = tracker.take_roots();
    assert!(roots.contains(&div));

    dom.cascade_subtrees(&Stylesheet::new(), &roots);

    // Confirm the typed inline_style was written, not lost to a
    // re-parse race with the observer.
    let inline_fg = dom.node(div).inline_style().unwrap().fg.clone();
    assert_eq!(
        inline_fg,
        Some(Value::Specified(TuiColor::Literal(Color::Rgb(255, 0, 0)))),
        "set_css_text must populate inline_style.fg",
    );
    assert_eq!(
        computed_fg(&dom, div),
        Color::Rgb(255, 0, 0),
        "set_css_text + cascade must land in computed.fg",
    );
}
