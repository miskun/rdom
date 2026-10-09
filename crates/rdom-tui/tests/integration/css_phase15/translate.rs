//! `translate`, `transform: translate()` and the transform family (CSS
//! Transforms 1 / 2, C15-TRANSLATE): whole-cell offsets that move a box's
//! paint, hit-testing and scrollable overflow but not the layout around
//! it, and the stacking contexts and containing blocks a transform makes.

use super::*;
use rdom_tui::{HitTestExt, TuiAccessors};

/// Transforms 2 §6.1, Transforms 1 §3: the box and its subtree move by the
/// offset; the layout around it — its siblings, its parent's `auto` height
/// — is the untransformed one.
#[test]
fn translate_moves_the_box_and_its_subtree_but_not_the_layout_around_it() {
    let mut dom = doc(
        r#"<div id="p"><div id="a">aa</div><div id="t"><div id="c">x</div></div><div id="b">bb</div></div>"#,
    );
    styled(
        &mut dom,
        "#p > div { height: 1 } #t { width: 4; translate: 3 1 }",
        30,
        8,
    );
    assert_eq!(rect(&dom, "t"), (3, 2, 4, 1));
    assert_eq!(rect(&dom, "c"), (3, 2, 4, 1), "the subtree moves with it");
    assert_eq!(rect(&dom, "a"), (0, 0, 30, 1));
    assert_eq!(rect(&dom, "b"), (0, 2, 30, 1), "the next sibling stays");
    assert_eq!(rect(&dom, "p").3, 3, "the parent's auto height stays");
}

/// Transforms 1 §5, Transforms 2 §6: the translate functions of
/// `transform` add to the `translate` property; the other functions —
/// rotation, scaling, skews, matrices — are inert in a cell grid.
#[test]
fn transform_translate_functions_add_up_and_the_others_are_inert() {
    let markup = r#"<div id="t">x</div>"#;
    let mut dom = doc(markup);
    styled(
        &mut dom,
        "#t { width: 2; height: 1; translate: 1;
              transform: translateX(2) translate(1, -1) translateY(2) }",
        30,
        8,
    );
    assert_eq!(rect(&dom, "t"), (4, 1, 2, 1));
    let mut dom = doc(markup);
    styled(
        &mut dom,
        "#t { width: 2; height: 1; transform: rotate(45deg) scale(2) skewX(10deg);
              rotate: 90deg; scale: 3 }",
        30,
        8,
    );
    assert_eq!(rect(&dom, "t"), (0, 0, 2, 1));
}

/// Transforms 1 §7: a percentage is of the reference box — the border box
/// by default (`view-box` is the border box of a CSS box), the content box
/// under `transform-box: content-box | fill-box`. Fractional offsets
/// round to whole cells once, ties to even (DIVERGENCES).
#[test]
fn percentages_are_of_the_reference_box_and_round_once() {
    let markup = r#"<div id="t">x</div>"#;
    let mut dom = doc(markup);
    styled(
        &mut dom,
        "#t { width: 10; height: 4; padding: 1; translate: 50% 50% }",
        40,
        20,
    );
    assert_eq!((rect(&dom, "t").0, rect(&dom, "t").1), (6, 3));
    let mut dom = doc(markup);
    styled(
        &mut dom,
        "#t { width: 10; height: 4; padding: 1; translate: 50% 50%; transform-box: content-box }",
        40,
        20,
    );
    assert_eq!((rect(&dom, "t").0, rect(&dom, "t").1), (5, 2));
    // 2.5 cells: to even, down; 1.5 cells: up; 50% + 50% of 5: once, 5.
    for (css, x) in [
        ("translate: 2.5", 2),
        ("translate: 1.5", 2),
        ("translate: 50%; transform: translateX(50%)", 5),
    ] {
        let mut dom = doc(markup);
        styled(
            &mut dom,
            &format!("#t {{ width: 5; height: 1; {css} }}"),
            40,
            4,
        );
        assert_eq!(rect(&dom, "t").0, x, "{css}");
    }
}

