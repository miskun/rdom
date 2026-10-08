//! C10-PSEUDO-CHAINS — a pseudo-element followed by user-action
//! pseudo-classes (Selectors 4 §3.6.3: "Some pseudo-elements may be
//! immediately followed by a combination of user action pseudo-classes";
//! the pseudo-class then describes the pseudo-element). `::before:hover`
//! styles the `::before` while the pointer is over *it* (§9.2), not over
//! its host; `:active` while it is pressed (§9.4). A pseudo-element is
//! never focused, so the focus pseudo-classes never match one.

use crossterm::event::{Event as CtEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use rdom_tui::ext::PseudoSlot;
use rdom_tui::prelude::*;
use rdom_tui::render::{Terminal, TestBackend};
use rdom_tui::{App, Color, HitTestExt};

use super::{el, lay_out, text_el};

const RED: Color = Color::Rgb(255, 0, 0);

/// An App over `dom` with `css` (parsed strictly), one frame drawn in a
/// 10 × 3 terminal.
fn app(dom: TuiDom, css: &str) -> App<TestBackend> {
    let sheet = rdom_css::from_css_strict(css).expect("sheet parses without warnings");
    let terminal = Terminal::new(TestBackend::new(10, 3)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.advance(0).unwrap();
    app
}

/// Feed one mouse event at `(column, row)` and draw the frame.
fn mouse(app: &mut App<TestBackend>, kind: MouseEventKind, column: u16, row: u16) {
    app.handle_event(CtEvent::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::empty(),
    }));
    app.advance(0).unwrap();
}

/// The foreground of `id`'s `slot` pseudo-element.
fn pseudo_fg(app: &App<TestBackend>, id: NodeId, slot: PseudoSlot) -> Option<Color> {
    app.dom().node(id).computed_pseudo(slot).map(|c| c.fg)
}

/// §3.6.3 / §9.2: `::before:hover` matches while the pointer is over the
/// `::before` box — over its host's own text it does not. Its trailing
/// pseudo-class counts in its specificity (§17), so it beats the plain
/// `::before` rule that comes after it.
#[test]
fn before_hover_matches_while_the_pointer_is_over_the_before() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let h = text_el(&mut dom, root, "div", "h", "xy");
    let mut app = app(
        dom,
        ".h::before:hover { color: red } .h::before { content: 'ab'; color: blue }",
    );
    assert_ne!(pseudo_fg(&app, h, PseudoSlot::Before), Some(RED));
    mouse(&mut app, MouseEventKind::Moved, 1, 0);
    assert_eq!(pseudo_fg(&app, h, PseudoSlot::Before), Some(RED));
    assert_ne!(
        app.dom().node(h).computed().map(|c| c.fg),
        Some(RED),
        "the host is not styled"
    );
    mouse(&mut app, MouseEventKind::Moved, 3, 0);
    assert_ne!(
        pseudo_fg(&app, h, PseudoSlot::Before),
        Some(RED),
        "over the host's text"
    );
    mouse(&mut app, MouseEventKind::Moved, 0, 0);
    assert_eq!(pseudo_fg(&app, h, PseudoSlot::Before), Some(RED));
    mouse(&mut app, MouseEventKind::Moved, 8, 2);
    assert_ne!(
        pseudo_fg(&app, h, PseudoSlot::Before),
        Some(RED),
        "the pointer left"
    );
}

/// §3.6.3 with CSS Lists 3 §3.1: `::marker:hover` styles a list item's
/// marker while the pointer is over the marker's cells. (An `inside`
/// one: an outside marker is not in the hit-test set, DIVERGENCES §2.)
#[test]
fn marker_hover_matches_over_the_marker() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let ul = el(&mut dom, root, "ul", "");
    let li = text_el(&mut dom, ul, "li", "", "x");
    let mut app = app(
        dom,
        "li { list-style-position: inside } li::marker:hover { color: red }",
    );
    mouse(&mut app, MouseEventKind::Moved, 4, 0);
    assert_eq!(pseudo_fg(&app, li, PseudoSlot::Marker), Some(RED));
    mouse(&mut app, MouseEventKind::Moved, 6, 0);
    assert_ne!(
        pseudo_fg(&app, li, PseudoSlot::Marker),
        Some(RED),
        "over the item's text"
    );
}

