//! `P7G-SETTER-MUTATION-1`: the direct style setters
//! (`TuiNodeMutExt::set_width`, `set_padding`, …, `set_inline_style`)
//! reflect into the `style` attribute like a CSSOM write, so a
//! listener's call reaches the next frame's cascade through the dirty
//! tracker — and leaves the tracker able to see later restyles below it.

use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind,
};
use rdom_core::{ListenerOptions, NodeId};

use crate::TuiDom;
use crate::layout::{Display, Size};
use crate::node::{TuiNodeExt, TuiNodeMutExt};
use crate::render::{Color, Terminal, TestBackend};
use crate::runtime::app::App;
use crate::style::{Stylesheet, TuiStyle};

const RED: Color = Color::Rgb(255, 0, 0);

fn mouse(app: &mut App<TestBackend>, kind: MouseEventKind, column: u16, row: u16) {
    app.handle_event(CtEvent::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::empty(),
    }));
    app.advance(0).unwrap();
}

fn press_key(app: &mut App<TestBackend>) {
    app.handle_event(CtEvent::Key(KeyEvent::new(
        KeyCode::Char('x'),
        KeyModifiers::empty(),
    )));
    app.advance(0).unwrap();
}

/// A 10×3 `#box` holding a 4×1 `.item` at its top-left, the pointer
/// resting on the box outside the item; a `keydown` listener sets the
/// box's width to 16 through `set_width` — the box's own selector state
/// does not change, so nothing else queues it. `.item:hover` is red.
fn box_app() -> (App<TestBackend>, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let bx = dom.create_element("div");
    dom.set_attribute(bx, "id", "box").unwrap();
    let item = dom.create_element("div");
    dom.set_attribute(item, "class", "item").unwrap();
    dom.append_child(root, bx).unwrap();
    dom.append_child(bx, item).unwrap();
    dom.add_event_listener(root, "keydown", ListenerOptions::default(), move |ctx| {
        ctx.dom.node_mut(bx).set_width(Size::Fixed(16));
    })
    .unwrap();
    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "#box",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(10))
                .height(Size::Fixed(3)),
        )
        .rule_unchecked(
            ".item",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(4))
                .height(Size::Fixed(1)),
        )
        .rule_unchecked(".item:hover", TuiStyle::new().fg(RED));
    let terminal = Terminal::new(TestBackend::new(20, 8)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.advance(0).unwrap();
    mouse(&mut app, MouseEventKind::Moved, 8, 2);
    assert_eq!(app.dom().hovered(), Some(bx));
    (app, bx, item)
}

fn width(app: &App<TestBackend>, id: NodeId) -> u16 {
    app.dom().node(id).layout_rect().unwrap().width
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

#[test]
fn set_width_from_a_listener_lays_out_on_the_next_frame() {
    let (mut app, bx, _) = box_app();
    assert_eq!(width(&app, bx), 10);
    press_key(&mut app);
    assert_eq!(width(&app, bx), 16, "the listener's set_width cascaded");
    assert_eq!(
        app.dom().node(bx).get_attribute("style"),
        Some("width: 16;"),
        "reflected into the style attribute, as a CSSOM write is"
    );
}

#[test]
fn a_hover_restyle_below_a_setter_written_element_still_happens() {
    let (mut app, _, item) = box_app();
    press_key(&mut app);
    assert_ne!(fg(&app, item), RED);
    mouse(&mut app, MouseEventKind::Moved, 1, 0);
    assert_eq!(app.dom().hovered(), Some(item));
    assert_eq!(fg(&app, item), RED, "the hover restyle below the box ran");
}

/// Every setter writes a `style` attribute whose parse carries the
/// value the setter stored.
#[test]
fn every_setter_reflects_a_round_tripping_style_attribute() {
    use crate::layout::{Border, Direction, Overflow, Padding};
    use rdom_style::layout::{MaxSize, MinSize};
    type Set = fn(&mut rdom_core::NodeMut<'_, crate::TuiExt>);
    let setters: [(&str, Set); 12] = [
        ("width", |n| {
            n.set_width(Size::Flex(2));
        }),
        ("height", |n| {
            n.set_height(Size::Fixed(3));
        }),
        ("min-width", |n| {
            n.set_min_width(Some(MinSize::Cells(4)));
        }),
        ("max-width", |n| {
            n.set_max_width(Some(MaxSize::Cells(40)));
        }),
        ("min-height", |n| {
            n.set_min_height(Some(MinSize::Auto));
        }),
        ("max-height", |n| {
            n.set_max_height(Some(MaxSize::Cells(9)));
        }),
        ("direction", |n| {
            n.set_direction(Direction::Row);
        }),
        ("padding", |n| {
            n.set_padding(Padding::symmetric(2, 1));
        }),
        ("border", |n| {
            n.set_border(Border::rounded());
        }),
        ("gap", |n| {
            n.set_gap(2);
        }),
        ("overflow", |n| {
            n.set_overflow(Overflow::Hidden);
        }),
        ("inline style", |n| {
            n.set_inline_style(TuiStyle::new().fg(RED).bold(true));
        }),
    ];
    for (name, set) in setters {
        let mut dom: TuiDom = TuiDom::new();
        let div = dom.create_element("div");
        dom.append_child(dom.root(), div).unwrap();
        set(&mut dom.node_mut(div));
        let attr = dom
            .node(div)
            .get_attribute("style")
            .unwrap_or_else(|| panic!("{name}: no style attribute"))
            .to_string();
        let parsed = rdom_css::parse_inline(&attr).style;
        let stored = dom.node(div).tui_ext().unwrap().inline_style_or_empty();
        assert_eq!(&parsed, stored, "{name}: `{attr}` round-trips");
    }
}
