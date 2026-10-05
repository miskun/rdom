//! C5-INTRINSIC — `min-content` / `max-content` / `fit-content` /
//! `fit-content(<length-percentage>)` on `width`, `height`, `min-*` and
//! `max-*` (CSS Sizing 3 §3.1–§3.3, §5.1).

use super::{el, lay_out, size};
use rdom_tui::{NodeId, TuiDom};

/// A `div.b` holding `text`, a child of `parent`.
fn text_box(dom: &mut TuiDom, parent: NodeId, class: &str, text: &str) -> NodeId {
    let b = el(dom, parent, "div", class);
    let t = dom.create_text_node(text);
    dom.append_child(b, t).unwrap();
    b
}

/// `div.b` with `text`, a child of the root, laid out under `css` in
/// 30 × 12.
fn one(css: &str, text: &str) -> (TuiDom, NodeId) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let wrap = el(&mut dom, root, "div", "wrap");
    let b = text_box(&mut dom, wrap, "b", text);
    lay_out(&mut dom, css, 30, 12);
    (dom, b)
}

// ── block width ────────────────────────────────────────────────────

/// CSS Sizing 3 §3.1 `max-content`: the box's max-content inline size —
/// its text unwrapped — plus padding and border (the keyword sizes the
/// content box, CSS UI 3 §3.1 adds the rest).
#[test]
fn max_content_width_is_the_unwrapped_text() {
    let (dom, b) = one(".b { width: max-content }", "aaa bbb");
    assert_eq!(size(&dom, b), (7, 1));
    let (dom, b) = one(".b { width: max-content; padding: 0 1 }", "aaa bbb");
    assert_eq!(size(&dom, b).0, 9);
}

/// CSS Sizing 3 §3.1 `min-content`: the min-content inline size — the
/// longest unbreakable word (§4.1) — and the text wraps to it.
#[test]
fn min_content_width_is_the_longest_word() {
    let (dom, b) = one(".b { width: min-content }", "aaa bbbbb cc");
    assert_eq!(size(&dom, b), (5, 3));
}

/// CSS Sizing 3 §3.1 `fit-content`: `min(max-content, max(min-content,
/// stretch-fit))` — the content's width while it fits the containing
/// block, the containing block's width once it does not.
#[test]
fn fit_content_width_shrink_wraps_up_to_the_available_space() {
    let (dom, b) = one(".b { width: fit-content }", "aaa bbb");
    assert_eq!(size(&dom, b).0, 7);
    let (dom, b) = one(
        ".wrap { width: 10 } .b { width: fit-content }",
        "aaaa bbbb cccc dddd",
    );
    assert_eq!(size(&dom, b), (10, 2));
}

/// CSS Sizing 3 §3.1 `fit-content(<length-percentage>)`: `min(max-content,
/// max(min-content, <limit>))` — the limit caps the max-content size but
/// never goes below the min-content size; a percentage is of the
/// containing block.
#[test]
fn fit_content_with_a_limit() {
    let (dom, b) = one(".b { width: fit-content(10) }", "aaa bbb");
    assert_eq!(size(&dom, b).0, 7);
    let (dom, b) = one(".b { width: fit-content(10) }", "aaaa bbbb cccc");
    assert_eq!(size(&dom, b), (10, 2));
    let (dom, b) = one(".b { width: fit-content(2) }", "aaaa bb");
    assert_eq!(size(&dom, b).0, 4);
    let (dom, b) = one(".b { width: fit-content(50%) }", "aaaa bbbb cccc dddd eeee");
    assert_eq!(size(&dom, b).0, 15);
}

/// CSS Sizing 3 §3.2 / §3.3: the keywords bound a size as well —
/// `min-width: min-content` keeps a narrow box from cutting a word,
/// `max-width: max-content` shrink-wraps an `auto` width.
#[test]
fn min_and_max_width_take_the_keywords() {
    let (dom, b) = one(".b { width: 2; min-width: min-content }", "abcd ef");
    assert_eq!(size(&dom, b).0, 4);
    let (dom, b) = one(".b { max-width: max-content }", "aaa bbb");
    assert_eq!(size(&dom, b).0, 7);
    let (dom, b) = one(".b { max-width: fit-content(4) }", "aaa bbb");
    assert_eq!(size(&dom, b).0, 4);
}

