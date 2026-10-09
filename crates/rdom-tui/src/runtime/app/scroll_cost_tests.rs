//! C15G-SCROLL-NO-RELAYOUT: what a scroll costs an `App`. A wheel tick
//! moves the scrolled boxes and paints — no box is laid out, however large
//! the document — and the boxes that depend on scroll position (a sticky
//! header, an anchored tooltip) are updated by moving them, not by a
//! layout.

use crossterm::event::{Event as CtEvent, KeyModifiers, MouseEvent, MouseEventKind};

use crate::TuiDom;
use crate::node::TuiNodeExt;
use crate::render::layout_pass::{LAYOUTS, ROUNDS};
use crate::render::{Terminal, TestBackend, VirtualScreen};
use crate::runtime::app::App;
use crate::style::Stylesheet;

const ROWS: usize = 2000;

/// A 10-row scroller of `ROWS` paragraphs (`extra` markup after them),
/// styled `css`, drawn once with the pointer over it.
fn app(extra: &str, css: &str) -> App<TestBackend> {
    let mut markup = String::from(r#"<div id="s"><h1>head</h1>"#);
    for i in 0..ROWS {
        markup.push_str(&format!(r#"<p id="p{i}">row {i} <b>bold</b></p>"#));
    }
    markup.push_str("</div>");
    markup.push_str(extra);
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    rdom_parser::parse_into(&mut dom, &markup, root).unwrap();
    let sheet = rdom_css::parse(&format!(
        "#s {{ overflow-y: auto; height: 10; width: 30 }} p, h1 {{ margin: 0 }} {css}"
    ));
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    let terminal = Terminal::new(TestBackend::new(40, 12)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    app.push_stylesheet(sheet.stylesheet);
    app.advance(0).unwrap();
    wheel(&mut app, MouseEventKind::Moved);
    app.advance(0).unwrap();
    app
}

fn wheel(app: &mut App<TestBackend>, kind: MouseEventKind) {
    app.handle_event(CtEvent::Mouse(MouseEvent {
        kind,
        column: 2,
        row: 3,
        modifiers: KeyModifiers::empty(),
    }));
}

fn screen(app: &App<TestBackend>) -> VirtualScreen {
    let mut screen = VirtualScreen::new(40, 12);
    screen.apply(app.terminal().backend().bytes());
    screen
}

/// One wheel tick down, drawn: the boxes `layout_node` laid out, the runs
/// of layout's phases 1–2, and the frame's stats.
fn tick(app: &mut App<TestBackend>) -> (usize, usize, super::redraw::FrameStats) {
    app.take_frame_stats();
    LAYOUTS.with(|c| c.set(0));
    ROUNDS.with(|c| c.set(0));
    wheel(app, MouseEventKind::ScrollDown);
    app.advance(0).unwrap();
    (
        LAYOUTS.with(std::cell::Cell::get),
        ROUNDS.with(std::cell::Cell::get),
        app.take_frame_stats(),
    )
}

/// CSS Overflow 3 §2: a scroll moves the content in the scrollport; it
/// changes no box's size or place in its flow, so no layout runs.
#[test]
fn a_wheel_tick_in_a_large_document_lays_nothing_out() {
    let mut app = app("", "");
    for n in 1..=3 {
        let (boxes, rounds, stats) = tick(&mut app);
        assert_eq!((boxes, rounds), (0, 0), "tick {n}: {stats:?}");
        assert_eq!(
            (stats.layouts, stats.scroll_updates, stats.paints),
            (0, 1, 1),
            "tick {n}: {stats:?}"
        );
        let top = screen(&app).row(0);
        assert!(
            top.starts_with(&format!("row {} ", n - 1)),
            "tick {n}: {top:?}"
        );
    }
}

/// CSS Position 3 §3.4 (sticky), CSS Anchor Positioning 1 §3 (an anchored
/// box follows its anchor across a scroll): the boxes that depend on the
/// scroll position are moved to where a layout would put them — without
/// one.
#[test]
fn sticky_and_anchored_boxes_follow_a_wheel_tick_without_a_layout() {
    let mut app = app(
        r#"<div id="t">tip</div>"#,
        "h1 { position: sticky; top: 0 } #p5 { anchor-name: --a } \
         #t { position: absolute; position-anchor: --a; top: anchor(top); left: anchor(right) }",
    );
    for n in 1..=3 {
        let (boxes, rounds, stats) = tick(&mut app);
        assert_eq!((boxes, rounds), (0, 0), "tick {n}: {stats:?}");
        assert_eq!((stats.layouts, stats.scroll_updates), (0, 1), "{stats:?}");
        let s = screen(&app);
        assert!(
            s.row(0).starts_with("head"),
            "the header sticks: {:?}",
            s.row(0)
        );
        // `#p5` is on row 6 unscrolled (after the header).
        let anchor_row = 6 - n;
        assert!(
            s.row(anchor_row as u16).contains("tip"),
            "tick {n}: the tooltip beside its anchor on row {anchor_row}: {:?}",
            s.row(anchor_row as u16)
        );
        let t = app.dom().get_element_by_id("t").unwrap();
        assert_eq!(
            app.dom().node(t).layout_rect().map(|r| r.y),
            Some(anchor_row)
        );
    }
}

/// A scroll key (the focused scroller's Down arrow) costs what a wheel
/// tick does: a scroll update, no layout.
#[test]
fn a_scroll_key_lays_nothing_out() {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState};
    let mut app = app("", "#s { scroll-behavior: auto }");
    let s = app.dom().get_element_by_id("s").unwrap();
    app.dom_mut().set_attribute(s, "tabindex", "0").unwrap();
    crate::runtime::focus::focus_node(app.dom_mut(), Some(s));
    app.advance(0).unwrap();
    app.take_frame_stats();
    LAYOUTS.with(|c| c.set(0));
    ROUNDS.with(|c| c.set(0));
    app.handle_event(CtEvent::Key(KeyEvent {
        code: KeyCode::Down,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    }));
    app.advance(0).unwrap();
    let stats = app.take_frame_stats();
    assert!(app.dom().node(s).ext().unwrap().scroll_y > 0, "{stats:?}");
    assert_eq!(
        (
            LAYOUTS.with(std::cell::Cell::get),
            ROUNDS.with(std::cell::Cell::get)
        ),
        (0, 0),
        "{stats:?}"
    );
    assert_eq!((stats.layouts, stats.scroll_updates), (0, 1), "{stats:?}");
}
