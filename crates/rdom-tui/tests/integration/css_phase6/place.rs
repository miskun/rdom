//! C6-PLACE — the `place-*` shorthands (CSS Box Alignment 3 §5.5, §6.4,
//! §6.5), `justify-self` / `justify-items` on block-level boxes (§6.1 /
//! §6.2, as Chromium 130 ships them), and `align-content` on block
//! containers (§5.1, Chromium 123); `justify-*` are ignored in flex.

use super::{el, lay_out, rect, size};
use rdom_tui::TuiDom;

/// §5.5: `place-content` sets `align-content` then `justify-content`;
/// one value sets both.
#[test]
fn place_content_sets_both_content_properties() {
    let at = |css: &str| {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let f = el(&mut dom, root, "div", "f");
        let ids: Vec<_> = (0..3).map(|_| el(&mut dom, f, "div", "i")).collect();
        lay_out(
            &mut dom,
            &format!(
                ".f {{ display: flex; flex-wrap: wrap; width: 10; height: 10 }} \
                 .i {{ width: 4; height: 1 }} {css}"
            ),
            20,
            14,
        );
        ids.iter()
            .map(|&i| (rect(&dom, i).x, rect(&dom, i).y))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        at(".f { place-content: center space-between }"),
        [(0, 4), (6, 4), (0, 5)]
    );
    assert_eq!(at(".f { place-content: center }"), [(1, 4), (5, 4), (3, 5)]);
}

/// §6.4 / §6.5: `place-items` and `place-self` set `align-items` /
/// `align-self`; their `justify-*` halves do not apply to flex items
/// (§6.1, Flexbox §8).
#[test]
fn place_items_and_self_in_flex() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let a = el(&mut dom, f, "div", "i");
    let b = el(&mut dom, f, "div", "i b");
    for n in [a, b] {
        let t = dom.create_text_node("x");
        dom.append_child(n, t).unwrap();
    }
    lay_out(
        &mut dom,
        ".f { display: flex; width: 12; height: 6; place-items: end center } \
         .i { width: 2 } .b { place-self: center end }",
        20,
        10,
    );
    assert_eq!((rect(&dom, a).x, rect(&dom, a).y), (0, 5));
    assert_eq!((rect(&dom, b).x, rect(&dom, b).y), (2, 2));
}

/// A block child holding `abc` in a 20-wide block container `.p`, laid
/// out under `css`; its x and width.
fn block(css: &str) -> (i32, u16) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", "p");
    let c = el(&mut dom, p, "div", "c");
    let t = dom.create_text_node("abc");
    dom.append_child(c, t).unwrap();
    lay_out(&mut dom, &format!(".p {{ width: 20 }} {css}"), 30, 10);
    (rect(&dom, c).x, size(&dom, c).0)
}

/// §6.1: on a block-level box `justify-self` other than `normal` /
/// `stretch` sizes an `auto` width as `fit-content` and aligns the box in
/// its containing block — `start` / `end` by the container's direction,
/// `left` / `right` physically, `self-start` / `self-end` by the box's
/// own, `center` (the leading space rounded down); `flex-start` /
/// `flex-end` are `start` / `end` outside flex.
#[test]
fn justify_self_aligns_a_block_box() {
    for v in ["normal", "stretch", "auto"] {
        assert_eq!(
            block(&format!(".c {{ justify-self: {v} }}")),
            (0, 20),
            "{v}"
        );
    }
    for v in ["start", "left", "flex-start", "self-start", "baseline"] {
        assert_eq!(block(&format!(".c {{ justify-self: {v} }}")), (0, 3), "{v}");
    }
    for v in ["end", "right", "flex-end", "self-end", "last baseline"] {
        assert_eq!(
            block(&format!(".c {{ justify-self: {v} }}")),
            (17, 3),
            "{v}"
        );
    }
    assert_eq!(block(".c { justify-self: center }"), (8, 3));
    assert_eq!(block(".c { justify-self: center; width: 6 }"), (7, 6));
    let rtl = ".p { direction: rtl }";
    assert_eq!(
        block(&format!("{rtl} .c {{ justify-self: start }}")),
        (17, 3)
    );
    assert_eq!(block(&format!("{rtl} .c {{ justify-self: left }}")), (0, 3));
    assert_eq!(
        block(&format!(
            "{rtl} .c {{ justify-self: self-start; direction: ltr }}"
        )),
        (0, 3)
    );
}

/// `auto` margins take the space first (CSS 2.1 §10.3.3); an
/// overflowing box overflows as asked unless `safe` (§4.4).
#[test]
fn justify_self_with_margins_and_overflow() {
    assert_eq!(
        block(".c { width: 6; margin-left: auto; justify-self: start }"),
        (14, 6)
    );
    assert_eq!(block(".c { width: 30; justify-self: center }"), (-5, 30));
    assert_eq!(
        block(".c { width: 30; justify-self: safe center }"),
        (0, 30)
    );
}

