//! CSS-COMPLETE Phase 2 review-gate fixes (`C2G-*`), end to end: a
//! sheet parsed by `rdom_css`, cascaded and laid out by `rdom-tui`.
//! One section per item; each test cites the spec text that fixes the
//! expected value.

use rdom_tui::render::Rect;
use rdom_tui::{CascadeExt, LayoutExt, LayoutRect, NodeId, TuiDom, TuiNodeExt, Viewport};

fn el(dom: &mut TuiDom, parent: NodeId, class: &str) -> NodeId {
    let id = dom.create_element("div");
    if !class.is_empty() {
        dom.set_attribute(id, "class", class).unwrap();
    }
    dom.append_child(parent, id).unwrap();
    id
}

/// Cascade `css` (strict: no warning allowed) and lay out at `cols` × `rows`.
fn lay_out(dom: &mut TuiDom, css: &str, cols: u16, rows: u16) {
    let sheet = rdom_css::from_css_strict(css).expect("sheet parses without warnings");
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, cols, rows));
}

fn rect(dom: &TuiDom, id: NodeId) -> LayoutRect {
    dom.node(id).layout_rect().expect("laid out")
}

// ── C2G-FLEX-SUM ─────────────────────────────────────────────────────

/// Lay out `items` (one class each) in a `width`-column flex row under
/// `css` and return their widths.
fn row_widths(width: u16, css: &str, items: &[&str]) -> Vec<u16> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let row = el(&mut dom, root, "row");
    let ids: Vec<NodeId> = items.iter().map(|c| el(&mut dom, row, c)).collect();
    lay_out(
        &mut dom,
        &format!(".row {{ display: flex; flex-direction: row; width: {width}; height: 1 }} {css}"),
        width,
        10,
    );
    ids.iter().map(|&id| rect(&dom, id).width).collect()
}

/// CSS Flexbox §9.7 step 4.b applies only when the unfrozen items'
/// flex factors sum to *less than one*. `0.1 + 0.2 + 0.7` is one: the
/// items share all 80 cells. (Parsed as `f32` and summed in `f64` the
/// sum is 0.99999999255, which used to trip 4.b and leave a cell free.)
#[test]
fn decimal_grow_factors_summing_to_one_fill_the_row() {
    let w = row_widths(
        80,
        ".a { flex: 0.1 } .b { flex: 0.2 } .c { flex: 0.7 }",
        &["a", "b", "c"],
    );
    assert_eq!(w, [8, 16, 56]);
    assert_eq!(w.iter().sum::<u16>(), 80);
}

/// §9.7 step 4.b still holds for a real deficit: factors summing to
/// 0.9 take 90% of the free space and leave the rest.
#[test]
fn decimal_grow_factors_summing_below_one_leave_space() {
    let w = row_widths(80, ".a { flex: 0.2 } .b { flex: 0.7 }", &["a", "b"]);
    assert_eq!(w, [16, 56], "0.9 of 80 = 72 cells shared, 8 left free");
}

/// §9.7 steps 4–5 with a fractional factor in the freeze loop: `.a`'s
/// share (8) violates its `max-width: 4`, so it is frozen at 4. The
/// remaining factors sum to 0.9 (step 4.b): `0.9 × 80 = 72` is less
/// than the 76 cells left, so `.b` and `.c` share 72 — 16 + 56.
#[test]
fn fractional_factor_with_max_width_in_the_freeze_loop() {
    let w = row_widths(
        80,
        ".a { flex: 0.1; max-width: 4 } .b { flex: 0.2 } .c { flex: 0.7 }",
        &["a", "b", "c"],
    );
    assert_eq!(w, [4, 16, 56]);
}

/// The shrink side of step 4.b: shrink factors `0.1 + 0.2 + 0.7` sum to
/// one, so all of the 40-cell overflow is taken and the row fits.
#[test]
fn decimal_shrink_factors_summing_to_one_absorb_the_overflow() {
    let w = row_widths(
        80,
        ".a { width: 40; flex-shrink: 0.1 } .b { width: 40; flex-shrink: 0.2 }
         .c { width: 40; flex-shrink: 0.7 }",
        &["a", "b", "c"],
    );
    assert_eq!(w.iter().sum::<u16>(), 80, "{w:?}");
}

// ── C2G-CALC-DEPTH ───────────────────────────────────────────────────

