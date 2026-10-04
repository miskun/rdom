//! `@property` in the parser (CSS Properties and Values API 1 §3): the
//! descriptors, and the rules that register nothing.

use rdom_css::{WarningKind, parse};

/// §3: `syntax`, `inherits` and `initial-value` register the property.
#[test]
fn property_rule_registers() {
    let r = parse(
        "@property --accent { syntax: '<color>'; inherits: false; initial-value: red } \
         @property --any { syntax: '*'; inherits: true }",
    );
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let regs = r.stylesheet.registered_properties();
    assert_eq!(regs.len(), 2);
    assert_eq!(regs[0].name, "accent");
    assert!(!regs[0].inherits);
    assert_eq!(regs[0].initial_value.as_deref(), Some("red"));
    assert!(regs[1].inherits && regs[1].initial_value.is_none());
}

/// §3: a missing required descriptor, an invalid name or syntax, or an
/// initial value that does not match registers nothing.
#[test]
fn invalid_property_rules_register_nothing() {
    for css in [
        "@property --a { inherits: false; initial-value: red }",
        "@property --a { syntax: '<color>'; initial-value: red }",
        "@property --a { syntax: '<color>'; inherits: false }",
        "@property --a { syntax: '<color>'; inherits: false; initial-value: 12 }",
        "@property --a { syntax: '<angle>'; inherits: false; initial-value: 0deg }",
        "@property a { syntax: '*'; inherits: false }",
    ] {
        let r = parse(css);
        assert!(r.stylesheet.registered_properties().is_empty(), "{css}");
        assert!(
            matches!(&r.warnings[..], [w] if matches!(w.kind, WarningKind::InvalidPropertyRule { .. })),
            "{css}: {:?}",
            r.warnings
        );
    }
}
