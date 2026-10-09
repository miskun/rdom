//! `@supports` (CSS Conditional 3 §6, 4 §6, 5 §5): the rules inside apply
//! when rdom supports what the condition asks.

use super::*;

/// `#a`'s color with `@supports <q> { #a { color: red } }` over blue.
fn color_under(q: &str) -> Color {
    let css = format!("#a {{ color: blue }} @supports {q} {{ #a {{ color: red }} }}");
    let mut dom = doc(r#"<div id="a">a</div>"#);
    styled(&mut dom, &css, 10, 2);
    fg(&dom, "a")
}

/// Conditional 3 §6.1: a declaration rdom parses — the real value
/// parser — holds; one it does not, fails; `not` / `and` / `or` combine.
#[test]
fn supports_tests_declarations_against_the_parser() {
    assert_eq!(color_under("(display: grid)"), RED);
    assert_eq!(color_under("(display: frobnicate)"), BLUE);
    assert_eq!(color_under("not (display: frobnicate)"), RED);
    assert_eq!(color_under("(display: grid) and (width: 10px)"), BLUE);
    assert_eq!(color_under("(display: frob) or (gap: 1)"), RED);
}

/// Conditional 4 §6.1 / 5 §5: `selector()`, the font functions, and a
/// `<general-enclosed>`, false — so its `not` holds (Conditional 3 §6.1,
/// C14G-CONDITIONAL-SPEC; it was unknown).
#[test]
fn supports_functions() {
    assert_eq!(color_under("selector(:has(+ p))"), RED);
    assert_eq!(color_under("selector(:frob)"), BLUE);
    assert_eq!(color_under("font-tech(variations)"), BLUE);
    assert_eq!(color_under("not font-format(woff2)"), RED);
    assert_eq!(color_under("frob(1)"), BLUE);
    assert_eq!(color_under("not frob(1)"), RED);
}

/// Nested in a style rule, with `@media` around and inside.
#[test]
fn supports_nests_with_media() {
    let css =
        "#a { color: blue; @supports (display: grid) { @media (width < 20) { color: red } } }";
    let mut dom = doc(r#"<div id="a">a</div>"#);
    styled(&mut dom, css, 10, 2);
    assert_eq!(fg(&dom, "a"), RED);
    styled(&mut dom, css, 30, 2);
    assert_eq!(fg(&dom, "a"), BLUE);
}
