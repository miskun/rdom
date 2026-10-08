//! C10-HIGHLIGHT — the CSS Custom Highlight API 1 data model: highlights
//! (sets of ranges with a priority and a type), the document's registry,
//! and the ranges kept live across DOM mutations (DOM §5.3).

use crate::{Dom, Highlight, HighlightType, Mutation, NodeId, Position, Range};

fn range(start: (NodeId, usize), end: (NodeId, usize)) -> Range {
    Range::ordered_unchecked(Position::new(start.0, start.1), Position::new(end.0, end.1))
}

/// A `<p>` holding the text `text`, under the root: (dom, p, text).
fn para(text: &str) -> (Dom, NodeId, NodeId) {
    let mut dom: Dom = Dom::new();
    let p = dom.create_element("p");
    let t = dom.create_text_node(text);
    dom.append_child(dom.root(), p).unwrap();
    dom.append_child(p, t).unwrap();
    (dom, p, t)
}

/// The ranges of the highlight `name`.
fn ranges(dom: &Dom, name: &str) -> Vec<Range> {
    dom.highlights().get(name).unwrap().ranges().to_vec()
}

/// CSS Custom Highlight API 1 §3: a `Highlight` is a set of ranges (no
/// duplicates) with a `priority` (initial 0) and a `type` (initial
/// `highlight`).
#[test]
fn a_highlight_is_a_set_of_ranges_with_a_priority_and_a_type() {
    let (_, _, t) = para("hello");
    let a = range((t, 0), (t, 2));
    let b = range((t, 3), (t, 5));
    let mut h = Highlight::new([a.clone(), b.clone(), a.clone()]);
    assert_eq!(h.len(), 2, "a set: no duplicate");
    assert_eq!((h.priority(), h.kind()), (0, HighlightType::Highlight));
    assert!(h.has(&b));
    assert!(!h.add(a.clone()), "already there");
    assert!(h.delete(&a));
    assert_eq!(h.ranges(), std::slice::from_ref(&b));
    h.clear();
    assert!(h.is_empty());
    let mut h = Highlight::new([b])
        .with_priority(2)
        .with_kind(HighlightType::SpellingError);
    assert_eq!((h.priority(), h.kind()), (2, HighlightType::SpellingError));
    h.set_priority(-1);
    h.set_kind(HighlightType::GrammarError);
    assert_eq!((h.priority(), h.kind()), (-1, HighlightType::GrammarError));
}

/// §4 `HighlightRegistry` (`CSS.highlights`): a map from names to
/// highlights, in registration order — setting a registered name
/// replaces its highlight in place — with `get`, `has`, `delete`,
/// `clear`. Changing it through `highlights_mut` fires
/// `Mutation::HighlightsChanged`, so a renderer repaints.
#[test]
fn the_registry_maps_names_in_registration_order() {
    struct Count(std::rc::Rc<std::cell::Cell<usize>>);
    impl crate::MutationObserver<()> for Count {
        fn observe(&mut self, _dom: &mut Dom, record: &Mutation) {
            if matches!(record, Mutation::HighlightsChanged) {
                self.0.set(self.0.get() + 1);
            }
        }
    }
    let (mut dom, _, t) = para("hello");
    let observed = std::rc::Rc::new(std::cell::Cell::new(0));
    dom.add_mutation_observer(Box::new(Count(observed.clone())));
    let h = |n| Highlight::new([range((t, 0), (t, n))]);
    dom.highlights_mut().set("b", h(1));
    dom.highlights_mut().set("a", h(2));
    dom.highlights_mut().set("b", h(3));
    let names: Vec<&str> = dom.highlights().iter().map(|(n, _)| n).collect();
    assert_eq!(names, ["b", "a"], "re-setting keeps the registration order");
    assert_eq!(ranges(&dom, "b"), [range((t, 0), (t, 3))]);
    assert!(dom.highlights().has("a"));
    assert!(dom.highlights_mut().delete("a"));
    assert!(!dom.highlights().has("a"));
    assert_eq!(dom.highlights().len(), 1);
    dom.highlights_mut().clear();
    assert!(dom.highlights().is_empty());
    assert_eq!(observed.get(), 5, "each access for change fires a record");
}

/// DOM §4.10 "replace data" steps 8–11: a live range's boundary inside
/// the replaced data moves to its start; one after it shifts by the
/// change in length. Text edits (`edit_text`) are precise; setting the
/// whole data replaces it all.
#[test]
fn ranges_follow_text_edits() {
    let (mut dom, _, t) = para("hello world");
    dom.highlights_mut()
        .set("w", Highlight::new([range((t, 6), (t, 11))]));
    dom.node_mut(t).edit_text(0, 0, "say ").unwrap();
    assert_eq!(ranges(&dom, "w"), [range((t, 10), (t, 15))], "shifted by 4");
    dom.node_mut(t).edit_text(11, 13, "").unwrap();
    assert_eq!(
        ranges(&dom, "w"),
        [range((t, 10), (t, 13))],
        "end inside the deletion moves to its start, then the rest"
    );
    dom.node_mut(t).set_data("hi").unwrap();
    assert_eq!(ranges(&dom, "w"), [range((t, 0), (t, 0))], "all replaced");
}

