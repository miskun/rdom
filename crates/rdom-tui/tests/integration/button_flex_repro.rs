//! Regression: `<button>` UA bracket pseudos must render even
//! when the button has an inline-element sibling in a flex row.
//!
//! Surfaced by M8's `interval_counter` demo: a button + span in a
//! flex row was being routed through the IFC paint path, which
//! packs text fragments but doesn't render inline-block elements'
//! UA `::before` / `::after` pseudos. Fixed in `ifc.rs` by
//! treating `Display::InlineBlock` as an IFC disqualifier — the
//! container now falls through to regular flex layout, where the
//! button is a flex item with its full pseudo chrome.
//!
//! See `IFC-INLINEBLOCK-PSEUDOS-1` in `specs/TECH_DEBT.md` for
//! the deeper gap that remains (mixed `<p>text <button>btn</button>
//! text</p>` content alongside inline elements is still unsupported).
use crate::common::{buffer_to_snapshot, render};
use rdom_tui::prelude::*;

fn snapshot_contains(snap: &str, needle: &str) -> bool {
    snap.lines().any(|line| line.contains(needle))
}

#[test]
fn button_brackets_render_with_inline_sibling_in_flex_row() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let row = dom.create_element("div");
    let btn = dom.create_element("button");
    let bt = dom.create_text_node("Start");
    dom.append_child(btn, bt).unwrap();
    dom.append_child(row, btn).unwrap();
    let span = dom.create_element("span");
    let st = dom.create_text_node("0");
    dom.append_child(span, st).unwrap();
    dom.append_child(row, span).unwrap();
    dom.append_child(root, row).unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        "div",
        TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
    );
    let buf = render(&mut dom, &sheet, Rect::new(0, 0, 30, 3));
    let snap = buffer_to_snapshot(&buf);

    assert!(
        snapshot_contains(&snap, "[ Start ]"),
        "button's UA brackets must render with inline sibling in flex row.\n{snap}"
    );
    assert!(
        snapshot_contains(&snap, "0"),
        "the inline span's text must still render.\n{snap}"
    );
    // Suppress unused warnings for the named element handles.
    let _ = (btn, span, row);
}

#[test]
fn button_brackets_render_alone_in_flex_row() {
    // Baseline: button as the sole child of a flex row.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let row = dom.create_element("div");
    let btn = dom.create_element("button");
    let t = dom.create_text_node("Start");
    dom.append_child(btn, t).unwrap();
    dom.append_child(row, btn).unwrap();
    dom.append_child(root, row).unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        "div",
        TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
    );
    let buf = render(&mut dom, &sheet, Rect::new(0, 0, 20, 3));
    let snap = buffer_to_snapshot(&buf);

    assert!(
        snapshot_contains(&snap, "[ Start ]"),
        "button's UA brackets must render when alone in flex row.\n{snap}"
    );
}

#[test]
fn two_buttons_in_flex_row_both_show_brackets() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let row = dom.create_element("div");
    for label in ["Start", "Stop"] {
        let btn = dom.create_element("button");
        let t = dom.create_text_node(label);
        dom.append_child(btn, t).unwrap();
        dom.append_child(row, btn).unwrap();
    }
    dom.append_child(root, row).unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        "div",
        TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
    );
    let buf = render(&mut dom, &sheet, Rect::new(0, 0, 30, 3));
    let snap = buffer_to_snapshot(&buf);

    assert!(
        snapshot_contains(&snap, "[ Start ]"),
        "first button's brackets missing.\n{snap}"
    );
    assert!(
        snapshot_contains(&snap, "[ Stop ]"),
        "second button's brackets missing.\n{snap}"
    );
}

#[test]
fn ifc_still_fires_for_plain_inline_only_content() {
    // Negative test: a <p> with text + inline <span> only (no
    // inline-block child) should STILL route through IFC so the
    // text + span flow as one paragraph. We assert by checking
    // that the inline_layout is populated on the <p>.
    use rdom_core::Dom;
    use rdom_tui::ext::TuiExt;
    use rdom_tui::layout::Display;

    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("Hello ");
    dom.append_child(p, t).unwrap();
    let span = dom.create_element("span");
    let st = dom.create_text_node("world");
    dom.append_child(span, st).unwrap();
    dom.append_child(p, span).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("p", TuiStyle::new())
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));
    let _ = render(&mut dom, &sheet, Rect::new(0, 0, 30, 3));

    let inline_layout = (&dom as &Dom<TuiExt>)
        .node(p)
        .ext()
        .and_then(|e| e.inline_layout.as_ref());
    assert!(
        inline_layout.is_some(),
        "plain text + inline-span paragraph should still establish an IFC",
    );
}
