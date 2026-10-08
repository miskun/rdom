//! C10-PSEUDO-CHAINS: the App keeps the pseudo-elements' pointer state
//! (`style::pseudo_pointer`) only while a sheet has a rule that reads it
//! (`::before:hover`, Selectors 4 §3.6.3), so a document without one
//! pays no second hit test per pointer move and restyles no host.

use crossterm::event::{Event as CtEvent, KeyModifiers, MouseEvent, MouseEventKind};

use crate::TuiDom;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::style::pseudo_pointer;

fn move_to(app: &mut App<TestBackend>, column: u16, row: u16) {
    app.handle_event(CtEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Moved,
        column,
        row,
        modifiers: KeyModifiers::empty(),
    }));
    app.advance(0).unwrap();
}

/// A `div.h` holding "xy" with a `::before` of "ab", under `css`.
fn app(css: &str) -> App<TestBackend> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let h = dom.create_element("div");
    dom.set_attribute(h, "class", "h").unwrap();
    dom.append_child(root, h).unwrap();
    let t = dom.create_text_node("xy");
    dom.append_child(h, t).unwrap();
    let sheet = rdom_css::from_css_strict(css).unwrap();
    let terminal = Terminal::new(TestBackend::new(10, 3)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.advance(0).unwrap();
    app
}

#[test]
fn pointer_state_is_kept_only_while_a_sheet_reads_it() {
    let mut plain = app(".h::before { content: 'ab' } .h:hover::before { color: red }");
    move_to(&mut plain, 0, 0);
    assert!(!pseudo_pointer::is_tracked(plain.dom()));
    assert!(pseudo_pointer::take_dirty(plain.dom_mut()).is_empty());

    let mut chained = app(".h::before { content: 'ab' } .h::before:hover { color: red }");
    assert!(pseudo_pointer::is_tracked(chained.dom()));
    // A sheet set without the rule stops it.
    chained.set_stylesheet(rdom_css::from_css_strict(".h::before { content: 'ab' }").unwrap());
    chained.advance(0).unwrap();
    assert!(!pseudo_pointer::is_tracked(chained.dom()));
}
