//! `DirtyTracker` tests.

use super::*;
use crate::{Color, TuiDom, TuiNodeExt, TuiNodeMutExt, TuiStyle};

/// `P7G-PAINT-ONLY-FRAME-1`: a text edit dirties the placeholder host
/// above it and its parent only when the text went from empty to
/// non-empty (or back) — the selector states text feeds
/// (`:placeholder-shown`, and `:empty`, `P7G-CORE-SMALL-1`) — and
/// always flags paint-dirty.
#[test]
fn text_emptiness_flips_dirty_the_placeholder_host() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let ta = dom.create_element("textarea");
    dom.set_attribute(ta, "placeholder", "p").unwrap();
    let t = dom.create_text_node("");
    dom.append_child(ta, t).unwrap();
    dom.append_child(root, ta).unwrap();
    let plain = dom.create_element("p");
    let pt = dom.create_text_node("");
    dom.append_child(plain, pt).unwrap();
    dom.append_child(root, plain).unwrap();
    let tracker = DirtyTracker::install(&mut dom);
    // No `+` / `~` in play: only the host itself restyles.
    tracker.set_sibling_combinators(false);

    dom.node_mut(t).set_node_value("a").unwrap();
    assert_eq!(tracker.take_roots(), vec![ta], "empty → text");
    dom.node_mut(ta).ext_mut().unwrap().style_dirty = false;
    assert!(tracker.take_paint_dirty());

    dom.node_mut(t).set_node_value("ab").unwrap();
    assert!(tracker.take_roots().is_empty(), "text → text: no restyle");
    assert!(tracker.take_paint_dirty(), "but a relayout");

    dom.node_mut(t).set_node_value("").unwrap();
    assert_eq!(tracker.take_roots(), vec![ta], "text → empty");

    // No placeholder above, but the parent's `:empty` flips: a
    // zero-length text node does not count (Selectors 4 §14.2).
    dom.node_mut(pt).set_node_value("x").unwrap();
    assert_eq!(tracker.take_roots(), vec![plain], "the parent's :empty");
    dom.node_mut(plain).ext_mut().unwrap().style_dirty = false;

    dom.node_mut(pt).set_node_value("xy").unwrap();
    assert!(tracker.take_roots().is_empty(), "text → text: no restyle");
}

/// `P7G-ROUTE-REDRAW-1`: `a[x] + b` reads `a`'s attribute, so while
/// the sheets may use a sibling combinator an attribute change on
/// `a` dirties `b` too; told they do not, only `a`'s subtree.
#[test]
fn a_state_change_dirties_siblings_only_while_sibling_combinators_may_apply() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("div");
    let b = dom.create_element("p");
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();
    let tracker = DirtyTracker::install(&mut dom);

    dom.set_attribute(a, "x", "1").unwrap();
    let roots = tracker.take_roots();
    assert!(roots.contains(&a) && roots.contains(&b), "{roots:?}");
    for id in [a, b] {
        dom.node_mut(id).ext_mut().unwrap().style_dirty = false;
    }

    tracker.set_sibling_combinators(false);
    dom.set_attribute(a, "x", "2").unwrap();
    assert_eq!(tracker.take_roots(), vec![a]);
}

#[test]
fn sibling_combinators_are_found_anywhere_in_a_selector() {
    use crate::style::Stylesheet;
    let uses = |sel: &str| {
        uses_sibling_combinators(&Stylesheet::bare().rule_unchecked(sel, TuiStyle::new()))
    };
    assert!(uses("a + b"));
    assert!(uses("a ~ b c"));
    assert!(uses("p:not(a + b)"));
    assert!(uses(":where(a ~ b) > c"));
    assert!(!uses("a > b c"));
    assert!(!uses("p:not(.x)"));
    assert!(
        !uses_sibling_combinators(&Stylesheet::new()),
        "the UA sheet has none"
    );
}

