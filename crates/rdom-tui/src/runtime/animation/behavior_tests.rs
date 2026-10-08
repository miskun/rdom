//! C12-BEHAVIOR: `transition-behavior: allow-discrete` (CSS Transitions 2
//! §3.1) — discrete properties transition, `display` keeping its shown
//! value for the whole of a transition to or from `none` (CSS Display 4
//! §2.9, Web Animations 2's display rule).

use std::time::{Duration, Instant};

use super::*;
use crate::layout::Display;
use crate::style::Stylesheet;
use crate::{CascadeExt, TuiDom};

/// `div` styled `base` then `changed` (CSS text), diffed at `start`.
fn changed(base: &str, changed: &str) -> (TuiDom, AnimationRegistry, NodeId, Instant) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let sheet = |css: &str| -> Stylesheet {
        let parsed = rdom_css::parse(&format!("div {{ {css} }}"));
        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
        parsed.stylesheet
    };
    dom.cascade(&sheet(base));
    let mut reg = AnimationRegistry::new();
    let start = Instant::now();
    diff_and_register(&mut dom, &mut reg, start);
    dom.cascade(&sheet(changed));
    diff_and_register(&mut dom, &mut reg, start);
    (dom, reg, div, start)
}

fn display(dom: &TuiDom, div: NodeId) -> Display {
    dom.node(div)
        .ext()
        .unwrap()
        .computed
        .as_ref()
        .unwrap()
        .display
}

/// CSS Transitions 2 §3.1: under `allow-discrete` a `display: block →
/// none` transition runs — `block` until it ends — and fires its events.
#[test]
fn allow_discrete_keeps_display_shown_on_the_way_out() {
    let t = "transition: display 100ms allow-discrete";
    let (mut dom, mut reg, div, start) = changed(
        &format!("display: block; {t}"),
        &format!("display: none; {t}"),
    );
    assert_eq!(reg.len(), 1);
    reg.advance(&mut dom, start + Duration::from_millis(90));
    assert_eq!(display(&dom, div), Display::Block, "shown while it runs");
    reg.advance(&mut dom, start + Duration::from_millis(100));
    assert_eq!(display(&dom, div), Display::None);
    assert!(reg.is_empty());
}

/// And `none → block` shows from the start (the non-`none` value for
/// every progress after 0).
#[test]
fn allow_discrete_shows_display_at_once_on_the_way_in() {
    let t = "transition: display 100ms allow-discrete";
    let (mut dom, mut reg, div, start) = changed(
        &format!("display: none; {t}"),
        &format!("display: block; {t}"),
    );
    reg.advance(&mut dom, start + Duration::from_millis(1));
    assert_eq!(display(&dom, div), Display::Block);
}

/// `normal` (the initial value): a discrete property never transitions.
#[test]
fn normal_behavior_changes_discrete_properties_at_once() {
    let t = "transition: display 100ms";
    let (dom, reg, div, _) = changed(
        &format!("display: block; {t}"),
        &format!("display: none; {t}"),
    );
    assert!(reg.is_empty());
    assert_eq!(display(&dom, div), Display::None);
}

/// Web Animations 1 §5.3.1: any other discrete property steps at 50 %.
#[test]
fn allow_discrete_steps_other_discrete_properties_at_the_midpoint() {
    use crate::layout::Align;
    let t = "transition: all 100ms linear allow-discrete";
    let (mut dom, mut reg, div, start) = changed(
        &format!("justify-content: start; {t}"),
        &format!("justify-content: end; {t}"),
    );
    let justify = |dom: &TuiDom| {
        dom.node(div)
            .ext()
            .unwrap()
            .computed
            .as_ref()
            .unwrap()
            .justify_content
            .keyword
    };
    reg.advance(&mut dom, start + Duration::from_millis(40));
    assert_eq!(justify(&dom), Align::Start);
    reg.advance(&mut dom, start + Duration::from_millis(60));
    assert_eq!(justify(&dom), Align::End);
}
