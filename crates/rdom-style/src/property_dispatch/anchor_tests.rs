//! Dispatch tests for the anchor positioning properties and functions
//! (C15-ANCHOR; CSS Anchor Positioning 1 §2–§5).

use super::*;
use crate::TuiStyle;

fn round(name: &str, value: &str) -> Option<String> {
    let mut style = TuiStyle::new();
    set(name, value, &mut style).ok()?;
    serialize(name, &style)
}

/// §2.1–§2.3: `anchor-name: none | <dashed-ident>#`, `anchor-scope: none |
/// all | <dashed-ident>#`, `position-anchor: auto | none | <anchor-name>`;
/// none inherit.
#[test]
fn anchor_names_parse() {
    for (name, css, out) in [
        ("anchor-name", "none", "none"),
        ("anchor-name", "--a", "--a"),
        ("anchor-name", "--a, --b", "--a, --b"),
        ("anchor-scope", "all", "all"),
        ("anchor-scope", "none", "none"),
        ("anchor-scope", "--a,--b", "--a, --b"),
        ("position-anchor", "auto", "auto"),
        ("position-anchor", "none", "none"),
        ("position-anchor", "--tip", "--tip"),
    ] {
        assert_eq!(round(name, css).as_deref(), Some(out), "{name}: {css}");
    }
    for (name, bad) in [
        ("anchor-name", "a"),
        ("anchor-name", "--"),
        ("anchor-name", "--a --b"),
        ("anchor-scope", "all, --a"),
        ("position-anchor", "--a, --b"),
        ("position-anchor", "tip"),
    ] {
        assert_eq!(round(name, bad), None, "{name}: {bad}");
    }
    for name in [
        "anchor-name",
        "anchor-scope",
        "position-anchor",
        "position-area",
        "position-try",
        "position-try-fallbacks",
        "position-try-order",
        "position-visibility",
    ] {
        assert!(!inherits(name), "{name}");
    }
}

/// §3.1: `position-area` — one or two keywords of one vocabulary, `none`.
#[test]
fn position_area_parses() {
    for (css, out) in [
        ("none", "none"),
        ("top", "top"),
        ("bottom span-right", "bottom span-right"),
        ("span-left top", "span-left top"),
        ("center", "center"),
        ("block-end span-inline-end", "block-end span-inline-end"),
        ("start end", "start end"),
        ("self-start center", "self-start center"),
        ("SPAN-ALL", "span-all"),
    ] {
        assert_eq!(round("position-area", css).as_deref(), Some(out), "{css}");
    }
    for bad in [
        "top bottom",
        "left right",
        "block-start top",
        "start self-end",
        "top left right",
        "middle",
    ] {
        assert_eq!(round("position-area", bad), None, "{bad}");
    }
}

/// §4: `position-try-fallbacks`, `position-try-order` and the
/// `position-try` shorthand.
#[test]
fn position_try_parses() {
    for (css, out) in [
        ("none", "none"),
        ("flip-block", "flip-block"),
        ("--a flip-inline", "--a flip-inline"),
        ("flip-block flip-start, --b", "flip-block flip-start, --b"),
        ("top span-left, flip-x", "top span-left, flip-x"),
    ] {
        assert_eq!(
            round("position-try-fallbacks", css).as_deref(),
            Some(out),
            "{css}"
        );
    }
    for bad in ["flip-block flip-block", "--a --b", "--a,", "flip"] {
        assert_eq!(round("position-try-fallbacks", bad), None, "{bad}");
    }
    assert_eq!(
        round("position-try-order", "most-height").as_deref(),
        Some("most-height")
    );
    assert_eq!(
        round("position-try", "most-width flip-block, --a").as_deref(),
        Some("most-width flip-block, --a")
    );
    assert_eq!(
        round("position-try", "flip-inline").as_deref(),
        Some("flip-inline")
    );
}

/// §5: `position-visibility: always | [ anchors-valid || anchors-visible
/// || no-overflow ]`.
#[test]
fn position_visibility_parses() {
    for (css, out) in [
        ("always", "always"),
        ("anchors-visible", "anchors-visible"),
        ("no-overflow anchors-valid", "anchors-valid no-overflow"),
    ] {
        assert_eq!(
            round("position-visibility", css).as_deref(),
            Some(out),
            "{css}"
        );
    }
    for bad in ["always no-overflow", "no-overflow no-overflow", "visible"] {
        assert_eq!(round("position-visibility", bad), None, "{bad}");
    }
}

/// §5.1: `anchor()` in the inset properties — a name and a side in either
/// order, a fallback — alone or in a math function; not in other
/// properties. §5.2: `anchor-size()` in the insets, sizes and margins.
#[test]
fn anchor_functions_parse_where_allowed() {
    for (name, css, out) in [
        ("top", "anchor(bottom)", "anchor(bottom)"),
        ("top", "anchor(--a bottom)", "anchor(--a bottom)"),
        ("left", "anchor(right --a, 2)", "anchor(--a right, 2)"),
        ("left", "anchor(50%)", "anchor(50%)"),
        ("right", "calc(anchor(left) + 1)", "calc(anchor(left) + 1)"),
        ("top", "anchor-size(height)", "anchor-size(height)"),
        ("width", "anchor-size(width)", "anchor-size(width)"),
        ("width", "anchor-size()", "anchor-size()"),
        (
            "min-width",
            "anchor-size(--a width)",
            "anchor-size(--a width)",
        ),
        ("max-height", "anchor-size(--a, 5)", "anchor-size(--a, 5)"),
        ("margin-left", "anchor-size(width)", "anchor-size(width)"),
    ] {
        assert_eq!(round(name, css).as_deref(), Some(out), "{name}: {css}");
    }
    for (name, bad) in [
        ("width", "anchor(top)"),
        ("padding-left", "anchor-size(width)"),
        ("top", "anchor(--a)"),
        ("top", "anchor(top bottom)"),
        ("top", "anchor-size(width height)"),
        ("color", "anchor(top)"),
    ] {
        assert_eq!(round(name, bad), None, "{name}: {bad}");
    }
}

/// §3.4: `anchor-center` is a `<self-position>` of the `*-self` and
/// `*-items` properties, not of the content ones.
#[test]
fn anchor_center_aligns_self_and_items() {
    for name in ["justify-self", "align-self", "justify-items", "align-items"] {
        assert_eq!(
            round(name, "anchor-center").as_deref(),
            Some("anchor-center"),
            "{name}"
        );
    }
    assert_eq!(
        round("align-self", "safe anchor-center").as_deref(),
        Some("safe anchor-center")
    );
    assert_eq!(round("justify-content", "anchor-center"), None);
}