/// Attribute data reaches the math-function parser through `attr()`
/// (CSS Values 5 §8.7, `type(<length>)`). A hostile attribute nesting
/// 20 000 `calc(` levels — or chaining 30 000 terms — is an invalid
/// `<length>`, so the fallback applies; it used to exhaust the stack
/// and abort the process (with the terminal left in raw mode).
#[test]
fn hostile_attr_calc_is_invalid_not_a_stack_overflow() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let nested = el(&mut dom, root, "a");
    let chained = el(&mut dom, root, "a");
    let n = 20_000;
    let deep = format!("{}1{}", "calc(".repeat(n), ")".repeat(n));
    dom.set_attribute(nested, "data-w", &deep).unwrap();
    let flat = format!("calc({})", vec!["1"; 30_000].join(" + "));
    dom.set_attribute(chained, "data-w", &flat).unwrap();
    lay_out(
        &mut dom,
        ".a { width: attr(data-w type(<length>), 7); height: 1 }",
        40,
        5,
    );
    assert_eq!(rect(&dom, nested).width, 7, "fallback");
    assert_eq!(rect(&dom, chained).width, 7, "fallback");
}

// ── C2G-VIEWPORT-DOC ─────────────────────────────────────────────────

/// CSS Values 4 §6.1.2: viewport-percentage lengths are relative to the
/// initial containing block, the document's viewport. The viewport is
/// the document's (`set_viewport`), so every cascade form reads it: a
/// headless `cascade` resolves `50vw` of 80 to 40, and a later subtree
/// re-cascade (the DirtyTracker pattern) resolves against the same 80.
#[test]
fn every_cascade_form_reads_the_document_viewport() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let a = el(&mut dom, root, "a");
    let b = el(&mut dom, root, "b");
    let sheet =
        rdom_css::from_css_strict(".a, .b { width: 50vw; height: 1 } .x { height: 10vh }").unwrap();
    dom.set_viewport(Viewport::new(80, 20));
    assert_eq!(dom.viewport(), Viewport::new(80, 20));
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 20));
    assert_eq!(rect(&dom, a).width, 40);

    dom.set_attribute(b, "class", "b x").unwrap();
    dom.cascade_subtrees(&sheet, &[b]);
    dom.layout_dom(Rect::new(0, 0, 80, 20));
    assert_eq!(
        (rect(&dom, b).width, rect(&dom, b).height),
        (40, 2),
        "the re-cascaded subtree keeps the 80 × 20 viewport"
    );
}

/// `layout_dom(area)` presents the document in `area`: it records the
/// area as the viewport, so the next cascade resolves against it.
#[test]
fn layout_records_its_area_as_the_viewport() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let a = el(&mut dom, root, "a");
    let sheet = rdom_css::from_css_strict(".a { width: 50vw; height: 1 }").unwrap();
    assert_eq!(dom.viewport(), Viewport::default(), "0 × 0 until set");
    dom.layout_dom(Rect::new(0, 0, 60, 10));
    assert_eq!(dom.viewport(), Viewport::new(60, 10));
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 60, 10));
    assert_eq!(rect(&dom, a).width, 30);
}

// ── C2G-CALC-SEMANTICS ───────────────────────────────────────────────

/// CSS Values 4 §10.9: an infinite top-level result clamps to the
/// property's range — symmetric in rdom, so `right` (which points
/// inward and is negated) cannot overflow. `calc(-infinity)` and
/// `calc(1 / 0)` insets lay out without a panic; division by zero is
/// IEEE, so `min(10 / 0, 3)` is 3.
#[test]
fn infinite_insets_lay_out_without_overflow() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let cb = el(&mut dom, root, "cb");
    let a = el(&mut dom, cb, "a");
    let b = el(&mut dom, cb, "b");
    let c = el(&mut dom, cb, "c");
    lay_out(
        &mut dom,
        ".cb { position: relative; width: 40; height: 10 }
         .a { position: absolute; right: calc(-infinity); width: 1; height: 1 }
         .b { position: absolute; bottom: calc(-infinity + 50%); left: calc(1 / 0); width: 1; height: 1 }
         .c { position: absolute; left: min(10 / 0, 3); top: calc(0 / 0); width: 1; height: 1 }",
        80,
        20,
    );
    let _ = (rect(&dom, a), rect(&dom, b));
    assert_eq!((rect(&dom, c).x, rect(&dom, c).y), (3, 0));
}

