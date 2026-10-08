//! C12-CURSOR — CSS UI 4 §4.1 `cursor`: the pointer shape over the
//! element under the pointer, sent to a terminal that takes OSC 22
//! pointer shapes, nothing to one that does not.

use super::PointerShapes;
use crate::TuiDom;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::style::Stylesheet;
use crossterm::event::{Event, KeyModifiers, MouseEvent, MouseEventKind};
use rdom_core::NodeId;

fn env<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
    move |name| {
        vars.iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| v.to_string())
    }
}

/// Conservative, as `SgrCapabilities::detect`: OSC 22 only where the
/// terminal is known to take CSS pointer names; a multiplexer, which
/// would have to pass it through, or an unknown terminal gets none.
#[test]
fn detection_is_conservative() {
    let osc = PointerShapes::Osc22;
    for vars in [
        &[("TERM", "xterm-kitty")][..],
        &[("KITTY_WINDOW_ID", "1"), ("TERM", "xterm-256color")],
        &[("TERM", "foot")],
        &[("TERM_PROGRAM", "WezTerm")],
        &[("TERM", "xterm-ghostty")],
    ] {
        assert_eq!(PointerShapes::detect(env(vars)), osc, "{vars:?}");
    }
    for vars in [
        &[("TERM", "xterm-256color")][..],
        &[("TERM_PROGRAM", "iTerm.app")],
        &[("TERM", "xterm-kitty"), ("TMUX", "/tmp/tmux,1,0")],
        &[("TERM", "screen-256color"), ("KITTY_WINDOW_ID", "1")],
        &[("TERM_PROGRAM", "tmux"), ("KITTY_WINDOW_ID", "1")],
        &[],
    ] {
        assert_eq!(
            PointerShapes::detect(env(vars)),
            PointerShapes::None,
            "{vars:?}"
        );
    }
}

/// A page: a `.p` box with the pointer cursor, a paragraph of text, an
/// empty box, and a link.
fn app(shapes: PointerShapes) -> (App<TestBackend>, [NodeId; 4]) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let mut el = |tag: &str, class: &str, text: &str| {
        let id = dom.create_element(tag);
        dom.set_attribute(id, "class", class).unwrap();
        if !text.is_empty() {
            let t = dom.create_text_node(text);
            dom.append_child(id, t).unwrap();
        }
        dom.append_child(root, id).unwrap();
        id
    };
    let p = el("div", "p", "click");
    let text = el("p", "t", "words");
    let empty = el("div", "e", "");
    let link = el("a", "", "link");
    dom.set_attribute(link, "href", "https://example.com")
        .unwrap();
    let sheet = rdom_css::parse(
        "* { margin: 0 } .p { cursor: pointer } .e { height: 1 } \
         .p:hover { cursor: url(hand.cur) 1 1, grab }",
    );
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    let terminal = Terminal::new(TestBackend::new(20, 6)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal)
        .unwrap()
        .with_pointer_shapes(shapes);
    app.push_stylesheet(sheet.stylesheet);
    app.advance(0).unwrap();
    (app, [p, text, empty, link])
}

fn move_to(app: &mut App<TestBackend>, x: u16, y: u16) -> String {
    app.terminal_mut().backend_mut().clear_bytes();
    app.handle_event(Event::Mouse(MouseEvent {
        kind: MouseEventKind::Moved,
        column: x,
        row: y,
        modifiers: KeyModifiers::NONE,
    }));
    app.advance(0).unwrap();
    let bytes = app.terminal_mut().backend_mut().take_bytes();
    String::from_utf8_lossy(&bytes).into_owned()
}

fn osc(name: &str) -> String {
    format!("\x1b]22;{name}\x1b\\")
}

/// §4.1: the pointer takes the shape of the element under it — its
/// `:hover` style's keyword after the image fallbacks a terminal cannot
/// draw — `auto` is the text pointer over text and the default one
/// elsewhere, a link shows `pointer` (HTML §15.3.4); each change is sent
/// once.
#[test]
fn the_pointer_follows_the_element_under_it() {
    let (mut app, _) = app(PointerShapes::Osc22);
    let out = move_to(&mut app, 1, 0);
    assert!(out.contains(&osc("grab")), "{out:?}");
    assert!(
        !move_to(&mut app, 2, 0).contains("\x1b]22;"),
        "no change, nothing sent"
    );
    assert!(move_to(&mut app, 1, 1).contains(&osc("text")));
    assert!(move_to(&mut app, 1, 2).contains(&osc("default")));
    assert!(move_to(&mut app, 1, 3).contains(&osc("pointer")));
}

/// A terminal without pointer shapes is sent nothing.
#[test]
fn a_terminal_without_pointer_shapes_is_sent_nothing() {
    let (mut app, _) = app(PointerShapes::None);
    assert!(!move_to(&mut app, 1, 0).contains("\x1b]22;"));
    assert!(!move_to(&mut app, 1, 1).contains("\x1b]22;"));
}

/// The shape is restored on exit: once an app has set one, leaving TUI
/// mode — at a normal exit or from the panic hook — sends `default`.
#[test]
fn leaving_tui_mode_restores_the_pointer() {
    super::note_shape_sent();
    let mut out = Vec::new();
    crate::render::backend_crossterm::restore_terminal(&mut out, || Ok(())).unwrap();
    assert!(String::from_utf8_lossy(&out).contains(&osc("default")));
}
