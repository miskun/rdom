//! Clipboard unit tests — selection serialization + the memory
//! backend. App-level copy/cut/paste flow is covered in
//! `runtime/app/tests.rs` since it drives the event loop.

use rdom_core::{Position, Range, Selection};

use crate::TuiDom;
use crate::layout::{Display, Size, UserSelect};
use crate::render::{LayoutExt, Rect};
use crate::runtime::selection::clipboard::{Clipboard, MemoryClipboard, serialize_selection};
use crate::style::{CascadeExt, Stylesheet, TuiStyle};

fn prepare(dom: &mut TuiDom, sheet: &Stylesheet) {
    dom.cascade(sheet);
    dom.layout_dom(Rect::new(0, 0, 40, 10));
}

// ── MemoryClipboard ─────────────────────────────────────────────────

#[test]
fn memory_clipboard_roundtrip() {
    let mut cb = MemoryClipboard::new();
    assert_eq!(cb.read_text(), None);
    cb.write_text("hello".to_string());
    assert_eq!(cb.read_text().as_deref(), Some("hello"));
}

#[test]
fn memory_clipboard_with_text_seeds() {
    let mut cb = MemoryClipboard::with_text("seeded");
    assert_eq!(cb.read_text().as_deref(), Some("seeded"));
}

#[test]
fn memory_clipboard_overwrites() {
    let mut cb = MemoryClipboard::with_text("old");
    cb.write_text("new".to_string());
    assert_eq!(cb.peek(), Some("new"));
}

// ── serialize_selection ─────────────────────────────────────────────

#[test]
fn serialize_single_node_range_returns_sliced_text() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("hello world");
    dom.append_child(p, t).unwrap();
    dom.append_child(root, p).unwrap();
    prepare(&mut dom, &Stylesheet::bare());

    let range = Range::ordered_unchecked(Position::new(t, 6), Position::new(t, 11));
    assert_eq!(serialize_selection(&dom, &range), "world");
}

#[test]
fn serialize_cross_node_range_concats_text_in_doc_order() {
    // <p>ab<code>XY</code>cd</p>, selection 1..1 across text nodes
    // yields "b" + "XY" + "c" = "bXYc".
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t_ab = dom.create_text_node("ab");
    dom.append_child(p, t_ab).unwrap();
    let code = dom.create_element("code");
    let t_xy = dom.create_text_node("XY");
    dom.append_child(code, t_xy).unwrap();
    dom.append_child(p, code).unwrap();
    let t_cd = dom.create_text_node("cd");
    dom.append_child(p, t_cd).unwrap();
    dom.append_child(root, p).unwrap();
    prepare(&mut dom, &Stylesheet::bare());

    let range = Range::ordered_unchecked(Position::new(t_ab, 1), Position::new(t_cd, 1));
    assert_eq!(serialize_selection(&dom, &range), "bXYc");
}

#[test]
fn serialize_skips_user_select_none_subtree() {
    // <p>ab<span class=chrome>HIDDEN</span>cd</p>, selection covers
    // everything. The chrome span should NOT leak into the clipboard.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t_ab = dom.create_text_node("ab");
    dom.append_child(p, t_ab).unwrap();
    let chrome = dom.create_element("span");
    dom.add_class(chrome, "chrome").unwrap();
    let t_hidden = dom.create_text_node("HIDDEN");
    dom.append_child(chrome, t_hidden).unwrap();
    dom.append_child(p, chrome).unwrap();
    let t_cd = dom.create_text_node("cd");
    dom.append_child(p, t_cd).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(40)),
        )
        .rule_unchecked(".chrome", TuiStyle::new().user_select(UserSelect::None));
    prepare(&mut dom, &sheet);

    let range = Range::ordered_unchecked(Position::new(t_ab, 0), Position::new(t_cd, 2));
    assert_eq!(serialize_selection(&dom, &range), "abcd");
}

#[test]
fn serialize_cjk_preserves_bytes() {
    // Byte offsets into a CJK string should pull out the exact bytes,
    // which decode back to valid UTF-8 because the offsets land on
    // grapheme boundaries.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("中文");
    dom.append_child(p, t).unwrap();
    dom.append_child(root, p).unwrap();
    prepare(&mut dom, &Stylesheet::bare());

    let range = Range::ordered_unchecked(Position::new(t, 0), Position::new(t, 3));
    assert_eq!(serialize_selection(&dom, &range), "中");
}

