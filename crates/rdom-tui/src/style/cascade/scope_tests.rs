//! `@scope` through the cascade (CSS Cascade 6 §2.5): scoping roots and
//! limits, scoped style rules (relative to `:scope`, `&` as
//! `:where(:scope)`), declarations directly in `@scope`, prelude-less
//! `@scope` (the owner node's parent), nested `@scope`, and scope
//! proximity in the cascade sort.

use super::*;
use crate::TuiDom;
use crate::style::{Color, Stylesheet};
use rdom_core::NodeId;

const RED: Color = Color::Rgb(255, 0, 0);
const BLUE: Color = Color::Rgb(0, 0, 255);

/// The test tree:
///
/// ```html
/// <div class="card" id="c1">
///   <p class="a"></p>
///   <div class="content"><p class="b"></p></div>
///   <div class="card" id="c2"><p class="c"></p></div>
/// </div>
/// <p class="out"></p>
/// ```
struct Tree {
    dom: TuiDom,
    c1: NodeId,
    content: NodeId,
    c2: NodeId,
    a: NodeId,
    b: NodeId,
    c: NodeId,
    out: NodeId,
}

fn el(dom: &mut TuiDom, parent: NodeId, tag: &str, attrs: &[(&str, &str)]) -> NodeId {
    let id = dom.create_element(tag);
    for (k, v) in attrs {
        dom.set_attribute(id, k, v).unwrap();
    }
    dom.append_child(parent, id).unwrap();
    id
}

fn tree() -> Tree {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let c1 = el(&mut dom, root, "div", &[("class", "card"), ("id", "c1")]);
    let a = el(&mut dom, c1, "p", &[("class", "a")]);
    let content = el(&mut dom, c1, "div", &[("class", "content")]);
    let b = el(&mut dom, content, "p", &[("class", "b")]);
    let c2 = el(&mut dom, c1, "div", &[("class", "card"), ("id", "c2")]);
    let c = el(&mut dom, c2, "p", &[("class", "c")]);
    let out = el(&mut dom, root, "p", &[("class", "out")]);
    Tree {
        dom,
        c1,
        content,
        c2,
        a,
        b,
        c,
        out,
    }
}

/// A UA-less sheet parsed from `css`, which must parse cleanly.
fn sheet(css: &str) -> Stylesheet {
    let parsed = rdom_css::parse(css);
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    parsed.stylesheet
}

fn cascade(css: &str) -> Tree {
    let mut t = tree();
    t.dom.cascade(&sheet(css));
    t
}

fn fg(t: &Tree, id: NodeId) -> Color {
    computed_of(&t.dom, id).fg
}

fn initial() -> Color {
    crate::style::ComputedStyle::initial().fg
}

// ── Roots, limits, relative selectors (§2.5.1, §2.5.2) ───────────────

/// §2.5.2: a scoped style rule matches only elements in scope — the
/// inclusive descendants of a scoping root; its selector is relative
/// to `:scope` with a descendant combinator.
#[test]
fn scoped_rules_match_inside_the_root_only() {
    let t = cascade("@scope (.card) { p { color: red } }");
    assert_eq!(fg(&t, t.a), RED);
    assert_eq!(fg(&t, t.b), RED);
    assert_eq!(fg(&t, t.out), initial());
}

/// §2.5.1: an element inside a scoping limit (`to (…)`) — or the limit
/// itself — is out of scope.
#[test]
fn the_lower_boundary_excludes_its_subtree() {
    let t = cascade("@scope (.card) to (.content) { p { color: red } div { color: blue } }");
    assert_eq!(fg(&t, t.a), RED);
    assert_eq!(fg(&t, t.b), initial(), "inside the limit");
    // `.content` matches no `.card` root's scope, so it keeps the
    // initial color… but `#c2` is in scope of `#c1`.
    assert_eq!(fg(&t, t.content), initial());
    assert_eq!(fg(&t, t.c2), BLUE);
    // `#c2` is a root too, and `p.c` is in its scope.
    assert_eq!(fg(&t, t.c), RED);
}

/// §2.5.2: `:scope` is the scoping root; a leading combinator is
/// relative to it.
#[test]
fn scope_pseudo_class_and_relative_selectors() {
    let t = cascade("@scope (.card) { :scope { color: red } }");
    assert_eq!(fg(&t, t.c1), RED);
    assert_eq!(fg(&t, t.c2), RED);
    assert_eq!(fg(&t, t.content), RED, "inherited");
    let t = cascade("@scope (#c1) { > p { color: red } :scope > div > p { color: blue } }");
    assert_eq!(fg(&t, t.a), RED);
    assert_eq!(fg(&t, t.b), BLUE);
    assert_eq!(fg(&t, t.c), BLUE);
}

/// §2.5.2: `&` in a scoped rule is `:where(:scope)` — the root, with no
/// specificity — so `& > p` is `(0, 0, 1)` and loses to `.card p`.
/// `C1G-SCOPE-AMP-SPEC`, checked against the current Editor's Drafts:
/// Cascade 6 (scoped style rules) — "The `&` selector is defined to
/// behave as `:where(:scope)`", and ":scope has a specificity of
/// (0,1,0), whereas & has a specificity of 0"; CSS Nesting 1 §3.3.1 —
/// "The `&` selector behaves like `:where(:scope)` in @scope rules". It
/// does *not* take the `<scope-start>`'s specificity (`:is(#c1)` would
/// be `(1, 0, 1)` and beat `.card p` and `div > p`).
#[test]
fn ampersand_is_where_scope() {
    let t = cascade("@scope (#c1) { & > p { color: red } }");
    assert_eq!(fg(&t, t.a), RED);
    assert_eq!(fg(&t, t.c), initial());
    let t = cascade("@scope (#c1) { & > p { color: red } } .card p { color: blue }");
    assert_eq!(fg(&t, t.a), BLUE);
    // `div > p` is (0, 0, 2): above `& > p`'s (0, 0, 1) even though the
    // scope's start is an id.
    let t = cascade("div > p { color: blue } @scope (#c1) { & > p { color: red } }");
    assert_eq!(fg(&t, t.a), BLUE);
}