/// §3.6.3 with CSS Lists 3 §3.5: an `outside` marker (the initial
/// position) is a box of its item hung in the list's padding, and
/// `::marker:hover` matches while the pointer is over it (C10G-MARKER-HIT;
/// browsers hit-test the marker to the item).
#[test]
fn outside_marker_hover_matches_over_the_marker() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let ul = el(&mut dom, root, "ul", "");
    let li = text_el(&mut dom, ul, "li", "", "x");
    let mut app = app(dom, "li::marker:hover { color: red }");
    mouse(&mut app, MouseEventKind::Moved, 2, 0);
    assert_eq!(pseudo_fg(&app, li, PseudoSlot::Marker), Some(RED));
    mouse(&mut app, MouseEventKind::Moved, 0, 0);
    assert_ne!(
        pseudo_fg(&app, li, PseudoSlot::Marker),
        Some(RED),
        "over the list's padding beside it"
    );
}

/// §3.6.3 with CSS Pseudo 4 §2.3: `::first-letter:hover` styles the
/// first letter while the pointer is over it, not over the rest of the
/// line.
#[test]
fn first_letter_hover_matches_over_the_letter() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = text_el(&mut dom, root, "p", "", "hello");
    let mut app = app(dom, "p::first-letter:hover { color: red }");
    mouse(&mut app, MouseEventKind::Moved, 0, 0);
    assert_eq!(pseudo_fg(&app, p, PseudoSlot::FirstLetter), Some(RED));
    mouse(&mut app, MouseEventKind::Moved, 2, 0);
    assert_ne!(pseudo_fg(&app, p, PseudoSlot::FirstLetter), Some(RED));
}

/// §9.4: `::after:active` matches while the `::after` is pressed — not
/// while it is only hovered, and not after the release.
#[test]
fn after_active_matches_while_the_after_is_pressed() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let h = text_el(&mut dom, root, "div", "h", "xy");
    let mut app = app(
        dom,
        ".h::after { content: 'Z' } .h::after:active { color: red }",
    );
    mouse(&mut app, MouseEventKind::Moved, 2, 0);
    assert_ne!(pseudo_fg(&app, h, PseudoSlot::After), Some(RED), "hovered");
    mouse(&mut app, MouseEventKind::Down(MouseButton::Left), 2, 0);
    assert_eq!(pseudo_fg(&app, h, PseudoSlot::After), Some(RED), "pressed");
    mouse(&mut app, MouseEventKind::Up(MouseButton::Left), 2, 0);
    assert_ne!(pseudo_fg(&app, h, PseudoSlot::After), Some(RED), "released");
}

/// §9.5–§9.7: `:focus`, `:focus-visible` and `:focus-within` describe
/// the focused element and its ancestors; a pseudo-element is never
/// focused and holds no focusable element. Each parses (the strict sheet
/// accepts it) and never matches — not even with its host focused and
/// the pseudo-element hovered.
#[test]
fn focus_pseudo_classes_never_match_a_pseudo_element() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let h = text_el(&mut dom, root, "div", "h", "xy");
    dom.set_attribute(h, "tabindex", "0").unwrap();
    let mut app = app(
        dom,
        ".h::before { content: 'ab' } \
         .h::before:focus, .h::before:focus-visible, .h::before:focus-within { color: red }",
    );
    mouse(&mut app, MouseEventKind::Down(MouseButton::Left), 0, 0);
    assert_eq!(app.dom().focused(), Some(h));
    assert_ne!(pseudo_fg(&app, h, PseudoSlot::Before), Some(RED));
}

/// The query behind it: which pseudo-element the pointer is over — its
/// host and slot — found through the same hit test as elements (CSS
/// Pseudo 4 §2: a positioned pseudo-element is hit like an element).
#[test]
fn hit_test_pseudo_names_the_pseudo_element_under_a_point() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let h = text_el(&mut dom, root, "div", "h", "xy");
    lay_out(
        &mut dom,
        ".h { position: relative; width: 4; height: 1 } \
         .h::before { content: '>' } \
         .h::after { position: absolute; left: 1; top: 2; content: 'zz' }",
        10,
        4,
    );
    assert_eq!(dom.hit_test_pseudo(0, 0), Some((h, PseudoSlot::Before)));
    assert_eq!(dom.hit_test_pseudo(1, 0), None, "the host's text");
    assert_eq!(dom.hit_test_pseudo(2, 2), Some((h, PseudoSlot::After)));
    assert_eq!(dom.hit_test_pseudo(5, 2), None);
}
