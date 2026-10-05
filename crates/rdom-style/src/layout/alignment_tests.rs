//! The typed alignment API checks each property's grammar (CSS Box
//! Alignment 3 §5–§6) as the parser does.

use super::*;
use crate::parse::token::tokenize;
use crate::parse::values::{
    parse_align_content, parse_align_items, parse_align_self, parse_justify_content,
    parse_justify_items, parse_justify_self, serialize_alignment,
};

const KEYWORDS: [Align; 17] = [
    Align::Normal,
    Align::Auto,
    Align::Stretch,
    Align::Start,
    Align::End,
    Align::Center,
    Align::FlexStart,
    Align::FlexEnd,
    Align::SelfStart,
    Align::SelfEnd,
    Align::Left,
    Align::Right,
    Align::SpaceBetween,
    Align::SpaceAround,
    Align::SpaceEvenly,
    Align::Baseline,
    Align::LastBaseline,
];

fn candidates() -> Vec<Alignment> {
    let mut out = Vec::new();
    for k in KEYWORDS {
        out.push(Alignment::new(k));
        out.push(Alignment::safe(k));
        out.push(Alignment::unsafe_(k));
        out.push(Alignment {
            legacy: true,
            ..Alignment::new(k)
        });
    }
    out
}

/// Every keyword, with each overflow position and with `legacy`, is
/// valid for a property exactly when its CSS text parses for it.
#[test]
fn the_typed_check_agrees_with_each_propertys_grammar() {
    type Parser = fn(&[crate::parse::token::Token]) -> Option<Alignment>;
    let properties: [(AlignProperty, Parser); 6] = [
        (AlignProperty::JustifyContent, parse_justify_content),
        (AlignProperty::AlignContent, parse_align_content),
        (AlignProperty::JustifyItems, parse_justify_items),
        (AlignProperty::AlignItems, parse_align_items),
        (AlignProperty::JustifySelf, parse_justify_self),
        (AlignProperty::AlignSelf, parse_align_self),
    ];
    for (property, parse) in properties {
        for a in candidates() {
            let css = serialize_alignment(a);
            let tokens = tokenize(&css).unwrap();
            let parses = parse(&tokens) == Some(a);
            assert_eq!(
                a.is_valid_for(property),
                parses,
                "{property:?}: {css} ({a:?})"
            );
        }
    }
}

/// CSS Box Alignment 3 §6.1: `align-self` takes no
/// `<content-distribution>`; §5.2: `justify-content` does.
#[test]
fn space_between_is_a_content_keyword() {
    let a = Alignment::new(Align::SpaceBetween);
    assert!(a.is_valid_for(AlignProperty::JustifyContent));
    assert!(!a.is_valid_for(AlignProperty::AlignSelf));
}
