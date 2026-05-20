//! C.2 integration tests — end-to-end `<a href>` click behavior
//! through the App.
//!
//! Covers the §4.3.1 contract:
//! - External schemes shell out via the url_opener.
//! - Internal schemes are a no-op (no opener call).
//! - Anchor without href is ignored.
//! - `event.preventDefault()` in a user listener suppresses the
//!   default action.
//! - Click on a descendant of `<a href>` still activates the anchor
//!   (closest() walk).
//! - Clicks on non-anchor elements are untouched.

use std::rc::Rc;

use crossterm::event::{
    Event as CtEvent, KeyModifiers, MouseButton, MouseEvent as CtMouseEvent, MouseEventKind,
};
use rdom_core::{ListenerOptions, NodeId};

use crate::TuiDom;
use crate::layout::{Display, Size};
use crate::render::{Rect, Terminal, TestBackend};
use crate::runtime::app::App;
use crate::runtime::url_opener::MemoryUrlOpener;
use crate::style::{Stylesheet, TuiStyle};

// ── Fixtures ────────────────────────────────────────────────────────

fn test_app(dom: TuiDom, sheet: Stylesheet, viewport: Rect) -> App<TestBackend> {
    let backend = TestBackend::new(viewport.width, viewport.height);
    let terminal = Terminal::new(backend).unwrap();
    App::with_backend(dom, sheet, terminal).unwrap()
}

/// Build a document with a single `<a>` element at (0,0)..(20,1)
/// and the given href. Returns (app, opener, anchor_id). The app
/// comes pre-swapped with a MemoryUrlOpener so tests can inspect
/// what got opened.
fn anchor_app(href: Option<&str>) -> (App<TestBackend>, Rc<MemoryUrlOpener>, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    if let Some(h) = href {
        dom.set_attribute(a, "href", h).unwrap();
    }
    let t = dom.create_text_node("click me");
    dom.append_child(a, t).unwrap();
    dom.append_child(root, a).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "a",
        TuiStyle::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(1)),
    );
    let opener = Rc::new(MemoryUrlOpener::new());
    let app = test_app(dom, sheet, Rect::new(0, 0, 20, 5)).with_url_opener(opener.clone());
    (app, opener, a)
}

/// Send a mousedown+mouseup pair at (x, y). Router synthesizes
/// a click on the common ancestor.
fn click_at(app: &mut App<TestBackend>, x: u16, y: u16) {
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Up(MouseButton::Left),
    ] {
        app.handle_event(CtEvent::Mouse(CtMouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: KeyModifiers::empty(),
        }));
    }
}

// ── External schemes shell out ─────────────────────────────────────

#[test]
fn click_on_a_with_https_opens_via_opener() {
    let (mut app, opener, _) = anchor_app(Some("https://example.com"));
    app.draw_if_dirty().unwrap(); // run layout so hit-test works
    click_at(&mut app, 3, 0);
    assert_eq!(opener.opened(), vec!["https://example.com".to_string()]);
}

#[test]
fn each_external_scheme_opens() {
    for href in [
        "http://x",
        "https://x",
        "mailto:a@b",
        "tel:+1",
        "sms:+1",
        "ftp://x",
        "file:///tmp",
    ] {
        let (mut app, opener, _) = anchor_app(Some(href));
        app.draw_if_dirty().unwrap();
        click_at(&mut app, 3, 0);
        assert_eq!(
            opener.opened(),
            vec![href.to_string()],
            "{href} should have been opened"
        );
    }
}

// ── Internal schemes are no-op at framework level ──────────────────

#[test]
fn click_on_a_with_relative_href_does_not_open() {
    let (mut app, opener, _) = anchor_app(Some("/items/default"));
    app.draw_if_dirty().unwrap();
    click_at(&mut app, 3, 0);
    assert!(opener.opened().is_empty());
}

#[test]
fn click_on_a_with_custom_scheme_does_not_open() {
    for href in [
        "tui://view/items",
        "app://home",
        "myapp://workspace/dev",
        "#section",
    ] {
        let (mut app, opener, _) = anchor_app(Some(href));
        app.draw_if_dirty().unwrap();
        click_at(&mut app, 3, 0);
        assert!(
            opener.opened().is_empty(),
            "{href} (internal) should not be opened by framework"
        );
    }
}

