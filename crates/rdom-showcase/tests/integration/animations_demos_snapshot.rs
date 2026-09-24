//! Initial-paint snapshots for the M8 Animations demos:
//! `transition_box`, `interval_counter`, `raf_progress`.
//!
//! Animation behavior (transition tweening, interval ticks, rAF
//! frames) is exercised by the substrate's own animation /
//! timers tests + the demos' behavioral tests in
//! `rdom-showcase/tests/`. These snapshots just pin the static
//! initial paint — when ticks haven't fired yet, the bar is at
//! 0%, the counter is 0, the box hasn't transitioned.
use rdom_showcase::demos::{interval_counter, raf_progress, transition_box};
use rdom_tui::prelude::*;

use crate::common::{assert_snapshot, buffer_to_snapshot, render};

#[test]
fn transition_box_initial_paint() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let demo_root = transition_box::build(&mut dom);
    dom.append_child(root, demo_root).unwrap();

    let sheet = transition_box::stylesheet();
    let buf = render(&mut dom, &sheet, Rect::new(0, 0, 60, 10));
    let snap = buffer_to_snapshot(&buf);
    assert_snapshot(&snap, "transition_box.snap");
}

#[test]
fn interval_counter_initial_paint() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let demo_root = interval_counter::build(&mut dom);
    dom.append_child(root, demo_root).unwrap();

    let sheet = interval_counter::stylesheet();
    // Wider viewport so the button's UA bracket chrome has room.
    let buf = render(&mut dom, &sheet, Rect::new(0, 0, 80, 10));
    let snap = buffer_to_snapshot(&buf);
    assert_snapshot(&snap, "interval_counter.snap");
}

#[test]
fn raf_progress_initial_paint() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let demo_root = raf_progress::build(&mut dom);
    dom.append_child(root, demo_root).unwrap();

    let sheet = raf_progress::stylesheet();
    let buf = render(&mut dom, &sheet, Rect::new(0, 0, 60, 10));
    let snap = buffer_to_snapshot(&buf);
    assert_snapshot(&snap, "raf_progress.snap");
}
