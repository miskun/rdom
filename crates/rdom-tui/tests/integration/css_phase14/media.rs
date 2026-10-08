//! `@media` (Media Queries 4 §2–§3, CSS Conditional 3 §3): the rules
//! inside apply while the query list matches the terminal.

use super::*;

const MARKUP: &str = r#"<div id="a">a</div>"#;

/// `id="a"`'s color under `css` in a `w` × `h` viewport.
fn color_at(css: &str, w: u16, h: u16) -> Color {
    let mut dom = doc(MARKUP);
    styled(&mut dom, css, w, h);
    fg(&dom, "a")
}

/// Media Queries 4 §4.1, §3: `width` is the viewport's, in cells — a
/// range and the `min-` spelling alike; the rule applies only while the
/// query matches.
#[test]
fn width_queries_the_viewport_in_cells() {
    let css = "#a { color: blue } @media (width >= 40) { #a { color: red } }";
    assert_eq!(color_at(css, 40, 10), RED);
    assert_eq!(color_at(css, 39, 10), BLUE);
    let css = "#a { color: blue } @media (min-width: 40) { #a { color: red } }";
    assert_eq!(color_at(css, 40, 10), RED);
    assert_eq!(color_at(css, 39, 10), BLUE);
    // §3: `<value> <op> <name> <op> <value>`, and `ch` is one column.
    let css = "#a { color: blue } @media (20ch < width <= 40) { #a { color: red } }";
    assert_eq!(color_at(css, 40, 10), RED);
    assert_eq!(color_at(css, 20, 10), BLUE);
    assert_eq!(color_at(css, 41, 10), BLUE);
}

/// §4.2–§4.4: `height`, `aspect-ratio` (columns over rows) and
/// `orientation` read the same cells.
#[test]
fn height_aspect_ratio_and_orientation_read_the_viewport() {
    let css = "#a { color: blue } @media (max-height: 10) { #a { color: red } }";
    assert_eq!(color_at(css, 80, 10), RED);
    assert_eq!(color_at(css, 80, 11), BLUE);
    let css = "#a { color: blue } @media (min-aspect-ratio: 2/1) { #a { color: red } }";
    assert_eq!(color_at(css, 80, 40), RED);
    assert_eq!(color_at(css, 79, 40), BLUE);
    let css = "#a { color: blue } @media (orientation: portrait) { #a { color: red } }";
    assert_eq!(color_at(css, 20, 30), RED);
    assert_eq!(color_at(css, 30, 30), BLUE);
}

/// §2.3: a terminal is `screen` (and `all`); `print` and any other type
/// never match, `not` negates the query.
#[test]
fn media_types() {
    let rule = |q: &str| format!("#a {{ color: blue }} @media {q} {{ #a {{ color: red }} }}");
    assert_eq!(color_at(&rule("screen"), 10, 5), RED);
    assert_eq!(color_at(&rule("all"), 10, 5), RED);
    assert_eq!(color_at(&rule("only screen"), 10, 5), RED);
    assert_eq!(color_at(&rule("print"), 10, 5), BLUE);
    assert_eq!(color_at(&rule("tv"), 10, 5), BLUE);
    assert_eq!(color_at(&rule("not print"), 10, 5), RED);
    assert_eq!(color_at(&rule("screen and (width < 5)"), 10, 5), BLUE);
    assert_eq!(color_at(&rule("not screen and (width < 5)"), 10, 5), RED);
}

/// §2.1: a list matches when any query does; §3.2: a query that does not
/// parse is `not all`, and the others stand.
#[test]
fn a_list_matches_when_any_query_does() {
    let css = "#a { color: blue } @media print, (width < 20) { #a { color: red } }";
    assert_eq!(color_at(css, 10, 5), RED);
    assert_eq!(color_at(css, 30, 5), BLUE);
    let css = "#a { color: blue } @media (width <> 3), screen { #a { color: red } }";
    assert_eq!(color_at(css, 10, 5), RED);
}

/// §2.5, §3.1: `and` / `or` / `not` combine features in three-valued
/// logic — a `<general-enclosed>` or an unknown feature is unknown, so
/// neither it nor its `not` matches, and `or` with a true side holds.
#[test]
fn conditions_combine_in_three_valued_logic() {
    let rule = |q: &str| format!("#a {{ color: blue }} @media {q} {{ #a {{ color: red }} }}");
    assert_eq!(color_at(&rule("(width > 5) and (height > 2)"), 10, 5), RED);
    assert_eq!(color_at(&rule("(width > 50) or (height > 2)"), 10, 5), RED);
    assert_eq!(color_at(&rule("not (width > 50)"), 10, 5), RED);
    assert_eq!(color_at(&rule("(frobnicate)"), 10, 5), BLUE);
    assert_eq!(color_at(&rule("not (frobnicate)"), 10, 5), BLUE);
    assert_eq!(color_at(&rule("not (frobnicate: 2)"), 10, 5), BLUE);
    assert_eq!(color_at(&rule("(frobnicate) or (width > 5)"), 10, 5), RED);
    assert_eq!(color_at(&rule("not (fn(x) and (width > 5))"), 10, 5), BLUE);
}

