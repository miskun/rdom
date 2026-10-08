//! C12-OUTLINE — CSS UI 4 §5: `outline`, `outline-style` (`auto`
//! included), `outline-width`, `outline-color` and `outline-offset`. An
//! outline is a ring drawn outside the border box that takes no room
//! (§5: "outlines do not take up space"), painted after the content of
//! its stacking context (CSS 2.1 Appendix E step 10) and clipped like the
//! box's content by an `overflow` ancestor.

use super::{cell, el, paint, rows};
use rdom_tui::{Color, TuiDom, TuiNodeExt};

/// One four-cell box with text, one cell in from every edge.
fn boxed(css: &str) -> Vec<String> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    el(&mut dom, root, "b", "abcd");
    let buf = paint(
        &mut dom,
        &format!(".b {{ width: 4; margin: 1 1 }} {css}"),
        8,
        3,
    );
    rows(&buf)
}

/// §5.1–§5.2: `outline: solid` draws a one-cell ring just outside the
/// border box, in the light single-line set (`medium` is light).
#[test]
fn an_outline_draws_a_ring_outside_the_border_box() {
    assert_eq!(
        boxed(".b { outline: solid }"),
        ["┌────┐", "│abcd│", "└────┘"]
    );
}

/// §5: the outline takes no room — the box and its sibling are laid out
/// where they are without one.
#[test]
fn an_outline_takes_no_room() {
    let layout = |css: &str| {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let a = el(&mut dom, root, "a", "aa");
        let c = el(&mut dom, root, "c", "cc");
        paint(&mut dom, css, 8, 4);
        (dom.node(a).layout_rect(), dom.node(c).layout_rect())
    };
    assert_eq!(
        layout(".a { outline: solid; outline-offset: 2 }"),
        layout(""),
    );
}

/// Appendix E step 10: the ring is drawn after the content of its
/// stacking context, so it covers the next box's text.
#[test]
fn an_outline_paints_over_the_next_box() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    el(&mut dom, root, "a", "aaaa");
    el(&mut dom, root, "c", "cccccc");
    let buf = paint(
        &mut dom,
        ".a { width: 4; margin: 1 1 0 } .c { width: 6 } .a { outline: solid }",
        8,
        3,
    );
    assert_eq!(rows(&buf), ["┌────┐", "│aaaa│", "└────┘"]);
}

/// §5.4: `outline-offset` moves the ring out (or, negative, in) by whole
/// cells; a pixel offset moves it one cell its way (DESIGN "Pixel lengths
/// select, cells measure").
#[test]
fn outline_offset_moves_the_ring() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    el(&mut dom, root, "b", "ab");
    let buf = paint(
        &mut dom,
        ".b { width: 2; margin: 2 2; outline: solid; outline-offset: 1 }",
        6,
        5,
    );
    assert_eq!(
        rows(&buf),
        ["┌────┐", "│    │", "│ ab │", "│    │", "└────┘"]
    );
    let mut dom = TuiDom::new();
    let root = dom.root();
    el(&mut dom, root, "b", "ab");
    let px = paint(
        &mut dom,
        ".b { width: 2; margin: 2 2; outline: solid; outline-offset: 2px }",
        6,
        5,
    );
    assert_eq!(rows(&px), rows(&buf), "2px is one cell out");
    // Negative: inside the border box, over its own content.
    assert_eq!(
        boxed(".b { width: 4; height: 3; margin: 0; outline: solid; outline-offset: -1 }"),
        ["┌──┐", "│  │", "└──┘"]
    );
}

/// §5.2–§5.3: the line styles draw from the border glyph set — `double`,
/// `dashed` on its straight runs, heavy from a `thick` or a 5px width.
#[test]
fn outline_styles_and_widths_pick_the_border_glyphs() {
    assert_eq!(
        boxed(".b { outline: double }"),
        ["╔════╗", "║abcd║", "╚════╝"]
    );
    assert_eq!(
        boxed(".b { outline: dashed }"),
        ["┌╌╌╌╌┐", "╎abcd╎", "└╌╌╌╌┘"]
    );
    assert_eq!(
        boxed(".b { outline: 5px solid }"),
        ["┏━━━━┓", "┃abcd┃", "┗━━━━┛"]
    );
    assert_eq!(
        boxed(".b { outline: thick solid }"),
        boxed(".b { outline: 5px solid }")
    );
}