/// §2.5.2: declarations directly in `@scope` apply to the scoping root
/// with zero specificity (`:where(:scope)`).
#[test]
fn declarations_in_scope_apply_to_the_root() {
    let t = cascade("@scope (#c2) { color: red }");
    assert_eq!(fg(&t, t.c2), RED);
    assert_eq!(fg(&t, t.c1), initial());
    let t = cascade("@scope (#c2) { color: red } div { color: blue }");
    assert_eq!(fg(&t, t.c2), BLUE, "zero specificity loses to `div`");
}

/// Outside `@scope`, `:scope` is `:root` (Selectors 4 §14.3).
#[test]
fn scope_outside_scope_is_root() {
    let scope = cascade(":scope { color: red }");
    let root = cascade(":root { color: red }");
    for id in [scope.c1, scope.a, scope.out] {
        assert_eq!(fg(&scope, id), fg(&root, id));
    }
    // A sheet whose `&` has no parent rule: `&` is `:scope` too.
    let amp = cascade("& { color: red }");
    assert_eq!(fg(&amp, amp.c1), fg(&root, root.c1));
}

// ── Scope proximity (Cascade 6 §6.1) ─────────────────────────────────

/// Cascade 6 §6.1: after specificity, the declaration whose scoping
/// root is fewer generations from the subject wins — before order of
/// appearance; unscoped rules count as infinitely far.
#[test]
fn scope_proximity_sorts_between_specificity_and_order() {
    // `#c2`'s rule is closer to `p.c` than `#c1`'s, though earlier.
    let t = cascade("@scope (#c2) { p { color: red } } @scope (#c1) { p { color: blue } }");
    assert_eq!(fg(&t, t.c), RED);
    assert_eq!(fg(&t, t.a), BLUE);
    // A scoped rule beats a later unscoped one of equal specificity…
    let t = cascade("@scope (.card) { p { color: red } } p { color: blue }");
    assert_eq!(fg(&t, t.a), RED);
    // …but specificity decides first.
    let t = cascade("@scope (.card) { p { color: red } } p.a { color: blue }");
    assert_eq!(fg(&t, t.a), BLUE);
    // One `@scope` rule with nested roots: the nearest root counts.
    // `p.c` is in scope of both `.card` roots; `#c2` (1 hop) beats the
    // later rule's `#c1` (2 hops).
    let t = cascade("@scope (.card) { p { color: red } } @scope (#c1) { p { color: blue } }");
    assert_eq!(fg(&t, t.c), RED);
}

// ── Nesting (§2.5.3) ─────────────────────────────────────────────────

/// Cascade 6 §2.5: `@scope` nested in a style rule takes its
/// `<scope-start>` relative to the rule; nested in another `@scope`,
/// relative to that scope — and the inner root must be in the outer
/// scope.
#[test]
fn nested_scope_rules() {
    let t = cascade("#c1 { @scope (.content) { p { color: red } } }");
    assert_eq!(fg(&t, t.b), RED);
    assert_eq!(fg(&t, t.a), initial());
    let t = cascade("@scope (#c1) to (#c2) { @scope (.card) { p { color: red } } }");
    assert_eq!(fg(&t, t.c), initial(), "#c2 is outside #c1's scope");
    let t = cascade("@scope (#c1) { @scope (#c2) { p { color: red } } }");
    assert_eq!(fg(&t, t.c), RED);
    assert_eq!(fg(&t, t.a), initial());
}

/// Style rules nest inside scoped rules as usual.
#[test]
fn rules_nest_inside_scoped_rules() {
    let t = cascade("@scope (#c1) to (#c2) { .content { p { color: red } } }");
    assert_eq!(fg(&t, t.b), RED);
}

// ── Prelude-less `@scope` (§2.5.1) ───────────────────────────────────

/// §2.5.1: with no `<scope-start>`, the scoping root is the parent
/// element of the sheet's owner node (`Stylesheet::set_owner_node`, a
/// `<style>` element's sheet).
#[test]
fn prelude_less_scope_roots_at_the_owner_nodes_parent() {
    let mut t = tree();
    let style = t.dom.create_element("style");
    t.dom.append_child(t.c2, style).unwrap();
    let mut s = sheet("@scope { p { color: red } :scope { color: blue } }");
    s.set_owner_node(Some(style));
    t.dom.cascade(&s);
    assert_eq!(fg(&t, t.c), RED);
    assert_eq!(fg(&t, t.c2), BLUE);
    assert_eq!(fg(&t, t.a), initial());
    // With no owner node the root is the document: every element is in
    // scope, and `:scope` names no element.
    let t = cascade("@scope { p { color: red } }");
    assert_eq!(fg(&t, t.a), RED);
    assert_eq!(fg(&t, t.out), RED);
}
