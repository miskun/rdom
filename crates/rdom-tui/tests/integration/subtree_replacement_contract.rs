//! Subtree-replacement contract — the substrate guarantees the
//! showcase (and any consumer doing live demo / page swaps) needs
//! when it atomically replaces a subtree's children.
//!
//! Each test asserts one piece of the contract:
//!   - cascade reset / no leakage to the new subtree
//!   - cascade no-op on unrelated siblings
//!   - `MutationObserver` records (removed + added) delivered
//!   - `DirtyTracker` accumulates the right roots
//!   - **focus disposition** when the focused element (or an
//!     ancestor of it) is detached — closes M1 D3
//!   - hover state cleared on detach
//!   - pointer capture cleared on detach
//!   - selection collapsed when an endpoint is in the detached
//!     subtree
//!   - no-spurious-cleanup on detach of an unrelated node
//!
//! When any of these RED on the first run, the substrate gets a
//! root-cause fix in the same milestone — paper-overs are not the
//! point. M1 D2 is the contract; the fixes that make it green are
//! the work.

use std::cell::RefCell;
use std::rc::Rc;

use rdom_core::{Mutation, MutationObserver, ObserverId, Selection};
use rdom_style::Color;
use rdom_tui::{CascadeExt, DirtyTracker, Stylesheet, TuiDom, TuiStyle};

// ─── Helpers ────────────────────────────────────────────────────────

/// Collect mutation records emitted to the registered observer.
struct Collector {
    records: Rc<RefCell<Vec<Mutation>>>,
}

impl MutationObserver<rdom_tui::TuiExt> for Collector {
    fn observe(&mut self, _dom: &mut TuiDom, record: &Mutation) {
        self.records.borrow_mut().push(record.clone());
    }
}

fn install_collector(dom: &mut TuiDom) -> (ObserverId, Rc<RefCell<Vec<Mutation>>>) {
    let records = Rc::new(RefCell::new(Vec::new()));
    let obs = Box::new(Collector {
        records: records.clone(),
    });
    let id = dom.add_mutation_observer(obs);
    (id, records)
}

fn computed_fg(dom: &TuiDom, id: rdom_core::NodeId) -> Color {
    rdom_tui::style::cascade::computed_of(dom, id).fg
}

// ─── Cascade reset ──────────────────────────────────────────────────

#[test]
fn cascade_picks_up_new_subtree_after_children_replaced() {
    // Before: main contains <old>. Sheet: "old { color: red }".
    // After replace_children: main contains <new>. Sheet:
    // "new { color: blue }". Cascade must compute blue for new —
    // the new subtree gets a fresh cascade just like the original
    // tree did.
    let mut dom: TuiDom = TuiDom::new();
    let main = dom.create_element("main");
    let old = dom.create_element("old");
    dom.append_child(dom.root(), main).unwrap();
    dom.append_child(main, old).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("old", TuiStyle::new().fg(Color::Rgb(255, 0, 0)))
        .rule_unchecked("new", TuiStyle::new().fg(Color::Rgb(0, 0, 255)));
    dom.cascade(&sheet);
    assert_eq!(computed_fg(&dom, old), Color::Rgb(255, 0, 0), "old → red");

    // Atomic replacement.
    let new = dom.create_element("new");
    dom.node_mut(main).replace_children([new.into()]).unwrap();

    dom.cascade(&sheet);
    assert_eq!(
        computed_fg(&dom, new),
        Color::Rgb(0, 0, 255),
        "new → blue after subtree swap"
    );
}

#[test]
fn cascade_leaves_unrelated_siblings_alone_after_children_replace() {
    // Two siblings under root: <main> and <aside>. Replace main's
    // children; aside's computed style must stay exactly what it
    // was. Catches accidental tree-wide invalidation.
    let mut dom: TuiDom = TuiDom::new();
    let main = dom.create_element("main");
    let aside = dom.create_element("aside");
    dom.append_child(dom.root(), main).unwrap();
    dom.append_child(dom.root(), aside).unwrap();
    let old = dom.create_element("old");
    dom.append_child(main, old).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("aside", TuiStyle::new().fg(Color::Rgb(0, 255, 0)))
        .rule_unchecked("new", TuiStyle::new().fg(Color::Rgb(0, 0, 255)));
    dom.cascade(&sheet);
    let aside_fg_before = computed_fg(&dom, aside);

    let new = dom.create_element("new");
    dom.node_mut(main).replace_children([new.into()]).unwrap();
    dom.cascade(&sheet);

    assert_eq!(
        computed_fg(&dom, aside),
        aside_fg_before,
        "<aside>'s computed style is stable across an unrelated subtree swap"
    );
}

// ─── MutationObserver ───────────────────────────────────────────────

