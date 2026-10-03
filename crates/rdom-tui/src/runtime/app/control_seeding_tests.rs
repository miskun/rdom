//! `INPUT-SEED-ON-INSERT-1`: an `<input>` / `<textarea>` inserted after
//! `App::build` gets its internal text node at the next frame boundary,
//! before that frame's layout, with no focus needed — so layout,
//! `value()`, `:placeholder-shown`, the caret and a form reset behave as
//! for a control present at mount.

use rdom_core::{NodeId, NodeType, Position};

use crate::TuiDom;
use crate::node::TuiNodeExt;
use crate::render::inline::cell_of_position;
use crate::render::{Color, Terminal, TestBackend};
use crate::runtime::app::App;
use crate::runtime::builtins::input;
use crate::style::{Stylesheet, TuiStyle};

const RED: Color = Color::Rgb(255, 0, 0);

fn app() -> App<TestBackend> {
    let dom: TuiDom = TuiDom::new();
    let sheet = Stylesheet::new().rule_unchecked(
        "input:placeholder-shown, textarea:placeholder-shown",
        TuiStyle::new().fg(RED),
    );
    let terminal = Terminal::new(TestBackend::new(30, 10)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.advance(0).unwrap();
    app
}

fn element(app: &mut App<TestBackend>, tag: &str, attrs: &[(&str, &str)]) -> NodeId {
    let dom = app.dom_mut();
    let e = dom.create_element(tag);
    for (k, v) in attrs {
        dom.set_attribute(e, k, v).unwrap();
    }
    e
}

/// The control's only child is a text node; returns it.
fn text_child(app: &App<TestBackend>, control: NodeId) -> NodeId {
    let kids: Vec<_> = app.dom().node(control).child_nodes().collect();
    let ids: Vec<NodeId> = kids.iter().map(|k| k.id()).collect();
    assert_eq!(kids.len(), 1, "one text child: {ids:?}");
    assert_eq!(kids[0].node_type(), NodeType::Text);
    kids[0].id()
}

#[test]
fn an_input_appended_after_build_is_seeded_by_the_next_frame_without_focus() {
    let mut app = app();
    let input_el = element(&mut app, "input", &[("value", "hi")]);
    let root = app.dom().root();
    app.dom_mut().append_child(root, input_el).unwrap();
    app.advance(0).unwrap();

    assert_eq!(app.dom().focused(), None, "nothing focused it");
    let text = text_child(&app, input_el);
    assert_eq!(input::value(app.dom(), input_el), "hi");
    // Seeded before that frame's layout: the text is laid out inside the
    // input's box.
    let (x, y) = cell_of_position(app.dom(), Position::new(text, 2)).expect("the text is laid out");
    let r = app.dom().node(input_el).layout_rect().unwrap();
    assert!(
        (x as i32) >= r.x && (x as i32) < r.x + r.width as i32 && y as i32 == r.y,
        "caret cell ({x}, {y}) inside {r:?}"
    );
    // The authored value is the default a form reset restores.
    assert_eq!(
        app.dom()
            .node(input_el)
            .ext()
            .unwrap()
            .default_value
            .as_deref(),
        Some("hi")
    );
}

#[test]
fn an_empty_input_appended_after_build_shows_its_placeholder() {
    let mut app = app();
    let input_el = element(&mut app, "input", &[("placeholder", "Name")]);
    let root = app.dom().root();
    app.dom_mut().append_child(root, input_el).unwrap();
    app.advance(0).unwrap();

    text_child(&app, input_el);
    assert!(app.dom().is_placeholder_shown(input_el));
    let fg = app.dom().node(input_el).computed().unwrap().fg;
    assert_eq!(fg, RED, ":placeholder-shown matches");
}

#[test]
fn a_replaced_subtree_seeds_every_control_in_it() {
    // A demo switch: the old view's children cleared, a new view built
    // off-tree and appended in one go.
    let mut app = app();
    let root = app.dom().root();
    let main = element(&mut app, "main", &[]);
    app.dom_mut().append_child(root, main).unwrap();
    let old = element(&mut app, "p", &[]);
    app.dom_mut().append_child(main, old).unwrap();
    app.advance(0).unwrap();

    let view = element(&mut app, "form", &[]);
    let row = element(&mut app, "div", &[]);
    let name = element(&mut app, "input", &[("type", "text"), ("value", "a")]);
    let email = element(&mut app, "input", &[("type", "email")]);
    let notes = element(&mut app, "textarea", &[]);
    let toggle = element(&mut app, "input", &[("type", "checkbox")]);
    {
        let dom = app.dom_mut();
        dom.append_child(row, name).unwrap();
        dom.append_child(row, email).unwrap();
        dom.append_child(view, row).unwrap();
        dom.append_child(view, notes).unwrap();
        dom.append_child(view, toggle).unwrap();
        dom.clear_children(main).unwrap();
        dom.append_child(main, view).unwrap();
    }
    app.advance(0).unwrap();

    text_child(&app, name);
    text_child(&app, email);
    text_child(&app, notes);
    assert_eq!(input::value(app.dom(), name), "a");
    assert_eq!(input::value(app.dom(), email), "");
    assert_eq!(
        app.dom().node(toggle).child_nodes().count(),
        0,
        "a checkbox has no editable text"
    );
}

#[test]
fn an_input_whose_type_turns_text_after_insertion_is_seeded() {
    let mut app = app();
    let control = element(&mut app, "input", &[("type", "checkbox")]);
    let root = app.dom().root();
    app.dom_mut().append_child(root, control).unwrap();
    app.advance(0).unwrap();
    assert_eq!(app.dom().node(control).child_nodes().count(), 0);

    app.dom_mut()
        .set_attribute(control, "type", "text")
        .unwrap();
    app.advance(0).unwrap();
    text_child(&app, control);
}

#[test]
fn reinserting_an_edited_input_keeps_its_text() {
    // A moved control keeps its live value: insertion seeds only a
    // control without a text node, it never re-reads `value`.
    let mut app = app();
    let control = element(&mut app, "input", &[("value", "old")]);
    let root = app.dom().root();
    app.dom_mut().append_child(root, control).unwrap();
    app.advance(0).unwrap();
    let text = text_child(&app, control);
    app.dom_mut()
        .node_mut(text)
        .set_node_value("typed")
        .unwrap();

    let holder = element(&mut app, "div", &[]);
    app.dom_mut().append_child(root, holder).unwrap();
    app.dom_mut().append_child(holder, control).unwrap();
    app.advance(0).unwrap();
    assert_eq!(text_child(&app, control), text);
    assert_eq!(input::value(app.dom(), control), "typed");
}
