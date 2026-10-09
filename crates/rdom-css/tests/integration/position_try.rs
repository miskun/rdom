//! C15-ANCHOR — CSS Anchor Positioning 1 §4.1 `@position-try`: a named
//! position option whose declarations are the inset, margin, sizing and
//! self-alignment properties, `position-anchor` and `position-area`. The
//! sheet keeps every rule in source order with its layer; which rule a
//! name resolves to is the backend's.

use rdom_css::{PositionTryDescriptorReason, WarningKind, parse};

/// §4.1: the prelude is a `<dashed-ident>`; the body's accepted
/// descriptors are kept.
#[test]
fn position_try_rules_keep_their_descriptors() {
    let r = parse(
        "@position-try --above { bottom: anchor(top); top: auto; margin-bottom: 1; \
         position-area: top; align-self: end; width: anchor-size(width) } \
         .a { position-try-fallbacks: --above }",
    );
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let rules = r.stylesheet.position_try_rules();
    assert_eq!(rules.len(), 1);
    assert_eq!(&*rules[0].name, "--above");
    let d = rules[0].declarations();
    assert!(d.bottom.is_some() && d.top.is_some() && d.width.is_some());
    assert!(d.anchor.position_area.is_some());
    assert_eq!(
        r.stylesheet.rules().len(),
        1,
        "the style rule after it is kept"
    );
}

/// §4.1: any other property, and `!important`, is dropped with a warning
/// naming the rule, the declaration and why (the shape of the
/// `@counter-style` warnings); a prelude that is not one `<dashed-ident>`
/// drops the rule.
#[test]
fn position_try_refuses_other_descriptors_and_bad_preludes() {
    let r = parse("@position-try --x { color: red; top: 1 !important; left: 2 }");
    let dropped: Vec<_> = r.warnings.iter().map(|w| w.kind.clone()).collect();
    assert_eq!(
        dropped,
        [
            WarningKind::PositionTryDescriptorDropped {
                name: "--x".into(),
                descriptor: "color".into(),
                reason: PositionTryDescriptorReason::NotADescriptor,
            },
            WarningKind::PositionTryDescriptorDropped {
                name: "--x".into(),
                descriptor: "top".into(),
                reason: PositionTryDescriptorReason::Important,
            },
        ]
    );
    let d = r.stylesheet.position_try_rules()[0].declarations();
    assert!(d.top.is_none() && d.left.is_some());
    for bad in ["x", "--a --b", ""] {
        let r = parse(&format!("@position-try {bad} {{ top: 1 }}"));
        assert!(r.stylesheet.position_try_rules().is_empty(), "{bad}");
        assert!(
            matches!(
                r.warnings.first().map(|w| &w.kind),
                Some(WarningKind::InvalidAtRulePrelude { .. })
            ),
            "{bad}: {:?}",
            r.warnings
        );
    }
}
