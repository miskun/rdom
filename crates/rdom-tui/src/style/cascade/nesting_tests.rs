//! CSS Nesting 1 through the cascade: nested style rules, `&`, the
//! implicit descendant combinator, relative selectors, interleaved
//! declarations, nested `@layer`, and the specificity of a nested
//! rule (`&` counts as `:is(<parent>)`).

use super::*;
use crate::TuiDom;
use crate::style::{Color, Stylesheet};
use rdom_core::NodeId;

const RED: Color = Color::Rgb(255, 0, 0);
const GREEN: Color = Color::Rgb(0, 128, 0);
const BLUE: Color = Color::Rgb(0, 0, 255);

/// The test tree:
///
/// ```html
/// <section class="wrap">
///   <div id="c" class="card">
///     <p class="a"></p><span></span><p class="b"></p>
///   </div>
/// </section>
/// ```
struct Tree {
    dom: TuiDom,
    wrap: NodeId,
    card: NodeId,
    a: NodeId,
    span: NodeId,
    b: NodeId,
}

fn tree() -> Tree {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let wrap = dom.create_element("section");
    dom.set_attribute(wrap, "class", "wrap").unwrap();
    let card = dom.create_element("div");
    dom.set_attribute(card, "id", "c").unwrap();
    dom.set_attribute(card, "class", "card").unwrap();
    let a = dom.create_element("p");
    dom.set_attribute(a, "class", "a").unwrap();
    let span = dom.create_element("span");
    let b = dom.create_element("p");
    dom.set_attribute(b, "class", "b").unwrap();
    dom.append_child(root, wrap).unwrap();
    dom.append_child(wrap, card).unwrap();
    for child in [a, span, b] {
        dom.append_child(card, child).unwrap();
    }
    Tree {
        dom,
        wrap,
        card,
        a,
        span,
        b,
    }
}

/// A UA-less sheet parsed from `css`, which must parse cleanly.
fn sheet(css: &str) -> Stylesheet {
    let parsed = rdom_css::parse(css);
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    parsed.stylesheet
}

/// The test tree cascaded against `css`.
fn cascade(css: &str) -> Tree {
    let mut t = tree();
    t.dom.cascade(&sheet(css));
    t
}

/// A node's computed `color`.
fn fg(t: &Tree, id: NodeId) -> Color {
    computed_of(&t.dom, id).fg
}

fn initial() -> Color {
    crate::style::ComputedStyle::initial().fg
}

// ── `&` and the implicit descendant combinator (§2, §3.1) ────────────

/// Nesting 1 §2: a nested style rule's `&` stands for the parent
/// rule's elements; `& p` is a `p` inside them.
#[test]
fn ampersand_descendant() {
    let t = cascade(".card { & p { color: red } }");
    assert_eq!(fg(&t, t.a), RED);
    assert_eq!(fg(&t, t.b), RED);
    assert_eq!(fg(&t, t.card), initial());
}

/// §2: a nested selector without `&` that does not start with a
/// combinator is relative to the parent with a descendant combinator.
#[test]
fn implicit_descendant_combinator() {
    let t = cascade(".card { p { color: red } }");
    assert_eq!(fg(&t, t.a), RED);
    assert_eq!(fg(&t, t.span), initial());
    // A nested rule that starts with an ident and a colon is a rule,
    // not a declaration (CSS Syntax 3 "consume a block's contents").
    let t = cascade(".card { p:first-child { color: red } }");
    assert_eq!(fg(&t, t.a), RED);
    assert_eq!(fg(&t, t.b), initial());
}

/// §2: `&` combines into a compound (`&.x`), can follow other
/// compounds (`.wrap &`) and sit in the middle of a selector.
#[test]
fn ampersand_in_any_position() {
    let t = cascade(".card { &#c { color: red } }");
    assert_eq!(fg(&t, t.card), RED);
    let t = cascade("div { .wrap & { color: red } }");
    assert_eq!(fg(&t, t.card), RED);
    let t = cascade("div { .nope & { color: red } }");
    assert_eq!(fg(&t, t.card), initial());
    let t = cascade(".a { .wrap & + span { color: red } }");
    assert_eq!(fg(&t, t.span), RED);
    // `&` alone is the parent's elements themselves.
    let t = cascade(".card { & { color: red } }");
    assert_eq!(fg(&t, t.card), RED);
    assert_eq!(fg(&t, t.wrap), initial());
}

/// §2: `&` may appear inside a pseudo-class argument; the selector
/// then contains `&` and is not made relative.
#[test]
fn ampersand_inside_not() {
    let t = cascade("p { color: blue; :not(&) { color: red } }");
    assert_eq!(fg(&t, t.span), RED);
    assert_eq!(fg(&t, t.card), RED);
    assert_eq!(fg(&t, t.a), BLUE, "a `p` is not `:not(p)`");
}

