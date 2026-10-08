//! `visibility` transitions (C6-VISIBILITY, CSS Display 3 §4).
use super::*;
use crate::layout::Visibility::{Collapse, Hidden, Visible};
use crate::style::Stylesheet;
use crate::style::transition::TransitionProperty;
use crate::{CascadeExt, TuiDom, TuiStyle};
use rdom_style::animation::lerp_visibility;

/// CSS Display 3 §4: with a `visible` end, every progress strictly
/// between 0 and 1 is `visible`; between two non-visible values a
/// discrete midpoint step.
#[test]
fn visibility_is_visible_inside_a_transition_with_a_visible_end() {
    for (a, b) in [(Hidden, Visible), (Visible, Hidden), (Collapse, Visible)] {
        assert_eq!(lerp_visibility(a, b, 0.0), a);
        assert_eq!(lerp_visibility(a, b, 0.01), Visible);
        assert_eq!(lerp_visibility(a, b, 0.99), Visible);
        assert_eq!(lerp_visibility(a, b, 1.0), b);
    }
    assert_eq!(lerp_visibility(Hidden, Collapse, 0.4), Hidden);
    assert_eq!(lerp_visibility(Hidden, Collapse, 0.6), Collapse);
}

/// CSS Transitions 1 §2.1 with CSS Display 3 §4: `visible → hidden`
/// under `transition: visibility 100ms` stays drawn until the end, and
/// `hidden → visible` is drawn from the start; the override clears when
/// the transition ends.
#[test]
fn a_visibility_transition_presents_visible_while_it_runs() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let sheet = |v| {
        Stylesheet::bare().rule_unchecked(
            "div",
            TuiStyle::new()
                .visibility(v)
                .transition_property(vec![TransitionProperty::named("visibility")])
                .transition_duration(vec![100])
                .transition_timing_function(vec![TimingFunction::Linear]),
        )
    };
    // The running value, while a transition runs.
    let presented = |dom: &TuiDom| {
        let ext = dom.node(div).ext().unwrap();
        ext.presentation
            .is_some()
            .then(|| ext.computed.as_ref().unwrap().visibility)
    };
    dom.cascade(&sheet(Visible));
    let mut reg = AnimationRegistry::new();
    let start = Instant::now();
    diff_and_register(&mut dom, &mut reg, start);
    dom.cascade(&sheet(Hidden));
    diff_and_register(&mut dom, &mut reg, start);
    assert_eq!(reg.len(), 1);
    reg.advance(&mut dom, start + Duration::from_millis(90));
    assert_eq!(presented(&dom), Some(Visible));
    reg.advance(&mut dom, start + Duration::from_millis(100));
    assert_eq!(presented(&dom), None);

    dom.cascade(&sheet(Visible));
    let later = start + Duration::from_millis(200);
    diff_and_register(&mut dom, &mut reg, later);
    reg.advance(&mut dom, later + Duration::from_millis(1));
    assert_eq!(presented(&dom), Some(Visible));
}
