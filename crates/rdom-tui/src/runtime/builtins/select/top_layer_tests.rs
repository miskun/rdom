//! C12-SELECT-TOP-LAYER — a drop-down `<select>`'s picker renders in the
//! top layer (HTML's `::picker(select)`, CSS Position 4 "top layer"): it
//! overlays the page without moving it, escapes `overflow` clipping and
//! stacking contexts, is hit before the page, and light-dismisses as a
//! popover does (HTML §6.12.2).

use crossterm::event::{Event, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use rdom_core::{NodeId, TopLayerKind};

use crate::TuiDom;
use crate::node::TuiNodeExt;
use crate::render::{Terminal, TestBackend, VirtualScreen};
use crate::runtime::app::App;
use crate::runtime::builtins::select;
use crate::runtime::hit_test::HitTestExt;

struct Page {
    app: App<TestBackend>,
    select: NodeId,
    options: Vec<NodeId>,
    after: NodeId,
}

/// A clipping, stacking box holding a drop-down select of three options,
/// then a paragraph below the box; and a positioned layer over the box.
fn page() -> Page {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let clip = dom.create_element("div");
    dom.set_attribute(clip, "class", "clip").unwrap();
    dom.append_child(root, clip).unwrap();
    let sel = dom.create_element("select");
    let mut options = Vec::new();
    for label in ["one", "two", "three"] {
        let o = dom.create_element("option");
        let t = dom.create_text_node(label);
        dom.append_child(o, t).unwrap();
        dom.append_child(sel, o).unwrap();
        options.push(o);
    }
    dom.append_child(clip, sel).unwrap();
    let after = dom.create_element("p");
    let t = dom.create_text_node("after");
    dom.append_child(after, t).unwrap();
    dom.append_child(root, after).unwrap();
    let sheet = rdom_css::parse(
        "* { margin: 0 } \
         .clip { height: 1; overflow: hidden; position: relative; z-index: 1 }",
    );
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    let terminal = Terminal::new(TestBackend::new(30, 8)).unwrap();
    let mut app = App::with_backend(dom, crate::style::Stylesheet::new(), terminal).unwrap();
    app.push_stylesheet(sheet.stylesheet);
    app.advance(0).unwrap();
    Page {
        app,
        select: sel,
        options,
        after,
    }
}

fn screen(app: &App<TestBackend>) -> VirtualScreen {
    let mut screen = VirtualScreen::new(30, 8);
    screen.apply(app.terminal().backend().bytes());
    screen
}

fn row(app: &App<TestBackend>, y: u16) -> String {
    let s = screen(app);
    (0..30)
        .map(|x| s.cell(x, y).unwrap().symbol().to_string())
        .collect::<String>()
        .trim_end()
        .to_string()
}

fn press_release(app: &mut App<TestBackend>, x: u16, y: u16) {
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Up(MouseButton::Left),
    ] {
        app.handle_event(Event::Mouse(MouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        }));
    }
    app.advance(0).unwrap();
}

/// The open select is in the top layer as a picker; closing takes it out.
#[test]
fn an_open_dropdown_is_in_the_top_layer() {
    let mut p = page();
    select::open(p.app.dom_mut(), p.select);
    assert_eq!(
        p.app.dom().top_layer_kind(p.select),
        Some(TopLayerKind::Picker)
    );
    select::close(p.app.dom_mut(), p.select);
    assert!(!p.app.dom().is_in_top_layer(p.select));
}

/// The picker overlays the page: the select keeps its one row in flow, so
/// nothing after it moves, and its options draw over the paragraph and
/// past the clipping box (CSS Position 4: the top layer is clipped by the
/// viewport only).
#[test]
fn the_picker_overlays_without_moving_or_clipping() {
    let mut p = page();
    let before = p.app.dom().node(p.after).layout_rect();
    select::open(p.app.dom_mut(), p.select);
    p.app.advance(0).unwrap();
    assert_eq!(p.app.dom().node(p.after).layout_rect(), before, "no reflow");
    assert_eq!(p.app.dom().node(p.select).layout_rect().unwrap().height, 1);
    assert!(row(&p.app, 0).contains("one"), "{:?}", row(&p.app, 0));
    assert!(row(&p.app, 1).contains("two"), "{:?}", row(&p.app, 1));
    assert!(row(&p.app, 2).contains("three"), "{:?}", row(&p.app, 2));
    // The paragraph under the picker is hidden by it: the options have a
    // background.
    assert!(!row(&p.app, 1).contains("after"));
    // Hit-testing reaches the options before the page.
    assert_eq!(p.app.dom().hit_test(2, 1), Some(p.options[1]));
}

/// A click on an option picks it and closes the picker; a press and
/// release outside it light-dismisses it, as a popover's (HTML §6.12.2),
/// picking nothing.
#[test]
fn the_picker_picks_and_light_dismisses() {
    let mut p = page();
    select::open(p.app.dom_mut(), p.select);
    p.app.advance(0).unwrap();
    press_release(&mut p.app, 2, 2);
    assert!(p.app.dom().node(p.options[2]).has_attribute("selected"));
    assert!(!select::is_open(p.app.dom(), p.select));
    select::open(p.app.dom_mut(), p.select);
    p.app.advance(0).unwrap();
    press_release(&mut p.app, 25, 6);
    assert!(!select::is_open(p.app.dom(), p.select), "light dismiss");
    assert!(!p.app.dom().is_in_top_layer(p.select));
    assert!(p.app.dom().node(p.options[2]).has_attribute("selected"));
}