/// `C1G-INVALIDATION`: an `@scope`'s `<scope-start>` / `<scope-end>`
/// (CSS Cascade 6 §2.5) and an `:is()` argument (the nesting `&`,
/// CSS Nesting 1 §2) are matched too.
#[test]
fn sibling_combinators_are_found_in_scope_preludes_and_is() {
    use rdom_style::{RuleContext, StyleSelector, Stylesheet};
    let uses = |css: &str| uses_sibling_combinators(&rdom_css::parse(css).stylesheet);
    assert!(uses("@scope (.a + .b) { p { color: red } }"));
    assert!(uses("@scope (main) to (.a ~ .b) { p { color: red } }"));
    assert!(!uses("@scope (main) to (.b) { p { color: red } }"));
    let parent = StyleSelector::parse(".a + .b, .q + .b").unwrap();
    let mut sheet = Stylesheet::bare();
    sheet.add_style_rule(
        &StyleSelector::parse_nested("p", &parent).unwrap(),
        TuiStyle::new(),
        RuleContext::default(),
    );
    assert!(uses_sibling_combinators(&sheet), "inside `:is()`");
}

/// Appending children one at a time marks each parent's siblings
/// once per drain (they stay dirty until the cascade), and a drain
/// re-arms the sibling marking so `:first-child` / `+` / `~`
/// re-evaluate after the next structural change.
#[test]
fn sibling_marking_is_once_per_parent_per_drain_and_rearms_after_drain() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let list = dom.create_element("ul");
    dom.append_child(root, list).unwrap();
    // A non-empty list: appending to an empty one would also dirty
    // the list itself (its `:empty` flips), covering every child.
    let first = dom.create_element("li");
    dom.append_child(list, first).unwrap();
    let tracker = DirtyTracker::install(&mut dom);

    let mut items = Vec::new();
    for _ in 0..2000 {
        let li = dom.create_element("li");
        dom.append_child(list, li).unwrap();
        items.push(li);
    }
    for &li in &items {
        assert!(dom.node(li).ext().unwrap().style_dirty);
    }
    let roots = tracker.take_roots();
    assert_eq!(
        roots.len(),
        2001,
        "each appended child is its own root, plus the sibling marked once"
    );
    assert_eq!(
        roots.iter().collect::<std::collections::HashSet<_>>().len(),
        2001,
        "no duplicate roots"
    );

    // Simulate the cascade clearing the flags.
    for &li in &items {
        dom.node_mut(li).ext_mut().unwrap().style_dirty = false;
    }
    // A structural change after the drain re-marks the siblings.
    let extra = dom.create_element("li");
    dom.append_child(list, extra).unwrap();
    assert!(
        dom.node(items[0]).ext().unwrap().style_dirty,
        "siblings re-marked after drain"
    );
    assert!(dom.node(extra).ext().unwrap().style_dirty);
}

#[test]
fn install_returns_tracker() {
    let mut dom: TuiDom = TuiDom::new();
    let tracker = DirtyTracker::install(&mut dom);
    assert!(tracker.observer_id().is_some());
    assert_eq!(dom.observer_count(), 1);
}

#[test]
fn uninstall_removes_observer() {
    let mut dom: TuiDom = TuiDom::new();
    let tracker = DirtyTracker::install(&mut dom);
    let _roots = tracker.uninstall(&mut dom);
    assert_eq!(dom.observer_count(), 0);
}

#[test]
fn set_attribute_marks_dirty() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();

    let tracker = DirtyTracker::install(&mut dom);
    dom.set_attribute(div, "id", "main").unwrap();

    let roots = tracker.take_roots();
    assert!(roots.contains(&div));
    assert!(dom.node(div).ext().unwrap().style_dirty);
}

#[test]
fn add_class_marks_dirty() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let tracker = DirtyTracker::install(&mut dom);
    dom.add_class(div, "active").unwrap();
    assert!(tracker.take_roots().contains(&div));
}

#[test]
fn tree_mutation_marks_subtree_and_siblings() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("div");
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(parent, a).unwrap();
    dom.append_child(parent, b).unwrap();
    dom.append_child(root, parent).unwrap();

    let tracker = DirtyTracker::install(&mut dom);
    // Insert a new child — siblings a, b should also be dirty
    // (sibling-dependent selectors might now match differently).
    let c = dom.create_element("c");
    dom.append_child(parent, c).unwrap();

    let roots = tracker.roots_snapshot();
    // Dedupe: parent's `parent` becomes a dirty root first (through the
    // insertion path), then sibling dirtying for a/b gets subsumed by
    // their parent's dirt... actually no, their parent `parent` is not
    // itself dirty, only its children. So a, b, c should each be roots.
    assert!(roots.contains(&c));
}

