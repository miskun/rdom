//! Inline-layout end-to-end tests. Exercise `compute_inline_layout`
//! against real `Dom<TuiExt>` + `Stylesheet` via cascade.

use super::*;
use crate::TuiDom;
use crate::layout::Display;
use crate::style::{CascadeExt, Stylesheet, TuiStyle};

fn prepared(template: impl FnOnce(&mut TuiDom) -> NodeId, sheet: &Stylesheet) -> (TuiDom, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let id = template(&mut dom);
    dom.cascade(sheet);
    (dom, id)
}

fn default_sheet() -> Stylesheet {
    Stylesheet::bare()
        .rule_unchecked("p", TuiStyle::new().display(Display::Block))
        .rule_unchecked("b", TuiStyle::new().display(Display::Inline))
        .rule_unchecked("i", TuiStyle::new().display(Display::Inline))
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline))
}

#[test]
fn single_line_fits() {
    let (dom, p) = prepared(
        |dom| {
            let root = dom.root();
            let p = dom.create_element("p");
            let t = dom.create_text_node("hello");
            dom.append_child(p, t).unwrap();
            let span = dom.create_element("span");
            dom.append_child(p, span).unwrap();
            dom.append_child(root, p).unwrap();
            p
        },
        &default_sheet(),
    );

    let layout = compute_inline_layout(&dom, p, 20);
    assert_eq!(layout.height(), 1);
    assert_eq!(layout.lines[0].fragments.len(), 1);
    assert_eq!(layout.lines[0].fragments[0].text, "hello");
    assert_eq!(layout.lines[0].width, 5);
}

#[test]
fn wraps_at_word_boundary() {
    let (dom, p) = prepared(
        |dom| {
            let root = dom.root();
            let p = dom.create_element("p");
            let t = dom.create_text_node("the quick brown fox");
            dom.append_child(p, t).unwrap();
            let span = dom.create_element("span");
            dom.append_child(p, span).unwrap();
            dom.append_child(root, p).unwrap();
            p
        },
        &default_sheet(),
    );

    let layout = compute_inline_layout(&dom, p, 10);
    // "the quick " fits at 10 → hits exactly. Then "brown fox".
    // Greedy: "the quick" on line 1 (9 chars), "brown fox" on line 2 (9 chars).
    assert!(layout.height() >= 2, "must wrap");
    let line_strings: Vec<String> = layout
        .lines
        .iter()
        .map(|l| {
            l.fragments
                .iter()
                .map(|f| f.text.clone())
                .collect::<String>()
        })
        .collect();
    assert_eq!(line_strings, vec!["the quick", "brown fox"]);
}

#[test]
fn long_word_overflows_its_line() {
    let (dom, p) = prepared(
        |dom| {
            let root = dom.root();
            let p = dom.create_element("p");
            let t = dom.create_text_node("supercalifragilisticexpialidocious");
            dom.append_child(p, t).unwrap();
            let span = dom.create_element("span");
            dom.append_child(p, span).unwrap();
            dom.append_child(root, p).unwrap();
            p
        },
        &default_sheet(),
    );

    let layout = compute_inline_layout(&dom, p, 10);
    // Single word, no break opportunities → one line, wider than content.
    assert_eq!(layout.height(), 1);
    assert!(layout.lines[0].width > 10);
}

#[test]
fn cjk_breaks_between_graphemes() {
    let (dom, p) = prepared(
        |dom| {
            let root = dom.root();
            let p = dom.create_element("p");
            let t = dom.create_text_node("中文中文中文");
            dom.append_child(p, t).unwrap();
            let span = dom.create_element("span");
            dom.append_child(p, span).unwrap();
            dom.append_child(root, p).unwrap();
            p
        },
        &default_sheet(),
    );

    // 6 CJK graphemes × 2 cells = 12 total. Width 6 → 3 per line.
    let layout = compute_inline_layout(&dom, p, 6);
    assert_eq!(layout.height(), 2);
    assert_eq!(layout.lines[0].width, 6);
    assert_eq!(layout.lines[1].width, 6);
}

