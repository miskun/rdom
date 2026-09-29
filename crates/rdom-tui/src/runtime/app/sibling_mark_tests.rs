//! `P7G-SIBLING-MARK-NARROW-1`: a state change dirties the changed
//! element's siblings only when some `+` / `~` combinator's left compound
//! can read that state — a pseudo-class for hover / focus / text
//! changes, the attribute's own name for an attribute change. One
//! `h1 + p` rule no longer makes a hover move in a long list restyle the
//! whole list.

use crossterm::event::{Event as CtEvent, KeyModifiers, MouseEvent, MouseEventKind};
use rdom_core::NodeId;

use crate::TuiDom;
use crate::layout::{Display, Size};
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::style::{Stylesheet, TuiStyle};

const ROWS: usize = 10;

/// A `<ul>` of ten one-row `<li>`s, the pointer on row 0, under `sheet`
/// plus the list's layout rules; one frame drawn, roots drained.
fn list_app(extra: &str) -> (App<TestBackend>, Vec<NodeId>) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let ul = dom.create_element("ul");
    dom.append_child(root, ul).unwrap();
    let rows: Vec<NodeId> = (0..ROWS)
        .map(|_| {
            let li = dom.create_element("li");
            dom.append_child(ul, li).unwrap();
            li
        })
        .collect();
    let sheet = Stylesheet::bare()
        .rule_unchecked("ul", TuiStyle::new().display(Display::Block))
        .rule_unchecked(
            "li",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(10))
                .height(Size::Fixed(1)),
        );
    let extra_sheet = rdom_css::parse(extra).stylesheet;
    let mut app =
        App::with_backend(dom, sheet, Terminal::new(TestBackend::new(20, 12)).unwrap()).unwrap();
    app.push_stylesheet(extra_sheet);
    app.advance(0).unwrap();
    move_to(&mut app, 0);
    app.advance(0).unwrap();
    (app, rows)
}

fn move_to(app: &mut App<TestBackend>, row: u16) {
    app.handle_event(CtEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Moved,
        column: 1,
        row,
        modifiers: KeyModifiers::empty(),
    }));
}

#[test]
fn an_h1_plus_p_rule_does_not_mark_siblings_on_hover() {
    let (mut app, rows) = list_app("h1 + p { color: red; }");
    move_to(&mut app, 1);
    let mut roots = app.dirty_roots_snapshot();
    roots.sort_unstable();
    assert_eq!(
        roots,
        vec![rows[0], rows[1]],
        "the two rows whose :hover flipped"
    );
}

#[test]
fn a_hover_plus_rule_marks_siblings_on_hover() {
    let (mut app, rows) = list_app("li:hover + li { color: red; }");
    move_to(&mut app, 1);
    let roots = app.dirty_roots_snapshot();
    assert!(
        rows.iter().all(|r| roots.contains(r)),
        "every row can match through the previous row's :hover: {roots:?}"
    );
}

#[test]
fn an_attribute_plus_rule_marks_siblings_for_that_attribute_only() {
    let (mut app, rows) = list_app("[x] + li { color: red; }");
    app.dom_mut().set_attribute(rows[3], "y", "1").unwrap();
    assert_eq!(app.dirty_roots_snapshot(), vec![rows[3]], "`y` is not read");
    app.advance(0).unwrap();
    app.dom_mut().set_attribute(rows[3], "x", "1").unwrap();
    let roots = app.dirty_roots_snapshot();
    assert!(
        rows.iter().all(|r| roots.contains(r)),
        "`x` is read by `[x] + li`: {roots:?}"
    );
}