#[test]
fn hover_changes_mark_both_prev_and_next() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();

    let tracker = DirtyTracker::install(&mut dom);
    dom.set_hovered(Some(a));
    let roots1 = tracker.take_roots();
    assert!(roots1.contains(&a));

    dom.set_hovered(Some(b));
    let roots2 = tracker.take_roots();
    // Both the old (a) and new (b) should now be dirty.
    assert!(roots2.contains(&a));
    assert!(roots2.contains(&b));
}

/// Clear every `style_dirty` flag, as a cascade pass would, so the
/// next record's marks can be read on their own.
fn settle(dom: &mut TuiDom, tracker: &DirtyTracker, nodes: [NodeId; 4]) {
    tracker.take_roots();
    for id in std::iter::once(dom.root()).chain(nodes) {
        if let Some(ext) = dom.node_mut(id).ext_mut() {
            ext.style_dirty = false;
        }
    }
}

/// `root > ul > li > (s1, s2)`, a tracker with no sibling
/// combinators in play.
fn hover_chain() -> (TuiDom, DirtyTracker, [NodeId; 4]) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let ul = dom.create_element("ul");
    let li = dom.create_element("li");
    let s1 = dom.create_element("span");
    let s2 = dom.create_element("span");
    dom.append_child(root, ul).unwrap();
    dom.append_child(ul, li).unwrap();
    dom.append_child(li, s1).unwrap();
    dom.append_child(li, s2).unwrap();
    let tracker = DirtyTracker::install(&mut dom);
    tracker.set_sibling_combinators(false);
    (dom, tracker, [ul, li, s1, s2])
}

fn dirty(dom: &TuiDom, id: NodeId) -> bool {
    dom.node(id).ext().is_some_and(|e| e.style_dirty)
}

/// Whether a subtree cascade of `roots` restyles `id`.
fn covered(dom: &TuiDom, roots: &[NodeId], id: NodeId) -> bool {
    let mut cur = Some(id);
    while let Some(n) = cur {
        if roots.contains(&n) {
            return true;
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    }
    false
}

/// `P7G-HOVER-ANCESTORS-1`: `:hover` matches the hovered element's
/// ancestors (Selectors 4 §9.2), so entering a child restyles the
/// whole chain it joins.
#[test]
fn hovering_a_child_restyles_its_ancestor_chain() {
    let (mut dom, tracker, [ul, li, s1, s2]) = hover_chain();
    dom.set_hovered(Some(s1));
    let roots = tracker.take_roots();
    assert_eq!(roots, vec![ul], "the topmost element that became hovered");
    assert!([ul, li, s1].iter().all(|&id| covered(&dom, &roots, id)));
    assert!(!dirty(&dom, s2), "a sibling is not marked on its own");
}

/// Moving between two children of the hovered `<li>` leaves the
/// shared part of the chain (`li` and up) hovered: only the two
/// children restyle.
#[test]
fn moving_between_siblings_restyles_only_the_unshared_chain() {
    let (mut dom, tracker, nodes @ [ul, li, s1, s2]) = hover_chain();
    dom.set_hovered(Some(s1));
    settle(&mut dom, &tracker, nodes);
    dom.set_hovered(Some(s2));
    assert_eq!(tracker.take_roots(), vec![s1, s2]);
    assert!(
        !dirty(&dom, li) && !dirty(&dom, ul),
        "the shared chain keeps :hover"
    );
}

/// Moving from a child to its parent: the parent stays hovered, only
/// the child leaves the chain.
#[test]
fn moving_from_a_child_to_its_parent_restyles_only_the_child() {
    let (mut dom, tracker, nodes @ [_, li, s1, _]) = hover_chain();
    dom.set_hovered(Some(s1));
    settle(&mut dom, &tracker, nodes);
    dom.set_hovered(Some(li));
    assert_eq!(tracker.take_roots(), vec![s1]);
    assert!(!dirty(&dom, li));
}

/// Leaving the document restyles the whole old chain.
#[test]
fn leaving_restyles_the_whole_old_chain() {
    let (mut dom, tracker, nodes @ [ul, li, s1, _]) = hover_chain();
    dom.set_hovered(Some(s1));
    settle(&mut dom, &tracker, nodes);
    dom.set_hovered(None);
    let roots = tracker.take_roots();
    assert_eq!(roots, vec![ul]);
    assert!(covered(&dom, &roots, li) && covered(&dom, &roots, s1));
}

/// `:active` (Selectors 4 §9.4) follows the same chain.
#[test]
fn active_changes_restyle_the_unshared_chain() {
    let (mut dom, tracker, nodes @ [ul, li, s1, s2]) = hover_chain();
    dom.set_active(Some(s1));
    assert_eq!(tracker.take_roots(), vec![ul]);
    settle(&mut dom, &tracker, nodes);
    dom.set_active(Some(s2));
    assert_eq!(tracker.take_roots(), vec![s1, s2]);
    assert!(!dirty(&dom, li));
    settle(&mut dom, &tracker, nodes);
    dom.set_active(None);
    assert_eq!(tracker.take_roots(), vec![ul]);
}

/// Removing the hovered element clears hover while its old ancestors
/// are still reachable, so their `:hover` restyles.
#[test]
fn removing_the_hovered_element_restyles_its_old_ancestors() {
    let (mut dom, tracker, nodes @ [_, li, s1, _]) = hover_chain();
    dom.set_hovered(Some(s1));
    settle(&mut dom, &tracker, nodes);
    dom.remove_child(li, s1).unwrap();
    let roots = tracker.take_roots();
    assert!(
        covered(&dom, &roots, li),
        "li no longer contains the hovered element: {roots:?}"
    );
}

#[test]
fn focus_changes_mark_prev_and_next() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();

    let tracker = DirtyTracker::install(&mut dom);
    dom.set_focused(Some(a));
    dom.set_focused(Some(b));
    let roots = tracker.take_roots();
    assert!(roots.contains(&a));
    assert!(roots.contains(&b));
}

