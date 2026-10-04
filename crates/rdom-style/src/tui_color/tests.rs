//! `TuiColor` resolution and the string color parser.

use super::*;
use std::collections::HashMap;

fn cx() -> ColorContext {
    ColorContext::new(Color::Reset)
}

// ── parse_color ──────────────────────────────────────────────────

#[test]
fn hex6_rgb() {
    assert_eq!(parse_color("#ff0000"), Some(Color::Rgb(255, 0, 0)));
    assert_eq!(parse_color("#00ff00"), Some(Color::Rgb(0, 255, 0)));
    assert_eq!(parse_color("#abcdef"), Some(Color::Rgb(0xab, 0xcd, 0xef)));
}

#[test]
fn hex3_rgb_expands_nibbles() {
    assert_eq!(parse_color("#abc"), Some(Color::Rgb(0xaa, 0xbb, 0xcc)));
    assert_eq!(parse_color("#f00"), Some(Color::Rgb(0xff, 0, 0)));
}

#[test]
fn hex4_rgba_keeps_alpha() {
    // `#rgba` — short form with alpha (CSS Color 4 §5.2).
    assert_eq!(parse_color("#f00f"), Some(Color::Rgb(0xff, 0, 0)));
    assert_eq!(parse_color("#f008"), Some(Color::Rgba(0xff, 0, 0, 0x88)));
}

#[test]
fn hex8_rrggbbaa_keeps_alpha() {
    // `#rrggbbaa` — long form with alpha (CSS Color 4 §5.2).
    assert_eq!(parse_color("#ff0000ff"), Some(Color::Rgb(0xff, 0, 0)));
    assert_eq!(
        parse_color("#ff000080"),
        Some(Color::Rgba(0xff, 0, 0, 0x80))
    );
}

#[test]
fn named_colors() {
    assert_eq!(parse_color("red"), Some(Color::Rgb(255, 0, 0)));
    assert_eq!(parse_color("BLUE"), Some(Color::Rgb(0, 0, 255)));
    assert_eq!(parse_color("gray"), Some(Color::Rgb(128, 128, 128)));
    assert_eq!(parse_color("grey"), Some(Color::Rgb(128, 128, 128)));
    assert_eq!(parse_color("reset"), Some(Color::Reset));
}

#[test]
fn indexed_decimal() {
    assert_eq!(parse_color("0"), Some(Color::Indexed(0)));
    assert_eq!(parse_color("204"), Some(Color::Indexed(204)));
    assert_eq!(parse_color("255"), Some(Color::Indexed(255)));
    // Out of u8 range → None.
    assert_eq!(parse_color("256"), None);
}

#[test]
fn unparseable_returns_none() {
    assert_eq!(parse_color(""), None);
    assert_eq!(parse_color("  "), None);
    assert_eq!(parse_color("notacolor"), None);
    assert_eq!(parse_color("#zzz"), None);
    assert_eq!(parse_color("#12345"), None); // 5-digit not allowed
    assert_eq!(parse_color("#1234567"), None); // 7-digit not allowed
}

#[test]
fn trims_whitespace() {
    assert_eq!(parse_color("   red   "), Some(Color::Rgb(255, 0, 0)));
}

// ── TuiColor builder ─────────────────────────────────────────────

#[test]
fn literal_from_color() {
    let c: TuiColor = Color::Rgb(255, 0, 0).into();
    assert_eq!(c, TuiColor::Literal(Color::Rgb(255, 0, 0)));
}

#[test]
fn var_constructor() {
    let v = TuiColor::var("accent");
    assert!(v.is_var());
    match v {
        TuiColor::Var { name, fallback } => {
            assert_eq!(name, "accent");
            assert!(fallback.is_none());
        }
        _ => unreachable!(),
    }
}

#[test]
fn var_with_fallback() {
    let v = TuiColor::var_with("accent", TuiColor::Literal(Color::Rgb(255, 0, 0)));
    match v {
        TuiColor::Var { name, fallback } => {
            assert_eq!(name, "accent");
            assert_eq!(
                fallback,
                Some(Box::new(TuiColor::Literal(Color::Rgb(255, 0, 0))))
            );
        }
        _ => unreachable!(),
    }
}

// ── resolve_tui_color ────────────────────────────────────────────

#[test]
fn literal_resolves_to_itself() {
    let vars = HashMap::new();
    let c = resolve_tui_color(
        &TuiColor::Literal(Color::Rgb(255, 0, 0)),
        &vars,
        Color::Reset,
        &cx(),
    );
    assert_eq!(c, Color::Rgb(255, 0, 0));
}

/// CSS Color 4 §6.4: `currentcolor` is the context's color, also
/// through a custom property.
#[test]
fn currentcolor_resolves_against_the_context() {
    let mut vars = HashMap::new();
    vars.insert("c".into(), "currentColor".into());
    let cx = ColorContext::new(Color::Rgb(1, 2, 3));
    assert_eq!(
        TuiColor::CurrentColor.resolve(&vars, &cx),
        Some(Color::Rgb(1, 2, 3))
    );
    assert_eq!(
        TuiColor::var("c").resolve(&vars, &cx),
        Some(Color::Rgb(1, 2, 3))
    );
    assert_eq!(
        TuiColor::var("c").substitute_vars(&vars),
        Some(TuiColor::CurrentColor)
    );
    assert_eq!(parse_color("currentcolor"), None);
    assert_eq!(
        TuiColor::parse("currentcolor"),
        Some(TuiColor::CurrentColor)
    );
}

