//! C14G-SKIP-WALKS — skipped contents (CSS Containment 2 §4) in the walks
//! that do not go through the box tree: Tab order, copy and the animation
//! clock. A closed `<details>`'s slot and `content-visibility: hidden`
//! skip their contents for every feature; `content-visibility: auto`'s
//! skipped contents stay "available as normal to user-agent features such
//! as find-in-page, tab order navigation, etc., and must be focusable and
//! selectable as normal" (§4), and focusing into them makes the element
//! relevant to the user (§4.4).

use crossterm::event::{Event as CtEvent, KeyCode, KeyEvent, KeyModifiers};
use rdom_core::{NodeId, Position, Range};

use crate::TuiDom;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::runtime::selection::clipboard::serialize_selection;

fn app(markup: &str, css: &str, h: u16) -> App<TestBackend> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    rdom_parser::parse_into(&mut dom, markup, root).unwrap();
    let parsed = rdom_css::parse(css);
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    let terminal = Terminal::new(TestBackend::new(30, h)).unwrap();
    // The UA sheet holds the closed `<details>` slot's
    // `content-visibility: hidden`.
    let mut app = App::with_backend(dom, crate::style::Stylesheet::new(), terminal).unwrap();
    app.push_stylesheet(parsed.stylesheet);
    app.advance(0).unwrap();
    app
}

fn id(app: &App<TestBackend>, name: &str) -> NodeId {
    app.dom().get_element_by_id(name).unwrap()
}

fn tab(app: &mut App<TestBackend>) {
    app.handle_event(CtEvent::Key(KeyEvent::new(
        KeyCode::Tab,
        KeyModifiers::empty(),
    )));
    app.advance(0).unwrap();
}

/// The copy of a selection from the first text of the document to the
/// end of its last (Ctrl+A).
fn copy_all(app: &App<TestBackend>) -> String {
    let dom = app.dom();
    let texts: Vec<NodeId> = descendants(dom, dom.root())
        .into_iter()
        .filter(|&n| dom.node(n).node_type() == rdom_core::NodeType::Text)
        .collect();
    let (first, last) = (texts[0], *texts.last().unwrap());
    let len = dom.node(last).node_value().unwrap().len();
    let range = Range::ordered_unchecked(Position::new(first, 0), Position::new(last, len));
    serialize_selection(dom, &range)
}

fn descendants(dom: &TuiDom, id: NodeId) -> Vec<NodeId> {
    let mut out = vec![id];
    for c in dom.node(id).child_nodes() {
        out.extend(descendants(dom, c.id()));
    }
    out
}

const DETAILS: &str = r#"<p>a</p><details id="d"><summary id="s">S</summary><button id="in">in</button><p>secret</p></details><button id="out">out</button>"#;

/// §4 (`hidden`, which a closed `<details>`'s `::details-content` slot is,
/// HTML §15.5.20): the skipped contents are not in the tab order — Tab
/// goes from the summary to the content after the `<details>`, and wraps
/// without stopping inside it.
#[test]
fn tab_skips_a_closed_details_and_reaches_what_follows() {
    let mut app = app(DETAILS, "", 10);
    tab(&mut app);
    assert_eq!(app.dom().focused(), Some(id(&app, "s")));
    tab(&mut app);
    assert_eq!(app.dom().focused(), Some(id(&app, "out")));
    tab(&mut app);
    assert_eq!(app.dom().focused(), Some(id(&app, "s")), "wrapped");
}

/// §4: `content-visibility: hidden`'s skipped contents are not focusable.
#[test]
fn tab_skips_content_visibility_hidden() {
    let mut app = app(
        r#"<div id="c"><button id="in">in</button></div><button id="out">out</button>"#,
        "#c { content-visibility: hidden }",
        10,
    );
    tab(&mut app);
    assert_eq!(app.dom().focused(), Some(id(&app, "out")));
}

/// HTML §3.2.7 (rendered text) with §4: the skipped contents of a closed
/// `<details>` and of `content-visibility: hidden` are not copied; an
/// off-screen `content-visibility: auto` element's are ("selectable as
/// normal").
#[test]
fn copy_leaves_out_skipped_contents_but_not_autos() {
    let app = app(
        &format!(
            r#"{DETAILS}<div id="h"><p>gone</p></div><div id="spacer"></div><div id="auto"><p>kept</p></div>"#
        ),
        "p { margin: 0 } #h { content-visibility: hidden } #spacer { height: 30 } \
         #auto { content-visibility: auto }",
        10,
    );
    assert!(
        crate::style::content_visibility::skips_contents(app.dom(), id(&app, "auto")),
        "the auto element is off-screen, skipping"
    );
    // The buttons' text is `user-select: none` (the UA's), so not copied.
    assert_eq!(copy_all(&app), "a\nS\nkept");
}