#[test]
fn click_on_a_with_javascript_scheme_is_not_opened() {
    // Security: javascript: is explicitly excluded.
    let (mut app, opener, _) = anchor_app(Some("javascript:alert(1)"));
    app.draw_if_dirty().unwrap();
    click_at(&mut app, 3, 0);
    assert!(opener.opened().is_empty());
}

// ── Anchor without href is ignored ─────────────────────────────────

#[test]
fn click_on_a_without_href_is_noop() {
    let (mut app, opener, _) = anchor_app(None);
    app.draw_if_dirty().unwrap();
    click_at(&mut app, 3, 0);
    assert!(opener.opened().is_empty());
}

// ── preventDefault suppresses ──────────────────────────────────────

#[test]
fn prevent_default_on_click_blocks_opener() {
    let (mut app, opener, a) = anchor_app(Some("https://example.com"));
    app.dom_mut()
        .add_event_listener(a, "click", ListenerOptions::default(), |ctx| {
            ctx.event.prevent_default();
        })
        .unwrap();
    app.draw_if_dirty().unwrap();
    click_at(&mut app, 3, 0);
    assert!(opener.opened().is_empty());
}

// ── closest() walk ─────────────────────────────────────────────────

#[test]
fn click_on_text_inside_a_still_activates_anchor() {
    // Text nodes ARE the target of clicks on characters. The
    // listener must walk up via closest("a[href]") to find the
    // anchor.
    let (mut app, opener, _) = anchor_app(Some("https://deep"));
    app.draw_if_dirty().unwrap();
    // Cell (2, 0) is inside the "click me" text.
    click_at(&mut app, 2, 0);
    assert_eq!(opener.opened(), vec!["https://deep".to_string()]);
}

#[test]
fn click_on_nested_element_inside_a_still_activates_anchor() {
    // <a href><span>label</span></a>. Click on the span.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    dom.set_attribute(a, "href", "https://nested").unwrap();
    let span = dom.create_element("span");
    let t = dom.create_text_node("label");
    dom.append_child(span, t).unwrap();
    dom.append_child(a, span).unwrap();
    dom.append_child(root, a).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "a",
            TuiStyle::new()
                .width(Size::Fixed(20))
                .height(Size::Fixed(1)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));

    let opener = Rc::new(MemoryUrlOpener::new());
    let mut app = test_app(dom, sheet, Rect::new(0, 0, 20, 5)).with_url_opener(opener.clone());
    app.draw_if_dirty().unwrap();
    click_at(&mut app, 2, 0);
    assert_eq!(opener.opened(), vec!["https://nested".to_string()]);
}

// ── Non-anchor clicks untouched ────────────────────────────────────

#[test]
fn click_on_non_anchor_does_nothing() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let d = dom.create_element("div");
    dom.append_child(root, d).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(1)),
    );
    let opener = Rc::new(MemoryUrlOpener::new());
    let mut app = test_app(dom, sheet, Rect::new(0, 0, 20, 5)).with_url_opener(opener.clone());
    app.draw_if_dirty().unwrap();
    click_at(&mut app, 3, 0);
    assert!(opener.opened().is_empty());
}

// ── Opener swap propagates ─────────────────────────────────────────

#[test]
fn with_url_opener_swap_is_visible_to_already_installed_listener() {
    // Start with the default SystemUrlOpener (not stubbed), then
    // swap in a MemoryUrlOpener. The listener installed in
    // App::build must see the swap.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    dom.set_attribute(a, "href", "https://swapped").unwrap();
    let t = dom.create_text_node("go");
    dom.append_child(a, t).unwrap();
    dom.append_child(root, a).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "a",
        TuiStyle::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(1)),
    );
    // Build WITHOUT the opener swap first — this goes through
    // build's default SystemUrlOpener. Then swap.
    let backend = TestBackend::new(20, 5);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();

    let opener = Rc::new(MemoryUrlOpener::new());
    app = app.with_url_opener(opener.clone());

    app.draw_if_dirty().unwrap();
    click_at(&mut app, 1, 0);
    assert_eq!(opener.opened(), vec!["https://swapped".to_string()]);
}
