//! C5-LOGICAL — the flow-relative properties (CSS Logical 1) in layout:
//! `horizontal-tb` maps the block axis to top / bottom, and `direction`
//! maps the inline axis (C5-WRITING).

use super::{el, lay_out, paint, rect, rows, size};
use rdom_tui::TuiDom;

/// `div.b` in a block `div.wrap`, laid out under `css` in 30 × 10.
fn one(css: &str) -> (TuiDom, rdom_tui::NodeId) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let wrap = el(&mut dom, root, "div", "wrap");
    let b = el(&mut dom, wrap, "div", "b");
    lay_out(&mut dom, css, 30, 10);
    (dom, b)
}

/// CSS Logical 1 §2–§4 in `horizontal-tb`, `ltr`: `inline-size` is the
/// width, `block-size` the height, `margin-block-start` the top margin,
/// `padding-inline` the left / right padding.
#[test]
fn flow_relative_sizes_and_spacing_lay_out() {
    let (dom, b) = one(".wrap { width: 20 } \
         .b { inline-size: 6; block-size: 2; padding-inline: 1 2; margin-block-start: 1; \
              margin-inline-start: 3 }");
    assert_eq!(size(&dom, b), (6 + 1 + 2, 2));
    assert_eq!((rect(&dom, b).x, rect(&dom, b).y), (3, 1));
}

/// CSS Logical 1 §4 with CSS Writing Modes 4 §2.1: under `rtl` the
/// inline-start side is the right one — `margin-inline-start` is the
/// right margin, `border-inline-start` the right border.
#[test]
fn inline_start_is_the_right_side_under_rtl() {
    let (dom, b) = one(".wrap { direction: rtl; width: 10 } \
         .b { width: 4; height: 1; margin-inline-start: 3 }");
    assert_eq!(rect(&dom, b).x, 10 - 3 - 4);

    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    let buf = paint(
        &mut dom,
        ".b { direction: rtl; width: 4; height: 1; border-inline-start: solid; \
              border-block: none }",
        6,
        2,
    );
    assert_eq!(size(&dom, b), (5, 1));
    assert_eq!(rows(&buf, 5, 1), ["    │"]);
}

/// The mapping reads the element's own `direction` — declared on the
/// same element, after the logical property, it still applies (CSS
/// Logical 1 §4: the computed `direction` decides).
#[test]
fn the_elements_own_direction_maps_its_logical_properties() {
    let (dom, b) = one(
        ".wrap { width: 10 } .b { margin-inline-start: 3; width: 4; height: 1; direction: rtl }",
    );
    assert_eq!(
        rect(&dom, b).x,
        0,
        "an over-constrained ltr block keeps its left margin: 0"
    );
    let (dom, b) = one(
        ".wrap { width: 10 } .b { margin-inline-end: 2; width: 4; height: 1; direction: rtl } \
         .wrap { direction: rtl }",
    );
    assert_eq!(
        rect(&dom, b).x,
        10 - 4,
        "rtl: inline-end is the left margin"
    );
}

/// CSS Logical 1 §4: a logical property and its physical twin cascade
/// by declaration order and specificity alike — across rules too.
#[test]
fn logical_and_physical_declarations_cascade_together() {
    let (dom, b) = one(
        ".wrap { width: 20 } .b { margin-left: 2; margin-inline-start: 5; width: 1; height: 1 }",
    );
    assert_eq!(rect(&dom, b).x, 5);
    let (dom, b) = one(
        ".wrap { width: 20 } .b { margin-inline-start: 5; margin-left: 2; width: 1; height: 1 }",
    );
    assert_eq!(rect(&dom, b).x, 2);
    let (dom, b) = one(
        ".wrap { width: 20 } div.b { inset-inline-start: 4 } .b { position: relative; left: 1; width: 1; height: 1 }",
    );
    assert_eq!(rect(&dom, b).x, 4, "the more specific logical rule wins");
}

/// The same under `rtl`, where inline-start is the right margin: the
/// block's direction-mapped form is built once with the sheet
/// (C5G-LOGICAL-COST) and keeps the declarations' order.
#[test]
fn declaration_order_holds_under_rtl() {
    let css = |decls: &str| {
        format!(".wrap {{ direction: rtl; width: 20 }} .b {{ {decls}; width: 1; height: 1 }}")
    };
    let (dom, b) = one(&css("margin-right: 2; margin-inline-start: 5"));
    assert_eq!(rect(&dom, b).x, 14, "the later logical margin (5) wins");
    let (dom, b) = one(&css("margin-inline-start: 5; margin-right: 2"));
    assert_eq!(rect(&dom, b).x, 17, "the later physical margin (2) wins");
}

/// CSS Logical 1 §6: `border-start-end-radius` is the corner at the
/// block-start and inline-end sides — top-right under `ltr`, top-left
/// under `rtl`.
#[test]
fn logical_corner_radii_follow_the_direction() {
    for (dir, want) in [("ltr", ["┌──╮", "└──┘"]), ("rtl", ["╭──┐", "└──┘"])]
    {
        let mut dom = TuiDom::new();
        let root = dom.root();
        el(&mut dom, root, "div", "b");
        let buf = paint(
            &mut dom,
            &format!(
                ".b {{ direction: {dir}; box-sizing: border-box; width: 4; height: 2; border: solid; \
                  border-start-end-radius: 1 }}"
            ),
            6,
            3,
        );
        assert_eq!(rows(&buf, 4, 2), want, "{dir}");
    }
}

/// CSSOM `cssText` / `length` list a declaration block's properties
/// once: a block-axis flow-relative property is its physical twin's
/// storage (DIVERGENCES §2), listed under that name; an inline-axis one
/// is kept as written.
#[test]
fn cssom_lists_shared_storage_once() {
    use rdom_tui::TuiAccessors;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    dom.set_attribute(b, "style", "inline-size: 4; margin-inline-start: 2")
        .unwrap();
    assert!(rdom_tui::seed_inline_styles(&mut dom).is_empty());
    let style = dom.node(b).style().expect("an element");
    assert_eq!(style.css_text(), "width: 4; margin-inline-start: 2;");
    assert_eq!(style.length(), 2);
    assert_eq!(style.get_property_value("inline-size"), "4");
}