/// §2: a relative selector starting with a combinator is anchored at
/// `&`: `> p`, `+ span`, `~ p`.
#[test]
fn relative_selectors() {
    let t = cascade(".wrap { > div { color: red } > p { color: blue } }");
    assert_eq!(fg(&t, t.card), RED);
    assert_eq!(fg(&t, t.a), RED, "inherited from the card, not matched");
    let t = cascade(".a { + span { color: red } ~ p { color: blue } }");
    assert_eq!(fg(&t, t.span), RED);
    assert_eq!(fg(&t, t.b), BLUE);
    assert_eq!(fg(&t, t.a), initial());
}

/// Nesting goes any number of levels deep.
#[test]
fn nesting_is_recursive() {
    let t = cascade(".wrap { .card { > .b { color: red } } }");
    assert_eq!(fg(&t, t.b), RED);
    assert_eq!(fg(&t, t.a), initial());
}

/// §2: a nested pseudo-element rule (`&::before`) styles the parent's
/// pseudo-element.
#[test]
fn nested_pseudo_element() {
    let t = cascade(".card { &::before { content: \"x\" } }");
    let ext = t.dom.node(t.card).ext().unwrap();
    let before = ext.computed_before.as_ref().expect("::before");
    assert_eq!(before.content.as_deref(), Some("x"));
}

// ── Declarations and nested rules (§3.2) ─────────────────────────────

/// Declarations may follow nested rules; the parent keeps them.
#[test]
fn declarations_interleave_with_nested_rules() {
    let t = cascade(".card { color: red; p { color: blue } background: red; color: green }");
    assert_eq!(fg(&t, t.card), GREEN);
    assert_eq!(fg(&t, t.a), BLUE);
    assert_eq!(fg(&t, t.span), GREEN);
}

/// Nesting 1 §3.2 (`CSSNestedDeclarations`): declarations after a
/// nested rule form their own rule, after it in order of appearance,
/// with the parent's selector — so they beat an earlier `&` rule of
/// equal specificity.
#[test]
fn trailing_declarations_follow_the_nested_rule() {
    let t = cascade(".card { color: red; & { color: blue } color: green }");
    assert_eq!(fg(&t, t.card), GREEN);
    let t = cascade(".card { color: red; & { color: blue } }");
    assert_eq!(fg(&t, t.card), BLUE);
}

/// An invalid nested selector drops that nested rule only.
#[test]
fn invalid_nested_rule_is_dropped_alone() {
    let parsed = rdom_css::parse(".card { color: red; %% { color: blue } p { color: green } }");
    assert_eq!(parsed.warnings.len(), 1, "{:?}", parsed.warnings);
    let mut t = tree();
    t.dom.cascade(&parsed.stylesheet);
    assert_eq!(computed_of(&t.dom, t.card).fg, RED);
    assert_eq!(computed_of(&t.dom, t.a).fg, GREEN);
}

// ── Nested at-rules (§3.2) ───────────────────────────────────────────

/// Nesting 1 §3.2 with Cascade 5 §6.4: a nested `@layer` holds
/// declarations (for the parent's elements) and nested rules, in the
/// layer — so unlayered declarations beat them.
#[test]
fn nested_layer() {
    let t = cascade(".card { @layer base { color: red; p { color: red } } }");
    assert_eq!(fg(&t, t.card), RED);
    assert_eq!(fg(&t, t.span), RED);
    let t = cascade(
        ".card { @layer base { color: red; > p { color: red } } } \
         div { color: blue } p { color: blue }",
    );
    assert_eq!(fg(&t, t.card), BLUE);
    assert_eq!(fg(&t, t.a), BLUE);
    // The nested layer is the same layer as a top-level one of the
    // same name, ordered by first declaration.
    let t = cascade(
        "@layer base, top; @layer top { #c { color: green } } \
         .card { @layer base { #c#c { color: red } } }",
    );
    assert_eq!(fg(&t, t.card), GREEN);
}

// ── Specificity (§2: `&` is `:is(<parent>)`) ─────────────────────────

/// Nesting 1 §2: the nesting selector's specificity is that of
/// `:is()` over the parent's selector list — its most specific item,
/// whichever item matched. `#c, .card { & p {} }` is `(1, 0, 1)` for
/// every `p`, so it beats a later `(0, 2, 1)`.
#[test]
fn nesting_selector_has_is_specificity() {
    let t = cascade("#c, .card { & p { color: blue } } .card p.a { color: red }");
    assert_eq!(fg(&t, t.a), BLUE);
    // A single-item parent: `.card { p {} }` is `.card p`, `(0, 1, 1)`,
    // which loses to a later `.card p.a` `(0, 2, 1)`.
    let t = cascade(".card { p { color: blue } } .card p.a { color: red }");
    assert_eq!(fg(&t, t.a), RED);
    // …and, at equal specificity, beats an earlier rule by order.
    let t = cascade(".card p { color: red } .card { p { color: blue } }");
    assert_eq!(fg(&t, t.a), BLUE);
}