// ── flex items ─────────────────────────────────────────────────────

/// CSS Flexbox §9.2 with CSS Sizing 3 §3.1: an item's definite main size
/// may be an intrinsic keyword — its flex base size is that content size.
#[test]
fn flex_item_main_size_takes_the_keywords() {
    let css = ".wrap { display: flex; flex-direction: row; width: 30; height: 4 } \
               .b { width: max-content; flex-shrink: 0 } .c { width: min-content }";
    let mut dom = TuiDom::new();
    let root = dom.root();
    let wrap = el(&mut dom, root, "div", "wrap");
    let b = text_box(&mut dom, wrap, "b", "hello world");
    let c = text_box(&mut dom, wrap, "c", "abc defgh");
    lay_out(&mut dom, css, 30, 12);
    assert_eq!(size(&dom, b).0, 11);
    assert_eq!(size(&dom, c).0, 5);
}

/// CSS Flexbox §9.4: only an `auto` cross size stretches; a keyword
/// cross size is the content's (a column's items keep their text width,
/// a row's items their content height).
#[test]
fn flex_item_cross_size_keywords_do_not_stretch() {
    let css = ".wrap { display: flex; flex-direction: column; width: 30; height: 8 } \
               .b { width: fit-content }";
    let mut dom = TuiDom::new();
    let root = dom.root();
    let wrap = el(&mut dom, root, "div", "wrap");
    let b = text_box(&mut dom, wrap, "b", "abc def");
    lay_out(&mut dom, css, 30, 12);
    assert_eq!(size(&dom, b).0, 7);

    let css = ".wrap { display: flex; flex-direction: row; width: 30; height: 8 } \
               .b { width: 3; height: max-content }";
    let mut dom = TuiDom::new();
    let root = dom.root();
    let wrap = el(&mut dom, root, "div", "wrap");
    let b = text_box(&mut dom, wrap, "b", "abc def");
    lay_out(&mut dom, css, 30, 12);
    assert_eq!(size(&dom, b), (3, 2));
}

// ── the block axis ─────────────────────────────────────────────────

/// CSS Sizing 3 §3.1: for a box's block size the keywords are
/// "equivalent to its automatic size" — the content height at the box's
/// width; and as bounds, `min-height: min-content` keeps a too-short
/// fixed height from cutting the content.
#[test]
fn block_axis_keywords_are_the_content_height() {
    for kw in [
        "min-content",
        "max-content",
        "fit-content",
        "fit-content(1)",
    ] {
        let (dom, b) = one(&format!(".b {{ width: 4; height: {kw} }}"), "aaa bbb ccc");
        assert_eq!(size(&dom, b), (4, 3), "{kw}");
    }
    let (dom, b) = one(
        ".b { width: 4; height: 1; min-height: min-content }",
        "aaa bbb",
    );
    assert_eq!(size(&dom, b).1, 2);
    let (dom, b) = one(
        ".b { width: 4; height: 9; max-height: max-content }",
        "aaa bbb",
    );
    assert_eq!(size(&dom, b).1, 2);
}

// ── positioned boxes ───────────────────────────────────────────────

/// CSS 2.1 §10.3.7 with CSS Sizing 3 §3.1: a positioned box with a
/// keyword width is that content width, even between two insets.
#[test]
fn positioned_box_takes_the_keywords() {
    let (dom, b) = one(
        ".b { position: absolute; left: 0; right: 0; top: 0; width: min-content }",
        "aa bbb",
    );
    assert_eq!(size(&dom, b), (3, 2));
}

/// The keywords are reachable from Rust through the prelude: a node
/// setter with `Size::Intrinsic` lays out like the CSS.
#[test]
fn keywords_from_rust_through_the_prelude() {
    use rdom_tui::prelude::*;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let wrap = el(&mut dom, root, "div", "wrap");
    let b = text_box(&mut dom, wrap, "b", "aaa bbbbb");
    dom.node_mut(b)
        .set_width(Size::Intrinsic(IntrinsicSize::MinContent))
        .set_max_height(MaxSize::Intrinsic(IntrinsicSize::MaxContent));
    lay_out(&mut dom, "", 30, 12);
    assert_eq!(size(&dom, b), (5, 2));
}
