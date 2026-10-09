//! Dispatch tests for the multi-column and fragmentation properties
//! (C15-COLUMNS; CSS Multi-column 1 §3–§7, CSS Fragmentation 3 §3, §5.4).

use super::*;
use crate::TuiStyle;

fn round(name: &str, value: &str) -> Option<String> {
    let mut style = TuiStyle::new();
    set(name, value, &mut style).ok()?;
    serialize(name, &style)
}

/// Multi-column 1 §3.1: `column-width: auto | <length [0,∞]>` — cells,
/// `ch`, a viewport unit, a math function; no percentage and no pixels
/// (geometry). §3.2: `column-count: auto | <integer [1,∞]>`.
#[test]
fn column_width_and_count_parse() {
    for (css, out) in [
        ("auto", "auto"),
        ("20", "20"),
        ("12ch", "12"),
        ("0", "0"),
        ("calc(10 + 2)", "12"),
    ] {
        assert_eq!(round("column-width", css).as_deref(), Some(out), "{css}");
    }
    assert!(round("column-width", "10vw").is_some());
    for bad in ["10px", "2em", "50%", "-1", "auto 3", ""] {
        assert_eq!(round("column-width", bad), None, "{bad}");
    }
    for (css, out) in [("auto", "auto"), ("3", "3"), ("calc(1 + 1)", "2")] {
        assert_eq!(round("column-count", css).as_deref(), Some(out), "{css}");
    }
    for bad in ["0", "-2", "2.5", "3ch", "none"] {
        assert_eq!(round("column-count", bad), None, "{bad}");
    }
}

/// §3.3: `columns: <'column-width'> || <'column-count'>` — a bare integer
/// is the count, a length with a unit the width, `auto` the one left.
#[test]
fn columns_shorthand_parses() {
    for (css, out) in [
        ("auto", "auto"),
        ("auto auto", "auto"),
        ("3", "3"),
        ("20ch", "20ch"),
        ("20ch 3", "20ch 3"),
        ("3 20ch", "20ch 3"),
        ("auto 3", "3"),
        ("20ch auto", "20ch"),
        ("0", "0ch"),
    ] {
        assert_eq!(round("columns", css).as_deref(), Some(out), "{css}");
    }
    for bad in ["3 4", "20ch 30ch", "auto auto auto", "10px", "", "2.5"] {
        assert_eq!(round("columns", bad), None, "{bad}");
    }
    let mut style = TuiStyle::new();
    set("columns", "12ch 2", &mut style).unwrap();
    assert_eq!(serialize("column-width", &style).as_deref(), Some("12"));
    assert_eq!(serialize("column-count", &style).as_deref(), Some("2"));
}

/// §4: `column-rule-style` a `<line-style>`, `-width` a `<line-width>`
/// (pixels select a weight), `-color` a `<color>`, and the `column-rule`
/// shorthand of the three.
#[test]
fn column_rule_parses() {
    assert_eq!(
        round("column-rule-style", "dashed").as_deref(),
        Some("dashed")
    );
    assert_eq!(
        round("column-rule-width", "thick").as_deref(),
        Some("thick")
    );
    assert_eq!(round("column-rule-width", "1px").as_deref(), Some("1px"));
    assert_eq!(round("column-rule-color", "red").as_deref(), Some("red"));
    for (css, out) in [
        ("solid", "solid"),
        ("thick dotted blue", "thick dotted blue"),
        ("blue solid", "solid blue"),
        ("medium none currentcolor", "medium"),
    ] {
        assert_eq!(round("column-rule", css).as_deref(), Some(out), "{css}");
    }
    for bad in ["solid dashed", "", "3 4 5 6"] {
        assert_eq!(round("column-rule", bad), None, "{bad}");
    }
}

/// §6.1 `column-span: none | all`, §7.1 `column-fill: auto | balance |
/// balance-all`; none of the multi-column properties inherit.
#[test]
fn column_span_and_fill_parse() {
    for k in ["none", "all"] {
        assert_eq!(round("column-span", k).as_deref(), Some(k));
    }
    assert_eq!(round("column-span", "2"), None);
    for k in ["auto", "balance", "balance-all"] {
        assert_eq!(round("column-fill", k).as_deref(), Some(k));
    }
    assert_eq!(round("column-fill", "Balance").as_deref(), Some("balance"));
    for name in [
        "columns",
        "column-count",
        "column-width",
        "column-rule",
        "column-rule-style",
        "column-rule-width",
        "column-rule-color",
        "column-span",
        "column-fill",
    ] {
        assert!(!inherits(name), "{name}");
    }
}

/// CSS Fragmentation 3 §3.1–§3.3, Fragmentation 4 §3.1: the break
/// properties' keywords, the legacy `page-break-*` aliases (§3.4: `always`
/// is `page`), `orphans` / `widows` (`<integer [1,∞]>`, inherited) and
/// `box-decoration-break` (§5.4).
#[test]
fn fragmentation_properties_parse() {
    for k in [
        "auto",
        "avoid",
        "always",
        "all",
        "avoid-page",
        "page",
        "left",
        "right",
        "recto",
        "verso",
        "avoid-column",
        "column",
        "avoid-region",
        "region",
    ] {
        assert_eq!(round("break-before", k).as_deref(), Some(k));
        assert_eq!(round("break-after", k).as_deref(), Some(k));
    }
    for k in [
        "auto",
        "avoid",
        "avoid-page",
        "avoid-column",
        "avoid-region",
    ] {
        assert_eq!(round("break-inside", k).as_deref(), Some(k));
    }
    assert_eq!(round("break-inside", "column"), None);
    let mut style = TuiStyle::new();
    set("page-break-before", "always", &mut style).unwrap();
    assert_eq!(serialize("break-before", &style).as_deref(), Some("page"));
    assert_eq!(
        serialize("page-break-before", &style).as_deref(),
        Some("always")
    );
    set("break-after", "column", &mut style).unwrap();
    assert_eq!(serialize("page-break-after", &style), None);
    assert_eq!(
        round("page-break-inside", "avoid").as_deref(),
        Some("avoid")
    );
    assert_eq!(round("page-break-inside", "avoid-page"), None);
    for name in ["orphans", "widows"] {
        assert_eq!(round(name, "3").as_deref(), Some("3"));
        assert_eq!(round(name, "0"), None);
        assert_eq!(round(name, "1.5"), None);
        assert!(inherits(name));
    }
    for k in ["slice", "clone"] {
        assert_eq!(round("box-decoration-break", k).as_deref(), Some(k));
    }
    for name in [
        "break-before",
        "break-after",
        "break-inside",
        "box-decoration-break",
    ] {
        assert!(!inherits(name), "{name}");
    }
}
