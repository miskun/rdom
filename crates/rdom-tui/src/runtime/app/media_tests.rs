//! C14-MEDIA part 2 — the `App`'s media environment: the preferences an
//! app reports (Media Queries 5 §12), `matchMedia` and its `change` event
//! (CSSOM View §4.2), a resize that restyles only when a query flips or a
//! viewport unit is in use, and `<style media>` (HTML §4.2.6).

use std::cell::RefCell;
use std::rc::Rc;

use crossterm::event::Event as CtEvent;
use rdom_core::NodeId;

use crate::render::{Color, Terminal, TestBackend};
use crate::runtime::app::App;
use crate::style::Stylesheet;
use crate::{MediaPreferences, TuiDom, TuiNodeExt};

const RED: Color = Color::Rgb(255, 0, 0);
const BLUE: Color = Color::Rgb(0, 0, 255);

/// An App `w` × `h` over `<div id=a>a</div>` styled by `css`, one frame
/// drawn.
fn app(css: &str, w: u16, h: u16) -> (App<TestBackend>, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.set_attribute(div, "id", "a").unwrap();
    let text = dom.create_text_node("a");
    dom.append_child(div, text).unwrap();
    dom.append_child(root, div).unwrap();
    let parsed = rdom_css::parse(css);
    assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    let terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
    let mut app = App::with_backend(dom, parsed.stylesheet, terminal).unwrap();
    app.draw_if_dirty().unwrap();
    (app, div)
}

fn fg(app: &App<TestBackend>, id: NodeId) -> Color {
    app.dom().node(id).computed().unwrap().fg
}

/// Resize the terminal to `w` × `h` as a `SIGWINCH` would, then draw.
fn resize(app: &mut App<TestBackend>, w: u16, h: u16) {
    app.terminal_mut().backend_mut().resize(w, h);
    app.handle_event(CtEvent::Resize(w, h));
    app.draw_if_dirty().unwrap();
}

// ─── Preferences ───────────────────────────────────────────────────────

/// Media Queries 5 §12.1: `prefers-reduced-motion` is the app's to report,
/// `no-preference` until it does; a preference set at run time restyles.
#[test]
fn the_app_reports_the_media_preferences() {
    let css = "#a { color: blue } @media (prefers-reduced-motion: reduce) { #a { color: red } }";
    let (app, div) = app(css, 10, 2);
    assert_eq!(app.media_preferences(), MediaPreferences::default());
    assert_eq!(fg(&app, div), BLUE);
    let mut app = app.with_media_preferences(MediaPreferences::new().with_reduced_motion(true));
    app.draw_if_dirty().unwrap();
    assert_eq!(fg(&app, div), RED);
    app.set_media_preferences(MediaPreferences::new());
    app.draw_if_dirty().unwrap();
    assert_eq!(fg(&app, div), BLUE);
}

/// A preference change that flips no query restyles nothing.
#[test]
fn a_preference_that_flips_no_query_does_not_cascade() {
    let css = "#a { color: blue } @media (prefers-reduced-motion: reduce) { #a { color: red } }";
    let (mut app, _) = app(css, 10, 2);
    app.take_frame_stats();
    app.set_media_preferences(MediaPreferences::new().with_contrast(crate::Contrast::More));
    app.draw_if_dirty().unwrap();
    assert_eq!(app.take_frame_stats().full_cascades, 0);
}

// ─── Resize ────────────────────────────────────────────────────────────

/// Media Queries 4 §4.1: a resize re-evaluates `width`, and the rules
/// whose query flipped restyle the tree.
#[test]
fn a_resize_that_flips_a_query_restyles() {
    let css = "#a { color: blue } @media (width < 20) { #a { color: red } }";
    let (mut app, div) = app(css, 30, 2);
    assert_eq!(fg(&app, div), BLUE);
    app.take_frame_stats();
    resize(&mut app, 10, 2);
    assert_eq!(fg(&app, div), RED);
    assert_eq!(app.take_frame_stats().full_cascades, 1);
}

/// A resize that flips no query and meets no viewport unit lays out
/// again — the boxes depend on the size — but restyles nothing.
#[test]
fn a_resize_that_flips_nothing_only_lays_out() {
    let css = "#a { color: blue } @media (width < 5) { #a { color: red } }";
    let (mut app, div) = app(css, 30, 2);
    app.take_frame_stats();
    resize(&mut app, 20, 2);
    let stats = app.take_frame_stats();
    assert_eq!(
        (stats.full_cascades, stats.subtree_cascades),
        (0, 0),
        "{stats:?}"
    );
    assert_eq!(stats.layouts, 1, "{stats:?}");
    assert_eq!(fg(&app, div), BLUE);
}

