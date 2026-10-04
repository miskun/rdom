//! CSS-COMPLETE Phase 2 review-gate fixes (`C2G-*`), end to end: a
//! sheet parsed by `rdom_css`, cascaded and laid out by `rdom-tui`.
//! One section per item; each test cites the spec text that fixes the
//! expected value.

use rdom_tui::render::Rect;
use rdom_tui::{CascadeExt, LayoutExt, LayoutRect, NodeId, TuiDom, TuiNodeExt};

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