#[test]
fn focus_changes_dirty_ancestor_chain_for_focus_within() {
    // `:focus-within` matches every ancestor of the focused
    // element. When focus moves, those ancestors' style
    // changes — so the cascade must re-run on at least one
    // root that covers them. The dirty tracker walks up from
    // prev/next and marks an ancestor that re-cascades the
    // whole chain.
    //
    // Tree: outer > middle > inner.
    // Focus inner → outer's `:focus-within` flips → outer's
    // subtree must be re-cascaded.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let outer = dom.create_element("div");
    let middle = dom.create_element("div");
    let inner = dom.create_element("span");
    dom.append_child(middle, inner).unwrap();
    dom.append_child(outer, middle).unwrap();
    dom.append_child(root, outer).unwrap();

    let tracker = DirtyTracker::install(&mut dom);
    dom.set_focused(Some(inner));
    let roots = tracker.take_roots();
    // The exact root pushed is an implementation detail (could
    // be `inner`, or its topmost element ancestor `outer`).
    // What matters is that SOMETHING re-cascades the chain —
    // either the topmost ancestor is a root, or every ancestor
    // along the chain has `style_dirty` set so its parent's
    // cascade visits them. The simplest pin: `outer` (the
    // topmost element ancestor) must end up either in roots
    // OR have `style_dirty = true`, so a future cascade pass
    // re-evaluates its `:focus-within` selector match.
    let outer_dirty =
        roots.contains(&outer) || dom.node(outer).ext().is_some_and(|e| e.style_dirty);
    assert!(
        outer_dirty,
        "outer must re-cascade so its :focus-within match flips when inner gets focus"
    );
}

#[test]
fn dedup_with_dirty_ancestor() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("div");
    let child = dom.create_element("span");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();

    let tracker = DirtyTracker::install(&mut dom);

    // Mutate parent first — parent gets dirty.
    dom.set_attribute(parent, "role", "banner").unwrap();
    // Now mutate child — ancestor is dirty, child should not be
    // added to the roots list (but its style_dirty flag still flips).
    dom.set_attribute(child, "id", "x").unwrap();

    let roots = tracker.take_roots();
    assert!(roots.contains(&parent));
    assert!(!roots.contains(&child));
    assert!(dom.node(child).ext().unwrap().style_dirty);
}

#[test]
fn take_roots_clears_list() {
    let mut dom: TuiDom = TuiDom::new();
    let div = dom.create_element("div");
    dom.append_child(dom.root(), div).unwrap();
    let tracker = DirtyTracker::install(&mut dom);
    dom.set_attribute(div, "x", "1").unwrap();
    assert!(!tracker.take_roots().is_empty());
    // Second take returns empty — state was cleared.
    assert!(tracker.take_roots().is_empty());
}