/// Transforms 1 §3: a transform applies to transformable elements only —
/// a non-atomic inline box is not one, so a span keeps its place in the
/// line.
#[test]
fn an_inline_box_is_not_transformable() {
    let mut dom = doc(r#"<p>ab<span id="s">cd</span>ef</p>"#);
    let buf = paint(&mut dom, "p { margin: 0 } #s { translate: 5 1 }", 12, 3);
    assert_eq!(row(&buf, 0), "abcdef      ");
}

/// Transforms 1 §2 / Transforms 2 §6: a transform other than `none` — an
/// inert one, and `translate: 0` too — establishes a stacking context, so
/// a `z-index: -1` child paints above the box's own background instead of
/// below it.
#[test]
fn a_transform_makes_a_stacking_context() {
    for transform in [
        "translate: 0",
        "rotate: 10deg",
        "transform: scale(1)",
        "scale: 1",
    ] {
        let mut dom = doc(r#"<div id="t"><div id="k">k</div></div>"#);
        let buf = paint(
            &mut dom,
            &format!(
                "#t {{ {transform}; background-color: rgb(0, 0, 255); height: 2 }}
                 #k {{ position: relative; z-index: -1; background-color: rgb(255, 0, 0); width: 1; height: 1 }}"
            ),
            6,
            3,
        );
        assert_eq!(buf.cell(0, 0).unwrap().bg, RED, "{transform}");
    }
    let mut dom = doc(r#"<div id="t"><div id="k">k</div></div>"#);
    let buf = paint(
        &mut dom,
        "#t { background-color: rgb(0, 0, 255); height: 2 }
         #k { position: relative; z-index: -1; background-color: rgb(255, 0, 0); width: 1; height: 1 }",
        6,
        3,
    );
    assert_eq!(buf.cell(0, 0).unwrap().bg, BLUE, "no transform: below");
}

/// Transforms 1 §2: a transformed element is the containing block of its
/// fixed and absolutely positioned descendants, which move with it.
#[test]
fn a_transform_contains_fixed_and_absolute_descendants() {
    let markup = r#"<p>x</p><div id="t"><span id="f">f</span><span id="a">a</span></div>"#;
    let mut dom = doc(markup);
    styled(
        &mut dom,
        "p { margin: 0 } #t { translate: 2 1; margin-left: 4; height: 3 }
         #f { position: fixed; top: 0; left: 1 } #a { position: absolute; top: 1; left: 0 }",
        30,
        8,
    );
    assert_eq!(rect(&dom, "t").0, 6);
    assert_eq!((rect(&dom, "f").0, rect(&dom, "f").1), (7, 2));
    assert_eq!((rect(&dom, "a").0, rect(&dom, "a").1), (6, 3));
    let mut dom = doc(markup);
    styled(
        &mut dom,
        "p { margin: 0 } #t { margin-left: 4; height: 3 }
         #f { position: fixed; top: 0; left: 1 }",
        30,
        8,
    );
    assert_eq!(
        (rect(&dom, "f").0, rect(&dom, "f").1),
        (1, 0),
        "the viewport"
    );
}

/// Transforms 1 §3: the transform affects painting and hit-testing — the
/// box paints and is hit where it moved to.
#[test]
fn a_translated_box_paints_and_hits_where_it_moved() {
    let mut dom = doc(r#"<div id="t">ab</div>"#);
    let buf = paint(&mut dom, "#t { width: 2; translate: 3 1 }", 8, 3);
    assert_eq!(row(&buf, 0), "        ");
    assert_eq!(row(&buf, 1), "   ab   ");
    let t = by_id(&dom, "t");
    assert_eq!(dom.hit_test(3, 1), Some(t));
    assert_ne!(dom.hit_test(0, 0), Some(t));
}

/// Transforms 1 §3 (CSS Overflow 3 §2.2): a transformed box contributes
/// its transformed extent to its scroll container's scrollable overflow.
#[test]
fn a_translated_box_extends_the_scrollable_overflow() {
    let mut dom = doc(r#"<div id="s"><div id="t">x</div></div>"#);
    styled(
        &mut dom,
        "#s { width: 10; height: 3; overflow: auto } #t { width: 4; height: 1; translate: 0 5 }",
        20,
        6,
    );
    assert_eq!(dom.node(by_id(&dom, "s")).scroll_height(), Some(6));
}

/// Transforms 1 §3: a flex item, an atomic inline and an absolutely
/// positioned box are transformable; each moves alone — the flex line, the
/// line box and the placement around it are the untransformed ones.
#[test]
fn flex_items_atomic_inlines_and_positioned_boxes_move_alone() {
    let mut dom =
        doc(r#"<div id="f"><div id="a">a</div><div id="t">t</div><div id="b">b</div></div>"#);
    styled(
        &mut dom,
        "#f { display: flex } #f > div { width: 2; height: 1 } #t { translate: 1 2 }",
        20,
        5,
    );
    assert_eq!(rect(&dom, "a"), (0, 0, 2, 1));
    assert_eq!(rect(&dom, "t"), (3, 2, 2, 1));
    assert_eq!(rect(&dom, "b"), (4, 0, 2, 1), "the next item stays");
    let mut dom = doc(r#"<p>ab<span id="t">cd</span>ef</p>"#);
    let buf = paint(
        &mut dom,
        "p { margin: 0 } #t { display: inline-block; translate: 0 1 }",
        8,
        3,
    );
    assert_eq!(row(&buf, 0), "ab  ef  ");
    assert_eq!(row(&buf, 1), "  cd    ");
    let mut dom = doc(r#"<div id="t">x</div>"#);
    styled(
        &mut dom,
        "#t { position: absolute; top: 1; left: 2; width: 1; height: 1; translate: 3 1 }",
        20,
        5,
    );
    assert_eq!(rect(&dom, "t"), (5, 2, 1, 1));
}

/// Transforms 1 §1 ("transformable element": table rows and row groups
/// are block-level boxes here) — `tr { translate }` moves the row and its
/// cells, as it moves any box; the rows around it stay
/// (C15G-TRANSLATE-GAPS, architect N4a).
#[test]
fn a_translated_table_row_moves_with_its_cells() {
    let mut dom =
        doc(r#"<table><tr id="a"><td id="x">x</td></tr><tr id="b"><td id="y">y</td></tr></table>"#);
    styled(&mut dom, "td { padding: 0 } #a { translate: 3 2 }", 20, 6);
    let (b, x, y) = (rect(&dom, "b"), rect(&dom, "x"), rect(&dom, "y"));
    let a = rect(&dom, "a");
    assert_eq!((a.0, a.1), (b.0 + 3, b.1 - 1 + 2), "the row moved");
    assert_eq!((x.0, x.1), (y.0 + 3, y.1 - 1 + 2), "its cell with it");
}

/// CSS Position 3 §3.4 with Transforms 1 §3: stickiness is computed on the
/// box as laid out — untransformed — and the translation applies after, so
/// a stuck `top: 0; translate: 0 2` header shows at row 2 of its scrollport
/// (it was pinned at row 0: the translated rect read as the natural one)
/// (C15G-TRANSLATE-GAPS, architect N4b).
#[test]
fn a_stuck_sticky_header_is_translated_after_sticking() {
    let lines: String = (0..20).map(|k| format!("<p>l{k}</p>")).collect();
    let mut dom = doc(&format!(r#"<div id="s"><h1 id="h">H</h1>{lines}</div>"#));
    styled(
        &mut dom,
        "p, h1 { margin: 0 } #s { height: 5; overflow: auto }
         #h { position: sticky; top: 0; translate: 0 2 }",
        20,
        5,
    );
    let s = by_id(&dom, "s");
    if let Some(ext) = dom.node_mut(s).ext_mut() {
        ext.scroll_y = 6;
    }
    dom.layout_dom(rdom_tui::render::Rect::new(0, 0, 20, 5));
    assert_eq!(rect(&dom, "h").1, 2);
}
