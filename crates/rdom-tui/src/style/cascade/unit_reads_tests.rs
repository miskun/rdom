//! What a cascade learns about the context sizes its styles read (CSS
//! Values 4 §6.1.2, CSS Conditional 5 §6.6): a document's "a style read
//! the viewport" describes that document's styles alone — a cascade
//! nested in another (another document's) does not mark the outer one,
//! and a cascade that panics part-way keeps what the styles it wrote
//! read. C14G-READ-COUNTERS: the reads are returned by the unit
//! resolvers, not sampled from a thread-wide count.

use super::element::hook;
use super::*;
use crate::TuiDom;
use crate::node::TuiNodeExt;
use crate::style::Stylesheet;

fn sheet(css: &str) -> Stylesheet {
    rdom_css::parse(css).stylesheet
}

/// A document of `n` `div`s, the first `#first`.
fn divs(n: usize) -> TuiDom {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    for i in 0..n {
        let div = dom.create_element("div");
        if i == 0 {
            dom.set_attribute(div, "id", "first").unwrap();
        }
        dom.append_child(root, div).unwrap();
    }
    dom.set_viewport(Viewport::new(40, 10));
    dom
}

/// Whether a resize must restyle `dom` (its conditions unchanged).
fn resize_restyles(dom: &mut TuiDom, css: &Stylesheet) -> bool {
    let registry = registered::document_registry(dom, &[css]);
    must_restyle(dom, &[css], &registry, true)
}

/// A cascade of a document with a viewport unit, run inside the cascade
/// of one without, marks the inner document only.
#[test]
fn a_nested_cascade_marks_only_its_own_document() {
    let mut outer = divs(1);
    let plain = sheet("div { color: red }");
    hook::set(|_| {
        let mut inner = divs(1);
        inner.cascade(&sheet("div { width: 50vw }"));
        let css = sheet("div { width: 50vw }");
        assert!(
            resize_restyles(&mut inner, &css),
            "the inner read the viewport"
        );
    });
    outer.cascade(&plain);
    hook::clear();
    assert!(
        !resize_restyles(&mut outer, &plain),
        "the outer document's styles read no viewport"
    );
}

/// A cascade that panics after writing a style that read the viewport
/// keeps that read: the written style is stale after a resize. (A first
/// cascade records the condition results, so only the read can answer.)
#[test]
fn a_panic_mid_cascade_keeps_the_reads_already_made() {
    let mut dom = divs(2);
    let css = sheet("#first { width: 50vw }");
    dom.cascade(&css);
    let second = dom.node(dom.root()).last_child().unwrap().id();
    hook::set(move |id| assert_ne!(id, second, "a panic mid-cascade"));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| dom.cascade(&css)));
    hook::clear();
    assert!(result.is_err(), "the hook panicked");
    let first = dom.node(dom.root()).first_child().unwrap().id();
    assert_eq!(
        dom.node(first).computed().map(|c| c.width.clone()),
        Some(crate::layout::Size::Fixed(20)),
        "the first div's style was written"
    );
    assert!(resize_restyles(&mut dom, &css), "its viewport read is kept");
}