#[test]
fn mutation_observer_sees_remove_and_add_for_children_replace() {
    // replace_children must emit ChildListChanged records that
    // identify every removed child and every added child — even
    // though they're emitted as separate records (one per
    // detach + one for the appends).
    let mut dom: TuiDom = TuiDom::new();
    let main = dom.create_element("main");
    let old_a = dom.create_element("old_a");
    let old_b = dom.create_element("old_b");
    dom.append_child(dom.root(), main).unwrap();
    dom.append_child(main, old_a).unwrap();
    dom.append_child(main, old_b).unwrap();

    let (_id, records) = install_collector(&mut dom);

    let new_a = dom.create_element("new_a");
    let new_b = dom.create_element("new_b");
    dom.node_mut(main)
        .replace_children([new_a.into(), new_b.into()])
        .unwrap();

    // The full set of (removed, added) sets across all
    // ChildListChanged records under `main` must contain both old
    // children and both new children. Exact record granularity is
    // an implementation detail — we assert the union.
    let mut removed_seen = Vec::new();
    let mut added_seen = Vec::new();
    for r in records.borrow().iter() {
        if let Mutation::ChildListChanged {
            parent,
            added,
            removed,
        } = r
            && *parent == main
        {
            added_seen.extend(added.iter().copied());
            removed_seen.extend(removed.iter().copied());
        }
    }
    assert!(
        removed_seen.contains(&old_a) && removed_seen.contains(&old_b),
        "both old children appear in removed records: {removed_seen:?}"
    );
    assert!(
        added_seen.contains(&new_a) && added_seen.contains(&new_b),
        "both new children appear in added records: {added_seen:?}"
    );
}

// ─── DirtyTracker ───────────────────────────────────────────────────

#[test]
fn dirty_tracker_marks_parent_when_children_replaced() {
    // DirtyTracker is the runtime's signal that a subtree needs
    // re-cascade. After replace_children, the tracker must include
    // the parent (or a root that covers the change) so the next
    // paint actually re-cascades.
    let mut dom: TuiDom = TuiDom::new();
    let main = dom.create_element("main");
    let old = dom.create_element("old");
    dom.append_child(dom.root(), main).unwrap();
    dom.append_child(main, old).unwrap();

    let tracker = DirtyTracker::install(&mut dom);
    assert!(
        tracker.roots_snapshot().is_empty(),
        "freshly-installed tracker is empty"
    );

    let new = dom.create_element("new");
    dom.node_mut(main).replace_children([new.into()]).unwrap();

    let roots = tracker.roots_snapshot();
    assert!(
        roots.contains(&main) || roots.contains(&new),
        "tracker dirty roots must cover the replaced subtree (got {roots:?})"
    );
}

// ─── Focus disposition (closes M1 D3) ──────────────────────────────

#[test]
fn focused_clears_when_focused_element_is_detached() {
    // Browser contract: detaching the focused element resets focus
    // (the browser moves focus to body and fires blur; rdom's
    // current shape doesn't fire events here — see DIVERGENCES —
    // but `dom.focused()` MUST stop pointing at a detached node).
    let mut dom: TuiDom = TuiDom::new();
    let main = dom.create_element("main");
    let btn = dom.create_element("button");
    dom.append_child(dom.root(), main).unwrap();
    dom.append_child(main, btn).unwrap();
    dom.set_focused(Some(btn));
    assert_eq!(dom.focused(), Some(btn));

    dom.remove_child(main, btn).unwrap();

    assert_eq!(
        dom.focused(),
        None,
        "focus must clear when the focused element itself is detached"
    );
}

#[test]
fn focused_clears_when_focused_descendant_is_detached_via_replace_children() {
    // Same contract via the showcase nav path: parent's children
    // are replaced, focus was on a descendant of one of the
    // removed children. Focus must clear, even though we never
    // called remove_child on the focused node directly.
    let mut dom: TuiDom = TuiDom::new();
    let main = dom.create_element("main");
    let panel = dom.create_element("panel");
    let btn = dom.create_element("button");
    dom.append_child(dom.root(), main).unwrap();
    dom.append_child(main, panel).unwrap();
    dom.append_child(panel, btn).unwrap();
    dom.set_focused(Some(btn));

    let new_panel = dom.create_element("panel");
    dom.node_mut(main)
        .replace_children([new_panel.into()])
        .unwrap();

    assert_eq!(
        dom.focused(),
        None,
        "focus must clear when a deep descendant of a removed subtree was focused"
    );
}

#[test]
fn focused_unaffected_when_unrelated_node_is_detached() {
    // Cleanup must not be over-eager: detaching a node that is
    // *not* the focused element and *not* an ancestor of it must
    // leave focus alone.
    let mut dom: TuiDom = TuiDom::new();
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(dom.root(), a).unwrap();
    dom.append_child(dom.root(), b).unwrap();
    dom.set_focused(Some(a));

    dom.remove_child(dom.root(), b).unwrap();

    assert_eq!(
        dom.focused(),
        Some(a),
        "detaching an unrelated sibling must not disturb focus"
    );
}

// ─── Hover ──────────────────────────────────────────────────────────

#[test]
fn hovered_clears_when_hovered_element_is_detached() {
    let mut dom: TuiDom = TuiDom::new();
    let div = dom.create_element("div");
    dom.append_child(dom.root(), div).unwrap();
    dom.set_hovered(Some(div));

    dom.remove_child(dom.root(), div).unwrap();

    assert_eq!(
        dom.hovered(),
        None,
        "hover state must clear when the hovered node is detached"
    );
}

