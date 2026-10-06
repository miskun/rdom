//! `@counter-style` in the parser (CSS Counter Styles 3 §3): every
//! descriptor, the rules that define nothing, and the order a name's
//! definitions apply in.

use rdom_css::{WarningKind, parse};
use rdom_style::counters::{CounterRange, SpeakAs, System};

/// §3: each descriptor parses into the rule — `system`, `symbols`,
/// `additive-symbols`, `negative`, `prefix`, `suffix`, `range`, `pad`,
/// `fallback`, `speak-as`.
#[test]
fn every_descriptor_parses() {
    let r = parse(
        r#"@counter-style thumbs {
             system: fixed 3;
             symbols: "a" b 'c';
             negative: "(" ")";
             prefix: "<";
             suffix: ">";
             range: 1 5, 10 infinite;
             pad: 2 "0";
             fallback: lower-roman;
             speak-as: bullets;
           }
           @counter-style weights {
             system: additive;
             additive-symbols: 10 X, V 5, 1 I;
           }
           @counter-style based { system: extends decimal; suffix: ") " }"#,
    );
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let defs = r.stylesheet.counter_styles();
    assert_eq!(defs.len(), 3);
    let (name, rule) = (&defs[0].name, &defs[0].rule);
    assert_eq!(&**name, "thumbs");
    assert_eq!(rule.system, Some(System::Fixed(3)));
    assert_eq!(
        rule.symbols.as_deref(),
        Some(&["a".to_string(), "b".to_string(), "c".to_string()][..])
    );
    assert_eq!(rule.negative, Some(("(".to_string(), ")".to_string())));
    assert_eq!(rule.prefix.as_deref(), Some("<"));
    assert_eq!(rule.suffix.as_deref(), Some(">"));
    assert_eq!(
        rule.range,
        Some(CounterRange::Ranges(vec![(1, 5), (10, i64::MAX)].into()))
    );
    assert_eq!(rule.pad, Some((2, "0".to_string())));
    assert_eq!(rule.fallback.as_deref(), Some("lower-roman"));
    assert_eq!(rule.speak_as, Some(SpeakAs::Bullets));
    let weights = &defs[1].rule;
    assert_eq!(
        weights.additive_symbols.as_deref(),
        Some(
            &[
                (10, "X".to_string()),
                (5, "V".to_string()),
                (1, "I".to_string())
            ][..]
        )
    );
    assert_eq!(defs[2].rule.system, Some(System::Extends("decimal".into())));
}

/// §3: a rule defines nothing — and warns — when its name is `none`, a
/// CSS-wide keyword or one of the styles that cannot be overridden
/// (`decimal`, `disc`, `square`, `circle`, `disclosure-open`,
/// `disclosure-closed`); when its symbols do not suit its system
/// (§3.1: alphabetic and numeric need two); when `extends` comes with
/// `symbols`; when `additive-symbols` weights do not descend.
#[test]
fn invalid_rules_define_nothing() {
    for css in [
        "@counter-style none { system: cyclic; symbols: a }",
        "@counter-style DECIMAL { system: cyclic; symbols: a }",
        "@counter-style disc { system: cyclic; symbols: a }",
        "@counter-style inherit { system: cyclic; symbols: a }",
        "@counter-style x { system: alphabetic; symbols: a }",
        "@counter-style x { system: cyclic }",
        "@counter-style x { system: extends decimal; symbols: a }",
        "@counter-style x { system: additive; additive-symbols: 1 a, 5 b }",
        "@counter-style x",
    ] {
        let r = parse(css);
        assert!(r.stylesheet.counter_styles().is_empty(), "{css}");
        assert!(
            matches!(&r.warnings[..], [w] if matches!(w.kind, WarningKind::InvalidCounterStyleRule { .. })),
            "{css}: {:?}",
            r.warnings
        );
    }
}

/// §3: an invalid descriptor is dropped, the rule kept — and the drop
/// is reported. A negative `pad` or reversed range bounds are invalid.
#[test]
fn an_invalid_descriptor_is_dropped() {
    let r = parse("@counter-style x { system: cyclic; symbols: a; pad: -1 z; range: 5 1 }");
    let defs = r.stylesheet.counter_styles();
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].rule.pad, None);
    assert_eq!(defs[0].rule.range, None);
    assert_eq!(r.warnings.len(), 2, "{:?}", r.warnings);
}

/// CSS Cascade 5 §6.4.3 for name-defining at-rules: a later definition
/// of a name wins, and an unlayered one beats a layered one.
#[test]
fn definitions_keep_their_order_and_layer() {
    let r = parse(
        "@layer base { @counter-style x { system: cyclic; symbols: L } }
         @counter-style x { system: cyclic; symbols: A }
         @counter-style x { system: cyclic; symbols: B }",
    );
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let defs = r.stylesheet.counter_styles();
    assert_eq!(defs.len(), 3);
    assert!(defs[0].layer.is_some());
    assert!(defs[1].layer.is_none() && defs[2].layer.is_none());
}
