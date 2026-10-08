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
    assert_eq!(h.size(), 2, "a set: no duplicate");
    assert_eq!((h.priority, h.kind), (0, HighlightType::Highlight));
    assert!(h.has(&b));
    assert!(!h.add(a.clone()), "already there");
    assert!(h.delete(&a));
    assert_eq!(h.ranges(), std::slice::from_ref(&b));
    h.clear();
    assert_eq!(h.size(), 0);
    let h = Highlight::new([b])
        .with_priority(2)
        .with_type(HighlightType::SpellingError);
    assert_eq!((h.priority, h.kind), (2, HighlightType::SpellingError));
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
