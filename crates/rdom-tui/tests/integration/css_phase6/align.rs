//! C6-ALIGN — `align-items` / `align-self` in flex layout (CSS Flexbox
//! §8.3, §9.4 steps 8 / 11, CSS Box Alignment 3 §6): each item placed on
//! its line's cross axis — stretched, packed, centered or baseline-
//! aligned — and cross-axis `auto` margins (Flexbox §8.1).

use super::{el, lay_out, rect, size};
use rdom_tui::TuiDom;

/// Items `.i` (each holding `x`, plus its extra class) in a flex
/// container `.f` under `css`; their rects as `(x, y, width, height)`.
fn items(css: &str, classes: &[&str]) -> Vec<(i32, i32, u16, u16)> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let ids: Vec<_> = classes
        .iter()
        .map(|c| {
            let id = el(&mut dom, f, "div", &format!("i {c}"));
            let t = dom.create_text_node("x");
            dom.append_child(id, t).unwrap();
            id
        })
        .collect();
    lay_out(
        &mut dom,
        &format!(".f {{ display: flex; width: 12; height: 6 }} .i {{ width: 2 }} {css}"),
        20,
        10,
    );
    ids.iter()
        .map(|&n| {
            let r = rect(&dom, n);
            (r.x, r.y, r.width, r.height)
        })
        .collect()
}

/// `(y, height)` of the one item under `css`.
fn one(css: &str) -> (i32, u16) {
    let r = items(css, &[""])[0];
    (r.1, r.3)
}

/// §8.3 / Box Alignment §6.1: `normal` behaves as `stretch` for flex
/// items, which fills the line; the positions pack the item toward
/// cross-start (`flex-start`, `start`, `self-start` — the block-start
/// edge of a row's cross axis), cross-end or the center (rounded down).
#[test]
fn align_items_places_each_item() {
    for v in ["normal", "stretch"] {
        assert_eq!(one(&format!(".f {{ align-items: {v} }}")), (0, 6), "{v}");
    }
    for v in ["flex-start", "start", "self-start"] {
        assert_eq!(one(&format!(".f {{ align-items: {v} }}")), (0, 1), "{v}");
    }
    for v in ["flex-end", "end", "self-end"] {
        assert_eq!(one(&format!(".f {{ align-items: {v} }}")), (5, 1), "{v}");
    }
    assert_eq!(one(".f { align-items: center }"), (2, 1));
}

/// §8.3: `align-self` overrides the container's `align-items` for one
/// item; `auto` takes `align-items`.
#[test]
fn align_self_overrides_align_items() {
    let r = items(
        ".f { align-items: flex-end } .a { align-self: center } .b { align-self: auto } \
         .c { align-self: stretch }",
        &["a", "b", "c"],
    );
    assert_eq!(
        r.iter().map(|r| (r.1, r.3)).collect::<Vec<_>>(),
        [(2, 1), (5, 1), (0, 6)]
    );
}

/// §9.4 step 11: a stretched item's cross size is clamped by its
/// `max-*` / `min-*`; one with a definite cross size, or an `auto` cross
/// margin (§8.1), is not stretched — the margins take the free space.
#[test]
fn stretch_respects_limits_and_auto_margins() {
    assert_eq!(one(".i { max-height: 3 }"), (0, 3));
    assert_eq!(one(".i { height: 2 }"), (0, 2));
    assert_eq!(one(".i { margin-top: auto }"), (5, 1));
    assert_eq!(one(".i { margin-top: auto; margin-bottom: auto }"), (2, 1));
    // An `auto` margin beats `align-self`.
    assert_eq!(
        one(".i { margin-bottom: auto; align-self: flex-end }"),
        (0, 1)
    );
}

/// §8.3 `baseline`: the participating items of a line are shifted so
/// their first baselines — their first content rows (C5G-ATOM-BOX's
/// rows) — share a row, the item with the most rows above it flush with
/// cross-start; `last baseline` aligns the last content rows, the group
/// flush with cross-end.
#[test]
fn baseline_alignment() {
    let r = items(
        ".f { align-items: baseline } .a { padding-top: 2 } .c { padding-top: 1 }",
        &["a", "b", "c"],
    );
    assert_eq!(r.iter().map(|r| r.1).collect::<Vec<_>>(), [0, 2, 1]);
    assert_eq!(r.iter().map(|r| r.3).collect::<Vec<_>>(), [3, 1, 2]);
    let r = items(
        ".f { align-items: last baseline } .a { padding-bottom: 2 }",
        &["a", "b"],
    );
    assert_eq!(r.iter().map(|r| r.1).collect::<Vec<_>>(), [3, 3]);
    let r = items(".f { align-items: first baseline }", &["", ""]);
    assert_eq!(r.iter().map(|r| r.1).collect::<Vec<_>>(), [0, 0]);
}