// ── C2G-VIEWPORT-FIELDS ──────────────────────────────────────────────

/// CSS Values 4 §6.1.2: viewport-percentage lengths are absolute at
/// computed-value time. Every property the dispatch table knows is set
/// to `10vw` (`10vw 10vw` for the two-value ones); each one that takes
/// it must come out of the cascade with no viewport unit left anywhere
/// in its computed style, and lay out (layout's resolve asserts it meets
/// none). The list comes from the property table, so a length property
/// added later is covered without editing this test.
#[test]
fn every_length_property_resolves_viewport_units_in_the_cascade() {
    let mut accepted = Vec::new();
    for &name in rdom_style::property_dispatch::property_names() {
        for value in ["10vw", "10vw 10vw"] {
            let css = format!(".a {{ {name}: {value} }}");
            let Ok(sheet) = rdom_css::from_css_strict(&css) else {
                continue;
            };
            let mut dom = TuiDom::new();
            let root = dom.root();
            let cb = el(&mut dom, root, "cb");
            let a = el(&mut dom, cb, "a");
            let _child = el(&mut dom, a, "");
            dom.set_viewport(Viewport::new(80, 20));
            dom.cascade(&sheet);
            let computed = format!("{:?}", rdom_tui::style::cascade::computed_of(&dom, a));
            assert!(
                !computed.contains("Viewport("),
                "`{name}: {value}` left a viewport unit in the computed style: {computed}"
            );
            dom.layout_dom(Rect::new(0, 0, 80, 20));
            accepted.push(name);
            break;
        }
    }
    for name in [
        "width",
        "max-height",
        "padding",
        "margin-left",
        "gap",
        "inset",
        "top",
    ] {
        assert!(accepted.contains(&name), "{name} takes 10vw: {accepted:?}");
    }
}

// ── C2G-MAX-NONE ─────────────────────────────────────────────────────

/// CSS Sizing 3 §5.2: `max-width` / `max-height` take `none` — no limit,
/// the initial value — so a later rule lifts an earlier limit.
#[test]
fn max_size_none_lifts_a_limit() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let cb = el(&mut dom, root, "cb");
    let limited = el(&mut dom, cb, "a");
    let lifted = el(&mut dom, cb, "a b");
    lay_out(
        &mut dom,
        ".cb { width: 40; height: 20 }
         .a { width: 30; height: 5; max-width: 10; max-height: 2 }
         .a.b { max-width: none; max-height: NONE }",
        80,
        30,
    );
    assert_eq!(
        (rect(&dom, limited).width, rect(&dom, limited).height),
        (10, 2)
    );
    assert_eq!(
        (rect(&dom, lifted).width, rect(&dom, lifted).height),
        (30, 5)
    );
}

// ── C2G-FLEX-SHORTHAND ───────────────────────────────────────────────

/// CSS Flexbox §7.2: `flex: 0 1 auto` (the initial value) keeps a shrink
/// factor of 1, so two 30-cell items in a 40-cell row shrink to fit
/// (§9.7). The shorthand used to set the shrink to 0 for any zero grow.
#[test]
fn flex_zero_one_auto_shrinks() {
    let w = row_widths(
        40,
        ".a { flex: 0 1 auto; width: 30 } .b { flex: 0 2 auto; width: 30 }",
        &["a", "b"],
    );
    assert_eq!(w.iter().sum::<u16>(), 40, "{w:?}");
    assert_eq!(w, [24, 16], "20 of overflow, shrunk 30 × 1 : 30 × 2");
}
/// The basis is stored on the computed style (C6-FLEX-LONGHANDS lays it
/// out): `flex: 2 30%` computes `flex_basis` 30%.
#[test]
fn flex_basis_reaches_the_computed_style() {
    use rdom_tui::style::cascade::computed_of;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let a = el(&mut dom, root, "a");
    lay_out(&mut dom, ".a { flex: 2 30% }", 40, 5);
    assert_eq!(
        computed_of(&dom, a).flex_basis,
        rdom_tui::layout::FlexBasis::Calc(Box::new(rdom_style::calc::CalcExpr::Percent(30.0)))
    );
}

// ── C2G-ATTR-PARSE ───────────────────────────────────────────────────

