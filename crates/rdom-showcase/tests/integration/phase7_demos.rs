//! Paint snapshots and interaction tests for the Phase 6 / 7 demos:
//! `form_lifecycle`, `translucency`, `selection_hosts`,
//! `lists_generated`, `scroll_live_style`. Each runs in a headless `App`
//! (the runtime seeds the form controls, marks `:invalid` and reads the
//! `<style>` element), and the snapshot is what the App painted.

use crossterm::event::{Event as CtEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use rdom_showcase::demos::{
    form_lifecycle, lists_generated, scroll_live_style, selection_hosts, translucency,
};
use rdom_tui::node::TuiNodeExt;
use rdom_tui::render::{Terminal, TestBackend};
use rdom_tui::runtime::app::App;
use rdom_tui::{Color, NodeId, Stylesheet, TuiAccessors, TuiAccessorsMut, TuiDom, VirtualScreen};

use crate::common::assert_snapshot;

/// Mount `demo_root` in a `w×h` headless App and draw the first frame.
fn app_with(dom: TuiDom, sheet: Stylesheet, w: u16, h: u16) -> App<TestBackend> {
    let terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.advance(0).unwrap();
    app
}

fn mount(build: impl FnOnce(&mut TuiDom) -> NodeId) -> TuiDom {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let demo = build(&mut dom);
    dom.append_child(root, demo).unwrap();
    dom
}

/// What the App has painted so far, one trimmed line per row.
fn painted(app: &App<TestBackend>) -> String {
    let size = app.terminal().size();
    let mut screen = VirtualScreen::new(size.width, size.height);
    screen.apply(app.terminal().backend().bytes());
    let mut out = String::new();
    for row in screen.rows() {
        out.push_str(row.trim_end());
        out.push('\n');
    }
    out
}

fn text(app: &App<TestBackend>, id: NodeId) -> String {
    app.dom().node(id).text_content()
}

fn click(app: &mut App<TestBackend>, id: NodeId) {
    app.dom_mut().node_mut(id).click();
    app.advance(0).unwrap();
}

// ── Form lifecycle ──────────────────────────────────────────────────

fn form_app() -> (App<TestBackend>, form_lifecycle::Parts) {
    let mut parts = None;
    let dom = mount(|dom| {
        let p = form_lifecycle::build_parts(dom);
        parts = Some(p);
        p.root
    });
    (
        app_with(dom, form_lifecycle::stylesheet(), 80, 16),
        parts.unwrap(),
    )
}

#[test]
fn form_lifecycle_initial_paint() {
    let (app, _) = form_app();
    assert_snapshot(&painted(&app), "form_lifecycle.snap");
}

#[test]
fn form_lifecycle_marks_the_invalid_fields() {
    let (app, parts) = form_app();
    let fg = |id| app.dom().node(id).computed().unwrap().fg;
    assert_eq!(fg(parts.name), Color::Rgb(255, 120, 120), "required, empty");
    assert_eq!(
        fg(parts.code),
        Color::Rgb(255, 120, 120),
        "pattern mismatch"
    );
}

#[test]
fn save_is_blocked_while_a_field_is_invalid() {
    let (mut app, parts) = form_app();
    click(&mut app, parts.save);
    assert_eq!(text(&app, parts.status), "blocked: fix the fields in red");
}

#[test]
fn save_submits_once_the_fields_are_valid() {
    let (mut app, parts) = form_app();
    app.dom_mut().node_mut(parts.name).set_value("Ada").unwrap();
    app.dom_mut().node_mut(parts.code).set_value("ABC").unwrap();
    click(&mut app, parts.save);
    assert_eq!(
        text(&app, parts.status),
        r#"submit by Save, validated, post: name="Ada" code="ABC""#,
        "the disabled fieldset's field is not submitted"
    );
}

#[test]
fn draft_submits_without_validation() {
    let (mut app, parts) = form_app();
    click(&mut app, parts.draft);
    assert_eq!(
        text(&app, parts.status),
        r#"submit by Draft, not validated, post: name="" code="abc""#
    );
}

#[test]
fn reset_restores_the_default_values() {
    let (mut app, parts) = form_app();
    app.dom_mut().node_mut(parts.name).set_value("Ada").unwrap();
    app.dom_mut().node_mut(parts.code).set_value("XYZ").unwrap();
    click(&mut app, parts.reset);
    assert_eq!(
        app.dom().node(parts.name).input_value().as_deref(),
        Some("")
    );
    assert_eq!(
        app.dom().node(parts.code).input_value().as_deref(),
        Some("abc")
    );
    assert_eq!(text(&app, parts.status), "reset: defaults restored");
}

// ── Translucency, lists, selection hosts ────────────────────────────

#[test]
fn translucency_initial_paint() {
    let app = app_with(
        mount(translucency::build),
        translucency::stylesheet(),
        60,
        14,
    );
    assert_snapshot(&painted(&app), "translucency.snap");
}

#[test]
fn lists_generated_initial_paint() {
    let app = app_with(
        mount(lists_generated::build),
        lists_generated::stylesheet(),
        60,
        14,
    );
    assert_snapshot(&painted(&app), "lists_generated.snap");
}

#[test]
fn selection_hosts_initial_paint() {
    let app = app_with(
        mount(selection_hosts::build),
        selection_hosts::stylesheet(),
        70,
        14,
    );
    assert_snapshot(&painted(&app), "selection_hosts.snap");
}

fn mouse(app: &mut App<TestBackend>, kind: MouseEventKind, x: u16, y: u16) {
    app.handle_event(CtEvent::Mouse(MouseEvent {
        kind,
        column: x,
        row: y,
        modifiers: KeyModifiers::empty(),
    }));
    app.advance(0).unwrap();
}

/// The row `class` paints its first line on, and the text node under it.
fn row_of(app: &App<TestBackend>, class: &str) -> (u16, NodeId) {
    let el = app
        .dom()
        .query_selector(&format!(".{class} p, p.{class}"))
        .unwrap()
        .id();
    let y = app.dom().node(el).tui_ext().unwrap().layout.y as u16;
    let t = app.dom().node(el).first_child().unwrap().id();
    (y, t)
}

#[test]
fn a_click_in_the_user_select_all_line_selects_all_of_it() {
    let mut app = app_with(
        mount(selection_hosts::build),
        selection_hosts::stylesheet(),
        70,
        14,
    );
    let (y, t) = row_of(&app, "all");
    mouse(&mut app, MouseEventKind::Down(MouseButton::Left), 10, y);
    mouse(&mut app, MouseEventKind::Up(MouseButton::Left), 10, y);
    let sel = app.dom().selection().expect("the click selected");
    let len = text(&app, t).len();
    assert_eq!(
        (sel.anchor.node, sel.anchor.offset, sel.focus.offset),
        (t, 0, len)
    );
}

#[test]
fn a_drag_from_the_contain_box_stays_inside_it() {
    let mut app = app_with(
        mount(selection_hosts::build),
        selection_hosts::stylesheet(),
        70,
        14,
    );
    let (y, t) = row_of(&app, "contain");
    let (tail_y, _) = row_of(&app, "tail");
    mouse(&mut app, MouseEventKind::Down(MouseButton::Left), 8, y);
    mouse(
        &mut app,
        MouseEventKind::Drag(MouseButton::Left),
        20,
        tail_y,
    );
    mouse(&mut app, MouseEventKind::Up(MouseButton::Left), 20, tail_y);
    let sel = app.dom().selection().expect("the drag selected");
    assert_eq!(sel.anchor.node, t);
    assert_eq!(sel.focus.node, t, "the focus is clamped to the host");
}

// ── Smooth scroll + live style ──────────────────────────────────────

fn scroll_app() -> (App<TestBackend>, scroll_live_style::Parts) {
    let mut parts = None;
    let dom = mount(|dom| {
        let p = scroll_live_style::build_parts(dom);
        parts = Some(p);
        p.root
    });
    (
        app_with(dom, scroll_live_style::stylesheet(), 80, 16),
        parts.unwrap(),
    )
}

#[test]
fn scroll_live_style_initial_paint() {
    let (app, _) = scroll_app();
    assert_snapshot(&painted(&app), "scroll_live_style.snap");
}

#[test]
fn bottom_scrolls_smoothly_to_the_last_entry() {
    let (mut app, parts) = scroll_app();
    click(&mut app, parts.bottom);
    // A 6-row box with a rounded border: a 4-row scrollport.
    let max = app.dom().node(parts.log).scroll_height().unwrap() - 4;
    app.advance(16).unwrap();
    let mid = app.dom().node(parts.log).scroll_top().unwrap();
    assert!(mid > 0 && mid < max, "in flight: {mid} of {max}");
    app.advance(300).unwrap();
    assert_eq!(app.dom().node(parts.log).scroll_top(), Some(max));
    click(&mut app, parts.top);
    app.advance(300).unwrap();
    assert_eq!(app.dom().node(parts.log).scroll_top(), Some(0));
}

#[test]
fn theme_rewrites_the_style_element_and_restyles_the_log() {
    let (mut app, parts) = scroll_app();
    let fg = |app: &App<TestBackend>| app.dom().node(parts.first).computed().unwrap().fg;
    assert_eq!(fg(&app), Color::Rgb(120, 200, 255));
    click(&mut app, parts.theme);
    assert_eq!(fg(&app), Color::Rgb(255, 180, 120));
    assert_eq!(text(&app, parts.theme_name), "theme: warm");
    click(&mut app, parts.theme);
    assert_eq!(fg(&app), Color::Rgb(120, 200, 255));
}
