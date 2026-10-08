//! C12-KEYFRAMES — CSS animations run on the app's clock, in the same
//! effect stack as transitions: each frame their keyframe values are
//! composited onto the computed style layout and paint read (CSS
//! Animations 1 §3–§4, Web Animations 1 §4–§5).
//!
//! `#a` is a `<div>` holding a `<span>`; widths are read from the laid-out
//! box, so the tests see what layout saw.

use rdom_core::NodeId;

use crate::TuiDom;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::style::Stylesheet;

/// An app over `<div id=a><span>x</span></div>` styled by `css`, one frame
/// drawn at clock 0.
pub(super) fn animated(css: &str) -> (App<TestBackend>, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.set_attribute(div, "id", "a").unwrap();
    let span = dom.create_element("span");
    let t = dom.create_text_node("x");
    dom.append_child(span, t).unwrap();
    dom.append_child(div, span).unwrap();
    dom.append_child(root, div).unwrap();
    let sheet = rdom_css::parse(css);
    assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    let terminal = Terminal::new(TestBackend::new(40, 12)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    app.push_stylesheet(sheet.stylesheet);
    app.advance(0).unwrap();
    (app, div)
}

/// Whether the app's animations ask for the next frame.
fn needs_frames(app: &App<TestBackend>) -> bool {
    app.animations.needs_frames(app.scheduler.borrow().now())
}

pub(super) fn width(app: &App<TestBackend>, id: NodeId) -> u16 {
    app.dom().node(id).ext().unwrap().layout.width
}

const GROW: &str = "@keyframes grow { from { width: 2 } to { width: 10 } } ";

/// §3–§4: `width: 2 → 10` over 100ms linear is 6 cells half-way; with
/// no fill the animation stops applying once it ends, and the box takes
/// its own width back.
#[test]
fn a_keyframe_animation_moves_the_box_on_the_clock() {
    let (mut app, div) = animated(&format!(
        "{GROW} #a {{ width: 4; animation: grow 100ms linear }}"
    ));
    assert_eq!(width(&app, div), 2, "the first keyframe at once");
    app.advance(50).unwrap();
    assert_eq!(width(&app, div), 6, "half-way");
    app.advance(25).unwrap();
    assert_eq!(width(&app, div), 8);
    app.advance(25).unwrap();
    assert_eq!(width(&app, div), 4, "ended: its own width");
}

/// §3: a property missing from the `from` / `to` keyframe takes the
/// element's own value there (the implicit keyframes).
#[test]
fn missing_end_keyframes_take_the_underlying_value() {
    let (mut app, div) = animated(
        "@keyframes to10 { to { width: 10 } } #a { width: 2; animation: to10 100ms linear }",
    );
    app.advance(50).unwrap();
    assert_eq!(width(&app, div), 6);
}

/// §4.8: `backwards` applies the first keyframe during the delay,
/// `forwards` keeps the last after the end; without them the element's
/// own value shows there.
#[test]
fn fill_modes_hold_the_end_keyframes() {
    let (mut app, div) = animated(&format!(
        "{GROW} #a {{ width: 4; animation: grow 100ms linear 50ms }}"
    ));
    assert_eq!(width(&app, div), 4, "the delay, no fill");
    let (mut app2, div2) = animated(&format!(
        "{GROW} #a {{ width: 4; animation: grow 100ms linear 50ms both }}"
    ));
    assert_eq!(width(&app2, div2), 2, "the delay, backwards");
    app.advance(100).unwrap();
    app2.advance(100).unwrap();
    assert_eq!((width(&app, div), width(&app2, div2)), (6, 6));
    app.advance(100).unwrap();
    app2.advance(100).unwrap();
    assert_eq!(width(&app, div), 4, "after, no fill");
    assert_eq!(width(&app2, div2), 10, "after, forwards");
}

/// §4.4–§4.5, Web Animations 1 §4.10–§4.11: `alternate` runs every odd
/// iteration backwards; a fractional count ends part-way, which
/// `forwards` holds.
#[test]
fn iterations_alternate_and_end_part_way() {
    let (mut app, div) = animated(&format!(
        "{GROW} #a {{ width: 4; animation: grow 100ms linear 1.5 alternate forwards }}"
    ));
    app.advance(125).unwrap();
    assert_eq!(width(&app, div), 8, "the second iteration, backwards");
    app.advance(200).unwrap();
    assert_eq!(width(&app, div), 6, "held half-way back");
}

/// §4.4: `infinite` never ends.
#[test]
fn an_infinite_animation_keeps_running() {
    let (mut app, div) = animated(&format!(
        "{GROW} #a {{ width: 4; animation: grow 100ms linear infinite }}"
    ));
    app.advance(1050).unwrap();
    assert_eq!(width(&app, div), 6);
    assert!(needs_frames(&app));
}

/// §3: a keyframe's `animation-timing-function` eases the interval it
/// starts; the animation's own easing the others.
#[test]
fn a_keyframe_easing_applies_to_its_interval() {
    let (mut app, div) = animated(
        "@keyframes g { from { width: 2; animation-timing-function: steps(2, jump-end) } \
         to { width: 10 } } #a { animation: g 100ms linear }",
    );
    app.advance(25).unwrap();
    assert_eq!(width(&app, div), 2, "the first step");
    app.advance(35).unwrap();
    assert_eq!(width(&app, div), 6, "the second step");
}

/// §3: a percentage keyframe splits the run; a property only some
/// keyframes name runs between those (CSS Animations 1 §3, Web
/// Animations 1 §5.3.3: property-specific keyframes).
#[test]
fn intermediate_keyframes_split_the_run() {
    let (mut app, div) = animated(
        "@keyframes g { 0% { width: 2 } 20% { width: 12 } 100% { width: 2 } } \
         #a { animation: g 100ms linear }",
    );
    app.advance(10).unwrap();
    assert_eq!(width(&app, div), 7);
    app.advance(50).unwrap();
    assert_eq!(width(&app, div), 7, "60%: half-way down from 12");
}

/// CSS Cascade 5 §6.1: the animation origin is under every `!important`
/// declaration — an important width does not animate.
#[test]
fn an_important_declaration_beats_the_animation() {
    let (mut app, div) = animated(&format!(
        "{GROW} #a {{ width: 4 !important; animation: grow 100ms linear }}"
    ));
    app.advance(50).unwrap();
    assert_eq!(width(&app, div), 4);
}

/// §3 and CSS Cascade 5 §6.4.3: the last `@keyframes` of a name wins, by
/// layer order — the unlayered rule over the layered one before it, the
/// later of two in one layer.
#[test]
fn the_last_keyframes_rule_of_a_name_wins_by_layer() {
    let (mut app, div) = animated(
        "@keyframes g { to { width: 10 } } @layer base { @keyframes g { to { width: 20 } } } \
         #a { width: 2; animation: g 100ms linear }",
    );
    app.advance(50).unwrap();
    assert_eq!(width(&app, div), 6, "the unlayered rule, 2 → 10");
}

/// Web Animations 1 §5.4.5, CSS Animations 2 §3: a CSS animation sorts
/// above a CSS transition of the same property — its value shows.
#[test]
fn an_animation_sorts_above_a_transition() {
    let (mut app, div) = animated(&format!(
        "{GROW} #a {{ width: 4; transition: width 100ms linear }} \
         #a.on {{ width: 20; animation: grow 100ms linear }}"
    ));
    app.dom_mut().set_attribute(div, "class", "on").unwrap();
    app.advance(0).unwrap();
    app.advance(50).unwrap();
    assert_eq!(
        width(&app, div),
        6,
        "the animation's, not the transition's 12"
    );
}

/// §4.1: changing `animation-name` restarts — the old animation is
/// cancelled, the new one starts at the change.
#[test]
fn a_new_animation_name_restarts() {
    let (mut app, div) = animated(
        "@keyframes g { from { width: 2 } to { width: 10 } } \
         @keyframes h { from { width: 2 } to { width: 10 } } \
         #a { animation: g 100ms linear } #a.on { animation-name: h }",
    );
    app.advance(50).unwrap();
    assert_eq!(width(&app, div), 6);
    app.dom_mut().set_attribute(div, "class", "on").unwrap();
    app.advance(0).unwrap();
    assert_eq!(width(&app, div), 2, "h starts from its beginning");
}

/// §4: a change to another longhand updates the running animation in
/// place — its start time kept.
#[test]
fn a_new_duration_updates_the_running_animation() {
    let (mut app, div) = animated(&format!(
        "{GROW} #a {{ animation: grow 100ms linear }} #a.on {{ animation-duration: 200ms }}"
    ));
    app.advance(50).unwrap();
    assert_eq!(width(&app, div), 6);
    app.dom_mut().set_attribute(div, "class", "on").unwrap();
    app.advance(0).unwrap();
    assert_eq!(width(&app, div), 4, "50ms of 200ms");
}

/// §4.6: `paused` holds the animation where it is; `running` resumes it
/// from there.
#[test]
fn paused_holds_and_running_resumes() {
    let (mut app, div) = animated(&format!(
        "{GROW} #a {{ animation: grow 100ms linear }} #a.p {{ animation-play-state: paused }}"
    ));
    app.advance(25).unwrap();
    app.dom_mut().set_attribute(div, "class", "p").unwrap();
    app.advance(0).unwrap();
    app.advance(200).unwrap();
    assert_eq!(width(&app, div), 4, "held at 25ms");
    assert!(!needs_frames(&app), "a paused animation needs no frames");
    app.dom_mut().set_attribute(div, "class", "").unwrap();
    app.advance(0).unwrap();
    app.advance(25).unwrap();
    assert_eq!(width(&app, div), 6, "resumed: 50ms in");
}

/// CSS Animations 1 §3 ("setting display: none terminates any running
/// animation"): hiding the element cancels; showing it again starts the
/// animation over.
#[test]
fn display_none_cancels_and_showing_restarts() {
    let (mut app, div) = animated(&format!(
        "{GROW} #a {{ animation: grow 100ms linear }} #a.off {{ display: none }}"
    ));
    app.advance(50).unwrap();
    app.dom_mut().set_attribute(div, "class", "off").unwrap();
    app.advance(0).unwrap();
    assert!(!needs_frames(&app));
    app.advance(30).unwrap();
    app.dom_mut().set_attribute(div, "class", "").unwrap();
    app.advance(0).unwrap();
    assert_eq!(width(&app, div), 2, "from the start");
    app.advance(50).unwrap();
    assert_eq!(width(&app, div), 6);
}

/// §3: a `::before` animates as the element does, from its own keyframe
/// values.
#[test]
fn a_pseudo_element_animates() {
    let (mut app, div) = animated(&format!(
        "{GROW} #a {{ position: relative }} \
         #a::before {{ content: 'x'; position: absolute; animation: grow 100ms linear }}"
    ));
    let pseudo_width = |app: &App<TestBackend>| {
        let ext = app.dom().node(div).ext().unwrap();
        ext.positioned_pseudos().next().map(|p| p.border_box.width)
    };
    app.advance(50).unwrap();
    assert_eq!(pseudo_width(&app), Some(6));
}

/// `Element.getAnimations()` (Web Animations 1 §6.7), as rdom reads it:
/// the element's running animations, by name, with their play state.
#[test]
fn get_animations_lists_the_running_animations() {
    use crate::runtime::animation::{AnimationKind, AnimationPlayState};
    let (mut app, div) = animated(&format!(
        "{GROW} #a {{ animation: grow 100ms linear, grow 1s linear paused }}"
    ));
    app.advance(10).unwrap();
    let list = app.get_animations(div);
    let names: Vec<_> = list
        .iter()
        .map(|a| match a.kind() {
            AnimationKind::CssAnimation(name) => (name.to_string(), a.play_state()),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(
        names,
        [
            ("grow".to_string(), AnimationPlayState::Running),
            ("grow".to_string(), AnimationPlayState::Paused)
        ]
    );
    assert_eq!(list[0].current_time_ms(), Some(10.0));
    assert_eq!(list[1].current_time_ms(), Some(0.0));
}

/// CSS Animations 2 §3.2, Web Animations 1 §5.4.4: under `add` each
/// keyframe's value adds to the element's own (4 + 0 → 4 + 4); a
/// keyframe's own `animation-composition` overrides the animation's.
#[test]
fn composition_adds_keyframes_to_the_underlying_value() {
    let (mut app, div) = animated(
        "@keyframes g { from { width: 0 } to { width: 4 } } \
         #a { width: 4; animation: g 100ms linear; animation-composition: add }",
    );
    assert_eq!(width(&app, div), 4);
    app.advance(50).unwrap();
    assert_eq!(width(&app, div), 6, "4 + 2");
    let (mut app, div) = animated(
        "@keyframes g { from { width: 0 } to { width: 4; animation-composition: replace } } \
         #a { width: 4; animation: g 100ms linear; animation-composition: accumulate }",
    );
    app.advance(50).unwrap();
    assert_eq!(width(&app, div), 4, "4 + 0 → 4");
}

/// CSS Easing 1 §2.3.1 / Web Animations 1 §5.3.3 (C12G-MISC): a
/// keyframe interval's step easing takes the before flag in the
/// animation's before phase — `steps(2, jump-start)` filled backwards
/// through a delay shows the first keyframe (progress 0 is the step
/// before the jump), not the first step.
#[test]
fn keyframe_steps_take_the_before_flag_in_the_delay() {
    let (mut app, div) = animated(&format!(
        "{GROW} #a {{ width: 4 }} #a.on {{ animation: grow 100ms steps(2, jump-start) 50ms backwards }}"
    ));
    app.dom_mut().set_attribute(div, "class", "on").unwrap();
    app.advance(0).unwrap();
    assert_eq!(width(&app, div), 2, "the before phase: progress 0, no jump");
    app.advance(60).unwrap();
    assert_eq!(width(&app, div), 6, "past the delay: the first jump");
}
