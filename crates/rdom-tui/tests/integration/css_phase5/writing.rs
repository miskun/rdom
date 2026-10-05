//! C5-WRITING — `direction` (CSS Writing Modes 4 §2.1) and
//! `writing-mode` (§3.1). `rtl` puts the inline-start edge on the right;
//! the vertical writing modes compute but lay out as `horizontal-tb`
//! (DIVERGENCES). No bidi reordering (DIVERGENCES).

use super::{el, lay_out, paint, rect, rows, size};
use rdom_tui::layout::{TextDirection, WritingMode};
use rdom_tui::{NodeId, TuiDom, TuiNodeExt};

fn text(dom: &mut TuiDom, parent: NodeId, s: &str) {
    let t = dom.create_text_node(s);
    dom.append_child(parent, t).unwrap();
}

// ── the cascade ────────────────────────────────────────────────────

/// The HTML rendering section's 'Bidirectional text' rules: `[dir=rtl]` and
/// `[dir=ltr]` set `direction` (the attribute value is ASCII
/// case-insensitive); `direction` and `writing-mode` inherit (CSS
/// Writing Modes 4 §2.1 / §3.1).
#[test]
fn dir_attribute_sets_direction_and_both_inherit() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let outer = el(&mut dom, root, "div", "outer");
    dom.set_attribute(outer, "dir", "RTL").unwrap();
    let inner = el(&mut dom, outer, "div", "inner");
    let ltr = el(&mut dom, outer, "div", "");
    dom.set_attribute(ltr, "dir", "ltr").unwrap();
    lay_out(&mut dom, ".outer { writing-mode: vertical-rl }", 20, 5);
    let c = |id| dom.node(id).computed().unwrap().clone();
    assert_eq!(c(outer).text_direction, TextDirection::Rtl);
    assert_eq!(c(inner).text_direction, TextDirection::Rtl, "inherited");
    assert_eq!(c(ltr).text_direction, TextDirection::Ltr);
    assert_eq!(c(inner).writing_mode, WritingMode::VerticalRl, "inherited");
}

// ── inline flow ────────────────────────────────────────────────────

/// CSS Writing Modes 4 §2.1: under `rtl` the inline-start edge is the
/// right one, so each line starts there (CSS Text 3 §7.1: `text-align:
/// start`, the initial value). The characters keep their order — rdom
/// does not reorder bidi text (DIVERGENCES), which for left-to-right
/// text is what a browser shows too.
#[test]
fn rtl_lines_start_at_the_right_edge() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", "p");
    text(&mut dom, p, "ab cd efgh");
    let buf = paint(&mut dom, ".p { direction: rtl; width: 6 }", 6, 3);
    assert_eq!(rows(&buf, 6, 2), [" ab cd", "  efgh"]);
}

// ── block layout ───────────────────────────────────────────────────

/// CSS 2.1 §10.3.3: an over-constrained block box drops `margin-right`
/// when its containing block is `ltr` and `margin-left` when it is
/// `rtl` — so a narrow block sits at the right edge.
#[test]
fn rtl_block_sits_at_the_inline_start_edge() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let wrap = el(&mut dom, root, "div", "wrap");
    let a = el(&mut dom, wrap, "div", "a");
    let b = el(&mut dom, wrap, "div", "b");
    lay_out(
        &mut dom,
        ".wrap { direction: rtl; width: 10 } .a, .b { width: 4; height: 1 } .b { margin-right: 1 }",
        20,
        5,
    );
    assert_eq!((rect(&dom, a).x, rect(&dom, b).x), (6, 5));
}

// ── flex layout ────────────────────────────────────────────────────

/// CSS Flexbox §2.1 / §5.1: a `row`'s main-start is the inline-start
/// edge — the right one under `rtl` — so items run right to left, each
/// item's main-start margin its right one.
#[test]
fn rtl_flex_row_runs_right_to_left() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let row = el(&mut dom, root, "div", "row");
    let a = el(&mut dom, row, "div", "a");
    let b = el(&mut dom, row, "div", "b");
    lay_out(
        &mut dom,
        ".row { display: flex; flex-direction: row; direction: rtl; width: 12; height: 1 } \
         .a { width: 2; margin-right: 1 } .b { width: 3 }",
        20,
        5,
    );
    assert_eq!((rect(&dom, a).x, rect(&dom, b).x), (9, 6));
    assert_eq!(size(&dom, a).0, 2);
}

