//! Paint snapshot for the `ua_chrome` example. Builds the same DOM
//! the example builds, renders it through the full
//! cascade → layout → paint pipeline at a fixed viewport, and
//! compares the painted buffer against a checked-in golden file
//! at `tests/snapshots/ua_chrome.snap`.
//!
//! Catches visible regressions to the UA chrome (button brackets,
//! summary triangle, ul bullets, dialog border + padding) without
//! depending on a TTY. The DOM construction below mirrors the
//! example's `main` — if either side changes, the other must
//! follow.
//!
//! To regenerate the golden after an intentional UA / chrome
//! change:
//!
//! ```sh
//! UPDATE_SNAPSHOTS=1 cargo test -p rdom-tui --test ua_chrome_snapshot
//! ```
//!
//! Then `git diff` the snapshot to review the visual change before
//! committing.

mod common;

use rdom_tui::prelude::*;

use common::{assert_snapshot, buffer_to_snapshot, render};

#[test]
fn ua_chrome_paints_naked_native_built_ins() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();

    let screen = dom.create_element("screen");
    dom.append_child(root, screen).unwrap();

    // Title.
    let h1 = dom.create_element("h1");
    let h1_t = dom.create_text_node("rdom UA chrome — pure defaults, no author CSS");
    dom.append_child(h1, h1_t).unwrap();
    dom.append_child(screen, h1).unwrap();

    section(&mut dom, screen, "Buttons", |dom, sec| {
        for label in ["Save", "Cancel", "Continue"] {
            let btn = dom.create_element("button");
            let t = dom.create_text_node(label);
            dom.append_child(btn, t).unwrap();
            dom.append_child(sec, btn).unwrap();
        }
    });

    section(&mut dom, screen, "Lists", |dom, sec| {
        let ul = dom.create_element("ul");
        for item in ["unordered first", "unordered second"] {
            let li = dom.create_element("li");
            let t = dom.create_text_node(item);
            dom.append_child(li, t).unwrap();
            dom.append_child(ul, li).unwrap();
        }
        dom.append_child(sec, ul).unwrap();

        let ol = dom.create_element("ol");
        for item in ["ordered first", "ordered second"] {
            let li = dom.create_element("li");
            let t = dom.create_text_node(item);
            dom.append_child(li, t).unwrap();
            dom.append_child(ol, li).unwrap();
        }
        dom.append_child(sec, ol).unwrap();
    });

    section(&mut dom, screen, "Disclosure", |dom, sec| {
        let details = dom.create_element("details");
        let summary = dom.create_element("summary");
        let s_t = dom.create_text_node("Click or press Enter to toggle");
        dom.append_child(summary, s_t).unwrap();
        dom.append_child(details, summary).unwrap();
        let p = dom.create_element("p");
        let p_t = dom.create_text_node("Hidden until the disclosure is opened.");
        dom.append_child(p, p_t).unwrap();
        dom.append_child(details, p).unwrap();
        dom.append_child(sec, details).unwrap();
    });

    section(&mut dom, screen, "Dialog (always open)", |dom, sec| {
        let dialog = dom.create_element("dialog");
        dom.set_attribute(dialog, "open", "").unwrap();
        let d_t = dom.create_text_node("Native dialog chrome: border + padding.");
        dom.append_child(dialog, d_t).unwrap();
        dom.append_child(sec, dialog).unwrap();
    });

    // Author CSS — structural shell only (same as the example).
    let sheet = Stylesheet::new()
        .rule(
            "screen",
            TuiStyle::new()
                .direction(Direction::Column)
                .width(Size::Flex(1))
                .height(Size::Flex(1))
                .padding(Padding::symmetric(2, 1))
                .gap(1),
        )
        .unwrap();

    // Fixed viewport — wide enough for the dialog's chrome to land
    // within a single row's worth of border, tall enough that no
    // section gets clipped.
    let buf = render(&mut dom, &sheet, Rect::new(0, 0, 60, 30));
    let snap = buffer_to_snapshot(&buf);
    assert_snapshot(&snap, "ua_chrome.snap");
}

fn section(
    dom: &mut TuiDom,
    parent: rdom_tui::NodeId,
    title: &str,
    build: impl FnOnce(&mut TuiDom, rdom_tui::NodeId),
) {
    let sec = dom.create_element("section");
    let label = dom.create_element("h3");
    let t = dom.create_text_node(title);
    dom.append_child(label, t).unwrap();
    dom.append_child(sec, label).unwrap();
    build(dom, sec);
    dom.append_child(parent, sec).unwrap();
}