/// DESIGN "Pixel lengths select, cells measure": a pixel length has no
/// cell measure, so `(min-width: 600px)` is unknown — neither it nor its
/// negation matches.
#[test]
fn a_pixel_length_is_unknown() {
    let rule = |q: &str| format!("#a {{ color: blue }} @media {q} {{ #a {{ color: red }} }}");
    assert_eq!(color_at(&rule("(min-width: 600px)"), 100, 5), BLUE);
    assert_eq!(color_at(&rule("not (min-width: 600px)"), 100, 5), BLUE);
    assert_eq!(color_at(&rule("(max-width: 40em)"), 10, 5), BLUE);
}

/// The terminal mapping (DIVERGENCES §2): a grid device, a mouse that
/// hovers with a fine pointer, 24-bit color, fast updates, scripting.
#[test]
fn the_terminal_answers_the_device_features() {
    let rule = |q: &str| format!("#a {{ color: blue }} @media {q} {{ #a {{ color: red }} }}");
    for yes in [
        "(grid)",
        "(hover: hover)",
        "(any-hover: hover)",
        "(pointer: fine)",
        "(any-pointer: fine)",
        "(color)",
        "(color >= 8)",
        "(update: fast)",
        "(scripting: enabled)",
        "(prefers-reduced-motion: no-preference)",
        "(prefers-contrast: no-preference)",
        "(forced-colors: none)",
        "(inverted-colors: none)",
        "(dynamic-range: standard)",
        "(color-gamut: srgb)",
        "(display-mode: standalone)",
        "(prefers-color-scheme: dark)",
    ] {
        assert_eq!(color_at(&rule(yes), 10, 5), RED, "{yes}");
    }
    for no in [
        "(pointer: coarse)",
        "(monochrome)",
        "(color-index)",
        "(resolution >= 1dppx)",
        "(prefers-reduced-motion)",
        "(forced-colors)",
        "(prefers-color-scheme: light)",
        "(color-gamut: p3)",
        "(hover: maybe)",
    ] {
        assert_eq!(color_at(&rule(no), 10, 5), BLUE, "{no}");
    }
}

/// Media Queries 5 §12.5: `prefers-color-scheme` reads the document's
/// preferred scheme — the one `light-dark()` uses.
#[test]
fn prefers_color_scheme_reads_the_document_scheme() {
    let css = "#a { color: blue } @media (prefers-color-scheme: light) { #a { color: red } }";
    let mut dom = doc(MARKUP);
    dom.set_color_scheme(rdom_tui::ColorScheme::Light);
    styled(&mut dom, css, 10, 5);
    assert_eq!(fg(&dom, "a"), RED);
}

/// CSS Conditional 3 §3, CSS Cascade 5 §6.4: `@media` inside `@layer`
/// keeps the layer, and `@layer` inside `@media` too.
#[test]
fn media_and_layers_nest() {
    let css = "@layer base { @media (width > 5) { #a { color: red } } }
               #a { color: blue }";
    assert_eq!(color_at(css, 10, 5), BLUE, "unlayered beats the layer");
    let css = "@media (width > 5) { @layer base { #a { color: red } } }
               @layer base { #a { color: blue } }";
    assert_eq!(color_at(css, 10, 5), BLUE, "the later rule of one layer");
    let css = "@media (width > 50) { @layer base { #a { color: red } } }
               #a { color: green }";
    assert_eq!(color_at(css, 10, 5), GREEN);
}

/// CSS Nesting 1 §3.2: `@media` nested in a style rule holds the parent's
/// declarations (and nested rules) under the condition.
#[test]
fn media_nests_in_a_style_rule() {
    let css = "#a { color: blue; @media (width < 20) { color: red; } }";
    assert_eq!(color_at(css, 10, 5), RED);
    assert_eq!(color_at(css, 30, 5), BLUE);
    let css = "div { @media (width < 20) { &#a { color: red } } }";
    assert_eq!(color_at(css, 10, 5), RED);
    // Conditional rules nest in each other: both must match.
    let css =
        "#a { color: blue } @media (width < 20) { @media (height > 3) { #a { color: red } } }";
    assert_eq!(color_at(css, 10, 5), RED);
    assert_eq!(color_at(css, 10, 2), BLUE);
}

/// CSS Cascade 5 §3: `@import url media` imports the sheet's rules under
/// the media query.
#[test]
fn an_import_media_list_conditions_its_rules() {
    let loader = |url: &str| -> Result<String, String> {
        assert_eq!(url, "narrow.css");
        Ok("#a { color: red }".to_string())
    };
    let css = "@import url(narrow.css) (width < 20);";
    let parsed = rdom_css::parse_with_loader(css, &loader);
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    let mut dom = doc(MARKUP);
    dom.set_viewport(Viewport::new(10, 5));
    let sheet = parsed.stylesheet;
    let sheet2 = rdom_css::parse("#a { color: blue }").stylesheet;
    dom.cascade_all(&[&sheet2, &sheet]);
    assert_eq!(
        fg(&dom, "a"),
        RED,
        "imported rule applies in a narrow viewport"
    );
    dom.set_viewport(Viewport::new(30, 5));
    dom.cascade_all(&[&sheet2, &sheet]);
    assert_eq!(fg(&dom, "a"), BLUE, "and not in a wide one");
}