/// §6.2: `justify-self: auto` takes the parent's `justify-items`; a
/// `legacy` value is inherited by `justify-items: legacy` (the initial
/// value), so it reaches grandchildren.
#[test]
fn justify_items_defaults_the_childrens_justify_self() {
    assert_eq!(block(".p { justify-items: center }"), (8, 3));
    assert_eq!(
        block(".p { justify-items: end } .c { justify-self: start }"),
        (0, 3)
    );
    let mut dom = TuiDom::new();
    let root = dom.root();
    let g = el(&mut dom, root, "div", "g");
    let p = el(&mut dom, g, "div", "p");
    let c = el(&mut dom, p, "div", "c");
    let t = dom.create_text_node("abc");
    dom.append_child(c, t).unwrap();
    lay_out(
        &mut dom,
        ".g { width: 30; justify-items: legacy center } .p { width: 20 }",
        40,
        10,
    );
    assert_eq!(rect(&dom, p).x, 5);
    assert_eq!((rect(&dom, c).x, size(&dom, c).0), (13, 3));
    // Without `legacy` the value is not passed on.
    lay_out(
        &mut dom,
        ".g { width: 30; justify-items: center } .p { width: 20 }",
        40,
        10,
    );
    assert_eq!(rect(&dom, p).x, 5);
    assert_eq!((rect(&dom, c).x, size(&dom, c).0), (5, 20));
}

/// §6.1: `justify-self` does not apply to flex items.
#[test]
fn justify_self_is_ignored_in_flex() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let i = el(&mut dom, f, "div", "i");
    lay_out(
        &mut dom,
        ".f { display: flex; width: 10; height: 2 } .i { width: 2; justify-self: end }",
        20,
        10,
    );
    assert_eq!(rect(&dom, i).x, 0);
}

/// §5.1: `align-content` on a block container shifts its content in its
/// block axis when the container is taller than the content; the
/// distributions fall back (`space-between` → `start`, `space-around` /
/// `space-evenly` → `center`, `stretch` → `start`). A scroll container's
/// overflowing content starts at its start.
#[test]
fn align_content_on_a_block_container() {
    let y = |css: &str| {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let b = el(&mut dom, root, "div", "b");
        let c = el(&mut dom, b, "div", "c");
        lay_out(
            &mut dom,
            &format!(".b {{ height: 10; width: 6 }} .c {{ height: 2 }} {css}"),
            20,
            14,
        );
        rect(&dom, c).y - rect(&dom, b).y
    };
    for v in [
        "normal",
        "start",
        "flex-start",
        "stretch",
        "space-between",
        "baseline",
    ] {
        assert_eq!(y(&format!(".b {{ align-content: {v} }}")), 0, "{v}");
    }
    for v in ["center", "space-around", "space-evenly"] {
        assert_eq!(y(&format!(".b {{ align-content: {v} }}")), 4, "{v}");
    }
    for v in ["end", "flex-end", "last baseline"] {
        assert_eq!(y(&format!(".b {{ align-content: {v} }}")), 8, "{v}");
    }
    assert_eq!(y(".b { align-content: center; height: auto }"), 0);
    assert_eq!(y(".b { align-content: center } .c { height: 12 }"), -1);
    assert_eq!(y(".b { align-content: safe center } .c { height: 12 }"), 0);
    assert_eq!(
        y(".b { align-content: center; overflow-y: auto } .c { height: 12 }"),
        0
    );
}

/// §5.1: a block container whose `align-content` is not `normal` is an
/// independent formatting context — its first child's top margin stays
/// inside it instead of collapsing through (CSS 2.1 §8.3.1).
#[test]
fn align_content_makes_a_block_container_independent() {
    let offset = |css: &str| {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let o = el(&mut dom, root, "div", "o");
        let b = el(&mut dom, o, "div", "b");
        let c = el(&mut dom, b, "div", "c");
        lay_out(
            &mut dom,
            &format!(".c {{ height: 1; margin-top: 2 }} {css}"),
            20,
            10,
        );
        rect(&dom, c).y - rect(&dom, b).y
    };
    assert_eq!(offset(""), 0);
    assert_eq!(offset(".b { align-content: start }"), 2);
}

/// §5.1 with CSS 2.1 §10.7 (C6G-BLOCK-ALIGN): an `auto`-height block
/// container is as tall as its content only until `min-height` makes it
/// taller — then `align-content` places the content in that height.
#[test]
fn align_content_uses_the_resolved_height() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let o = el(&mut dom, root, "div", "");
    let b = el(&mut dom, o, "div", "b");
    let c = el(&mut dom, b, "div", "c");
    let a = el(&mut dom, b, "div", "a");
    lay_out(
        &mut dom,
        ".b { min-height: 10; width: 6; align-content: center } .c { height: 2 } \
         .a { position: absolute; height: 1; width: 1 }",
        20,
        14,
    );
    assert_eq!(size(&dom, b).1, 10);
    assert_eq!(rect(&dom, c).y - rect(&dom, b).y, 4);
    // The out-of-flow box's static position moves with the content
    // (CSS 2.1 §10.6.4: where its hypothetical box would be, after `c`).
    assert_eq!(rect(&dom, a).y - rect(&dom, b).y, 6);
}

/// Box Alignment 3 §4.2 (C6G-BLOCK-ALIGN): the fallback of `first
/// baseline` in self-alignment is `safe self-start`, of `last baseline`
/// `safe self-end` — the box's own start and end, so an `rtl` box in an
/// `ltr` container falls back to its right edge for `baseline`.
#[test]
fn justify_self_baseline_falls_back_on_the_boxs_own_direction() {
    let x = |value: &str| {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let p = el(&mut dom, root, "div", "p");
        let c = el(&mut dom, p, "div", "c");
        lay_out(
            &mut dom,
            &format!(".p {{ width: 20 }} .c {{ width: 4; direction: rtl; justify-self: {value} }}"),
            20,
            4,
        );
        rect(&dom, c).x
    };
    assert_eq!(x("baseline"), 16);
    assert_eq!(x("last baseline"), 0);
}
