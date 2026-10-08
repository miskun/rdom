//! C11-HAS — Selectors 4 §4.5 `:has()` through a sheet, and the targeted
//! restyle when what an anchor's match reads changes: its subtree, its
//! later siblings, and the interaction states inside them.

use crossterm::event::{Event as CtEvent, KeyModifiers, MouseEvent, MouseEventKind};
use rdom_tui::render::TestBackend;
use rdom_tui::{App, NodeId, TuiDom};

use super::{BLUE, RED, UNSTYLED, app, app_fg, cascade, el, fg};

/// Selectors 4 §4.5: an anchor styled by what it contains, a child and
/// a later sibling; `:has()` in a non-subject compound styles the
/// anchor's descendants.
#[test]
fn has_styles_through_a_sheet() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let card = el(&mut dom, root, "div", "card");
    let body = el(&mut dom, card, "div", "");
    el(&mut dom, body, "span", "err");
    let plain = el(&mut dom, root, "div", "card");
    let h = el(&mut dom, root, "h2", "");
    el(&mut dom, root, "p", "note");
    cascade(
        &mut dom,
        ".card:has(.err) { color: red } .card:has(.err) div { background-color: blue } \
         h2:has(+ p.note) { color: blue }",
    );
    assert_eq!(fg(&dom, card), RED);
    assert_eq!(fg(&dom, plain), UNSTYLED);
    assert_eq!(fg(&dom, h), BLUE);
    use rdom_tui::TuiNodeExt;
    assert_eq!(dom.node(body).computed().unwrap().bg, BLUE);
}

/// A class added deep inside an anchor, or removed, restyles it — and
/// what its `:has()` styles below it.
#[test]
fn a_change_inside_restyles_the_anchor() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let card = el(&mut dom, root, "div", "card");
    let body = el(&mut dom, card, "div", "");
    let leaf = el(&mut dom, body, "span", "");
    let mut app = app(
        dom,
        ".card:has(.err) { color: red } .card:has(.err) span { color: blue }",
    );
    assert_eq!(app_fg(&app, card), UNSTYLED);
    app.dom_mut().add_class(leaf, "err").unwrap();
    app.advance(0).unwrap();
    assert_eq!(app_fg(&app, card), RED);
    assert_eq!(app_fg(&app, leaf), BLUE);
    app.dom_mut().remove_class(leaf, "err").unwrap();
    app.advance(0).unwrap();
    assert_eq!(app_fg(&app, card), UNSTYLED);
}

/// Selectors 4 §4.5: `+` / `~` relative selectors read the anchor's later
/// siblings (and what is inside them) — a class change there restyles
/// the anchor, which no sibling combinator of the sheet would.
#[test]
fn a_change_in_a_later_sibling_restyles_the_anchor() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let wrap = el(&mut dom, root, "div", "");
    let h = el(&mut dom, wrap, "h2", "");
    let p = el(&mut dom, wrap, "p", "");
    let aside = el(&mut dom, wrap, "aside", "");
    let deep = el(&mut dom, aside, "span", "");
    let mut app = app(
        dom,
        "h2:has(+ p.note) { color: red } h2:has(~ aside .warn) { background-color: blue }",
    );
    assert_eq!(app_fg(&app, h), UNSTYLED);
    app.dom_mut().add_class(p, "note").unwrap();
    app.dom_mut().add_class(deep, "warn").unwrap();
    app.advance(0).unwrap();
    assert_eq!(app_fg(&app, h), RED);
    use rdom_tui::TuiNodeExt;
    assert_eq!(app.dom().node(h).computed().unwrap().bg, BLUE);
}

/// Inserting or removing a child, or a later sibling, restyles the
/// anchors that read it. (A child-list change restyles the parent's
/// children anyway, so the sibling case holds through that too; the
/// `ul` — the parent itself, below a wrapper — is the anchor-only case.)
#[test]
fn inserting_and_removing_restyles_the_anchor() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let wrap = el(&mut dom, root, "div", "");
    let ul = el(&mut dom, wrap, "ul", "");
    el(&mut dom, ul, "li", "");
    let heads = el(&mut dom, root, "div", "");
    let h = el(&mut dom, heads, "h2", "");
    let mut app = app(
        dom,
        "ul:has(> li.sel) { color: red } h2:has(+ p) { color: blue }",
    );
    let sel = app.dom_mut().create_element("li");
    app.dom_mut().add_class(sel, "sel").unwrap();
    app.dom_mut().append_child(ul, sel).unwrap();
    let p = app.dom_mut().create_element("p");
    app.dom_mut().append_child(heads, p).unwrap();
    app.advance(0).unwrap();
    assert_eq!(app_fg(&app, ul), RED);
    assert_eq!(app_fg(&app, h), BLUE);
    app.dom_mut().remove_child(ul, sel).unwrap();
    let em = app.dom_mut().create_element("em");
    app.dom_mut().insert_before(heads, em, Some(p)).unwrap();
    app.advance(0).unwrap();
    assert_eq!(app_fg(&app, ul), UNSTYLED);
    assert_eq!(app_fg(&app, h), UNSTYLED, "`+ p` no longer");
}

/// Feed one mouse move to `(column, row)` and draw the frame.
fn hover(app: &mut App<TestBackend>, column: u16, row: u16) {
    app.handle_event(CtEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Moved,
        column,
        row,
        modifiers: KeyModifiers::empty(),
    }));
    app.advance(0).unwrap();
}

/// Selectors 4 §4.5 with §9: `:has(:hover)`, `:has(:checked)` and
/// `:has(:focus)` follow the state as it changes.
#[test]
fn has_follows_interaction_states() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let row = el(&mut dom, root, "div", "row");
    let cell = el(&mut dom, row, "span", "");
    let t = dom.create_text_node("hi");
    dom.append_child(cell, t).unwrap();
    let form = el(&mut dom, root, "div", "form");
    let check = el(&mut dom, form, "input", "");
    dom.set_attribute(check, "type", "checkbox").unwrap();
    let boxed = el(&mut dom, root, "div", "box");
    let field = el(&mut dom, boxed, "input", "");
    let mut app: App<TestBackend> = app(
        dom,
        ".row:has(:hover) { color: red } .form:has(:checked) { color: red } \
         .box:has(:focus) { color: blue }",
    );
    let _: NodeId = t;
    assert_eq!(app_fg(&app, row), UNSTYLED);
    hover(&mut app, 0, 0);
    assert_eq!(app_fg(&app, row), RED);
    hover(&mut app, 19, 9);
    assert_eq!(app_fg(&app, row), UNSTYLED);
    app.dom_mut().set_attribute(check, "checked", "").unwrap();
    app.advance(0).unwrap();
    assert_eq!(app_fg(&app, form), RED);
    app.dom_mut().remove_attribute(check, "checked").unwrap();
    app.advance(0).unwrap();
    assert_eq!(app_fg(&app, form), UNSTYLED);
    app.dom_mut().set_focused(Some(field));
    app.advance(0).unwrap();
    assert_eq!(app_fg(&app, boxed), BLUE);
    app.dom_mut().set_focused(None);
    app.advance(0).unwrap();
    assert_eq!(app_fg(&app, boxed), UNSTYLED);
}