/// CSS Box 4 §3 with Writing Modes 4 §2.1: `margin-trim: inline-start`
/// trims the margin on the inline-start edge — the right one in `rtl`.
#[test]
fn rtl_margin_trim_inline_start_is_the_right_edge() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let row = el(&mut dom, root, "div", "row");
    let a = el(&mut dom, row, "div", "a");
    lay_out(
        &mut dom,
        ".row { display: flex; flex-direction: row; direction: rtl; width: 10; height: 1; \
                margin-trim: inline-start } \
         .a { width: 2; margin: 0 3 }",
        20,
        5,
    );
    assert_eq!(rect(&dom, a).x, 8);
}

// ── positioned boxes ───────────────────────────────────────────────

/// CSS 2.1 §9.4.3 / §10.3.7: when both `left` and `right` are set (and,
/// for an absolutely positioned box, `width` too), the inline-end one is
/// ignored — `left` wins under `ltr`, `right` under `rtl`.
#[test]
fn rtl_positioned_boxes_keep_the_right_inset() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let wrap = el(&mut dom, root, "div", "wrap");
    let r = el(&mut dom, wrap, "div", "r");
    let a = el(&mut dom, wrap, "div", "a");
    lay_out(
        &mut dom,
        ".wrap { position: relative; width: 20; height: 5; direction: rtl } \
         .r { position: relative; left: 2; right: 3; height: 1 } \
         .a { position: absolute; left: 1; right: 2; width: 3; top: 0; height: 1 }",
        30,
        10,
    );
    assert_eq!(rect(&dom, r).x, -3);
    assert_eq!(rect(&dom, a).x, 20 - 2 - 3);
}

// ── scrollbars ─────────────────────────────────────────────────────

/// Chromium and Gecko put an `rtl` scroll container's vertical
/// scrollbar on its left (inline-start) side; rdom does the same — the
/// gutter is the left column, the content moves right by it.
#[test]
fn rtl_vertical_scrollbar_is_on_the_left() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let s = el(&mut dom, root, "div", "s");
    for _ in 0..6 {
        let line = el(&mut dom, s, "div", "line");
        text(&mut dom, line, "x");
    }
    let buf = paint(
        &mut dom,
        ".s { direction: rtl; width: 5; height: 3; overflow-y: scroll } .line { height: 1 }",
        5,
        3,
    );
    let content = dom.node(s).content_layout_rect().unwrap();
    assert_eq!((content.x, content.width), (1, 4));
    let col = |x| {
        (0..3)
            .map(|y| buf.cell(x, y).unwrap().symbol().to_string())
            .collect::<String>()
    };
    assert_ne!(col(0).trim(), "", "the track paints in column 0");
    assert_eq!(
        col(4),
        "xxx",
        "the content's inline-start is the right edge"
    );
}

// ── writing-mode ───────────────────────────────────────────────────

/// CSS Writing Modes 4 §3.1: the vertical modes compute, but rdom lays
/// every box out as `horizontal-tb` (DIVERGENCES): a `vertical-rl` box
/// is as wide and as tall as a horizontal one.
#[test]
fn vertical_writing_modes_lay_out_as_horizontal() {
    for mode in [
        "horizontal-tb",
        "vertical-rl",
        "vertical-lr",
        "sideways-rl",
        "sideways-lr",
    ] {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let p = el(&mut dom, root, "div", "p");
        text(&mut dom, p, "abc def");
        lay_out(
            &mut dom,
            &format!(".p {{ writing-mode: {mode}; width: 4 }}"),
            20,
            5,
        );
        assert_eq!(size(&dom, p), (4, 2), "{mode}");
    }
}
