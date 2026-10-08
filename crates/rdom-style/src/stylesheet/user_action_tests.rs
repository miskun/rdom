//! C10-PSEUDO-CHAINS: user-action pseudo-classes after a pseudo-element
//! (Selectors 4 §3.6.3) — parsed into the rule's
//! [`UserActionState`], counted in its specificity.

use super::selector_text::extract_pseudo_chain;
use super::*;
use crate::TuiStyle;

/// `extract_pseudo_chain` with an owned core.
fn chain(selector: &str) -> Result<(String, PseudoElementTarget, UserActionState), String> {
    extract_pseudo_chain(selector).map(|(core, pseudo, state)| (core.into_owned(), pseudo, state))
}

/// Selectors 4 §3.6.3: "Some pseudo-elements may be immediately followed
/// by a combination of user action pseudo-classes" — `::before:hover`
/// is the `::before` while it is hovered. The names are ASCII
/// case-insensitive (§4.1); the legacy single-colon spelling of the
/// pseudo-element (§15) takes them too.
#[test]
fn a_pseudo_element_takes_trailing_user_action_pseudo_classes() {
    let hover = UserActionState::HOVER;
    assert_eq!(
        chain("a::before:hover").unwrap(),
        ("a".into(), PseudoElementTarget::Before, hover)
    );
    assert_eq!(
        chain("li::marker:HOVER:active").unwrap(),
        (
            "li".into(),
            PseudoElementTarget::Marker,
            hover.with(UserActionState::ACTIVE)
        )
    );
    assert_eq!(
        chain("p::first-letter:hover").unwrap(),
        ("p".into(), PseudoElementTarget::FirstLetter, hover)
    );
    assert_eq!(
        chain("::after:focus-visible").unwrap(),
        (
            "*".into(),
            PseudoElementTarget::After,
            UserActionState::FOCUS_VISIBLE
        )
    );
    assert_eq!(
        chain("p:before:hover").unwrap(),
        ("p".into(), PseudoElementTarget::Before, hover)
    );
}

/// A pseudo-class before the pseudo-element belongs to the element
/// (`a:hover::before`, `a:hover`): it stays in the core selector for
/// rdom-core to match, and the rule carries no state.
#[test]
fn a_pseudo_class_before_the_pseudo_element_stays_on_the_element() {
    assert_eq!(
        chain("a:hover::before").unwrap(),
        (
            "a:hover".into(),
            PseudoElementTarget::Before,
            UserActionState::EMPTY
        )
    );
    assert_eq!(
        chain("a:hover").unwrap(),
        (
            "a:hover".into(),
            PseudoElementTarget::None,
            UserActionState::EMPTY
        )
    );
    assert_eq!(
        chain(".a\\:hover").unwrap().2,
        UserActionState::EMPTY,
        "an escaped colon is part of the class name"
    );
    // `::scrollbar-thumb:vertical`'s axis is no user action.
    assert_eq!(
        chain("::scrollbar-thumb:vertical").unwrap().1,
        PseudoElementTarget::ScrollbarThumbVertical
    );
}

/// Only user-action pseudo-classes may follow a pseudo-element
/// (§3.6.3), and only after the pseudo-elements rdom tracks pointer
/// state for: anything else makes the selector invalid.
#[test]
fn other_trailing_pseudo_classes_are_invalid() {
    assert!(chain("a::before:first-child").is_err());
    assert!(
        chain("a::before :hover").is_err(),
        "a descendant of ::before"
    );
    assert!(chain("a::before:hovr").is_err());
    assert!(chain("a::before:not(:hover)").is_err());
    assert!(chain("p::selection:hover").is_err());
    assert!(chain("p::first-line:hover").is_err());
}

/// Selectors 4 §17: each trailing pseudo-class counts in the "B" column,
/// as any pseudo-class does — `a::before:hover` is (0, 1, 2).
#[test]
fn trailing_pseudo_classes_count_as_pseudo_classes() {
    let s = Stylesheet::bare()
        .rule("a::before", TuiStyle::new())
        .unwrap()
        .rule("a::before:hover:active", TuiStyle::new())
        .unwrap();
    let (plain, chained) = (&s.rules()[0], &s.rules()[1]);
    assert_eq!(plain.pseudo_state, UserActionState::EMPTY);
    assert_eq!(
        chained.pseudo_state,
        UserActionState::HOVER.with(UserActionState::ACTIVE)
    );
    assert_eq!(chained.pseudo, PseudoElementTarget::Before);
    assert_eq!(
        chained.specificity.class_attr_pseudo,
        plain.specificity.class_attr_pseudo + 2
    );
    assert_eq!(
        chained.specificity.type_pseudo_el,
        plain.specificity.type_pseudo_el
    );
}

/// A pseudo-element is never focused (Selectors 4 §9.5–§9.7 match
/// elements): the focus pseudo-classes never hold of one, the others
/// hold as the pseudo-element's own pointer state says.
#[test]
fn the_state_matches_the_pointer_state_and_never_focus() {
    let hover = UserActionState::HOVER;
    assert!(UserActionState::EMPTY.matches(false, false));
    assert!(hover.matches(true, false));
    assert!(!hover.matches(false, true));
    assert!(hover.with(UserActionState::ACTIVE).matches(true, true));
    assert!(!hover.with(UserActionState::ACTIVE).matches(true, false));
    assert!(!UserActionState::FOCUS.matches(true, true));
    assert!(!UserActionState::FOCUS_WITHIN.matches(true, true));
}

/// Selectors 4 §3.6.3 lets user-action pseudo-classes follow a nested
/// marker as they follow `::marker` (C10G-PSEUDO-MARKER).
#[test]
fn a_nested_marker_takes_trailing_user_action_pseudo_classes() {
    assert_eq!(
        chain("li::after::marker:hover").unwrap(),
        (
            "li".into(),
            PseudoElementTarget::AfterMarker,
            UserActionState::HOVER
        )
    );
}
