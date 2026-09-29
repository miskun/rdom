//! `P7G-HOVER-ANCESTORS-1`: `:hover` and `:active` match the element
//! holding the state and every ancestor of it (Selectors 4 §9.2 /
//! §9.4), so `li:hover` / `li:active` apply while the pointer is over,
//! or presses, a `<span>` inside the `<li>`.

use crossterm::event::{Event as CtEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use rdom_core::NodeId;

use crate::TuiDom;
use crate::layout::{Display, Size};
use crate::render::{Color, Terminal, TestBackend};
use crate::runtime::app::App;
use crate::style::{Stylesheet, TuiStyle};

const RED: Color = Color::Rgb(255, 0, 0);
const BLUE: Color = Color::Rgb(0, 0, 255);

fn mouse(app: &mut App<TestBackend>, kind: MouseEventKind, column: u16, row: u16) {
    app.handle_event(CtEvent::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::empty(),
    }));
    app.advance(0).unwrap();
}

fn fg(app: &App<TestBackend>, id: NodeId) -> Color {
    app.dom()
        .node(id)
        .ext()
        .unwrap()
        .computed
        .as_ref()
        .unwrap()
        .fg
}

/// A 10×3 `<li>` on rows 0–2 holding a 4×1 `<span>` at its top-left;
/// `li:hover` is red, `li:active` blue. One frame drawn.
fn list_app() -> (App<TestBackend>, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let li = dom.create_element("li");
    let span = dom.create_element("span");
    dom.append_child(root, li).unwrap();
    dom.append_child(li, span).unwrap();
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "li",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(10))
                .height(Size::Fixed(3)),
        )
        .rule_unchecked(
            "span",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(4))
                .height(Size::Fixed(1)),
        )
        .rule_unchecked("li:hover", TuiStyle::new().fg(RED))
        .rule_unchecked("li:active", TuiStyle::new().fg(BLUE));
    let terminal = Terminal::new(TestBackend::new(20, 8)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.advance(0).unwrap();
    (app, li, span)
}

#[test]
fn li_hover_applies_while_the_pointer_is_over_a_child_span() {
    let (mut app, li, span) = list_app();
    mouse(&mut app, MouseEventKind::Moved, 1, 0);
    assert_eq!(app.dom().hovered(), Some(span), "the span is the hit");
    assert_eq!(fg(&app, li), RED, "its <li> matches :hover");
    mouse(&mut app, MouseEventKind::Moved, 8, 2);
    assert_eq!(app.dom().hovered(), Some(li));
    assert_eq!(fg(&app, li), RED, "still hovered, now directly");
    mouse(&mut app, MouseEventKind::Moved, 15, 7);
    assert_eq!(app.dom().hovered(), None);
    assert_ne!(fg(&app, li), RED, "leaving drops :hover");
}

#[test]
fn li_active_applies_while_a_child_span_is_pressed() {
    let (mut app, li, span) = list_app();
    mouse(&mut app, MouseEventKind::Moved, 1, 0);
    mouse(&mut app, MouseEventKind::Down(MouseButton::Left), 1, 0);
    assert_eq!(app.dom().active(), Some(span), "the pressed element");
    assert_eq!(fg(&app, li), BLUE, "its <li> matches :active");
    mouse(&mut app, MouseEventKind::Up(MouseButton::Left), 1, 0);
    assert_eq!(app.dom().active(), None, "the release ends the activation");
    assert_eq!(fg(&app, li), RED, "back to :hover only");
}

/// `P7G-ACTIVE-UNTIL-CLICK-1`: the activation lasts through the
/// release's own events — `mouseup` and `click` listeners still see the
/// pressed element as `:active` (as in Blink, which clears it after the
/// release is handled) — and ends once they have run.
#[test]
fn active_holds_through_mouseup_and_click_and_clears_after() {
    use std::cell::RefCell;
    use std::rc::Rc;
    let (mut app, li, span) = list_app();
    /// Event type, `Dom::active()`, whether the `<li>` matched `:active`.
    type Seen = Vec<(&'static str, Option<NodeId>, bool)>;
    let seen: Rc<RefCell<Seen>> = Rc::default();
    for ty in ["mouseup", "click"] {
        let seen = seen.clone();
        app.dom_mut()
            .add_event_listener(li, ty, rdom_core::ListenerOptions::default(), move |ctx| {
                let matches = ctx.dom.node(li).matches(":active");
                seen.borrow_mut().push((ty, ctx.dom.active(), matches));
            })
            .unwrap();
    }
    mouse(&mut app, MouseEventKind::Moved, 1, 0);
    mouse(&mut app, MouseEventKind::Down(MouseButton::Left), 1, 0);
    mouse(&mut app, MouseEventKind::Up(MouseButton::Left), 1, 0);
    assert_eq!(
        *seen.borrow(),
        vec![("mouseup", Some(span), true), ("click", Some(span), true)]
    );
    assert_eq!(app.dom().active(), None, "cleared after the click");
    assert_eq!(fg(&app, li), RED, "and restyled: back to :hover only");
}
