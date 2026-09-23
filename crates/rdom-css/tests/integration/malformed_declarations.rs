//! A declaration segment that is not `name : value` is dropped per
//! CSS Syntax 3 §5.4.4 — but dropped *with a warning*, so a typo like
//! `color red;` is visible instead of silently vanishing.

use rdom_css::{WarningKind, parse, parse_inline};

#[test]
fn missing_colon_is_reported_and_the_rest_of_the_block_survives() {
    let r = parse("a { color red; width: 5 }");
    assert_eq!(r.stylesheet.rules().len(), 1);
    assert!(
        r.stylesheet.rules()[0].style.width.is_some(),
        "width still applied"
    );
    assert!(
        r.warnings.iter().any(
            |w| matches!(&w.kind, WarningKind::MalformedDeclaration(text) if text == "color red")
        ),
        "{:?}",
        r.warnings
    );
}

#[test]
fn missing_name_is_reported() {
    let r = parse_inline(": red; width: 5");
    assert!(
        r.warnings
            .iter()
            .any(|w| matches!(&w.kind, WarningKind::MalformedDeclaration(_))),
        "{:?}",
        r.warnings
    );
    assert!(r.style.width.is_some(), "width still applied");
}

#[test]
fn empty_segments_are_not_malformed() {
    // Trailing and doubled semicolons are fine.
    let r = parse("a { width: 5;; }");
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
}