/// §9.4 step 8: in a multi-line container a line holding baseline-
/// aligned items is as tall as their aligned extent, which can exceed
/// its tallest item — and the container's `auto` height follows (§9.9).
#[test]
fn baseline_alignment_sizes_a_line() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let f = el(&mut dom, root, "div", "f");
    let a = el(&mut dom, f, "div", "i a");
    let b = el(&mut dom, f, "div", "i b");
    for n in [a, b] {
        let t = dom.create_text_node("x");
        dom.append_child(n, t).unwrap();
    }
    lay_out(
        &mut dom,
        ".f { display: flex; flex-wrap: wrap; align-items: baseline; width: 12 } \
         .i { width: 2 } .a { padding-top: 2 } .b { padding-bottom: 2 }",
        20,
        10,
    );
    assert_eq!((rect(&dom, a).y, rect(&dom, b).y), (0, 2));
    assert_eq!(size(&dom, f).1, 5);
}

/// Box Alignment §4.4: an item larger than its line overflows as asked
/// (`center` both ways, the leading space rounded down) unless `safe`,
/// which aligns it as `start`.
#[test]
fn overflowing_items() {
    assert_eq!(one(".i { height: 8 } .f { align-items: center }"), (-1, 8));
    assert_eq!(
        one(".i { height: 8 } .f { align-items: unsafe center }"),
        (-1, 8)
    );
    assert_eq!(
        one(".i { height: 8 } .f { align-items: safe center }"),
        (0, 8)
    );
    assert_eq!(
        one(".i { height: 8 } .f { align-items: flex-end }"),
        (-2, 8)
    );
    assert_eq!(
        one(".i { height: 8 } .f { align-items: safe flex-end }"),
        (0, 8)
    );
}

/// §5.2: under `wrap-reverse` cross-start is the line's bottom edge, so
/// `flex-start` packs there while `start` stays the block-start (top)
/// edge (Box Alignment §4.2).
#[test]
fn wrap_reverse_flips_flex_start_not_start() {
    let w = ".f { flex-wrap: wrap-reverse }";
    assert_eq!(
        one(&format!("{w} .f {{ align-items: flex-start }}")),
        (5, 1)
    );
    assert_eq!(one(&format!("{w} .f {{ align-items: start }}")), (0, 1));
    assert_eq!(one(&format!("{w} .f {{ align-items: flex-end }}")), (0, 1));
}

/// A column's cross axis is the inline axis: `start` / `flex-start` are
/// the container's inline-start edge (the right one under `rtl`), and
/// `self-start` / `self-end` the item's own (CSS Writing Modes 4 §2.1);
/// `baseline` has no baseline on that axis and falls back to
/// `safe self-start` (Box Alignment §9.3).
#[test]
fn a_columns_items_align_on_the_inline_axis() {
    let x = |css: &str| {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let f = el(&mut dom, root, "div", "f");
        let i = el(&mut dom, f, "div", "i");
        let t = dom.create_text_node("x");
        dom.append_child(i, t).unwrap();
        lay_out(
            &mut dom,
            &format!(".f {{ display: flex; flex-direction: column; width: 10; height: 4 }} {css}"),
            20,
            10,
        );
        (rect(&dom, i).x, rect(&dom, i).width)
    };
    assert_eq!(x(".f { align-items: center }"), (4, 1));
    assert_eq!(x(".f { align-items: end }"), (9, 1));
    let rtl = ".f { direction: rtl }";
    assert_eq!(
        x(&format!("{rtl} .f {{ align-items: flex-start }}")),
        (9, 1)
    );
    assert_eq!(x(&format!("{rtl} .f {{ align-items: start }}")), (9, 1));
    assert_eq!(
        x(&format!("{rtl} .f {{ align-items: self-start }}")),
        (9, 1)
    );
    assert_eq!(
        x(&format!(
            "{rtl} .f {{ align-items: self-start }} .i {{ direction: ltr }}"
        )),
        (0, 1)
    );
    assert_eq!(
        x(&format!(
            "{rtl} .f {{ align-items: self-end }} .i {{ direction: ltr }}"
        )),
        (9, 1)
    );
    assert_eq!(x(".f { align-items: baseline }"), (0, 1));
}
