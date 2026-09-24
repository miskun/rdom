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

/// `CSS-WARNING-POSITION-1`: declaration warnings carry the position of
/// the declaration they are about, not the `{` of the block, and a
/// tokenizer error inside a block is reported in document coordinates.
#[test]
fn declaration_warnings_point_at_the_declaration() {
    let src = "a {\n  color: red;\n  bogus: 1;\n  width: nope;\n  color red\n}";
    let r = parse(src);
    let at = |pred: &dyn Fn(&WarningKind) -> bool| {
        let w = r
            .warnings
            .iter()
            .find(|w| pred(&w.kind))
            .unwrap_or_else(|| panic!("no matching warning in {:?}", r.warnings));
        (w.line, w.column)
    };
    assert_eq!(
        at(&|k| matches!(k, WarningKind::UnknownProperty(p) if p == "bogus")),
        (3, 3)
    );
    assert_eq!(
        at(&|k| matches!(k, WarningKind::InvalidValue { property, .. } if property == "width")),
        (4, 3)
    );
    assert_eq!(
        at(&|k| matches!(k, WarningKind::MalformedDeclaration(t) if t == "color red")),
        (5, 3)
    );

    let r = parse("a {\n  content: \"open\n}");
    let w = r
        .warnings
        .iter()
        .find(|w| matches!(w.kind, WarningKind::UnterminatedString))
        .unwrap();
    assert_eq!((w.line, w.column), (2, 12), "absolute, not body-relative");
}