/// CSS Values 5 §8.7: `<attr-type>` is `type(<syntax>)`, `raw-string`,
/// `number` or a CSS unit; anything else fails `attr()`'s grammar, so
/// the declaration is invalid at parse time and the earlier one stands:
/// `width: 10; width: attr(x bogus)` is 10.
#[test]
fn an_invalid_attr_type_drops_the_declaration_at_parse_time() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let a = el(&mut dom, root, "a");
    dom.set_attribute(a, "x", "30").unwrap();
    let parsed = rdom_css::parse(".a { width: 10; width: attr(x bogus); height: 1 }");
    assert_eq!(parsed.warnings.len(), 1, "{:?}", parsed.warnings);
    dom.cascade(&parsed.stylesheet);
    dom.layout_dom(Rect::new(0, 0, 40, 5));
    assert_eq!(rect(&dom, a).width, 10);
}

/// `:root` custom properties are seeded from the sheets for the whole
/// tree; an `attr()` in one reads the root element's attributes when the
/// root is an element (Selectors 4 §14.1: `:root` is the document's root
/// element). A fragment root has none, so the fallback applies.
#[test]
fn root_custom_property_attr_reads_the_root_element() {
    let css = ":root { --w: attr(data-w type(<length>), 3) } .a { width: var(--w); height: 1 }";
    let mut dom = TuiDom::with_root_tag("html");
    let root = dom.root();
    dom.set_attribute(root, "data-w", "7").unwrap();
    let a = el(&mut dom, root, "a");
    lay_out(&mut dom, css, 40, 5);
    assert_eq!(rect(&dom, a).width, 7);

    let mut dom = TuiDom::new();
    let root = dom.root();
    let a = el(&mut dom, root, "a");
    lay_out(&mut dom, css, 40, 5);
    assert_eq!(rect(&dom, a).width, 3, "a fragment root has no attributes");
}

// ── C2G-LAYOUT-SAFETY ────────────────────────────────────────────────

/// Padding sums are saturating: `padding: 0 40000` (left + right past
/// `u16::MAX`) and `padding: 40000 0` lay out — in a flex row (the
/// intrinsic main size, `aspect-ratio: auto && <ratio>`'s content box),
/// a flex column (an inline formatting context's height) and block flow
/// (auto height) — without an arithmetic overflow.
#[test]
fn huge_padding_lays_out_without_overflow() {
    for (container, item) in [
        ("display: flex", "padding: 0 40000"),
        (
            "display: flex",
            "padding: 0 40000; width: 10; aspect-ratio: auto 1",
        ),
        (
            "display: flex",
            "padding: 40000 0; height: 10; aspect-ratio: auto 1",
        ),
        ("display: flex; flex-direction: column", "padding: 0 40000"),
        ("display: flex; flex-direction: column", "padding: 40000 0"),
        ("", "padding: 40000 0"),
        ("", "padding: 40000"),
    ] {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let c = el(&mut dom, root, "c");
        let a = el(&mut dom, c, "a");
        let text = dom.create_text_node("some words to wrap");
        dom.append_child(a, text).unwrap();
        lay_out(
            &mut dom,
            &format!(".c {{ {container}; width: 40 }} .a {{ {item} }}"),
            80,
            20,
        );
        let _ = rect(&dom, a);
    }
}

/// CSS 2.1 §10.7 (as block flow does): a `max-height` percentage
/// against a containing block whose height is indefinite — a flex
/// container with `height: auto` — is `none`, on the main axis of a
/// column and the cross axis of a row; `min-height` likewise is 0.
/// Against a definite height it resolves.
#[test]
fn max_height_percent_in_an_auto_height_flex_container_is_none() {
    let heights = |container: &str| {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let c = el(&mut dom, root, "c");
        let a = el(&mut dom, c, "a");
        lay_out(
            &mut dom,
            &format!(
                ".c {{ {container}; width: 20 }} .a {{ height: 8; width: 4; max-height: 25% }}"
            ),
            40,
            20,
        );
        rect(&dom, a).height
    };
    assert_eq!(
        heights("display: flex; flex-direction: column"),
        8,
        "column, auto"
    );
    assert_eq!(heights("display: flex"), 8, "row, auto");
    assert_eq!(
        heights("display: flex; flex-direction: column; height: 20"),
        5
    );
    assert_eq!(heights("display: flex; height: 20"), 5);
    assert_eq!(heights(""), 8, "block flow, for reference");
}
