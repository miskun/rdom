//! C5-LOGICAL — the flow-relative properties (CSS Logical 1) in layout:
//! `horizontal-tb` maps the block axis to top / bottom, and `direction`
//! maps the inline axis (C5-WRITING).

use super::{el, lay_out, paint, rect, rows, size};
use rdom_tui::{TuiDom, TuiNodeExt};

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

/// C5G-CSSOM-LOGICAL — CSSOM §6.6: `getPropertyValue` reads a logical
/// longhand through the shorthand that set it last, and
/// `getPropertyPriority` the logical declaration's own priority; `cssText`
/// writes a set logical shorthand once.
#[test]
fn cssom_reads_logical_longhands_and_priorities() {
    use rdom_tui::TuiAccessors;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    dom.set_attribute(b, "style", "margin-inline: 1 2").unwrap();
    assert!(rdom_tui::seed_inline_styles(&mut dom).is_empty());
    let style = dom.node(b).style().expect("an element");
    assert_eq!(style.get_property_value("margin-inline-start"), "1");
    assert_eq!(style.get_property_value("margin-inline-end"), "2");
    assert_eq!(style.css_text(), "margin-inline: 1 2;");
    assert_eq!(style.length(), 1);

    dom.set_attribute(
        b,
        "style",
        "margin-inline-start: 1 !important; margin-left: 2",
    )
    .unwrap();
    assert!(rdom_tui::seed_inline_styles(&mut dom).is_empty());
    let style = dom.node(b).style().expect("an element");
    assert_eq!(
        style.get_property_priority("margin-inline-start"),
        "important"
    );
    assert_eq!(style.get_property_priority("margin-inline-end"), "");

    dom.set_attribute(
        b,
        "style",
        "margin-inline-start: 1; margin-left: 2 !important",
    )
    .unwrap();
    assert!(rdom_tui::seed_inline_styles(&mut dom).is_empty());
    let style = dom.node(b).style().expect("an element");
    assert_eq!(style.get_property_priority("margin-inline-start"), "");
    assert_eq!(style.get_property_priority("margin-left"), "important");
}

/// C5G-LOGICAL-IMPORTANT — CSS Cascade 4 §6.4 / CSS Logical 1 §4: an
/// `!important` inline-axis declaration makes important only the side it
/// maps to for the element's direction. A normal `border-right-color`
/// beside an important `border-inline-start-color` stays normal, so a
/// more specific normal rule beats it.
#[test]
fn logical_importance_marks_only_the_mapped_side() {
    for (dir, (left, right)) in [("ltr", ("red", "lime")), ("rtl", ("reset", "red"))] {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let b = el(&mut dom, root, "div", "a b");
        dom.set_attribute(b, "dir", dir).unwrap();
        lay_out(
            &mut dom,
            ".a { border: solid; border-inline-start-color: red !important; \
                  border-right-color: blue } \
             .a.b { border-right-color: lime }",
            10,
            3,
        );
        let c = dom.node(b).computed().unwrap();
        let name = |col: rdom_tui::Color| match col {
            rdom_tui::Color::Rgb(255, 0, 0) => "red",
            rdom_tui::Color::Rgb(0, 255, 0) => "lime",
            rdom_tui::Color::Rgb(0, 0, 255) => "blue",
            _ => "reset",
        };
        assert_eq!(
            (name(c.border_color.left), name(c.border_color.right)),
            (left, right),
            "{dir}"
        );
    }
}

/// The same for the insets, and between the declarations of one block:
/// an important `inset-inline-start` beats a later normal `left` in its
/// block (importance before order, CSS Cascade 4 §6.4), and leaves
/// `right` normal.
#[test]
fn logical_importance_orders_against_the_physical_side() {
    let (dom, b) = one(".wrap { position: relative; width: 20 } \
         .b { position: relative; width: 2; height: 1; \
              inset-inline-start: 3 !important; left: 1; right: 2 } \
         .wrap .b { right: 5 }");
    let c = dom.node(b).computed().unwrap();
    assert_eq!(c.left, rdom_tui::layout::Length::Cells(3));
    assert_eq!(c.right, rdom_tui::layout::Length::Cells(5));
}

/// CSSOM: a normal physical declaration beside an important logical one
/// is not important (`getPropertyPriority`), and `removeProperty` of a
/// logical longhand removes that declaration only — the physical one
/// stays, and a shorthand's other component too.
#[test]
fn cssom_priority_and_removal_keep_the_physical_declaration() {
    use rdom_tui::{TuiAccessors, TuiAccessorsMut};
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    dom.set_attribute(
        b,
        "style",
        "margin-inline-start: 1 !important; margin-left: 2",
    )
    .unwrap();
    assert!(rdom_tui::seed_inline_styles(&mut dom).is_empty());
    let style = dom.node(b).style().expect("an element");
    assert_eq!(style.get_property_priority("margin-left"), "");

    dom.set_attribute(b, "style", "left: 3; inset-inline-start: 1")
        .unwrap();
    assert!(rdom_tui::seed_inline_styles(&mut dom).is_empty());
    dom.node_mut(b)
        .style_mut()
        .unwrap()
        .remove_property("inset-inline-start")
        .unwrap();
    let style = dom.node(b).style().expect("an element");
    assert_eq!(style.get_property_value("left"), "3");
    assert_eq!(style.get_property_value("inset-inline-start"), "");

    dom.set_attribute(b, "style", "padding-inline: 1 2")
        .unwrap();
    assert!(rdom_tui::seed_inline_styles(&mut dom).is_empty());
    dom.node_mut(b)
        .style_mut()
        .unwrap()
        .remove_property("padding-inline-start")
        .unwrap();
    let style = dom.node(b).style().expect("an element");
    assert_eq!(style.get_property_value("padding-inline-start"), "");
    assert_eq!(style.get_property_value("padding-inline-end"), "2");
}