/// CSS Values 4 §6.1.2: a viewport unit in use is relative to the new
/// size, so a resize restyles the tree.
#[test]
fn a_resize_restyles_a_document_using_viewport_units() {
    let css = "#a { width: 50vw }";
    let (mut app, div) = app(css, 30, 2);
    app.take_frame_stats();
    resize(&mut app, 20, 2);
    assert_eq!(app.take_frame_stats().full_cascades, 1);
    let width = app.dom().node(div).layout_rect().map(|r| r.width);
    assert_eq!(width, Some(10));
}

/// C14G-CONTAINER-LOOP (architect N6): the viewport-read flag describes
/// the styles in use — once the last viewport unit is gone (its sheet
/// replaced), a resize no longer restyles the tree.
#[test]
fn a_resize_stops_restyling_once_the_viewport_units_are_gone() {
    let (mut app, _) = app("#a { width: 50vw }", 30, 2);
    app.set_stylesheet(rdom_css::parse("#a { width: 5 }").stylesheet);
    app.draw_if_dirty().unwrap();
    app.take_frame_stats();
    resize(&mut app, 20, 2);
    let stats = app.take_frame_stats();
    assert_eq!(stats.full_cascades, 0, "{stats:?}");
}

// ─── matchMedia ────────────────────────────────────────────────────────

/// CSSOM View §4.2: `matchMedia` returns a live list — its serialized
/// `media` and whether it matches now.
#[test]
fn match_media_answers_now() {
    let (mut app, _) = app("", 30, 2);
    let wide = app.match_media("(MIN-WIDTH: 20)");
    assert_eq!(wide.media(), "(width >= 20)");
    assert!(wide.matches());
    assert!(!app.match_media("print").matches());
    assert!(app.match_media("(hover)").matches());
}

/// §4.2 "evaluate media queries and report changes": a list whose result
/// flipped fires `change` with its new `matches`, once per flip; one that
/// did not flip fires nothing; a removed listener is not called.
#[test]
fn match_media_reports_changes() {
    let (mut app, div) = app("", 30, 2);
    let narrow = app.match_media("(width < 20)");
    let seen: Rc<RefCell<Vec<bool>>> = Rc::default();
    let log = seen.clone();
    narrow.add_listener(move |cx, event| {
        log.borrow_mut().push(event.matches);
        // A listener may change the document, as a script would.
        cx.dom.set_attribute(div, "class", "narrow").unwrap();
    });
    let other = app.match_media("(height > 100)");
    let removed = other.add_listener(|_, _| panic!("did not flip"));
    resize(&mut app, 25, 2);
    assert!(seen.borrow().is_empty(), "no flip");
    resize(&mut app, 10, 2);
    assert_eq!(*seen.borrow(), [true]);
    assert!(narrow.matches());
    assert_eq!(app.dom().get_attribute(div, "class"), Some("narrow"));
    resize(&mut app, 30, 2);
    assert_eq!(*seen.borrow(), [true, false]);
    assert!(other.remove_listener(removed));
    let gone = narrow.add_listener(|_, _| panic!("removed"));
    assert!(narrow.remove_listener(gone));
    resize(&mut app, 10, 2);
}

/// CSSOM View §4.2 (API B2): a list with a listener stays alive while it
/// has one, as a browser keeps it — the chained form, whose handle is
/// dropped at once, still hears its flips (it was dropped with the handle).
#[test]
fn a_chained_listener_fires() {
    let (mut app, _) = app("", 30, 2);
    let seen: Rc<RefCell<Vec<bool>>> = Rc::default();
    let log = seen.clone();
    app.match_media("(width < 20)")
        .add_listener(move |_, event| log.borrow_mut().push(event.matches));
    resize(&mut app, 10, 2);
    assert_eq!(*seen.borrow(), [true]);
}

/// The `EventTarget` form (API N8): `add_event_listener("change", …)`
/// hears the flips; a listener for another type never fires; either is
/// removed by its id.
#[test]
fn add_event_listener_change_fires() {
    let (mut app, _) = app("", 30, 2);
    let list = app.match_media("(width < 20)");
    let seen: Rc<RefCell<Vec<bool>>> = Rc::default();
    let log = seen.clone();
    list.add_event_listener("change", move |_, event| {
        log.borrow_mut().push(event.matches)
    });
    let other = list.add_event_listener("click", |_, _| panic!("no click on a list"));
    resize(&mut app, 10, 2);
    assert_eq!(*seen.borrow(), [true]);
    assert!(list.remove_event_listener(other));
}