#[test]
fn roots_snapshot_does_not_clear() {
    let mut dom: TuiDom = TuiDom::new();
    let div = dom.create_element("div");
    dom.append_child(dom.root(), div).unwrap();
    let tracker = DirtyTracker::install(&mut dom);
    dom.set_attribute(div, "x", "1").unwrap();
    let s1 = tracker.roots_snapshot();
    let s2 = tracker.roots_snapshot();
    assert_eq!(s1, s2);
}

#[test]
fn character_data_change_does_not_dirty_cascade_but_flags_paint() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let t = dom.create_text_node("hello");
    dom.append_child(root, t).unwrap();
    let tracker = DirtyTracker::install(&mut dom);
    dom.node_mut(t).set_node_value("world").unwrap();
    // Text data change fires CharacterDataChanged — selectors don't
    // depend on text, so cascade is not dirty.
    assert!(tracker.roots_snapshot().is_empty());
    // But painted output changed, so paint_dirty IS set.
    assert!(tracker.paint_dirty_snapshot());
}

#[test]
fn take_paint_dirty_clears_flag() {
    let mut dom: TuiDom = TuiDom::new();
    let t = dom.create_text_node("hi");
    dom.append_child(dom.root(), t).unwrap();
    let tracker = DirtyTracker::install(&mut dom);
    dom.node_mut(t).set_node_value("ho").unwrap();
    assert!(tracker.take_paint_dirty());
    // Second take returns false — flag was cleared.
    assert!(!tracker.take_paint_dirty());
}

#[test]
fn set_hovered_to_same_does_not_dirty() {
    let mut dom: TuiDom = TuiDom::new();
    let a = dom.create_element("a");
    dom.append_child(dom.root(), a).unwrap();
    dom.set_hovered(Some(a));
    let tracker = DirtyTracker::install(&mut dom);
    // No-op: already hovering a.
    dom.set_hovered(Some(a));
    assert!(tracker.take_roots().is_empty());
}

#[test]
fn duplicate_dirty_is_deduplicated() {
    let mut dom: TuiDom = TuiDom::new();
    let div = dom.create_element("div");
    dom.append_child(dom.root(), div).unwrap();
    let tracker = DirtyTracker::install(&mut dom);
    dom.set_attribute(div, "x", "1").unwrap();
    dom.set_attribute(div, "y", "2").unwrap();
    dom.set_attribute(div, "z", "3").unwrap();
    // Three mutations on the same node → only one roots entry.
    let roots = tracker.take_roots();
    assert_eq!(roots.iter().filter(|&&r| r == div).count(), 1);
}