/// §5.2 `auto`: the UA's focus-ring look — a light ring with rounded
/// corners in the accent color (`outline-color: auto`, §5.3); with a
/// line style, `auto` is `currentcolor`.
#[test]
fn outline_auto_draws_the_focus_ring() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    el(&mut dom, root, "b", "abcd");
    let buf = paint(
        &mut dom,
        ".b { width: 4; margin: 1 1; color: red; outline: auto }",
        8,
        3,
    );
    assert_eq!(rows(&buf), ["╭────╮", "│abcd│", "╰────╯"]);
    let accent = Color::Rgb(30, 144, 255);
    assert_eq!(cell(&buf, 0, 0).fg, accent);
    assert_eq!(cell(&buf, 0, 1).fg, accent);
    let mut dom = TuiDom::new();
    let root = dom.root();
    el(&mut dom, root, "b", "abcd");
    let buf = paint(
        &mut dom,
        ".b { width: 4; margin: 1 1; color: red; outline: solid }",
        8,
        3,
    );
    assert_eq!(cell(&buf, 0, 0).fg, Color::Rgb(255, 0, 0), "currentcolor");
    let mut dom = TuiDom::new();
    let root = dom.root();
    el(&mut dom, root, "b", "abcd");
    let buf = paint(
        &mut dom,
        ".b { width: 4; margin: 1 1; outline: solid lime }",
        8,
        3,
    );
    assert_eq!(cell(&buf, 5, 2).fg, Color::Rgb(0, 255, 0));
}

/// §5.2–§5.3: `none` (the initial style) and a zero width draw nothing.
#[test]
fn no_style_or_no_width_draws_nothing() {
    let blank = ["", " abcd", ""];
    assert_eq!(boxed(""), blank);
    assert_eq!(boxed(".b { outline: none }"), blank);
    assert_eq!(boxed(".b { outline: 0 solid }"), blank);
}

/// CSS Overflow 3 §3: an `overflow: hidden` ancestor clips the outline as
/// it clips the box's content (CSS UI 4 §5).
#[test]
fn an_overflow_ancestor_clips_the_outline() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "p", "");
    el(&mut dom, p, "b", "ab");
    let buf = paint(
        &mut dom,
        ".p { width: 4; height: 2; margin: 1; overflow: hidden } \
         .b { width: 2; outline: solid }",
        8,
        4,
    );
    // The ring's left and top run outside `.p` and are clipped; its
    // right edge is inside and drawn.
    assert_eq!(rows(&buf), ["", " ab│", " ──┘", ""]);
}

/// Appendix E step 10: outlines come after the context's positioned
/// descendants — a `z-index: 1` box over the ring does not hide it.
#[test]
fn an_outline_is_drawn_after_positioned_descendants() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    el(&mut dom, root, "b", "abcd");
    el(&mut dom, root, "over", "xxxxxxxx");
    let buf = paint(
        &mut dom,
        ".b { width: 4; margin: 1 1; outline: solid } \
         .over { position: absolute; top: 0; left: 0; z-index: 1 }",
        8,
        3,
    );
    assert_eq!(rows(&buf)[0], "┌────┐xx");
}

// ── C12G-OUTLINE-INLINE ───────────────────────────────────────────

/// `<div class=p>ab <span class=f>link</span> cd</div>`, one cell in.
fn inline(css: &str, w: u16, h: u16) -> Vec<String> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "p", "ab ");
    let span = dom.create_element("span");
    dom.set_attribute(span, "class", "f").unwrap();
    let t = dom.create_text_node("link");
    dom.append_child(span, t).unwrap();
    dom.append_child(p, span).unwrap();
    let tail = dom.create_text_node(" cd");
    dom.append_child(p, tail).unwrap();
    rows(&paint(
        &mut dom,
        &format!(".p {{ margin: 1 1 }} {css}"),
        w,
        h,
    ))
}

/// CSS UI 4 §5: an outline is drawn around an inline box too — the
/// `a:focus-visible { outline: auto }` DIVERGENCES `FOCUS-VOCAB-1`
/// recommends rings a link in a line. The ring is the row and column of
/// cells just outside the inline box's fragment, over the neighboring
/// text, taking no room.
#[test]
fn an_inline_element_draws_its_outline() {
    assert_eq!(
        inline(".f { outline: solid }", 14, 3),
        ["   ┌────┐", " ab│link│cd", "   └────┘"]
    );
}

/// §5.1: an inline box broken across lines has an outline around each
/// fragment — rdom draws one rectangle per line box (the spec lets the
/// outline of a fragmented inline be non-rectangular; DIVERGENCES §1).
/// `ab <span>cd ef</span>` in six columns breaks the span after `cd`;
/// three-row lines leave room for both rings.
#[test]
fn a_wrapped_inline_element_rings_each_line_fragment() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "p", "ab ");
    let span = dom.create_element("span");
    dom.set_attribute(span, "class", "f").unwrap();
    let t = dom.create_text_node("cd ef");
    dom.append_child(span, t).unwrap();
    dom.append_child(p, span).unwrap();
    let got = rows(&paint(
        &mut dom,
        ".p { width: 6; line-height: 3; margin: 0 1 } .f { outline: solid }",
        10,
        7,
    ));
    assert_eq!(
        got,
        ["   ┌──┐", " ab│cd│", "   └──┘", "┌──┐", "│ef│", "└──┘", ""],
        "{got:#?}"
    );
}