/// A list with no listener that nobody holds is forgotten (it is not
/// evaluated again); one with a listener is kept.
#[test]
fn an_unheld_list_without_listeners_is_forgotten() {
    let (mut app, _) = app("", 30, 2);
    drop(app.match_media("(width < 20)"));
    app.match_media("(width < 10)").add_listener(|_, _| {});
    resize(&mut app, 25, 2);
    assert_eq!(app.media_watches.len(), 1);
}

/// A preference change reports too (Media Queries 5 §12).
#[test]
fn match_media_reports_a_preference_change() {
    let (mut app, _) = app("", 30, 2);
    let reduce = app.match_media("(prefers-reduced-motion: reduce)");
    let seen: Rc<RefCell<Vec<bool>>> = Rc::default();
    let log = seen.clone();
    reduce.add_listener(move |_, event| log.borrow_mut().push(event.matches));
    app.set_media_preferences(MediaPreferences::new().with_reduced_motion(true));
    app.draw_if_dirty().unwrap();
    assert_eq!(*seen.borrow(), [true]);
}

// ─── <style media> ─────────────────────────────────────────────────────

/// HTML §4.2.6: a `<style media>` sheet applies only while its media list
/// matches (CSSOM `StyleSheet.media`).
#[test]
fn a_style_element_media_attribute_conditions_its_sheet() {
    let (mut app, div) = app("", 30, 2);
    let root = app.dom().root();
    let style = app.dom_mut().create_element("style");
    app.dom_mut()
        .set_attribute(style, "media", "(width < 20)")
        .unwrap();
    let text = app.dom_mut().create_text_node("#a { color: red }");
    app.dom_mut().append_child(style, text).unwrap();
    app.dom_mut().append_child(root, style).unwrap();
    app.draw_if_dirty().unwrap();
    assert_ne!(fg(&app, div), RED, "wide: the sheet does not apply");
    resize(&mut app, 10, 2);
    assert_eq!(fg(&app, div), RED, "narrow: it does");
}

/// `Stylesheet::set_media` is the sheet-level media list: every rule of
/// the sheet, its definitions too, apply only while it matches.
#[test]
fn a_sheet_media_list_conditions_every_rule() {
    let mut sheet = Stylesheet::bare().rule_unchecked("#a", crate::TuiStyle::new().fg(RED));
    sheet.set_media(Some(crate::MediaList::parse("(width < 20)")));
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.set_attribute(div, "id", "a").unwrap();
    dom.append_child(root, div).unwrap();
    let terminal = Terminal::new(TestBackend::new(30, 2)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.draw_if_dirty().unwrap();
    assert_ne!(fg(&app, div), RED);
    resize(&mut app, 10, 2);
    assert_eq!(fg(&app, div), RED);
}

/// Media Queries 5 §12.1, as browsers do it: no engine hook — a sheet's
/// `@media (prefers-reduced-motion: reduce)` turns the animation off
/// through the cascade.
#[test]
fn reduced_motion_turns_an_animation_off_through_css() {
    let css = "@keyframes k { to { color: red } } #a { animation: k 1s infinite } \
               @media (prefers-reduced-motion: reduce) { #a { animation: none } }";
    let (app, div) = app(css, 10, 2);
    assert_eq!(app.get_animations(div).len(), 1);
    let mut app = app.with_media_preferences(MediaPreferences::new().with_reduced_motion(true));
    app.draw_if_dirty().unwrap();
    assert!(app.get_animations(div).is_empty());
}

// ─── content-visibility: auto in an App ────────────────────────────────

/// CSS Containment 2 §4: an `auto` element that starts skipping its
/// contents fires `contentvisibilityautostatechange` at itself with
/// `skipped`, after the frame that laid it out.
#[test]
fn content_visibility_auto_fires_its_state_change() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    rdom_parser::parse_into(
        &mut dom,
        r#"<body><div id="spacer"></div><div id="c"><p>a</p></div></body>"#,
        root,
    )
    .unwrap();
    let c = dom.get_element_by_id("c").unwrap();
    let seen: Rc<RefCell<Vec<bool>>> = Rc::default();
    let log = seen.clone();
    dom.add_event_listener(
        c,
        "contentvisibilityautostatechange",
        rdom_core::ListenerOptions::default(),
        move |ev| {
            log.borrow_mut()
                .push(ev.event.detail.as_content_visibility_skipped().unwrap())
        },
    )
    .unwrap();
    let sheet =
        rdom_css::parse("#spacer { height: 10 } #c { content-visibility: auto }").stylesheet;
    let terminal = Terminal::new(TestBackend::new(10, 3)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.draw_if_dirty().unwrap();
    assert_eq!(*seen.borrow(), [true]);
    resize(&mut app, 10, 20);
    app.draw_if_dirty().unwrap();
    assert_eq!(
        *seen.borrow(),
        [true, false],
        "on screen once the terminal is taller"
    );
}
