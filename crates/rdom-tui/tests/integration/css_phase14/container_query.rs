//! `@container` and the container-relative units (CSS Conditional 5
//! §6.4–§6.6): a rule applies while its query holds against the nearest
//! matching query container's content box — known only after layout, so
//! the cascade and layout interleave (`styled` runs `cascade` then
//! `layout_dom`, which re-cascades a container's subtree once its size is
//! known).

use super::*;

const CARD: &str = r#"<div id="outer"><div id="card"><p id="t">title</p></div></div>"#;

/// §6.4: a size query against the nearest `inline-size` container's
/// content-box width, in cells.
#[test]
fn a_size_query_reads_the_container_width() {
    let css = "#card { container-type: inline-size; padding: 0 1; box-sizing: border-box }
               #t { color: blue }
               @container (width > 20) { #t { color: red } }";
    let mut dom = doc(CARD);
    styled(&mut dom, &format!("{css} #card {{ width: 25 }}"), 40, 5);
    assert_eq!(fg(&dom, "t"), RED, "content box 23 > 20");
    let mut dom = doc(CARD);
    styled(&mut dom, &format!("{css} #card {{ width: 22 }}"), 40, 5);
    assert_eq!(fg(&dom, "t"), BLUE, "content box 20");
}

/// C14G-PX-BREAKPOINTS: a container feature maps `px` as a media feature
/// does — 8px a column — so `(width > 160px)` is 20 columns.
#[test]
fn a_pixel_container_query_maps_eight_pixels_a_column() {
    let css = "#card { container-type: inline-size }
               #t { color: blue }
               @container (width > 160px) { #t { color: red } }";
    let mut dom = doc(CARD);
    styled(&mut dom, &format!("{css} #card {{ width: 21 }}"), 40, 5);
    assert_eq!(fg(&dom, "t"), RED, "21 > 20");
    let mut dom = doc(CARD);
    styled(&mut dom, &format!("{css} #card {{ width: 20 }}"), 40, 5);
    assert_eq!(fg(&dom, "t"), BLUE, "20");
}

/// §6.4: the container is the nearest ancestor of the right type — a
/// `normal` element is skipped for a size query — and a name selects
/// among them.
#[test]
fn the_nearest_container_of_the_name_and_type_answers() {
    let css = "#outer { container: side / inline-size; width: 30 }
               #card { container-type: inline-size; width: 10 }
               #t { color: blue }
               @container side (width >= 30) { #t { color: red } }";
    let mut dom = doc(CARD);
    styled(&mut dom, css, 40, 5);
    assert_eq!(fg(&dom, "t"), RED, "named: the outer one");
    let css = "#outer { container-type: inline-size; width: 30 }
               #card { width: 10 }
               #t { color: blue }
               @container (width >= 30) { #t { color: red } }";
    let mut dom = doc(CARD);
    styled(&mut dom, css, 40, 5);
    assert_eq!(fg(&dom, "t"), RED, "#card is no size container");
}

/// §6.5: an `inline-size` container answers no block-axis query — unknown,
/// so neither it nor its `not` matches; `size` answers both, and
/// `aspect-ratio` / `orientation` read both axes.
#[test]
fn an_inline_size_container_answers_no_height_query() {
    let rule = |q: &str, ty: &str| {
        format!(
            "#card {{ container-type: {ty}; width: 20; height: 4 }} #t {{ color: blue }}
             @container {q} {{ #t {{ color: red }} }}"
        )
    };
    let color = |css: &str| {
        let mut dom = doc(CARD);
        styled(&mut dom, css, 40, 10);
        fg(&dom, "t")
    };
    assert_eq!(color(&rule("(height > 2)", "inline-size")), BLUE);
    assert_eq!(color(&rule("not (height > 2)", "inline-size")), BLUE);
    assert_eq!(color(&rule("(height > 2)", "size")), RED);
    assert_eq!(
        color(&rule(
            "(orientation: landscape) and (aspect-ratio: 5)",
            "size"
        )),
        RED
    );
    assert_eq!(
        color(&rule("(inline-size: 20) and (block-size: 4)", "size")),
        RED
    );
}

/// Nested containers resolve from the outside in: the inner one's width
/// depends on a rule the outer one's query applies.
#[test]
fn nested_containers_converge() {
    let css = "#outer { container-type: inline-size; width: 30 }
               #card { container-type: inline-size; width: 5 }
               @container (width >= 30) { #card { width: 20 } }
               #t { color: blue }
               @container (width >= 20) { #t { color: red } }";
    let mut dom = doc(CARD);
    styled(&mut dom, css, 40, 5);
    assert_eq!(
        dom.node(by_id(&dom, "card")).layout_rect().unwrap().width,
        20
    );
    assert_eq!(fg(&dom, "t"), RED);
}

/// §6.6: `cqw` / `cqi` are 1% of the nearest size container's content
/// width, `cqh` / `cqb` of a `size` container's height; with none, the
/// small viewport (here the terminal).
#[test]
fn container_units_resolve_against_the_container() {
    let mut dom = doc(CARD);
    styled(
        &mut dom,
        "#card { container-type: inline-size; width: 40 } #t { width: 50cqw }",
        60,
        5,
    );
    assert_eq!(dom.node(by_id(&dom, "t")).layout_rect().unwrap().width, 20);
    let mut dom = doc(CARD);
    styled(&mut dom, "#t { width: 50cqi }", 60, 5);
    assert_eq!(
        dom.node(by_id(&dom, "t")).layout_rect().unwrap().width,
        30,
        "no container: the viewport"
    );
    let mut dom = doc(CARD);
    styled(
        &mut dom,
        "#card { container-type: size; width: 40; height: 8 } #t { height: 50cqh; width: calc(10cqmin + 1) }",
        60,
        20,
    );
    let r = dom.node(by_id(&dom, "t")).layout_rect().unwrap();
    assert_eq!((r.width, r.height), (2, 4));
}

/// §6.4 style queries: every element is a style container; a custom
/// property's computed value is compared.
#[test]
fn a_style_query_reads_a_custom_property() {
    let css = "#card { --theme: dark } #t { color: blue }
               @container style(--theme: dark) { #t { color: red } }";
    let mut dom = doc(CARD);
    styled(&mut dom, css, 40, 5);
    assert_eq!(fg(&dom, "t"), RED);
    let css = "#card { --theme: light } #t { color: blue }
               @container style(--theme: dark) { #t { color: red } }";
    let mut dom = doc(CARD);
    styled(&mut dom, css, 40, 5);
    assert_eq!(fg(&dom, "t"), BLUE);
    // A standard property in `style()` is not supported: unknown.
    let css = "#card { color: green } #t { color: blue }
               @container not style(color: green) { #t { color: red } }";
    let mut dom = doc(CARD);
    styled(&mut dom, css, 40, 5);
    assert_eq!(fg(&dom, "t"), BLUE);
}

/// CSS Nesting 1 §3.2 and Conditional 3: `@container` nests in a style
/// rule and in `@media`.
#[test]
fn container_nests() {
    let css = "#card { container-type: inline-size; width: 25 }
               #t { color: blue; @container (width > 20) { color: red } }";
    let mut dom = doc(CARD);
    styled(&mut dom, css, 40, 5);
    assert_eq!(fg(&dom, "t"), RED);
    let css = "#card { container-type: inline-size; width: 25 } #t { color: blue }
               @media (width > 30) { @container (width > 20) { #t { color: red } } }";
    let mut dom = doc(CARD);
    styled(&mut dom, css, 40, 5);
    assert_eq!(fg(&dom, "t"), RED);
    let mut dom = doc(CARD);
    styled(&mut dom, css, 30, 5);
    assert_eq!(fg(&dom, "t"), BLUE);
}

/// A resize that changes a container's size re-evaluates its queries at
/// the next layout.
#[test]
fn a_container_resize_requeries() {
    let css = "#card { container-type: inline-size; width: 50% } #t { color: blue }
               @container (width >= 30) { #t { color: red } }";
    let mut dom = doc(CARD);
    styled(&mut dom, css, 40, 5);
    assert_eq!(fg(&dom, "t"), BLUE);
    dom.set_viewport(Viewport::new(60, 5));
    dom.layout_dom(Rect::new(0, 0, 60, 5));
    assert_eq!(fg(&dom, "t"), RED, "the card is 30 wide now");
}

/// `scroll-state()` queries are not evaluated (DIVERGENCES §2): unknown.
#[test]
fn a_scroll_state_query_is_unknown() {
    let css = "#card { container-type: scroll-state } #t { color: blue }
               @container scroll-state(scrollable: top) { #t { color: red } }
               @container not scroll-state(scrollable: top) { #t { color: red } }";
    let mut dom = doc(CARD);
    styled(&mut dom, css, 40, 5);
    assert_eq!(fg(&dom, "t"), BLUE);
}
