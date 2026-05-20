//! UA chrome showcase — what naked HTML built-ins look like with
//! rdom's UA stylesheet only.
//!
//! Demonstrates the UA polish landed in the OOTB round:
//! - `<button>` bracket chrome + `:focus` indicator
//! - `<details>` / `<summary>` disclosure triangle (▶ collapsed, ▼ open)
//! - `<ul>` / `<li>` bullet markers (• via `ul > li::before`)
//! - `<dialog open>` modal chrome (border + padding)
//!
//! The only author CSS is the structural `<screen>` shell that
//! gives the demo padding + gap between sections. Every native
//! HTML element below renders with UA defaults alone — exactly
//! what an author gets out of the box, before they write any of
//! their own CSS.
//!
//! Controls:
//!   Tab / Shift-Tab  — cycle focus through interactive elements.
//!   Click            — toggle `<details>`, focus a button.
//!   Enter / Space    — activate the focused button or summary.
//!   Ctrl-C           — quit.
//!
//! Run: `cargo run -p rdom-tui --example ua_chrome`

use std::io;

use rdom_tui::prelude::*;

fn main() -> io::Result<()> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();

    let screen = dom.create_element("screen");
    dom.append_child(root, screen).unwrap();

    // ── Title ──
    let h1 = dom.create_element("h1");
    let h1_t = dom.create_text_node("rdom UA chrome — pure defaults, no author CSS");
    dom.append_child(h1, h1_t).unwrap();
    dom.append_child(screen, h1).unwrap();

    // ── Section: <button> ──
    section(&mut dom, screen, "Buttons", |dom, sec| {
        for label in ["Save", "Cancel", "Continue"] {
            let btn = dom.create_element("button");
            let t = dom.create_text_node(label);
            dom.append_child(btn, t).unwrap();
            dom.append_child(sec, btn).unwrap();
        }
    });

    // ── Section: <ul> + <ol> ──
    // Both render with `• ` bullets in 0.1.0 — `<ol>` gets the
    // same UA marker until CSS counters land (`UA-OL-1`).
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

    // ── Section: <details> / <summary> ──
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

    // ── Section: <dialog> ──
    section(&mut dom, screen, "Dialog (always open)", |dom, sec| {
        let dialog = dom.create_element("dialog");
        dom.set_attribute(dialog, "open", "").unwrap();
        let d_t = dom.create_text_node("Native dialog chrome: border + padding.");
        dom.append_child(dialog, d_t).unwrap();
        dom.append_child(sec, dialog).unwrap();
    });

    // Author CSS — STRUCTURAL ONLY. `<screen>` is a custom layout
    // shell; everything inside it uses UA defaults.
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

    App::new(dom, sheet)?.run()
}

/// Add a labeled `<section>` to `parent`, populated by `build`.
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
