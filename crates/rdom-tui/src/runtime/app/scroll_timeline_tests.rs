//! C12-SCROLL-DRIVEN — Scroll-driven Animations 1: an animation on a
//! scroll or view progress timeline takes its progress from a scroll
//! container's offset (the unified scrollport, C8G-SCROLLPORT), so
//! scrolling moves it with no clock tick.

use rdom_core::NodeId;

use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::style::Stylesheet;
use crate::{TuiAccessors, TuiAccessorsMut, TuiDom};

/// `<div id=s><div id=bar></div><p>…12 lines…</p></div>` styled by `css`,
/// one frame drawn: `#s` is a scroll container 4 rows tall over 13 rows
/// of content (range 9) when the sheet makes it one.
fn scroller(css: &str) -> (App<TestBackend>, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let s = dom.create_element("div");
    dom.set_attribute(s, "id", "s").unwrap();
    let bar = dom.create_element("div");
    dom.set_attribute(bar, "id", "bar").unwrap();
    dom.append_child(s, bar).unwrap();
    for i in 0..12 {
        let p = dom.create_element("p");
        let t = dom.create_text_node(&format!("line {i}"));
        dom.append_child(p, t).unwrap();
        dom.append_child(s, p).unwrap();
    }
    dom.append_child(root, s).unwrap();
    let sheet = rdom_css::parse(css);
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    let terminal = Terminal::new(TestBackend::new(30, 10)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    app.push_stylesheet(sheet.stylesheet);
    app.advance(0).unwrap();
    // The first layout gives the scroll range the timeline reads.
    app.advance(0).unwrap();
    (app, s, bar)
}

fn width(app: &App<TestBackend>, id: NodeId) -> u16 {
    app.dom().node(id).ext().unwrap().layout.width
}

fn scroll_top(app: &mut App<TestBackend>, id: NodeId, y: i32) {
    app.dom_mut().node_mut(id).set_scroll_top(y).unwrap();
    app.advance(0).unwrap();
}

const FILL: &str = "@keyframes fill { from { width: 0 } to { width: 18 } } \
    #s { height: 4; overflow: auto } p { margin: 0; height: 1 } #bar { height: 1 } ";

/// §2.1: `scroll()` follows the nearest scroll container on its block
/// axis — 0% at the top of the range, 100% at its end; with `auto` the
/// duration fills the range (CSS Animations 2 §3.3). Scrolling moves the
/// animation with the clock standing still.
#[test]
fn a_scroll_timeline_follows_the_scroll_offset() {
    let (mut app, s, bar) = scroller(&format!(
        "{FILL} #bar {{ animation: fill linear; animation-timeline: scroll() }}"
    ));
    assert_eq!(width(&app, bar), 0, "at the top");
    scroll_top(&mut app, s, 3);
    assert_eq!(width(&app, bar), 6, "a third of the way");
    scroll_top(&mut app, s, 9);
    assert_eq!(width(&app, bar), 18, "the end of the range is 100%");
    assert!(
        !needs_frames(&app),
        "a scroll timeline needs no clock frames"
    );
}

fn needs_frames(app: &App<TestBackend>) -> bool {
    app.animations.needs_frames(app.scheduler.borrow().now())
}

/// §2.1.2 / Web Animations 1: a timeline with no scroll container is
/// inactive — the animation is idle, its element shows its own value.
#[test]
fn without_a_scroll_container_the_animation_is_idle() {
    let (app, _, bar) = scroller(
        "@keyframes fill { from { width: 0 } to { width: 18 } } \
         #bar { width: 7; animation: fill linear both; animation-timeline: scroll() }",
    );
    assert_eq!(width(&app, bar), 7);
}

/// §2.2: a named scroll timeline (`scroll-timeline`) is visible to the
/// scroller's descendants; `timeline-scope` on an ancestor makes it
/// visible to an element outside the scroller (§4.2).
#[test]
fn a_named_timeline_reaches_through_timeline_scope() {
    let (mut app, s, bar) = scroller(&format!(
        "{FILL} #s {{ scroll-timeline: --list }} \
         #bar {{ animation: fill linear; animation-timeline: --list }}"
    ));
    scroll_top(&mut app, s, 3);
    assert_eq!(width(&app, bar), 6, "a descendant sees it");

    // An element outside the scroller: unseen without a scope.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let outer = dom.create_element("div");
    let s = dom.create_element("div");
    dom.set_attribute(s, "id", "s").unwrap();
    for _ in 0..13 {
        let p = dom.create_element("p");
        let t = dom.create_text_node("x");
        dom.append_child(p, t).unwrap();
        dom.append_child(s, p).unwrap();
    }
    let bar = dom.create_element("div");
    dom.set_attribute(bar, "id", "bar").unwrap();
    dom.append_child(outer, s).unwrap();
    dom.append_child(outer, bar).unwrap();
    dom.append_child(root, outer).unwrap();
    let sheet = rdom_css::parse(&format!(
        "{FILL} #s {{ scroll-timeline: --list }} div:has(> #s) {{ timeline-scope: --list }} \
         #bar {{ width: 3; animation: fill linear; animation-timeline: --list }}"
    ));
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    let terminal = Terminal::new(TestBackend::new(30, 12)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    app.push_stylesheet(sheet.stylesheet);
    app.advance(0).unwrap();
    app.advance(0).unwrap();
    scroll_top(&mut app, s, 3);
    assert_eq!(width(&app, bar), 6, "the scope hoists it to the sibling");
}

/// CSSOM View §4, Scroll-driven Animations 1 §2.1: the progress is the
/// distance from the scroll origin — under `rtl` the origin is the right
/// edge, `scrollLeft` runs 0 → −range, and the progress 0% → 100%.
#[test]
fn an_rtl_scroller_counts_from_its_origin() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let s = dom.create_element("div");
    dom.set_attribute(s, "id", "s").unwrap();
    let wide = dom.create_element("div");
    dom.set_attribute(wide, "id", "wide").unwrap();
    let bar = dom.create_element("div");
    dom.set_attribute(bar, "id", "bar").unwrap();
    dom.append_child(s, bar).unwrap();
    dom.append_child(s, wide).unwrap();
    dom.append_child(root, s).unwrap();
    let sheet = rdom_css::parse(
        "@keyframes fill { from { height: 0 } to { height: 4 } } \
         #s { direction: rtl; width: 10; height: 6; overflow: auto } #wide { width: 20; height: 1 } \
         #bar { width: 1; animation: fill linear; animation-timeline: scroll(inline) }",
    );
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    let terminal = Terminal::new(TestBackend::new(30, 10)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    app.push_stylesheet(sheet.stylesheet);
    app.advance(0).unwrap();
    app.advance(0).unwrap();
    let height = |app: &App<TestBackend>| app.dom().node(bar).ext().unwrap().layout.height;
    assert_eq!(height(&app), 0, "at the origin");
    let range = *app.dom().node(s).scroll_range().unwrap().x().start();
    assert!(range < 0, "an rtl range runs below 0: {range}");
    app.dom_mut()
        .node_mut(s)
        .set_scroll_left(range / 2)
        .unwrap();
    app.advance(0).unwrap();
    assert_eq!(height(&app), 2, "half-way from the origin");
}

/// §3.1: `view()` follows the element's own crossing of its scroller's
/// scrollport — `cover` 0% as its start edge meets the scrollport's end,
/// 100% as its end edge leaves the start; §4.3 `animation-range` narrows
/// it to a named range.
#[test]
fn a_view_timeline_follows_the_subject_through_the_scrollport() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let s = dom.create_element("div");
    dom.set_attribute(s, "id", "s").unwrap();
    let mut subject = None;
    for i in 0..20 {
        let p = dom.create_element("p");
        if i == 8 {
            dom.set_attribute(p, "id", "subject").unwrap();
            subject = Some(p);
        }
        let t = dom.create_text_node("x");
        dom.append_child(p, t).unwrap();
        dom.append_child(s, p).unwrap();
    }
    let subject = subject.unwrap();
    dom.append_child(root, s).unwrap();
    // The subject: row 8 of 20, one row tall, in a scrollport 4 rows
    // tall — `cover` runs from offset 4 (its top at the port's bottom
    // edge) to offset 9 (its bottom at the port's top edge).
    let sheet = rdom_css::parse(
        "@keyframes w { from { width: 0 } to { width: 10 } } \
         #s { height: 4; overflow: auto } p { margin: 0; height: 1 } \
         #subject { animation: w linear both; animation-timeline: view() }",
    );
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    let terminal = Terminal::new(TestBackend::new(30, 10)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    app.push_stylesheet(sheet.stylesheet);
    app.advance(0).unwrap();
    app.advance(0).unwrap();
    let w = |app: &App<TestBackend>| app.dom().node(subject).ext().unwrap().layout.width;
    assert_eq!(w(&app), 0, "before it enters: the backwards fill");
    scroll_top(&mut app, s, 6);
    assert_eq!(w(&app), 4, "two fifths of the way through cover");
    scroll_top(&mut app, s, 12);
    assert_eq!(w(&app), 10, "past it: the forwards fill");
}

/// §4.3: `animation-range: contain` attaches the animation to the part of
/// the crossing where the subject is wholly visible (offsets 5 to 8 for
/// a one-row subject at row 8 in a four-row port).
#[test]
fn animation_range_attaches_to_a_named_range() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let s = dom.create_element("div");
    dom.set_attribute(s, "id", "s").unwrap();
    let mut subject = None;
    for i in 0..20 {
        let p = dom.create_element("p");
        if i == 8 {
            dom.set_attribute(p, "id", "subject").unwrap();
            subject = Some(p);
        }
        let t = dom.create_text_node("x");
        dom.append_child(p, t).unwrap();
        dom.append_child(s, p).unwrap();
    }
    let subject = subject.unwrap();
    dom.append_child(root, s).unwrap();
    let sheet = rdom_css::parse(
        "@keyframes w { from { width: 0 } to { width: 6 } } \
         #s { height: 4; overflow: auto } p { margin: 0; height: 1 } \
         #subject { animation: w linear both; animation-timeline: view(); animation-range: contain }",
    );
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    let terminal = Terminal::new(TestBackend::new(30, 10)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    app.push_stylesheet(sheet.stylesheet);
    app.advance(0).unwrap();
    app.advance(0).unwrap();
    let w = |app: &App<TestBackend>| app.dom().node(subject).ext().unwrap().layout.width;
    scroll_top(&mut app, s, 6);
    assert_eq!(w(&app), 2, "a third into contain");
    scroll_top(&mut app, s, 8);
    assert_eq!(w(&app), 6, "contain's end");
}

/// CSS Animations 2 §4.2 on a progress timeline: scrolling into the range
/// fires `animationstart`, out of it `animationend`.
#[test]
fn scrolling_fires_the_animation_events() {
    use rdom_core::ListenerOptions;
    use std::cell::RefCell;
    use std::rc::Rc;
    let (mut app, s, bar) = scroller(&format!(
        "{FILL} #bar {{ animation: fill linear; animation-timeline: scroll(); \
         animation-range: 30% 60% }}"
    ));
    let log: Rc<RefCell<Vec<String>>> = Rc::default();
    for name in ["animationstart", "animationend"] {
        let log = log.clone();
        app.dom_mut()
            .add_event_listener(bar, name, ListenerOptions::default(), move |ctx| {
                log.borrow_mut().push(ctx.event.event_type.clone());
            })
            .unwrap();
    }
    scroll_top(&mut app, s, 4);
    scroll_top(&mut app, s, 8);
    assert_eq!(*log.borrow(), ["animationstart", "animationend"]);
}

/// Scroll-driven Animations 1 §5: a timeline the frame's layout made
/// active (the scroller had no range before its first layout) is stale;
/// the frame steps it again, so the first painted frame already shows the
/// animation, not the element's own width.
#[test]
fn the_first_frame_shows_a_timeline_its_layout_made() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let s = dom.create_element("div");
    dom.set_attribute(s, "id", "s").unwrap();
    let bar = dom.create_element("div");
    dom.set_attribute(bar, "id", "bar").unwrap();
    dom.append_child(s, bar).unwrap();
    for _ in 0..12 {
        let p = dom.create_element("p");
        let t = dom.create_text_node("x");
        dom.append_child(p, t).unwrap();
        dom.append_child(s, p).unwrap();
    }
    dom.append_child(root, s).unwrap();
    let sheet = rdom_css::parse(&format!(
        "{FILL} #bar {{ width: 5; animation: fill linear; animation-timeline: scroll() }}"
    ));
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    let terminal = Terminal::new(TestBackend::new(30, 10)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    app.push_stylesheet(sheet.stylesheet);
    app.advance(0).unwrap();
    assert_eq!(width(&app, bar), 0, "the timeline's 0%, in the first frame");
}
