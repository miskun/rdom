//! CSS-COMPLETE Phase 3 review-gate fixes (`C3G-*`), end to end: a
//! sheet parsed by `rdom_css`, cascaded, laid out and painted by
//! `rdom-tui`. One section per item; each test cites the spec text that
//! fixes the expected value.

use rdom_tui::style::cascade::computed_of;
use rdom_tui::{CascadeExt, Color, NodeId, TuiDom};

fn el(dom: &mut TuiDom, parent: NodeId, class: &str) -> NodeId {
    let id = dom.create_element("div");
    if !class.is_empty() {
        dom.set_attribute(id, "class", class).unwrap();
    }
    dom.append_child(parent, id).unwrap();
    id
}

/// Cascade `css` (strict: no warning allowed).
fn cascade(dom: &mut TuiDom, css: &str) {
    let sheet = rdom_css::from_css_strict(css).expect("sheet parses without warnings");
    dom.cascade(&sheet);
}

// ── C3G-COLOR-DEPTH ──────────────────────────────────────────────────

/// Attribute data reaches the color parser through `attr()` (CSS Values
/// 5 §8.7, `type(<color>)`). A hostile attribute nesting 10 000
/// `color-mix(` levels is an invalid `<color>`, so the fallback applies;
/// it used to exhaust the stack and abort the process.
#[test]
fn hostile_attr_color_is_invalid_not_a_stack_overflow() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let a = el(&mut dom, root, "a");
    let n = 10_000;
    let deep = format!(
        "{}red{}",
        "color-mix(in srgb, ".repeat(n),
        ", red)".repeat(n)
    );
    dom.set_attribute(a, "data-c", &deep).unwrap();
    cascade(
        &mut dom,
        ".a { color: attr(data-c type(<color>), rgb(0 0 255)) }",
    );
    assert_eq!(computed_of(&dom, a).fg, Color::Rgb(0, 0, 255), "fallback");
}

// ── C3G-RELATIVE-COMMA ───────────────────────────────────────────────

/// CSS Color 5 §4: a relative color's channel keywords work inside math
/// functions, whose arguments are comma-separated (Values 4 §10.2) —
/// with the origin from `var()`, substituted before the color parses.
#[test]
fn relative_color_with_a_comma_math_function_from_var() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let a = el(&mut dom, root, "a");
    let b = el(&mut dom, root, "b");
    cascade(
        &mut dom,
        ":root { --c: rgb(200 10 20) } \
         .a { color: rgb(from var(--c) min(r, 100) g b) } \
         .b { color: oklch(from var(--c) clamp(0.2, l, 0.5) c h) }",
    );
    assert_eq!(computed_of(&dom, a).fg, Color::Rgb(100, 10, 20));
    let expected = rdom_tui::parse_color("oklch(from rgb(200 10 20) 0.5 c h)").expect("a color");
    assert_eq!(computed_of(&dom, b).fg, expected);
}