#[test]
fn hyphen_breaks_after() {
    let (dom, p) = prepared(
        |dom| {
            let root = dom.root();
            let p = dom.create_element("p");
            let t = dom.create_text_node("state-of-the-art");
            dom.append_child(p, t).unwrap();
            let span = dom.create_element("span");
            dom.append_child(p, span).unwrap();
            dom.append_child(root, p).unwrap();
            p
        },
        &default_sheet(),
    );

    // "state-of-the-art" = 16 chars. At width 10: "state-of-" (9) line 1,
    // "the-art" (7) line 2.
    let layout = compute_inline_layout(&dom, p, 10);
    assert!(layout.height() >= 2);
    let line_texts: Vec<String> = layout
        .lines
        .iter()
        .map(|l| {
            l.fragments
                .iter()
                .map(|f| f.text.clone())
                .collect::<String>()
        })
        .collect();
    assert_eq!(line_texts, vec!["state-of-", "the-art"]);
}

#[test]
fn fragment_owner_is_direct_element_parent() {
    // <p>a <b>bold</b> c</p>: "a " and " c" belong to p; "bold" to b.
    let (dom, p) = prepared(
        |dom| {
            let root = dom.root();
            let p = dom.create_element("p");
            let t1 = dom.create_text_node("a ");
            let b = dom.create_element("b");
            let bt = dom.create_text_node("bold");
            dom.append_child(b, bt).unwrap();
            let t2 = dom.create_text_node(" c");
            dom.append_child(p, t1).unwrap();
            dom.append_child(p, b).unwrap();
            dom.append_child(p, t2).unwrap();
            dom.append_child(root, p).unwrap();
            p
        },
        &default_sheet(),
    );

    let layout = compute_inline_layout(&dom, p, 20);
    let line = &layout.lines[0];
    // Fragments: [p:"a ", b:"bold", p:" c"] (3 fragments).
    assert_eq!(line.fragments.len(), 3);
    assert_eq!(line.fragments[0].text, "a ");
    assert_eq!(line.fragments[1].text, "bold");
    assert_eq!(line.fragments[2].text, " c");
    // b's fragment node differs from p's.
    assert_ne!(line.fragments[0].node, line.fragments[1].node);
    assert_eq!(line.fragments[0].node, line.fragments[2].node);
}

#[test]
fn wrap_preserves_fragment_ownership() {
    // <p>aaa <b>bbb ccc</b> ddd</p> wrapping so <b> spans two lines.
    let (dom, p) = prepared(
        |dom| {
            let root = dom.root();
            let p = dom.create_element("p");
            let t1 = dom.create_text_node("aaa ");
            let b = dom.create_element("b");
            let bt = dom.create_text_node("bbb ccc");
            dom.append_child(b, bt).unwrap();
            let t2 = dom.create_text_node(" ddd");
            dom.append_child(p, t1).unwrap();
            dom.append_child(p, b).unwrap();
            dom.append_child(p, t2).unwrap();
            dom.append_child(root, p).unwrap();
            p
        },
        &default_sheet(),
    );

    // Width 8: "aaa bbb " (7 chars, fits ≤8) ... actually "aaa" (3) + " bbb" (4) = 7. + " ccc" (4)=11 > 8, wrap.
    // Line 1: "aaa bbb"; Line 2: "ccc ddd"
    let layout = compute_inline_layout(&dom, p, 8);
    let line1 = &layout.lines[0];
    let line2 = &layout.lines[1];
    let l1_text: String = line1.fragments.iter().map(|f| f.text.clone()).collect();
    let l2_text: String = line2.fragments.iter().map(|f| f.text.clone()).collect();
    assert_eq!(l1_text, "aaa bbb");
    assert_eq!(l2_text, "ccc ddd");
    // Line 2: "ccc" is b's, " ddd" is p's — different owners.
    let b_on_line2: Vec<_> = line2
        .fragments
        .iter()
        .filter(|f| f.text.contains("ccc"))
        .collect();
    assert_eq!(b_on_line2.len(), 1);
}

#[test]
fn whitespace_collapse_during_pack() {
    let (dom, p) = prepared(
        |dom| {
            let root = dom.root();
            let p = dom.create_element("p");
            let t = dom.create_text_node("   foo    bar   ");
            dom.append_child(p, t).unwrap();
            let span = dom.create_element("span");
            dom.append_child(p, span).unwrap();
            dom.append_child(root, p).unwrap();
            p
        },
        &default_sheet(),
    );

    let layout = compute_inline_layout(&dom, p, 20);
    assert_eq!(layout.height(), 1);
    let text: String = layout.lines[0]
        .fragments
        .iter()
        .map(|f| f.text.clone())
        .collect();
    assert_eq!(text, "foo bar");
}