#[test]
fn hovered_clears_when_hovered_descendant_is_detached_via_replace_children() {
    let mut dom: TuiDom = TuiDom::new();
    let main = dom.create_element("main");
    let inner = dom.create_element("inner");
    dom.append_child(dom.root(), main).unwrap();
    dom.append_child(main, inner).unwrap();
    dom.set_hovered(Some(inner));

    let replacement = dom.create_element("replacement");
    dom.node_mut(main)
        .replace_children([replacement.into()])
        .unwrap();

    assert_eq!(dom.hovered(), None, "hover clears on descendant detach");
}

// ─── Pointer capture ────────────────────────────────────────────────

#[test]
fn pointer_capture_clears_when_captor_is_detached() {
    let mut dom: TuiDom = TuiDom::new();
    let knob = dom.create_element("knob");
    dom.append_child(dom.root(), knob).unwrap();
    dom.set_pointer_capture(knob).unwrap();

    dom.remove_child(dom.root(), knob).unwrap();

    assert_eq!(
        dom.pointer_capture(),
        None,
        "pointer capture must clear when the captor is detached — otherwise drag \
         events would keep routing to a node that no longer exists"
    );
}

// ─── Selection collapse ─────────────────────────────────────────────

#[test]
fn selection_collapses_when_anchor_in_detached_subtree() {
    let mut dom: TuiDom = TuiDom::new();
    let main = dom.create_element("main");
    let target = dom.create_element("target");
    let other = dom.create_element("other");
    dom.append_child(dom.root(), main).unwrap();
    dom.append_child(main, target).unwrap();
    dom.append_child(dom.root(), other).unwrap();
    dom.set_selection(Some(Selection::caret(rdom_core::Position::new(target, 0))));

    // Remove main → target goes with it.
    dom.remove_child(dom.root(), main).unwrap();

    assert!(
        dom.selection().is_none(),
        "selection must collapse to None when its anchor was in the detached subtree"
    );
}

#[test]
fn selection_collapses_when_focus_in_detached_subtree() {
    let mut dom: TuiDom = TuiDom::new();
    let main = dom.create_element("main");
    let anchor_el = dom.create_element("anchor");
    let focus_el = dom.create_element("focus");
    dom.append_child(dom.root(), anchor_el).unwrap();
    dom.append_child(dom.root(), main).unwrap();
    dom.append_child(main, focus_el).unwrap();
    dom.set_selection(Some(Selection {
        anchor: rdom_core::Position::new(anchor_el, 0),
        focus: rdom_core::Position::new(focus_el, 0),
    }));

    dom.remove_child(dom.root(), main).unwrap();

    assert!(
        dom.selection().is_none(),
        "selection must collapse to None when its focus was in the detached subtree"
    );
}

// ─── Cleanup propagates through every detach path ──────────────────

#[test]
fn drop_subtree_purges_interaction_state() {
    // `drop_subtree` frees the subtree from the arena entirely. The
    // cleanup must run *before* the free, otherwise a stale focused
    // NodeId would point at a recycled slot — the worst kind of bug,
    // because the same id could later be handed out for a fresh node.
    let mut dom: TuiDom = TuiDom::new();
    let main = dom.create_element("main");
    let btn = dom.create_element("button");
    dom.append_child(dom.root(), main).unwrap();
    dom.append_child(main, btn).unwrap();
    dom.set_focused(Some(btn));
    dom.set_hovered(Some(btn));

    dom.drop_subtree(main).unwrap();

    assert_eq!(
        dom.focused(),
        None,
        "drop_subtree must purge focused state before freeing the subtree"
    );
    assert_eq!(
        dom.hovered(),
        None,
        "drop_subtree must purge hovered state before freeing the subtree"
    );
}

#[test]
fn replace_with_purges_interaction_state_on_replaced_node() {
    // `ChildNode.replaceWith(...)` inserts the new siblings then
    // detaches self. The replaced node carries its subtree with it
    // — interaction state pointing inside that subtree must clear.
    let mut dom: TuiDom = TuiDom::new();
    let old = dom.create_element("old");
    let inner = dom.create_element("inner");
    dom.append_child(dom.root(), old).unwrap();
    dom.append_child(old, inner).unwrap();
    dom.set_focused(Some(inner));

    let new = dom.create_element("new");
    dom.node_mut(old).replace_with([new.into()]).unwrap();

    assert_eq!(
        dom.focused(),
        None,
        "replace_with must purge focus that was inside the replaced node's subtree"
    );
}

#[test]
fn selection_unaffected_when_unrelated_node_is_detached() {
    let mut dom: TuiDom = TuiDom::new();
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    let c = dom.create_element("c");
    dom.append_child(dom.root(), a).unwrap();
    dom.append_child(dom.root(), b).unwrap();
    dom.append_child(dom.root(), c).unwrap();
    dom.set_selection(Some(Selection::caret(rdom_core::Position::new(a, 0))));

    dom.remove_child(dom.root(), c).unwrap();

    assert!(
        dom.selection().is_some(),
        "selection survives detach of an unrelated node"
    );
}
