//! C8G-ABSPOS-EXTENT (architect N1), C9G-MISC-CORRECTNESS: how many times
//! one frame runs layout's phases 1–2. A frame lays out once, then once
//! more when a re-snap (CSS Scroll Snap 1 §5.4), a focused element's scroll
//! into view (HTML's focusing steps) or a caret reveal moved an offset —
//! all three against the first layout (each corrects for the offsets moved
//! since: `scrollbar::state::laid_out`), so one relayout; each `layout_dom`
//! runs the phases at most `positioned_overflow::MAX_ROUNDS` times, while
//! absolutely positioned boxes change their scroll containers' reach. The
//! pin is a frame that needs all of it.

use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers,
};
use rdom_core::{Position, Selection};

use crate::TuiDom;
use crate::accessors::TuiAccessorsMut;
use crate::render::layout_pass::ROUNDS;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;

fn el(dom: &mut TuiDom, parent: rdom_core::NodeId, class: &str) -> rdom_core::NodeId {
    let id = dom.create_element("div");
    dom.set_attribute(id, "class", class).unwrap();
    dom.append_child(parent, id).unwrap();
    id
}

/// One frame with a new positioned box whose scrollbar narrows it, a
/// snap target moved by an insertion, a typed line to reveal and an
/// element focused in a scroller: three runs (the stale reach, the new
/// one, the narrowed one), then one after the re-snap, the focus scroll
/// and the caret reveal together.
#[test]
fn a_frame_runs_layout_at_most_twice_the_round_cap() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let port = el(&mut dom, root, "port");
    let snap = el(&mut dom, root, "snap");
    let items: Vec<_> = (0..5).map(|_| el(&mut dom, snap, "item")).collect();
    let list = el(&mut dom, root, "list");
    let rows: Vec<_> = (0..6).map(|_| el(&mut dom, list, "row")).collect();
    dom.set_attribute(rows[5], "tabindex", "0").unwrap();
    let ta = dom.create_element("textarea");
    let text = dom.create_text_node("aa bb cc dd");
    dom.append_child(ta, text).unwrap();
    dom.append_child(root, ta).unwrap();
    let sheet = rdom_css::from_css_strict(
        ".port { position: relative; overflow: auto; width: 20; height: 4 } \
         .abs { position: absolute; left: 0; right: 0; height: 30 } \
         .snap { overflow-y: scroll; height: 4; width: 10; \
                 scroll-snap-type: y mandatory } \
         .item { height: 3; scroll-snap-align: start } .new { height: 2 } \
         textarea { width: 6; height: 2; overflow-y: auto } \
         .list { overflow-y: auto; height: 2; width: 8 } .row { height: 1 }",
    )
    .unwrap();
    let terminal = Terminal::new(TestBackend::new(30, 14)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.draw_if_dirty().unwrap();
    app.dom_mut().node_mut(snap).scroll_to(0, 3).unwrap();
    app.draw_if_dirty().unwrap();
    app.dom_mut().set_focused(Some(ta));
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(text, 11))));
    app.draw_if_dirty().unwrap();

    // The frame's causes.
    let dom = app.dom_mut();
    let abs = dom.create_element("div");
    dom.set_attribute(abs, "class", "abs").unwrap();
    dom.append_child(port, abs).unwrap();
    let new = dom.create_element("div");
    dom.set_attribute(new, "class", "item new").unwrap();
    dom.insert_before(snap, new, Some(items[0])).unwrap();
    for ch in [' ', 'e', 'e'] {
        app.handle_event(CtEvent::Key(KeyEvent {
            code: KeyCode::Char(ch),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }));
    }

    // Script focus in another scroller, keeping the typed caret's reveal
    // pending: focusing does not move the selection.
    crate::runtime::focus::focus_node(app.dom_mut(), Some(rows[5]));

    ROUNDS.with(|c| c.set(0));
    app.draw_if_dirty().unwrap();
    let runs = ROUNDS.with(|c| c.get());
    use crate::accessors::TuiAccessors;
    assert_eq!(app.dom().node(snap).scroll_top(), Some(5), "re-snapped");
    assert_eq!(app.dom().node(ta).scroll_top(), Some(1), "caret revealed");
    assert_eq!(app.dom().node(list).scroll_top(), Some(4), "focus revealed");
    assert_eq!(runs, 4, "3 rounds, then one for the three services");
    assert!(runs <= 2 * crate::render::layout_pass::MAX_ROUNDS);
}
