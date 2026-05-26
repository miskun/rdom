//! Status bar — keyboard-shortcut hints driven by focus.
//!
//! The status bar lives outside the bordered `.app` panel (Phase 1a
//! relocated it to a sibling of `.app` under the `.app-shell` flex
//! column). This module Phase-1b's it from "empty 1-row strip" to
//! "contextual shortcut hints + transient scroll info."
//!
//! ## Authoring model
//!
//! Each hint is a `(key, label)` pair rendered as two adjacent
//! spans: `<span class="key">↑↓</span> <span class="label">navigate</span>`.
//! Multiple hints get separated by `<span class="sep">·</span>`.
//! The cascade styles `.key` bold + bright, `.label` muted, `.sep`
//! dim — no per-hint styling logic in this module.
//!
//! ## Resolution
//!
//! On every `focusin` (which bubbles, per UI Events §3.4), walk the
//! focused element's ancestor chain looking at `class` attributes
//! to decide which hints apply. First match wins. Falls back to a
//! global default when no class matches.
//!
//! ## Coexistence with the scroll listener
//!
//! `nav::wire_scroll_indicator` also writes into this element when
//! a descendant of `view_content` scrolls. That transient takeover
//! is intentional — scroll info is the most relevant status info
//! while the user is actively scrolling. When focus changes (or the
//! demo is swapped), the focus listener restores hints.

use rdom_tui::{ListenerOptions, NodeId, TuiDom};

/// The global default hint set — shown when no focused element's
/// ancestor chain carries a more specific class. Phase 1b keeps
/// this aligned with the sidebar's keys (which is what gets
/// autofocused at boot), so the default reads as "what you can do
/// right now" rather than "what you could do somewhere else."
const DEFAULT_HINTS: &[(&str, &str)] = &[("↑↓", "navigate"), ("Enter", "select")];

/// Source-disclosure-specific hints — shown when the focused
/// element is anywhere inside the `<details class="source-disclosure">`.
const SOURCE_HINTS: &[(&str, &str)] = &[("Enter", "toggle source")];

/// Seed the status bar with the default hint set. Called once
/// during `build_shell`, BEFORE the App's `focus_node` machinery
/// has fired its first `focusin`.
pub fn seed_default_hints(dom: &mut TuiDom, status_bar: NodeId) {
    write_hints(dom, status_bar, DEFAULT_HINTS);
}

/// Install a `focusin` listener at the document root that
/// recomputes the status bar hints whenever focus moves. Single
/// listener handles every focusable element — `focusin` bubbles
/// per UI Events §3.4.
pub fn wire_focus_hints(dom: &mut TuiDom, status_bar: NodeId) {
    let root = dom.root();
    dom.add_event_listener(root, "focusin", ListenerOptions::default(), move |ctx| {
        let hints = hints_for(ctx.dom, ctx.event.target);
        write_hints(ctx.dom, status_bar, hints);
    })
    .expect("dom.root() is valid");
}

/// Walk `focused`'s ancestor chain looking at class attributes to
/// decide which hint set to show. Returns a static slice — no
/// allocation per focus change.
fn hints_for(dom: &TuiDom, focused: Option<NodeId>) -> &'static [(&'static str, &'static str)] {
    let Some(id) = focused else {
        return DEFAULT_HINTS;
    };
    let mut cur = Some(id);
    while let Some(n) = cur {
        let node = dom.node(n);
        if let Some(class) = node.get_attribute("class") {
            for c in class.split_whitespace() {
                if c == "source-disclosure" {
                    return SOURCE_HINTS;
                }
                if c == "sidebar" {
                    return DEFAULT_HINTS;
                }
            }
        }
        cur = node.parent_node().map(|p| p.id());
    }
    DEFAULT_HINTS
}

/// Replace the status bar's children with the rendered form of
/// `hints` — `<span class="key">…</span> <span class="label">…</span>`
/// pairs separated by `<span class="sep">·</span>`.
fn write_hints(dom: &mut TuiDom, status_bar: NodeId, hints: &[(&str, &str)]) {
    let _ = dom.clear_children(status_bar);
    for (i, (key, label)) in hints.iter().enumerate() {
        if i > 0 {
            let sep = dom.create_element("span");
            let _ = dom.set_attribute(sep, "class", "sep");
            let t = dom.create_text_node(" · ");
            let _ = dom.append_child(sep, t);
            let _ = dom.append_child(status_bar, sep);
        }
        let key_span = dom.create_element("span");
        let _ = dom.set_attribute(key_span, "class", "key");
        let key_text = dom.create_text_node(key);
        let _ = dom.append_child(key_span, key_text);
        let _ = dom.append_child(status_bar, key_span);

        // 1-char gap between key and label.
        let gap = dom.create_text_node(" ");
        let _ = dom.append_child(status_bar, gap);

        let label_span = dom.create_element("span");
        let _ = dom.set_attribute(label_span, "class", "label");
        let label_text = dom.create_text_node(label);
        let _ = dom.append_child(label_span, label_text);
        let _ = dom.append_child(status_bar, label_span);
    }
}