#[test]
fn var_resolves_via_map() {
    let mut vars = HashMap::new();
    vars.insert("accent".into(), "#ff0000".into());
    let c = resolve_tui_color(&TuiColor::var("accent"), &vars, Color::Reset, &cx());
    assert_eq!(c, Color::Rgb(255, 0, 0));
}

#[test]
fn var_missing_falls_back_to_inherit() {
    let vars = HashMap::new();
    let c = resolve_tui_color(
        &TuiColor::var("missing"),
        &vars,
        Color::Rgb(0, 0, 255),
        &cx(),
    );
    assert_eq!(c, Color::Rgb(0, 0, 255));
}

#[test]
fn var_unparseable_falls_back() {
    let mut vars = HashMap::new();
    vars.insert("broken".into(), "not-a-color".into());
    let c = resolve_tui_color(
        &TuiColor::var_with("broken", TuiColor::Literal(Color::Rgb(0, 128, 0))),
        &vars,
        Color::Reset,
        &cx(),
    );
    assert_eq!(c, Color::Rgb(0, 128, 0));
}

#[test]
fn var_explicit_fallback_wins_over_inherit() {
    let vars = HashMap::new();
    let c = resolve_tui_color(
        &TuiColor::var_with("missing", TuiColor::Literal(Color::Rgb(255, 0, 0))),
        &vars,
        Color::Rgb(0, 0, 255), // inherit fallback
        &cx(),
    );
    assert_eq!(c, Color::Rgb(255, 0, 0));
}

#[test]
fn var_fallback_chains() {
    let vars = HashMap::new();
    // var(--a, var(--b, red))
    let expr = TuiColor::var_with(
        "a",
        TuiColor::var_with("b", TuiColor::Literal(Color::Rgb(255, 0, 0))),
    );
    let c = resolve_tui_color(&expr, &vars, Color::Reset, &cx());
    assert_eq!(c, Color::Rgb(255, 0, 0));
}

#[test]
fn var_fallback_chain_early_resolve() {
    let mut vars = HashMap::new();
    vars.insert("b".into(), "green".into());
    // var(--a, var(--b, red)) — a missing, b = green → green.
    let expr = TuiColor::var_with(
        "a",
        TuiColor::var_with("b", TuiColor::Literal(Color::Rgb(255, 0, 0))),
    );
    let c = resolve_tui_color(&expr, &vars, Color::Reset, &cx());
    assert_eq!(c, Color::Rgb(0, 128, 0));
}

#[test]
fn var_primary_hit_skips_fallback() {
    let mut vars = HashMap::new();
    vars.insert("a".into(), "cyan".into());
    let expr = TuiColor::var_with("a", TuiColor::Literal(Color::Rgb(255, 0, 0)));
    let c = resolve_tui_color(&expr, &vars, Color::Reset, &cx());
    assert_eq!(c, Color::Rgb(0, 255, 255));
}

// ── CSS named keywords + transparent (T2) ────────────────────────

/// CSS Color 4 §6.3: `transparent` is transparent black
/// (`rgb(0 0 0 / 0)`), not the terminal default.
#[test]
fn transparent_keyword_is_transparent_black() {
    assert_eq!(parse_color("transparent"), Some(Color::TRANSPARENT));
    assert_eq!(parse_color("Transparent"), Some(Color::TRANSPARENT));
    assert_eq!(parse_color("TRANSPARENT"), Some(Color::TRANSPARENT));
    assert_eq!(parse_color("rgb(0 0 0 / 0)"), Some(Color::TRANSPARENT));
}

/// CSS named colors outside the ANSI-16 overlap resolve via the
/// `color::named` lookup table. Spot-check the headliners.
#[test]
fn css_named_outside_ansi_overlap_resolves_to_rgb() {
    assert_eq!(parse_color("dodgerblue"), Some(Color::Rgb(30, 144, 255)));
    assert_eq!(parse_color("rebeccapurple"), Some(Color::Rgb(102, 51, 153)));
    assert_eq!(parse_color("aliceblue"), Some(Color::Rgb(240, 248, 255)));
    assert_eq!(parse_color("crimson"), Some(Color::Rgb(220, 20, 60)));
}

/// CSS named lookup is case-insensitive per the spec.
#[test]
fn css_named_lookup_is_case_insensitive() {
    assert_eq!(parse_color("DodgerBlue"), Some(Color::Rgb(30, 144, 255)));
    assert_eq!(parse_color("REBECCAPURPLE"), Some(Color::Rgb(102, 51, 153)));
}

/// Names in the ANSI-16 overlap still resolve to the ANSI
/// variant for now — the ANSI palette is deleted in a later
/// commit (T6), at which point these flip to CSS RGB values.
/// Documenting the current behavior explicitly so the T6 commit
/// captures the visual shift.
#[test]
fn ansi_overlap_keywords_still_resolve_to_ansi_for_now() {
    // Will become `Color::Rgb(173, 216, 230)` (CSS lightblue =
    // #ADD8E6) after T6 deletes the ANSI variants.
    assert_eq!(parse_color("lightblue"), Some(Color::Rgb(173, 216, 230)));
    // Will become `Color::Rgb(169, 169, 169)` (CSS darkgray =
    // #A9A9A9) after T6.
    assert_eq!(parse_color("darkgray"), Some(Color::Rgb(169, 169, 169)));
}

/// Spelling variants from the CSS spec resolve identically.
#[test]
fn css_spelling_variants_resolve_alike() {
    // `gainsboro` only has one spelling; pick a real spelling
    // pair from CSS3.
    assert_eq!(parse_color("dimgray"), parse_color("dimgrey"));
    assert_eq!(parse_color("slategray"), parse_color("slategrey"));
    assert_eq!(parse_color("lightslategray"), parse_color("lightslategrey"));
}