/// DOM §4.2.3 "insert" step 6 and "remove" steps 4–7: a boundary in the
/// parent after the insertion point moves right; a boundary inside a
/// removed node moves to the parent at the node's index, and one in the
/// parent after it moves left.
#[test]
fn ranges_follow_tree_changes() {
    let (mut dom, p, t) = para("abc");
    let b = dom.create_element("b");
    dom.append_child(p, b).unwrap();
    dom.highlights_mut().set(
        "x",
        Highlight::new([range((p, 1), (p, 2)), range((t, 1), (t, 2))]),
    );
    let i = dom.create_element("i");
    dom.prepend_child(p, i).unwrap();
    assert_eq!(
        ranges(&dom, "x"),
        [range((p, 2), (p, 3)), range((t, 1), (t, 2))]
    );
    dom.remove_child(p, t).unwrap();
    assert_eq!(
        ranges(&dom, "x"),
        [range((p, 1), (p, 2)), range((p, 1), (p, 1))],
        "the text's range moves to (p, 1); the element range shifts left"
    );
}

thread_local! {
    /// Sibling hops `Dom::child_index` walked.
    pub(crate) static INDEX_HOPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// C10G-HIGHLIGHT-COST — DOM §4.2.3 "insert" step 6 moves a boundary in
/// the parent only past the insertion index, and an appended child's index
/// is the old child count, which no boundary exceeds: appending costs no
/// walk of the siblings while a highlight is registered (a log appending
/// lines under a search highlight walked every earlier line per append).
/// An insertion elsewhere, or a removal, walks them only when a boundary
/// sits in the parent (or, for a removal, inside the removed node).
#[test]
fn appending_under_a_highlight_walks_no_siblings() {
    let (mut dom, p, t) = para("hit");
    dom.highlights_mut()
        .set("search", Highlight::new([range((t, 0), (t, 3))]));
    let log = dom.create_element("div");
    dom.append_child(dom.root(), log).unwrap();
    INDEX_HOPS.with(|c| c.set(0));
    for _ in 0..2000 {
        let line = dom.create_element("div");
        dom.append_child(log, line).unwrap();
    }
    let first = dom.node(log).first_child().unwrap().id();
    let line = dom.create_element("div");
    dom.insert_before(log, line, Some(first)).unwrap();
    // Removing lines moves no boundary either: none is in the log.
    for _ in 0..100 {
        let last = dom.node(log).last_child().unwrap().id();
        dom.remove_child(log, last).unwrap();
    }
    assert_eq!(INDEX_HOPS.with(std::cell::Cell::get), 0);
    // A boundary in the log itself: an append still walks nothing.
    dom.highlights_mut()
        .set("top", Highlight::new([range((log, 0), (log, 1))]));
    for _ in 0..500 {
        let line = dom.create_element("div");
        dom.append_child(log, line).unwrap();
    }
    assert_eq!(INDEX_HOPS.with(std::cell::Cell::get), 0, "appends");
    assert_eq!(ranges(&dom, "top"), [range((log, 0), (log, 1))]);
    // A boundary in the parent still moves (§4.2.3 step 6).
    let mid = range((p, 0), (p, 1));
    dom.highlights_mut().set("mid", Highlight::new([mid]));
    let x = dom.create_element("b");
    dom.insert_before(p, x, Some(t)).unwrap();
    assert_eq!(ranges(&dom, "mid"), [range((p, 0), (p, 2))]);
}

/// DOM §4.2.3 "insert" step 4: inserting a `DocumentFragment` first removes
/// its children from it, which moves a live range inside them to `(fragment,
/// 0)` ("remove" step 4) — the fragment's children go through the remove
/// hook as any other removal does.
#[test]
fn inserting_a_fragment_removes_its_children_from_it_first() {
    for append in [true, false] {
        let (mut dom, p, t) = para("x");
        let fragment = dom.create_document_fragment();
        let b = dom.create_element("b");
        let inner = dom.create_text_node("inner");
        dom.append_child(b, inner).unwrap();
        dom.append_child(fragment, b).unwrap();
        dom.highlights_mut()
            .set("h", Highlight::new([range((inner, 1), (inner, 3))]));
        if append {
            dom.append_child(p, fragment).unwrap();
        } else {
            dom.insert_before(p, fragment, Some(t)).unwrap();
        }
        assert_eq!(
            ranges(&dom, "h"),
            [range((fragment, 0), (fragment, 0))],
            "append {append}"
        );
    }
}

/// What an observer of `HighlightsChanged` sees: the registry's names at
/// each record, and the registry's generation.
struct Seen(std::rc::Rc<std::cell::RefCell<Vec<Vec<String>>>>);

impl crate::MutationObserver<()> for Seen {
    fn observe(&mut self, dom: &mut Dom, record: &Mutation) {
        if matches!(record, Mutation::HighlightsChanged) {
            let names = dom.highlights().iter().map(|(n, _)| n.to_owned()).collect();
            self.0.borrow_mut().push(names);
        }
    }
}

/// C10G-HIGHLIGHT-API — `Mutation::HighlightsChanged` is a record of a
/// change (DOM §4.3: a mutation record is queued after the mutation): it
/// fires once the change is made — an observer reading the registry sees
/// the new state — and only when something changed: a read through
/// `highlights_mut`, a `get_mut` miss, a `get_mut` hit left alone, a
/// `delete` of an unknown name and a `clear` of an empty registry fire
/// nothing and leave `generation` where it was.
#[test]
fn highlights_changed_fires_after_a_change_and_only_then() {
    let (mut dom, _, t) = para("hello");
    let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    dom.add_mutation_observer(Box::new(Seen(seen.clone())));
    dom.highlights_mut()
        .set("a", Highlight::new([range((t, 0), (t, 1))]));
    assert_eq!(*seen.borrow(), [vec!["a".to_owned()]], "after the change");
    let generation = dom.highlights().generation();
    assert!(dom.highlights_mut().get_mut("missing").is_none());
    assert!(dom.highlights_mut().get("a").is_some());
    assert!(!dom.highlights_mut().delete("missing"));
    {
        let mut registry = dom.highlights_mut();
        let h = registry.get_mut("a").unwrap();
        assert!(!h.add(range((t, 0), (t, 1))), "already there: no change");
        h.set_priority(0);
    }
    assert_eq!(seen.borrow().len(), 1, "nothing changed");
    assert_eq!(dom.highlights().generation(), generation);
    dom.highlights_mut()
        .get_mut("a")
        .unwrap()
        .add(range((t, 2), (t, 3)));
    assert_eq!(seen.borrow().len(), 2, "a range added");
    assert!(dom.highlights().generation() != generation);
    dom.highlights_mut().get_mut("a").unwrap().set_priority(3);
    dom.highlights_mut().clear();
    dom.highlights_mut().clear();
    assert_eq!(
        *seen.borrow(),
        [
            vec!["a".to_owned()],
            vec!["a".to_owned()],
            vec!["a".to_owned()],
            vec![]
        ],
        "the priority, then the clear; clearing nothing fires nothing"
    );
}

/// C10G-HIGHLIGHT-API — DOM §5.5 "set the start or end": an offset past
/// the node's length (a text node's data, an element's child count) is an
/// `IndexSizeError`; rdom's byte offsets must also fall on a character. `Dom::range_between` checks both points and orders
/// them in document order (§5.2), so any two valid points make a range.
#[test]
fn a_checked_range_validates_and_orders_its_points() {
    let (mut dom, p, t) = para("hello");
    let a = Position::new(t, 4);
    let b = Position::new(t, 1);
    assert_eq!(dom.range_between(a, b), Ok(range((t, 1), (t, 4))));
    assert_eq!(dom.range_between(b, b), Ok(range((t, 1), (t, 1))));
    assert_eq!(
        dom.range_between(Position::new(t, 5), b)
            .map(|r| r.end.offset),
        Ok(5)
    );
    assert_eq!(
        dom.range_between(Position::new(t, 6), b),
        Err(crate::DomError::InvalidOffset { node: t, offset: 6 })
    );
    assert_eq!(
        dom.range_between(Position::new(p, 0), Position::new(p, 1)),
        Ok(range((p, 0), (p, 1)))
    );
    assert_eq!(
        dom.range_between(Position::new(p, 2), b),
        Err(crate::DomError::InvalidOffset { node: p, offset: 2 })
    );
    let e = dom.create_text_node("é");
    dom.append_child(p, e).unwrap();
    assert_eq!(
        dom.range_between(Position::new(e, 1), b),
        Err(crate::DomError::InvalidOffset { node: e, offset: 1 }),
        "inside a UTF-8 character"
    );
    let detached = dom.create_text_node("x");
    assert!(dom.range_between(Position::new(detached, 0), b).is_err());
}

/// C10G-HIGHLIGHT-API — `Dom::descendants`: the inclusive descendants of a
/// node minus the node, in tree order (DOM §4.2: pre-order, depth-first),
/// text nodes included — the walk a search over the document's text needs
/// (`TreeWalker` / `NodeIterator`'s order) — iterative at any depth.
#[test]
fn descendants_walk_a_subtree_in_tree_order() {
    let (mut dom, p, t) = para("a");
    let b = dom.create_element("b");
    let bt = dom.create_text_node("b");
    dom.append_child(b, bt).unwrap();
    dom.append_child(p, b).unwrap();
    let tail = dom.create_text_node("c");
    dom.append_child(p, tail).unwrap();
    let order: Vec<NodeId> = dom.descendants(dom.root()).collect();
    assert_eq!(order, [p, t, b, bt, tail]);
    assert_eq!(dom.descendants(b).collect::<Vec<_>>(), [bt]);
    assert_eq!(dom.descendants(bt).count(), 0);
    let mut deep = tail;
    let mut cur = dom.root();
    for _ in 0..10_000 {
        let d = dom.create_element("div");
        dom.append_child(cur, d).unwrap();
        cur = d;
        deep = d;
    }
    assert_eq!(dom.descendants(dom.root()).last(), Some(deep));
}
