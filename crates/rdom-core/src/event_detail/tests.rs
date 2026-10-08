use super::*;

// --- MouseButton ---

#[test]
fn mouse_button_left_middle_right_are_unit_variants() {
    // Pattern-match each named variant to assert it exists
    // and is a unit variant.
    assert!(matches!(MouseButton::Left, MouseButton::Left));
    assert!(matches!(MouseButton::Middle, MouseButton::Middle));
    assert!(matches!(MouseButton::Right, MouseButton::Right));
}

#[test]
fn mouse_button_other_carries_i16() {
    // 3 is browser-back per the DOM table; we don't bake that
    // mapping into the type — `Other` is just the catch-all.
    let back = MouseButton::Other(3);
    match back {
        MouseButton::Other(n) => assert_eq!(n, 3),
        _ => panic!("Other(3) didn't match Other"),
    }
}

#[test]
fn mouse_button_is_copy_and_eq() {
    let b = MouseButton::Left;
    let c = b; // Copy.
    assert_eq!(b, c);
    assert_ne!(MouseButton::Left, MouseButton::Right);
    assert_ne!(MouseButton::Other(3), MouseButton::Other(4));
}

// --- KeyboardModifiers ---

#[test]
fn keyboard_modifiers_default_is_all_false() {
    let m = KeyboardModifiers::default();
    assert!(!m.ctrl);
    assert!(!m.shift);
    assert!(!m.alt);
    assert!(!m.meta);
}

#[test]
fn keyboard_modifiers_field_struct_round_trips() {
    let m = KeyboardModifiers {
        ctrl: true,
        shift: false,
        alt: true,
        meta: false,
    };
    assert!(m.ctrl);
    assert!(!m.shift);
    assert!(m.alt);
    assert!(!m.meta);
}

#[test]
fn keyboard_modifiers_is_copy_and_eq() {
    let a = KeyboardModifiers {
        ctrl: true,
        ..Default::default()
    };
    let b = a; // Copy.
    assert_eq!(a, b);

    let c = KeyboardModifiers {
        shift: true,
        ..Default::default()
    };
    assert_ne!(a, c);
}

// --- InputType ---

#[test]
fn input_type_named_variants_exist() {
    // Round-trip every named variant through equality. This
    // also serves as a compile-time inventory of the shipped
    // set — adding a new variant requires updating this list.
    let named = [
        InputType::InsertText,
        InputType::InsertReplacementText,
        InputType::InsertLineBreak,
        InputType::InsertParagraph,
        InputType::InsertFromPaste,
        InputType::InsertFromDrop,
        InputType::DeleteContentBackward,
        InputType::DeleteContentForward,
        InputType::DeleteByCut,
        InputType::DeleteWordBackward,
        InputType::DeleteWordForward,
        InputType::HistoryUndo,
        InputType::HistoryRedo,
    ];
    // Each variant is distinct from its neighbors.
    for (i, a) in named.iter().enumerate() {
        for (j, b) in named.iter().enumerate() {
            if i == j {
                assert_eq!(a, b);
            } else {
                assert_ne!(a, b);
            }
        }
    }
}

#[test]
fn input_type_other_carries_string() {
    let it = InputType::Other("formatBold".into());
    match &it {
        InputType::Other(s) => assert_eq!(s, "formatBold"),
        _ => panic!("Other didn't match Other"),
    }
}

#[test]
fn input_type_other_differs_from_named_with_same_label() {
    // `Other("insertText")` is not equal to the named
    // `InsertText` variant — the named set is closed.
    assert_ne!(InputType::Other("insertText".into()), InputType::InsertText);
}

// --- ToggleState ---

#[test]
fn toggle_state_variants_exist_and_differ() {
    assert!(matches!(ToggleState::Open, ToggleState::Open));
    assert!(matches!(ToggleState::Closed, ToggleState::Closed));
    assert_ne!(ToggleState::Open, ToggleState::Closed);
}

#[test]
fn toggle_state_is_copy() {
    let s = ToggleState::Open;
    let t = s; // Copy.
    assert_eq!(s, t);
}

// --- EventDetail ---

