//! What the cascade does *not* do: work counters (`ladder::probe`) for
//! the paths that must stay free.
//!
//! The rollback states `revert` / `revert-layer` read (CSS Cascade 4
//! §7.3, Cascade 5 §7.4) are needed only when such a value is met, and
//! a pseudo-element no rule matches has no declarations to walk
//! (Cascade 4 §6: the cascade orders *declarations*; with none, the
//! box does not exist — CSS Pseudo-Elements 4 §4 for `::before` /
//! `::after` without `content`).

use super::ladder::probe::{LADDER_WALKS, ROLLBACK_ALLOCS, take};
use super::*;
use crate::TuiDom;
use crate::style::Stylesheet;

fn sheet(css: &str) -> Stylesheet {
    let parsed = rdom_css::parse(css);
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    let mut sheet = Stylesheet::new();
    for rule in parsed.stylesheet.rules() {
        sheet
            .add_rule(&rule.source_text, rule.style.clone())
            .unwrap();
    }
    sheet
}

fn one_div() -> TuiDom {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    dom
}

/// One element, one matched rule, no `revert`, no author pseudo-element
/// rules: two ladders — the element's and its `::selection` (the UA's
/// `*::selection` matches every element) — and no rollback memo. The
/// unmatched `::before`, `::after` and `::backdrop` walk nothing.
#[test]
fn no_revert_no_pseudo_match_walks_one_ladder() {
    let mut dom = one_div();
    let css = sheet("div { color: red }");
    take(&LADDER_WALKS);
    take(&ROLLBACK_ALLOCS);
    dom.cascade(&css);
    assert_eq!(take(&LADDER_WALKS), 2, "the element and UA ::selection");
    assert_eq!(take(&ROLLBACK_ALLOCS), 0, "no revert, no rollback memo");
}

/// A `revert` materialises the memo, for the box that meets it only.
#[test]
fn revert_in_a_pseudo_allocates_its_rollback_only() {
    let mut dom = one_div();
    let css = sheet("div { color: red } div::before { content: 'x'; color: revert }");
    take(&LADDER_WALKS);
    take(&ROLLBACK_ALLOCS);
    dom.cascade(&css);
    assert_eq!(take(&LADDER_WALKS), 3, "element, ::before, ::selection");
    assert_eq!(take(&ROLLBACK_ALLOCS), 1, "only ::before met a revert");
}

/// `C2G-STATELESS-REGISTRY` — the stateless `CascadeExt` forms keep the
/// sheet set's property registry (CSS Properties and Values 1 §2) on the
/// document: two cascades with the same, unchanged sheets build it once
/// and keep each element's match record (no reallocation); a changed
/// sheet builds it again.
#[test]
fn headless_cascades_with_the_same_sheets_build_one_registry() {
    use super::registry_probe::take_builds;
    let mut dom = one_div();
    let div = dom.node(dom.root()).first_child().unwrap().id();
    let mut css = sheet("div { color: red }");
    take_builds();
    dom.cascade(&css);
    let record = dom.node(div).ext().unwrap().matched.clone().unwrap();
    dom.cascade(&css);
    dom.cascade_subtrees(&css, &[div]);
    assert_eq!(take_builds(), 1, "one registry for one sheet set");
    let again = dom.node(div).ext().unwrap().matched.clone().unwrap();
    assert!(Rc::ptr_eq(&record, &again), "the match record is kept");

    css.add_rule("p", crate::style::TuiStyle::new()).unwrap();
    dom.cascade(&css);
    assert_eq!(take_builds(), 1, "a changed sheet is a new sheet set");
    let other = sheet("div { color: blue }");
    dom.cascade_all(&[&css, &other]);
    assert_eq!(take_builds(), 1, "so is another list");
}