#[test]
fn pre_mode_treats_whole_text_as_one_word() {
    // Pre mode: no collapse, no soft breaks → one line, overflows
    // if wider than content_width. (Hard break on \n is Phase E.)
    let (dom, p) = prepared(
        |dom| {
            let root = dom.root();
            let p = dom.create_element("p");
            let t = dom.create_text_node("hello world");
            dom.append_child(p, t).unwrap();
            let span = dom.create_element("span");
            dom.append_child(p, span).unwrap();
            dom.append_child(root, p).unwrap();
            p
        },
        &Stylesheet::bare()
            .rule_unchecked(
                "p",
                TuiStyle::new()
                    .display(Display::Block)
                    .white_space(WhiteSpace::Pre),
            )
            .rule_unchecked("span", TuiStyle::new().display(Display::Inline)),
    );

    let layout = compute_inline_layout(&dom, p, 5);
    // Pre: "hello world" is ONE word, overflows on one line.
    assert_eq!(layout.height(), 1);
    assert_eq!(layout.lines[0].width, 11);
}

#[test]
fn pre_wrap_mode_wraps_at_whitespace_and_preserves_spaces() {
    // PreWrap soft-wraps at spaces (like Normal) but keeps the space
    // character visible (like Pre). Long text overflowing
    // content_width should produce multiple lines.
    let (dom, p) = prepared(
        |dom| {
            let root = dom.root();
            let p = dom.create_element("p");
            let t = dom.create_text_node("hello world foo");
            dom.append_child(p, t).unwrap();
            let span = dom.create_element("span");
            dom.append_child(p, span).unwrap();
            dom.append_child(root, p).unwrap();
            p
        },
        &Stylesheet::bare()
            .rule_unchecked(
                "p",
                TuiStyle::new()
                    .display(Display::Block)
                    .white_space(WhiteSpace::PreWrap),
            )
            .rule_unchecked("span", TuiStyle::new().display(Display::Inline)),
    );

    let layout = compute_inline_layout(&dom, p, 8);
    // "hello world foo" doesn't fit on one 8-cell line — wraps.
    assert!(layout.height() >= 2);
}

#[test]
fn pre_wrap_mode_preserves_explicit_newlines_as_hard_breaks() {
    // PreWrap treats `\n` as a hard break, same as Pre. Even if the
    // content would fit horizontally, the newline still forces a
    // new line.
    let (dom, p) = prepared(
        |dom| {
            let root = dom.root();
            let p = dom.create_element("p");
            let t = dom.create_text_node("a\nb");
            dom.append_child(p, t).unwrap();
            let span = dom.create_element("span");
            dom.append_child(p, span).unwrap();
            dom.append_child(root, p).unwrap();
            p
        },
        &Stylesheet::bare()
            .rule_unchecked(
                "p",
                TuiStyle::new()
                    .display(Display::Block)
                    .white_space(WhiteSpace::PreWrap),
            )
            .rule_unchecked("span", TuiStyle::new().display(Display::Inline)),
    );

    let layout = compute_inline_layout(&dom, p, 20);
    // Two lines: "a", "b".
    assert_eq!(layout.height(), 2);
}

#[test]
fn nowrap_keeps_everything_on_one_line() {
    let (dom, p) = prepared(
        |dom| {
            let root = dom.root();
            let p = dom.create_element("p");
            let t = dom.create_text_node("the quick brown fox");
            dom.append_child(p, t).unwrap();
            let span = dom.create_element("span");
            dom.append_child(p, span).unwrap();
            dom.append_child(root, p).unwrap();
            p
        },
        &Stylesheet::bare()
            .rule_unchecked(
                "p",
                TuiStyle::new()
                    .display(Display::Block)
                    .white_space(WhiteSpace::NoWrap),
            )
            .rule_unchecked("span", TuiStyle::new().display(Display::Inline)),
    );

    let layout = compute_inline_layout(&dom, p, 10);
    assert_eq!(layout.height(), 1);
    // NoWrap collapses whitespace but never wraps → one line,
    // width = 19 (overflows content_width=10).
    assert_eq!(layout.lines[0].width, 19);
}