#[test]
fn event_detail_default_is_none() {
    let d: EventDetail = Default::default();
    assert!(matches!(d, EventDetail::None));
}

#[test]
fn event_detail_string_round_trip_via_as_string() {
    // The canonical step-2 failing test: an `EventDetail::String`
    // payload round-trips through `as_string()`. This is the
    // migration target for every pre-M4 reader that used
    // `event.detail.as_deref()`.
    let d = EventDetail::String("payload".into());
    assert_eq!(d.as_string(), Some("payload"));
}

#[test]
fn event_detail_as_string_returns_none_for_other_variants() {
    assert_eq!(EventDetail::None.as_string(), None);
    assert_eq!(
        EventDetail::Mouse(MouseDetail {
            button: MouseButton::Left,
            buttons: 0,
            client_x: 0,
            client_y: 0,
            delta_x: 0,
            delta_y: 0,
            modifiers: KeyboardModifiers::default(),
        })
        .as_string(),
        None
    );
}

#[test]
fn event_detail_as_transition_round_trips() {
    let d = EventDetail::Transition(Box::new(TransitionDetail {
        property_name: "color".into(),
        elapsed: 0.25,
        pseudo_element: None,
    }));
    let t = d.as_transition().expect("variant matches");
    assert_eq!(t.property_name, "color");
    assert!((t.elapsed - 0.25).abs() < f64::EPSILON);
    assert!(t.pseudo_element.is_none());
    assert_eq!(d.as_string(), None);
}

#[test]
fn event_detail_as_input_round_trips() {
    let d = EventDetail::Input(Box::new(InputDetail {
        input_type: InputType::InsertText,
        data: Some("a".into()),
        is_composing: false,
    }));
    let i = d.as_input().expect("variant matches");
    assert_eq!(i.input_type, InputType::InsertText);
    assert_eq!(i.data.as_deref(), Some("a"));
    assert!(!i.is_composing);
}

#[test]
fn event_detail_as_submit_round_trips_with_none_submitter() {
    let d = EventDetail::Submit(Box::new(SubmitDetail::new(None)));
    let s = d.as_submit().expect("variant matches");
    assert!(s.submitter.is_none());
}

#[test]
fn event_detail_as_toggle_round_trips() {
    let d = EventDetail::Toggle(Box::new(ToggleDetail {
        old_state: ToggleState::Closed,
        new_state: ToggleState::Open,
        source: None,
    }));
    let t = d.as_toggle().expect("variant matches");
    assert_eq!(t.old_state, ToggleState::Closed);
    assert_eq!(t.new_state, ToggleState::Open);
}

#[test]
fn event_detail_as_mouse_round_trips() {
    let d = EventDetail::Mouse(MouseDetail {
        button: MouseButton::Right,
        buttons: 0b010,
        client_x: 12,
        client_y: 7,
        delta_x: 0,
        delta_y: -1,
        modifiers: KeyboardModifiers {
            ctrl: true,
            ..Default::default()
        },
    });
    let m = d.as_mouse().expect("variant matches");
    assert_eq!(m.button, MouseButton::Right);
    assert_eq!(m.buttons, 0b010);
    assert_eq!(m.client_x, 12);
    assert_eq!(m.client_y, 7);
    assert_eq!(m.delta_y, -1);
    assert!(m.modifiers.ctrl);
    assert!(!m.modifiers.shift);
}

#[test]
fn event_detail_as_keyboard_round_trips() {
    let d = EventDetail::Keyboard(Box::new(KeyboardDetail {
        key: "Enter".into(),
        modifiers: KeyboardModifiers::default(),
        repeat: false,
    }));
    let k = d.as_keyboard().expect("variant matches");
    assert_eq!(k.key, "Enter");
    assert!(!k.repeat);
}

#[test]
fn event_detail_accessor_cross_check() {
    // A non-matching `as_*` accessor returns `None`, not panic
    // or wrong-variant data.
    let s = EventDetail::String("hello".into());
    assert!(s.as_transition().is_none());
    assert!(s.as_input().is_none());
    assert!(s.as_submit().is_none());
    assert!(s.as_toggle().is_none());
    assert!(s.as_mouse().is_none());
    assert!(s.as_keyboard().is_none());
}