#[test]
fn serialize_respects_dom_set_selection_via_selection_range() {
    // Integration check: Dom::set_selection + Dom::selection_range
    // produces a range compatible with serialize_selection.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("hello world");
    dom.append_child(p, t).unwrap();
    dom.append_child(root, p).unwrap();
    prepare(&mut dom, &Stylesheet::bare());

    dom.set_selection(Some(Selection::new(
        Position::new(t, 6),
        Position::new(t, 11),
    )));

    let range = dom.selection_range().expect("selection is set");
    assert_eq!(serialize_selection(&dom, &range), "world");
}

// ── Polish #7: OSC 52 clipboard ──────────────────────────────────

use crate::runtime::selection::clipboard::OscClipboard;
use std::sync::{Arc, Mutex};

/// `Arc<Mutex<Vec<u8>>>`-backed writer used by OSC clipboard tests.
/// Implements `Write` by appending to the shared buffer so
/// assertions can inspect exactly what bytes were emitted.
#[derive(Clone, Default)]
struct ByteSink(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for ByteSink {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl ByteSink {
    fn snapshot(&self) -> Vec<u8> {
        self.0.lock().unwrap().clone()
    }
}

#[test]
fn osc52_write_emits_base64_in_osc_sequence() {
    let sink = ByteSink::default();
    let snapshot = sink.clone();
    let mut cb = OscClipboard::with_writer(sink);
    cb.write_text("hello".into());
    let out = String::from_utf8(snapshot.snapshot()).unwrap();
    // ESC ] 5 2 ; c ; <base64> ESC \
    assert_eq!(out, "\x1b]52;c;aGVsbG8=\x1b\\");
}

#[test]
fn osc52_write_handles_three_byte_aligned_input_without_padding() {
    let sink = ByteSink::default();
    let snapshot = sink.clone();
    let mut cb = OscClipboard::with_writer(sink);
    cb.write_text("foo".into()); // 3 bytes → 4 base64 chars, no `=`.
    let out = String::from_utf8(snapshot.snapshot()).unwrap();
    assert_eq!(out, "\x1b]52;c;Zm9v\x1b\\");
}

#[test]
fn osc52_write_handles_two_byte_remainder_with_single_pad() {
    let sink = ByteSink::default();
    let snapshot = sink.clone();
    let mut cb = OscClipboard::with_writer(sink);
    cb.write_text("hi".into()); // 2 bytes → 4 chars with one `=`.
    let out = String::from_utf8(snapshot.snapshot()).unwrap();
    assert_eq!(out, "\x1b]52;c;aGk=\x1b\\");
}

#[test]
fn osc52_write_handles_empty_string() {
    let sink = ByteSink::default();
    let snapshot = sink.clone();
    let mut cb = OscClipboard::with_writer(sink);
    cb.write_text(String::new());
    let out = String::from_utf8(snapshot.snapshot()).unwrap();
    // Empty base64 → empty payload between `;c;` and terminator.
    assert_eq!(out, "\x1b]52;c;\x1b\\");
}

#[test]
fn osc52_write_roundtrips_unicode_via_base64() {
    let sink = ByteSink::default();
    let snapshot = sink.clone();
    let mut cb = OscClipboard::with_writer(sink);
    cb.write_text("héllo — 中".into());
    let out = String::from_utf8(snapshot.snapshot()).unwrap();
    assert!(out.starts_with("\x1b]52;c;"));
    assert!(out.ends_with("\x1b\\"));
    let payload = &out["\x1b]52;c;".len()..out.len() - "\x1b\\".len()];
    // Base64 of UTF-8 "héllo — 中":
    // h(68) é(C3 A9) l(6C) l(6C) o(6F) ' '(20) —(E2 80 94) ' '(20) 中(E4 B8 AD)
    // = 14 bytes → 20 base64 chars (including one `=` pad).
    assert_eq!(payload, "aMOpbGxvIOKAlCDkuK0=");
}

#[test]
fn osc52_read_returns_none_in_v1() {
    // Read is unsupported without an incoming terminal response
    // parser. Any future polish pass can flip this — for now it's
    // documented as write-only.
    let mut cb = OscClipboard::with_writer(ByteSink::default());
    assert!(cb.read_text().is_none());
}