fn needs_frames(app: &App<TestBackend>) -> bool {
    app.animations.needs_frames(app.scheduler.borrow().now())
}

const SPIN: &str = "@keyframes spin { from { margin-left: 0 } to { margin-left: 9 } } \
                    #spin { animation: spin 1s linear infinite }";

/// §4: skipped contents are not rendered, so an animation running in
/// them moves nothing on screen. It keeps its timeline — the engines
/// neither restyle skipped contents nor cancel what runs there, they
/// throttle it — and asks for no frames; opening the `<details>` again
/// brings the frames back.
#[test]
fn a_spinner_in_a_closed_details_asks_for_no_frames() {
    let mut app = app(
        r#"<details id="d" open><summary>S</summary><p id="spin">x</p></details>"#,
        SPIN,
        10,
    );
    app.advance(16).unwrap();
    assert!(needs_frames(&app), "open: the spinner moves");
    let d = id(&app, "d");
    app.dom_mut().remove_attribute(d, "open").unwrap();
    app.advance(16).unwrap();
    assert!(!needs_frames(&app), "the closed content's spinner is idle");
    app.take_frame_stats();
    app.advance(16).unwrap();
    app.advance(16).unwrap();
    let idle = app.take_frame_stats();
    assert_eq!(idle.paints, 0, "{idle:?}");
    let spin = id(&app, "spin");
    assert_eq!(app.get_animations(spin).len(), 1, "not cancelled");
    app.dom_mut().set_attribute(d, "open", "").unwrap();
    app.advance(16).unwrap();
    assert!(needs_frames(&app), "open again: it moves again");
}

/// §4 `content-visibility: hidden` too.
#[test]
fn a_spinner_in_hidden_contents_asks_for_no_frames() {
    let mut app = app(
        r#"<div id="c"><p id="spin">x</p></div>"#,
        &format!("{SPIN} #c.off {{ content-visibility: hidden }}"),
        10,
    );
    app.advance(16).unwrap();
    assert!(needs_frames(&app));
    let c = id(&app, "c");
    app.dom_mut().set_attribute(c, "class", "off").unwrap();
    app.advance(16).unwrap();
    assert!(!needs_frames(&app));
    assert_eq!(
        app.get_animations(id(&app, "spin")).len(),
        1,
        "not cancelled"
    );
}

/// §4 / §4.4: Tab reaches a button in an off-screen `auto` element's
/// skipped contents, and the focus makes the element relevant — it stops
/// skipping, and the focus fixup keeps the button focused.
#[test]
fn focus_into_auto_contents_makes_them_relevant() {
    let mut app = app(
        r#"<div id="spacer"></div><div id="c"><button id="b">b</button></div>"#,
        "#spacer { height: 30 } #c { content-visibility: auto }",
        5,
    );
    let c = id(&app, "c");
    assert!(crate::style::content_visibility::skips_contents(
        app.dom(),
        c
    ));
    let b = id(&app, "b");
    assert_eq!(
        crate::runtime::focus::tabindex::tab_index(app.dom(), b),
        Some(0),
        "a focusable area"
    );
    tab(&mut app);
    assert_eq!(app.dom().focused(), Some(id(&app, "b")));
    app.advance(16).unwrap();
    assert_eq!(app.dom().focused(), Some(id(&app, "b")), "not blurred");
    assert!(!crate::style::content_visibility::skips_contents(
        app.dom(),
        c
    ));
}

/// HTML §4.11.4's dialog focusing steps find no `autofocus` control in
/// skipped contents (§4).
#[test]
fn show_modal_does_not_autofocus_into_a_closed_details() {
    let mut app = app(
        r#"<dialog id="dlg"><details><summary>S</summary><button id="in" autofocus>in</button></details></dialog>"#,
        "",
        10,
    );
    let dlg = id(&app, "dlg");
    crate::runtime::builtins::dialog::show_modal(app.dom_mut(), dlg).unwrap();
    let focused = app.dom().focused();
    assert!(focused.is_some(), "the steps focused something");
    assert_ne!(focused, Some(id(&app, "in")));
}
