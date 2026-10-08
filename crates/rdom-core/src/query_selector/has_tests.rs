//! C11-HAS — the relational pseudo-class `:has()` (Selectors 4 §4.5).

use crate::{Dom, NodeId, SelectorCaches};

/// ```html
/// <section id=s>
///   <div id=d1><p class=x></p></div>
///   <div id=d2><span><a class=y></a></span></div>
///   <h2 id=h></h2><p id=p2></p><em id=e class=z></em>
/// </section>
/// ```
struct Tree {
    dom: Dom,
    s: NodeId,
}

fn tree() -> Tree {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let mk = |dom: &mut Dom, parent, tag: &str, id: &str, class: &str| {
        let e = dom.create_element(tag);
        if !id.is_empty() {
            dom.set_attribute(e, "id", id).unwrap();
        }
        if !class.is_empty() {
            dom.add_class(e, class).unwrap();
        }
        dom.append_child(parent, e).unwrap();
        e
    };
    let s = mk(&mut dom, root, "section", "s", "");
    let d1 = mk(&mut dom, s, "div", "d1", "");
    mk(&mut dom, d1, "p", "", "x");
    let d2 = mk(&mut dom, s, "div", "d2", "");
    let span = mk(&mut dom, d2, "span", "", "");
    mk(&mut dom, span, "a", "", "y");
    mk(&mut dom, s, "h2", "h", "");
    mk(&mut dom, s, "p", "p2", "");
    mk(&mut dom, s, "em", "e", "z");
    Tree { dom, s }
}

/// The ids of the elements matching `sel`, in tree order.
fn ids(t: &Tree, sel: &str) -> Vec<String> {
    t.dom
        .query_selector_all_in(t.dom.root(), sel)
        .unwrap_or_else(|e| panic!("{sel}: {e}"))
        .into_iter()
        .map(|id| t.dom.get_attribute(id, "id").unwrap_or("?").to_string())
        .collect()
}

/// Selectors 4 §4.5: `:has(<relative-selector-list>)` matches an
/// element (the anchor) when some relative selector matches an element
/// relative to it — by default a descendant, after `>` a child, after
/// `+` the next sibling, after `~` a later sibling — the rest of the
/// selector read as usual from there.
#[test]
fn has_matches_relative_to_the_anchor() {
    let t = tree();
    for (sel, want) in [
        ("div:has(.x)", vec!["d1"]),
        ("div:has(> span)", vec!["d2"]),
        ("div:has(> a)", vec![]),
        ("div:has(span > .y)", vec!["d2"]),
        ("div:has(> span > .y)", vec!["d2"]),
        ("section:has(> div + h2)", vec!["s"]),
        ("section:has(> h2 + div)", vec![]),
        ("h2:has(+ p)", vec!["h"]),
        ("h2:has(+ em)", vec![]),
        ("h2:has(~ em.z)", vec!["h"]),
        ("div:has(~ h2)", vec!["d1", "d2"]),
        ("div:has(+ div)", vec!["d1"]),
        ("div:has(+ div a)", vec!["d1"]),
        ("div:has(~ div > span)", vec!["d1"]),
        (":has(.x, .y)", vec!["s", "d1", "d2", "?"]),
        ("div:not(:has(.x))", vec!["d2"]),
        (":is(div:has(.y))", vec!["d2"]),
        ("div:HAS(.x)", vec!["d1"]),
    ] {
        assert_eq!(ids(&t, sel), want, "{sel}");
    }
}

/// Selectors 4 §4.5: the argument is an unforgiving relative selector
/// list; `:has()` is not valid inside `:has()`, nested or not.
#[test]
fn has_rejects_an_empty_list_and_nested_has() {
    let t = tree();
    for bad in [
        ":has()",
        ":has(> )",
        ":has(a,)",
        ":has(:has(a))",
        ":has(:not(:has(a)))",
        ":has(> > a)",
        ":has(a, !)",
    ] {
        assert!(t.dom.matches(t.s, bad).is_err(), "{bad}");
    }
}

/// Selectors 4 §15: `:has()` counts as its most specific argument.
#[test]
fn has_specificity_is_its_most_specific_argument() {
    let spec = |s: &str| crate::selectors::parse(s).unwrap().0[0].specificity();
    assert_eq!(spec(":has(#a, .b)"), (1, 0, 0));
    assert_eq!(spec("div:has(> span)"), (0, 0, 2));
    assert_eq!(spec(":has(+ .a .b)"), (0, 2, 0));
}

/// Matching `:has()` over a deep chain is linear: a pass's caches record,
/// per relative selector, whether each element's subtree holds a match,
/// so 400 nested `div`s (each a `:has(.x)` anchor) cost one walk of the
/// chain — not 400 · 400 / 2. The anchors tested are recorded for the
/// backend's invalidation.
#[test]
fn has_over_a_deep_chain_is_linear() {
    const N: usize = 400;
    let mut dom: Dom = Dom::new();
    let mut parent = dom.root();
    let mut chain = Vec::new();
    for _ in 0..N {
        let d = dom.create_element("div");
        dom.append_child(parent, d).unwrap();
        chain.push(d);
        parent = d;
    }
    let leaf = dom.create_element("p");
    dom.add_class(leaf, "x").unwrap();
    dom.append_child(parent, leaf).unwrap();
    let list = crate::selectors::parse("div:has(.x)").unwrap();
    let mut caches = SelectorCaches::new();
    let matched = chain
        .iter()
        .filter(|&&d| dom.matches_list_with(d, &list, None, &mut caches))
        .count();
    assert_eq!(matched, N);
    let visits = caches.work().has_nodes;
    assert!(
        visits <= 3 * N as u64,
        "{visits} nodes visited for {N} anchors"
    );
    assert_eq!(caches.has_anchors().count(), N);
}
