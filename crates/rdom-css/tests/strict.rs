//! §11.9 — Lenient vs strict modes. The default `parse` is
//! lenient (browser-faithful: malformed declarations become
//! warnings, others continue). `parse_strict` returns the first
//! warning as a `ParseError`. `parse_inline` and
//! `parse_inline_strict` mirror the same split for `style="…"`.

use rdom_css::{ParseErrorKind, parse, parse_strict};

#[test]
fn strict_clean_input_returns_ok() {
    let result = parse_strict("a { color: red; }");
    assert!(result.is_ok());
    let sheet = result.unwrap();
    assert_eq!(sheet.rules().len(), 1);
}

#[test]
fn strict_invalid_selector_errors() {
    let result = parse_strict("! {}");
    let err = result.expect_err("strict mode should reject ! selector");
    assert!(matches!(err.kind, ParseErrorKind::InvalidSelector(ref s) if s == "!"));
}

#[test]
fn strict_unknown_property_errors() {
    // Unknown property currently maps to ExpectedToken("valid
    // declaration") — see warning_to_error in lib.rs. The kind
    // is loose; what matters is that strict surfaces it as an
    // error.
    let result = parse_strict("a { unknown-prop: 5; }");
    let err = result.expect_err("strict mode should reject unknown-prop");
    assert!(matches!(err.kind, ParseErrorKind::ExpectedToken(_)));
}

#[test]
fn strict_invalid_value_errors() {
    let result = parse_strict("a { width: 5px; }");
    let err = result.expect_err("strict mode should reject px unit");
    assert!(matches!(err.kind, ParseErrorKind::ExpectedToken(_)));
}

#[test]
fn strict_unterminated_comment_errors() {
    let result = parse_strict("a /* hello");
    let err = result.expect_err("strict mode should reject unterminated comment");
    assert!(matches!(err.kind, ParseErrorKind::UnterminatedComment));
}

#[test]
fn lenient_returns_first_warning_strict_returns_first_error() {
    // Same input under both modes: warnings vs error.
    let source = "! {}";
    let lenient = parse(source);
    assert_eq!(lenient.warnings.len(), 1);
    let strict = parse_strict(source).expect_err("strict errors on !");
    // Strict surfaces *the same* first issue as lenient's first
    // warning.
    assert_eq!(strict.line, lenient.warnings[0].line);
    assert_eq!(strict.column, lenient.warnings[0].column);
}

#[test]
fn strict_first_error_wins_when_multiple() {
    // Strict surfaces only the *first* error; later ones are
    // not reached.
    let source = "a { unknown1: 1; } ! {} b { unknown2: 2; }";
    let err = parse_strict(source).expect_err("strict errors");
    // The first issue is the unknown property `unknown1`,
    // not the `!` selector that comes later.
    assert!(matches!(err.kind, ParseErrorKind::ExpectedToken(_)));
}
