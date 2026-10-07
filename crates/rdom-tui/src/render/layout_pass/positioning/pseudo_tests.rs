//! Placement of positioned `::before` / `::after`: size and position.

use crate::layout::LayoutRect;
use crate::prelude::*;
use crate::render::Rect;

/// The `::after` box of a `.h` host styled by `css`, laid out in a
/// 20 × 6 viewport.
fn after_rect(css: &str) -> LayoutRect {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let h = dom.create_element("div");
    dom.set_attribute(h, "class", "h").unwrap();
    dom.append_child(root, h).unwrap();
    let parsed = rdom_css::parse(css);
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    dom.cascade(&parsed.stylesheet);
    dom.layout_dom(Rect::new(0, 0, 20, 6));
    super::positioned_box(&dom, h, crate::ext::PseudoSlot::After)
        .and_then(|a| a.generated)
        .expect("a positioned ::after is placed")
        .border_box
}

const HOST: &str = ".h { position: relative; width: 10; height: 4 } ";

/// CSS 2.1 §10.3.7 / §10.6.4: a positioned box's `width` / `height`,
/// when not `auto`, are its size — a percentage of the containing
/// block's — whatever its content; `left` / `top` then place it.
#[test]
fn a_positioned_pseudo_takes_its_declared_size() {
    let r = after_rect(&format!(
        "{HOST} .h::after {{ position: absolute; left: 1; top: 1; \
         width: 4; height: 2; content: \"x\" }}"
    ));
    assert_eq!((r.x, r.y, r.width, r.height), (1, 1, 4, 2));
    let r = after_rect(&format!(
        "{HOST} .h::after {{ position: absolute; right: 0; bottom: 0; \
         width: 50%; height: 50%; content: \"x\" }}"
    ));
    assert_eq!(
        (r.x, r.y, r.width, r.height),
        (5, 2, 5, 2),
        "percentages of the 10 × 4 containing block, placed from the far edges"
    );
}

/// §10.3.7: with `width` set, `left` + `right` over-constrain the box —
/// the width holds (`right` is ignored, ltr); `auto` still spans
/// between the insets, and with one inset is the content's width.
#[test]
fn a_declared_width_wins_over_both_insets() {
    let r = after_rect(&format!(
        "{HOST} .h::after {{ position: absolute; left: 2; right: 2; top: 0; \
         width: 3; content: \"x\" }}"
    ));
    assert_eq!((r.x, r.width), (2, 3));
    let r = after_rect(&format!(
        "{HOST} .h::after {{ position: absolute; left: 2; right: 2; top: 0; content: \"x\" }}"
    ));
    assert_eq!((r.x, r.width), (2, 6));
    let r = after_rect(&format!(
        "{HOST} .h::after {{ position: absolute; left: 2; top: 0; content: \"abc\" }}"
    ));
    assert_eq!((r.x, r.width, r.height), (2, 3, 1));
}

/// CSS 2.1 §10.4 / §10.7: a positioned pseudo-element's size is clamped
/// by `max-*`, then `min-*`, as a positioned element's is; a keyword
/// bound is the content's intrinsic size (CSS Sizing 3 §3.1).
#[test]
fn a_positioned_pseudo_honours_min_and_max() {
    let r = after_rect(&format!(
        "{HOST} .h::after {{ position: absolute; left: 0; top: 0; \
         width: 8; max-width: 3; height: 1; min-height: 2; content: \"x\" }}"
    ));
    assert_eq!((r.width, r.height), (3, 2));
    let r = after_rect(&format!(
        "{HOST} .h::after {{ position: absolute; left: 0; right: 0; top: 0; \
         max-width: max-content; content: \"abcd\" }}"
    ));
    assert_eq!(r.width, 4);
}

/// C5G-REL-PSEUDO-INSETS — CSS 2.1 §9.4.3: a `position: relative` box
/// only shifts. With both `left` and `right` set, `right` is ignored
/// under `ltr` (`left` under `rtl`: the inline-start inset wins), and
/// its width stays its own — the content's for a pseudo-element; with
/// both `top` and `bottom`, `bottom` is ignored and the height holds.
/// The `::after` is laid out in flow (C10-PSEUDO-UNIFY) — the host's
/// only content, at its start under `ltr`, its end under `rtl` — and
/// moved from there.
#[test]
fn a_relative_pseudo_with_both_insets_only_shifts() {
    let rows = |dir: &str| {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let h = dom.create_element("div");
        dom.set_attribute(h, "class", "h").unwrap();
        dom.append_child(root, h).unwrap();
        let parsed = rdom_css::parse(&format!(
            "{HOST} .h {{ direction: {dir} }} .h::after {{ position: relative; \
             left: 1; right: 1; top: 1; bottom: 1; content: \"x\" }}"
        ));
        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
        dom.cascade(&parsed.stylesheet);
        let area = Rect::new(0, 0, 12, 3);
        dom.layout_dom(area);
        let mut buf = crate::render::Buffer::empty(area);
        dom.paint_dom(&mut buf, area);
        (0..3)
            .map(|y| {
                (0..12)
                    .map(|x| buf.cell(x, y).unwrap().symbol().to_string())
                    .collect()
            })
            .collect::<Vec<String>>()
    };
    assert_eq!(rows("ltr")[1], " x          ", "left wins");
    assert_eq!(rows("rtl")[1], "        x   ", "right wins");
}

/// The same holds for an element: both insets shift it without
/// stretching it (its block width is its containing block's).
#[test]
fn a_relative_element_with_both_insets_keeps_its_width() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let h = dom.create_element("div");
    dom.set_attribute(h, "class", "h").unwrap();
    dom.append_child(root, h).unwrap();
    let r = dom.create_element("div");
    dom.set_attribute(r, "class", "r").unwrap();
    dom.append_child(h, r).unwrap();
    let parsed = rdom_css::parse(&format!(
        "{HOST} .r {{ position: relative; left: 1; right: 3; height: 1 }}"
    ));
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    dom.cascade(&parsed.stylesheet);
    dom.layout_dom(Rect::new(0, 0, 20, 6));
    let rect = dom.node(r).ext().unwrap().layout;
    assert_eq!((rect.x, rect.width), (1, 10));
}

/// CSS 2.1 §10.1: "the containing block is formed by the padding edge of
/// the ancestor" — a positioned host's border is outside its absolutely
/// positioned `::after`'s containing block, its padding inside
/// (C7-ABSPOS-PADDING-EDGE).
#[test]
fn a_positioned_pseudos_containing_block_is_the_padding_box() {
    let r = after_rect(
        ".h { position: relative; width: 10; height: 2; border: solid; padding: 1 } \
         .h::after { position: absolute; inset: 0; content: \"x\" }",
    );
    assert_eq!((r.x, r.y, r.width, r.height), (1, 1, 12, 4));
}