#[test]
fn inline_style_setter_marks_dirty() {
    // `P7G-SETTER-MUTATION-1`: `TuiNodeMutExt::set_inline_style`
    // (like every direct style setter) reflects into the `style`
    // attribute, so the tracker sees an `AttributeChanged` and
    // queues the element — no manual `mark_dirty` needed.
    let mut dom: TuiDom = TuiDom::new();
    let div = dom.create_element("div");
    dom.append_child(dom.root(), div).unwrap();
    let tracker = DirtyTracker::install(&mut dom);
    dom.node_mut(div)
        .set_inline_style(TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
    assert_eq!(tracker.take_roots(), vec![div]);
}

/// `P7G-SETTER-MUTATION-1`: the dedupe skips a node only when an
/// ancestor is a queued root. An ancestor whose `style_dirty` flag is
/// set without being queued (a stale flag, a direct `TuiExt` write)
/// no longer swallows the roots below it.
#[test]
fn an_unqueued_dirty_flag_above_does_not_swallow_a_root() {
    let mut dom: TuiDom = TuiDom::new();
    let outer = dom.create_element("div");
    let inner = dom.create_element("div");
    dom.append_child(dom.root(), outer).unwrap();
    dom.append_child(outer, inner).unwrap();
    let tracker = DirtyTracker::install(&mut dom);
    dom.node_mut(outer).ext_mut().unwrap().style_dirty = true;
    dom.set_attribute(inner, "data-x", "1").unwrap();
    assert!(tracker.take_roots().contains(&inner));
}

#[test]
fn mark_dirty_escape_hatch() {
    // Companion test: after writing inline_style, the caller uses
    // tracker.mark_dirty() to trigger cascade invalidation.
    let mut dom: TuiDom = TuiDom::new();
    let div = dom.create_element("div");
    dom.append_child(dom.root(), div).unwrap();
    let tracker = DirtyTracker::install(&mut dom);
    dom.node_mut(div)
        .set_inline_style(TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
    tracker.mark_dirty(&mut dom, div);
    assert!(tracker.take_roots().contains(&div));
    assert!(dom.node(div).is_style_dirty());
}

/// C11-HAS: without a `:has()` rule a mutation looks for no anchor —
/// zero extra work, even in a document a cascade flagged anchors in;
/// with one, the walk is the changed element's ancestors only.
#[test]
fn has_invalidation_costs_nothing_without_a_has_rule() {
    use crate::CascadeExt;
    use crate::style::dirty_tracker::marks::probe;
    use crate::style::has_triggers::HasTriggers;
    const DEPTH: usize = 30;
    let mut dom: TuiDom = TuiDom::new();
    let mut parent = dom.root();
    let mut chain = Vec::new();
    for _ in 0..DEPTH {
        let d = dom.create_element("div");
        dom.append_child(parent, d).unwrap();
        chain.push(d);
        parent = d;
    }
    let leaf = parent;
    let anchored = rdom_css::parse("div:has(.x) { color: red }").stylesheet;
    dom.cascade(&anchored);
    let tracker = DirtyTracker::install(&mut dom);
    let mutate = |dom: &mut TuiDom| {
        dom.add_class(leaf, "x").unwrap();
        dom.set_attribute(leaf, "data-k", "1").unwrap();
        let c = dom.create_element("i");
        dom.append_child(leaf, c).unwrap();
        dom.set_hovered(Some(c));
    };
    let plain = rdom_css::parse("div .x { color: red } p:hover { color: red }").stylesheet;
    tracker.set_has_triggers(HasTriggers::of_sheets([&plain]));
    probe::take();
    mutate(&mut dom);
    assert_eq!(probe::take(), 0, "no :has() rule, no anchor walk");
    tracker.take_roots();
    dom.set_hovered(None);
    tracker.set_has_triggers(HasTriggers::of_sheets([&anchored]));
    probe::take();
    dom.remove_class(leaf, "x").unwrap();
    let steps = probe::take();
    assert!(
        steps > 0 && steps <= DEPTH as u64 + 1,
        "{steps} steps for depth {DEPTH}"
    );
    let roots = tracker.take_roots();
    assert!(
        roots.contains(&chain[0]),
        "the outermost anchor is restyled"
    );
}

/// C11-HAS: before the App names its sheets the tracker assumes every
/// change can reach an anchor — and still walks each change once: an
/// anchor it restyles is no input of another anchor's `:has()` (it does
/// not nest), so marking one starts no walk of its own.
#[test]
fn has_invalidation_with_unknown_sheets_walks_once_per_change() {
    use crate::CascadeExt;
    use crate::style::dirty_tracker::marks::probe;
    const DEPTH: usize = 30;
    let mut dom: TuiDom = TuiDom::new();
    let mut parent = dom.root();
    for _ in 0..DEPTH {
        let d = dom.create_element("div");
        dom.append_child(parent, d).unwrap();
        parent = d;
    }
    dom.cascade(&rdom_css::parse("div:has(.x) { color: red }").stylesheet);
    let tracker = DirtyTracker::install(&mut dom);
    probe::take();
    dom.add_class(parent, "x").unwrap();
    let steps = probe::take();
    assert!(steps <= 2 * DEPTH as u64, "{steps} steps for depth {DEPTH}");
    drop(tracker);
}

/// 5000 list rows, each a `:has()` anchor of `sheet` once cascaded, with
/// the tracker told the sheet's triggers.
fn has_rows(sheet: &str) -> (TuiDom, Vec<NodeId>, DirtyTracker) {
    use crate::CascadeExt;
    use crate::style::has_triggers::HasTriggers;
    const N: usize = 5000;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let ul = dom.create_element("ul");
    dom.append_child(root, ul).unwrap();
    let rows: Vec<NodeId> = (0..N)
        .map(|_| {
            let li = dom.create_element("li");
            dom.append_child(ul, li).unwrap();
            li
        })
        .collect();
    let sheet = rdom_css::parse(sheet).stylesheet;
    dom.cascade(&sheet);
    let tracker = DirtyTracker::install(&mut dom);
    tracker.set_has_triggers(HasTriggers::of_sheets([&sheet]));
    (dom, rows, tracker)
}

/// C11G-HAS-COST (architect N3): with `+` the only sibling relation, an
/// anchor is at most one earlier sibling away from a changed element or
/// its ancestors — so toggling a class on 5000 rows, a frame each, walks a
/// few steps each, not every earlier row (~12.5M steps) — and the anchor
/// before the row is still restyled.
#[test]
fn an_adjacent_has_walk_checks_one_earlier_sibling() {
    use crate::style::dirty_tracker::marks::probe;
    let (mut dom, rows, tracker) = has_rows("li:has(+ .on) { color: red }");
    probe::take();
    dom.add_class(rows[1], "on").unwrap();
    assert!(tracker.take_roots().contains(&rows[0]), "the anchor before");
    // One drain per change (a frame each), so no walk is shared.
    probe::take();
    for &li in &rows {
        dom.add_class(li, "on").unwrap();
        tracker.take_roots();
    }
    let steps = probe::take();
    assert!(steps <= 6 * rows.len() as u64, "{steps} steps");
    drop(tracker);
}

/// C11G-HAS-COST (architect N3): with `~` an anchor can be any earlier
/// sibling, but within one drain each earlier-sibling run is walked once:
/// 5000 rows toggled last to first cost a linear walk, not a quadratic
/// one.
#[test]
fn a_subsequent_sibling_has_walk_is_deduplicated_within_a_drain() {
    use crate::style::dirty_tracker::marks::probe;
    let (mut dom, rows, tracker) = has_rows("li:has(~ .on) { color: red }");
    probe::take();
    for &li in rows.iter().rev() {
        dom.add_class(li, "on").unwrap();
    }
    let steps = probe::take();
    assert!(steps <= 6 * rows.len() as u64, "{steps} steps");
    let roots = tracker.take_roots();
    assert!(roots.contains(&rows[0]) && roots.contains(&rows[rows.len() - 2]));
    // A new drain walks again.
    probe::take();
    dom.remove_class(rows[rows.len() - 1], "on").unwrap();
    assert!(probe::take() >= rows.len() as u64 - 1);
    assert!(tracker.take_roots().contains(&rows[0]));
}

/// C11G-DIR-AUTO-COST (architect N4): a `dir=auto` host's directionality
/// is its first strong character (HTML §3.2.6.4) — an edit that leaves it
/// where it was does not restyle the host's subtree; one that flips it
/// does. 100 appended log rows under `<main dir=auto>` restyle `main`
/// zero times; a Hebrew first row restyles it once.
#[test]
fn a_dir_auto_host_restyles_only_when_its_direction_flips() {
    use crate::CascadeExt;
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let main = dom.create_element("main");
    dom.set_attribute(main, "dir", "auto").unwrap();
    dom.append_child(root, main).unwrap();
    let first = dom.create_element("p");
    let first_text = dom.create_text_node("start");
    dom.append_child(first, first_text).unwrap();
    dom.append_child(main, first).unwrap();
    dom.cascade(&crate::style::Stylesheet::new());
    let tracker = DirtyTracker::install(&mut dom);
    let mut restyles = 0;
    for i in 0..100 {
        let row = dom.create_element("p");
        let t = dom.create_text_node(&format!("row {i}"));
        dom.append_child(row, t).unwrap();
        dom.append_child(main, row).unwrap();
        dom.node_mut(first_text)
            .set_node_value(&format!("tick {i}"))
            .unwrap();
        restyles += usize::from(tracker.take_roots().contains(&main));
    }
    assert_eq!(restyles, 0, "the direction stayed ltr");
    dom.node_mut(first_text).set_node_value("שלום").unwrap();
    assert!(tracker.take_roots().contains(&main), "ltr → rtl restyles");
    dom.node_mut(first_text)
        .set_node_value("שלום עולם")
        .unwrap();
    assert!(!tracker.take_roots().contains(&main), "still rtl");
    dom.node_mut(first_text).set_node_value("hello").unwrap();
    assert!(tracker.take_roots().contains(&main), "rtl → ltr restyles");
}

/// C11G-HAS-IS-SIBLING: a sibling step nested in `:is()` inside a `:has()`
/// argument relates elements the relative selector's own `+` / `~` does
/// not bound — in `.a:has(+ :is(.x ~ *))` the `.x` is any earlier sibling
/// of the anchor's next sibling, so it can stand *before* the anchor
/// (Selectors 4 §4.2, §4.5: `:is()` is not scoped to the anchor). A change
/// there must restyle the anchor, a *later* sibling, which the `:has()`
/// walk (earlier siblings only) does not reach. With the App's triggers
/// (both computed from the sheet): a class toggled on, the pointer moved
/// into, and a child appended to a `.x` two elements before the anchor,
/// and a `.x` inserted there. The reach is covered by `SiblingTriggers`,
/// not the `:has()` walk (`has_triggers` module doc): this pins it.
#[test]
fn a_nested_sibling_step_before_the_anchor_restyles_it() {
    use crate::CascadeExt;
    use crate::style::has_triggers::HasTriggers;
    use crate::style::sibling_triggers::SiblingTriggers;
    for (css, change) in [
        (".a:has(+ :is(.x ~ *)) { color: red }", "class"),
        (".a:has(+ :is(i:hover ~ *)) { color: red }", "hover"),
        (".a:has(+ :is(i:empty ~ *)) { color: red }", "child"),
        (".a:has(+ :not(.x ~ *)) { color: red }", "class"),
        (".a:has(+ :nth-child(2 of .x)) { color: red }", "class"),
        (".a:has(+ :is(.x ~ *)) { color: red }", "insert"),
    ] {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let p = dom.create_element("p");
        dom.append_child(root, p).unwrap();
        let x = dom.create_element("i");
        let other = dom.create_element("i");
        let anchor = dom.create_element("b");
        let next = dom.create_element("u");
        for e in [x, other, anchor, next] {
            dom.append_child(p, e).unwrap();
        }
        dom.add_class(other, "x").unwrap();
        dom.add_class(anchor, "a").unwrap();
        let child = dom.create_element("s");
        if change == "hover" {
            dom.append_child(x, child).unwrap();
        }
        let sheet = rdom_css::parse(css).stylesheet;
        dom.cascade(&sheet);
        assert!(
            dom.node(anchor).ext().is_some_and(|e| e.has_anchor),
            "{css}: flagged"
        );
        let tracker = DirtyTracker::install(&mut dom);
        tracker.set_sibling_triggers(SiblingTriggers::of_sheets([&sheet]));
        tracker.set_has_triggers(HasTriggers::of_sheets([&sheet]));
        match change {
            "class" => dom.add_class(x, "x").unwrap(),
            "hover" => dom.set_hovered(Some(child)),
            "insert" => {
                let new = dom.create_element("i");
                dom.add_class(new, "x").unwrap();
                dom.insert_before(p, new, Some(x)).unwrap();
            }
            _ => dom.append_child(x, child).unwrap(),
        }
        let roots = tracker.take_roots();
        assert!(covered(&dom, &roots, anchor), "{css}: {roots:?}");
    }
}

/// C13-COLUMN: the tracker restyles a table on a column change only when
/// a sheet can read columns (Selectors 4 §16) — `||`, `:nth-col()`,
/// `:nth-last-col()`, nested in a functional pseudo-class too.
#[test]
fn column_selectors_are_found_in_the_sheets() {
    let uses = |css: &str| uses_column_selectors(&rdom_css::parse(css).stylesheet);
    assert!(uses("col || td { color: red }"));
    assert!(uses("td:nth-col(2) { color: red }"));
    assert!(uses(":is(td:nth-last-col(1)) { color: red }"));
    assert!(!uses("td:nth-child(2), col + td { color: red }"));
    assert!(!uses_column_selectors(&crate::style::Stylesheet::new()));
}

/// C13-COLUMN: with no column selector in the sheets, a `span` change
/// marks only the element whose attribute changed.
#[test]
fn a_column_change_without_column_selectors_marks_only_its_element() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let table = dom.create_element("table");
    let colgroup = dom.create_element("colgroup");
    let col = dom.create_element("col");
    let tr = dom.create_element("tr");
    let td = dom.create_element("td");
    dom.append_child(root, table).unwrap();
    dom.append_child(table, colgroup).unwrap();
    dom.append_child(colgroup, col).unwrap();
    dom.append_child(table, tr).unwrap();
    dom.append_child(tr, td).unwrap();
    let tracker = DirtyTracker::install(&mut dom);
    crate::style::CascadeExt::cascade(&mut dom, &crate::style::Stylesheet::new());
    let _ = tracker.take_roots();
    tracker.set_column_selectors(false);
    dom.set_attribute(col, "span", "2").unwrap();
    assert_eq!(tracker.take_roots(), [col]);
    tracker.set_column_selectors(true);
    dom.set_attribute(col, "span", "3").unwrap();
    assert!(tracker.take_roots().contains(&table));
}
