//! Unit tests for the URL opener seam + scheme helpers. App-level
//! integration (`<a href>` click → opener) lives in
//! `runtime::builtins::a_href::tests`.

use super::*;

// ── scheme_of ──────────────────────────────────────────────────────

#[test]
fn scheme_of_absolute_url() {
    assert_eq!(scheme_of("https://example.com"), "https");
    assert_eq!(scheme_of("mailto:foo@bar.com"), "mailto");
    assert_eq!(scheme_of("tel:+1234"), "tel");
    assert_eq!(scheme_of("file:///tmp/x"), "file");
}

#[test]
fn scheme_of_returns_empty_for_relative_or_fragment() {
    assert_eq!(scheme_of("/items/default"), "");
    assert_eq!(scheme_of("./relative"), "");
    assert_eq!(scheme_of("../parent"), "");
    assert_eq!(scheme_of("#section"), "");
    assert_eq!(scheme_of(""), "");
}

#[test]
fn scheme_of_case_preserved_for_caller_to_lowercase() {
    // `scheme_of` returns what's there; `is_external_scheme`
    // lowercases for matching.
    assert_eq!(scheme_of("HTTPS://example.com"), "HTTPS");
}

#[test]
fn scheme_of_handles_custom_schemes() {
    assert_eq!(scheme_of("tui://view/items"), "tui");
    assert_eq!(scheme_of("myapp://workspace/local"), "myapp");
    assert_eq!(scheme_of("git://repo"), "git");
}

// ── is_external_scheme ─────────────────────────────────────────────

#[test]
fn external_schemes_match() {
    for s in [
        "http", "https", "mailto", "tel", "sms", "ftp", "file", "data", "blob",
    ] {
        assert!(is_external_scheme(s), "{s} should be external");
    }
}

#[test]
fn external_scheme_is_case_insensitive() {
    assert!(is_external_scheme("HTTPS"));
    assert!(is_external_scheme("MailTo"));
    assert!(is_external_scheme("MAILTO"));
}

#[test]
fn custom_schemes_are_not_external() {
    for s in ["tui", "app", "myapp", "git", "slack", ""] {
        assert!(!is_external_scheme(s), "{s} should be internal");
    }
}

#[test]
fn javascript_scheme_is_not_external_by_policy() {
    // Security: never auto-invoke javascript: links.
    assert!(!is_external_scheme("javascript"));
}

// ── MemoryUrlOpener ────────────────────────────────────────────────

#[test]
fn memory_opener_records_each_call_in_order() {
    let m = MemoryUrlOpener::new();
    assert!(m.opened().is_empty());
    m.open("https://a");
    m.open("mailto:x@y");
    assert_eq!(
        m.opened(),
        vec!["https://a".to_string(), "mailto:x@y".to_string()]
    );
}

#[test]
fn memory_opener_is_default_constructible() {
    let m = MemoryUrlOpener::default();
    m.open("https://z");
    assert_eq!(m.opened().len(), 1);
}
