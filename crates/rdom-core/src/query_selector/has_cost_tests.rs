//! C11G-HAS-COST — what `:has()` and backtracking combinators cost, by
//! count (`CacheWork`): the bounds TECH_DEBT `HAS-COST-1` states.

use crate::{Dom, NodeId, SelectorCaches};

/// `n` nested `tag`s under the root, outermost first.
fn nested(dom: &mut Dom, n: usize, tag: &str) -> Vec<NodeId> {
    let mut parent = dom.root();
    let mut chain = Vec::with_capacity(n);
    for _ in 0..n {
        let d = dom.create_element(tag);
        dom.append_child(parent, d).unwrap();
        chain.push(d);
        parent = d;
    }
    chain
}

fn work_of(dom: &Dom, ids: &[NodeId], sel: &str) -> (usize, crate::CacheWork) {
    let list = crate::selectors::parse(sel).unwrap();
    let mut caches = SelectorCaches::new();
    let matched = ids
        .iter()
        .filter(|&&d| dom.matches_list_with(d, &list, None, &mut caches))
        .count();
    (matched, caches.work())
}

/// Selectors 4 §4.5 over 1000 nested `div`s, each a `div:has(p div)`
/// anchor (architect N2: the search climbed past the anchor to the root
/// for every candidate, ~N³/6 compound tests). An argument of descendant
/// and child combinators is answered per (element, compound) once per
/// pass — linear — with no chain climb.
#[test]
fn a_downward_has_chain_over_nested_anchors_is_linear() {
    const N: usize = 1000;
    let mut dom: Dom = Dom::new();
    let chain = nested(&mut dom, N, "div");
    let (matched, work) = work_of(&dom, &chain, "div:has(p div)");
    assert_eq!(matched, 0, "there is no `p`");
    assert!(work.has_nodes <= 3 * N as u64, "{work:?}");
    assert_eq!(work.chain_steps, 0, "{work:?}");
    let (matched, work) = work_of(&dom, &chain, "div:has(> div div)");
    assert_eq!(matched, N - 2);
    assert!(work.has_nodes <= 3 * N as u64, "{work:?}");
}

/// A relative selector with a sibling step searches the anchor's later
/// siblings; a climb from a candidate below them stops at the anchor's
/// parent — no compound of the argument can sit there or above (§4.5:
/// the leftmost compound is a later sibling of the anchor).
#[test]
fn a_sibling_has_search_stops_at_the_anchors_parent() {
    const DEPTH: usize = 1000;
    let mut dom: Dom = Dom::new();
    let chain = nested(&mut dom, DEPTH, "section");
    let bottom = *chain.last().unwrap();
    let anchor = dom.create_element("div");
    dom.append_child(bottom, anchor).unwrap();
    let span = dom.create_element("span");
    dom.append_child(bottom, span).unwrap();
    let p = dom.create_element("p");
    dom.append_child(span, p).unwrap();
    let (matched, work) = work_of(&dom, &[anchor], "div:has(+ i p)");
    assert_eq!(matched, 0);
    assert!(work.chain_steps <= 4, "{work:?}");
}

/// Servo's matching outcomes bound backtracking (Selectors 4 §3.1): a
/// mixed `>` / descendant chain over a deep tree with no match costs
/// steps linear in the depth per compound, not exponential — pinned by
/// count (architect N11).
#[test]
fn backtracking_is_bounded_by_the_depth_per_compound() {
    const DEPTH: usize = 300;
    let mut dom: Dom = Dom::new();
    let chain = nested(&mut dom, DEPTH, "div");
    let leaf = dom.create_element("span");
    dom.append_child(*chain.last().unwrap(), leaf).unwrap();
    let (matched, work) = work_of(&dom, &[leaf], "p div > div div > div div span");
    assert_eq!(matched, 0);
    assert!(work.chain_steps <= 6 * DEPTH as u64, "{work:?}");
}

/// The anchors a pass evaluated are facts to flag, not cached answers: a
/// mutation in the middle of the pass (which drops the cached answers)
/// keeps those recorded before it (architect N7).
#[test]
fn has_anchors_survive_a_mutation_mid_pass() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let a = dom.create_element("div");
    let b = dom.create_element("div");
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();
    let list = crate::selectors::parse("div:has(.x)").unwrap();
    let mut caches = SelectorCaches::new();
    dom.matches_list_with(a, &list, None, &mut caches);
    let late = dom.create_element("i");
    dom.append_child(root, late).unwrap();
    dom.matches_list_with(b, &list, None, &mut caches);
    let anchors: Vec<NodeId> = caches.has_anchors().collect();
    assert_eq!(anchors, [a, b]);
}

/// The cached answers are per scoping root (architect N8): one rule
/// matched under two `@scope` roots in one pass reads `:scope` inside
/// `:has()` and inside `of S` against each.
#[test]
fn cached_answers_are_kept_per_scoping_root() {
    let mut dom: Dom = Dom::new();
    let root = dom.root();
    let host = dom.create_element("div");
    dom.append_child(root, host).unwrap();
    let r1 = dom.create_element("i");
    let r2 = dom.create_element("b");
    dom.append_child(host, r1).unwrap();
    dom.append_child(root, r2).unwrap();
    let has = crate::selectors::parse_scoped("div:has(:scope)").unwrap();
    let mut caches = SelectorCaches::new();
    assert!(dom.matches_list_with(host, &has, Some(r1), &mut caches));
    assert!(!dom.matches_list_with(host, &has, Some(r2), &mut caches));
    let nth = crate::selectors::parse_scoped(":nth-child(1 of :scope)").unwrap();
    let mut caches = SelectorCaches::new();
    let x = dom.create_element("p");
    let y = dom.create_element("p");
    dom.append_child(r2, x).unwrap();
    dom.append_child(r2, y).unwrap();
    assert!(dom.matches_list_with(x, &nth, Some(x), &mut caches));
    assert!(dom.matches_list_with(y, &nth, Some(y), &mut caches));
}
