//! The `<color>` grammar, one section per color function, checked
//! through the public string entry point (`crate::parse_color`).

use crate::{Color, parse_color};

fn rgb(r: u8, g: u8, b: u8) -> Option<Color> {
    Some(Color::Rgb(r, g, b))
}

fn rgba(r: u8, g: u8, b: u8, a: u8) -> Option<Color> {
    Some(Color::Rgba(r, g, b, a))
}

// ── rgb() / rgba(): CSS Color 4 §5.1 ────────────────────────────

/// CSS Color 4 §5.1: the modern syntax separates channels with
/// whitespace and the alpha with `/`.
#[test]
fn rgb_modern_space_syntax() {
    assert_eq!(parse_color("rgb(0 128 255)"), rgb(0, 128, 255));
    assert_eq!(parse_color("rgb(0 128 255 / 1)"), rgb(0, 128, 255));
    assert_eq!(parse_color("rgb(0 128 255 / 0.5)"), rgba(0, 128, 255, 128));
    assert_eq!(parse_color("rgb(0 128 255 / 50%)"), rgba(0, 128, 255, 128));
    assert_eq!(parse_color("RGB(0 0 0 / 0)"), rgba(0, 0, 0, 0));
}

/// CSS Color 4 §5.1: a channel is a `<number>` (0–255, fractions
/// allowed) or a `<percentage>` (0%–100%); the modern syntax may mix
/// them.
#[test]
fn rgb_percentages_and_fractions() {
    assert_eq!(parse_color("rgb(100% 50% 0%)"), rgb(255, 128, 0));
    assert_eq!(parse_color("rgb(100%, 0%, 0%)"), rgb(255, 0, 0));
    assert_eq!(parse_color("rgb(127.5 0.4 .6)"), rgb(128, 0, 1));
    assert_eq!(parse_color("rgb(255 50% 0)"), rgb(255, 128, 0));
}

/// CSS Color 4 §5.1: out-of-range channels clamp at parsed-value time.
#[test]
fn rgb_clamps_out_of_range_channels() {
    assert_eq!(parse_color("rgb(300 -20 0)"), rgb(255, 0, 0));
    assert_eq!(parse_color("rgb(0 0 0 / 2)"), rgb(0, 0, 0));
    assert_eq!(parse_color("rgb(0 0 0 / -1)"), rgba(0, 0, 0, 0));
    assert_eq!(parse_color("rgb(150% 0 0)"), rgb(255, 0, 0));
}

/// CSS Color 4 §4.4: `none` is a missing component, zero when the
/// color is used directly — in the modern syntax only.
#[test]
fn rgb_none_is_a_missing_component() {
    assert_eq!(parse_color("rgb(none 128 255)"), rgb(0, 128, 255));
    assert_eq!(parse_color("rgb(10 20 30 / none)"), rgba(10, 20, 30, 0));
    assert_eq!(parse_color("rgb(none, 0, 0)"), None);
}

/// CSS Color 4 §5.1 + Values 4 §10: math functions in channels and
/// alpha, typed `<number>` or `<percentage>`.
#[test]
fn rgb_math_functions_in_channels() {
    assert_eq!(parse_color("rgb(calc(100 + 28) 0 0)"), rgb(128, 0, 0));
    assert_eq!(parse_color("rgb(calc(50%) 0 0)"), rgb(128, 0, 0));
    assert_eq!(
        parse_color("rgb(min(10, 20) 0 0 / calc(1 / 2))"),
        rgba(10, 0, 0, 128)
    );
    assert_eq!(
        parse_color("rgb(0 0 0 / calc(25% * 2))"),
        rgba(0, 0, 0, 128)
    );
    // A length or an angle is no channel.
    assert_eq!(parse_color("rgb(calc(1ch) 0 0)"), None);
    assert_eq!(parse_color("rgb(calc(1deg) 0 0)"), None);
}

/// CSS Color 4 §5.1: the legacy syntax separates with commas, its
/// three channels are all numbers or all percentages, and it takes no
/// `none`; `rgba()` is the same function.
#[test]
fn rgb_legacy_comma_syntax() {
    assert_eq!(parse_color("rgb(0, 128, 255)"), rgb(0, 128, 255));
    assert_eq!(parse_color("rgba(0, 128, 255)"), rgb(0, 128, 255));
    assert_eq!(parse_color("rgba(10, 20, 30, 0.5)"), rgba(10, 20, 30, 128));
    assert_eq!(parse_color("rgb(10, 20, 30, 25%)"), rgba(10, 20, 30, 64));
    assert_eq!(parse_color("rgba(0 128 255 / 0.5)"), rgba(0, 128, 255, 128));
    // Mixed number / percentage channels are invalid in legacy form.
    assert_eq!(parse_color("rgb(255, 50%, 0)"), None);
}

/// CSS Color 4 §5.1: malformed argument lists are invalid.
#[test]
fn rgb_rejects_malformed_arguments() {
    for bad in [
        "rgb(0 0)",
        "rgb(0 0 0 0)",
        "rgb(0, 0 0)",
        "rgb(0 0 0 /)",
        "rgb(0 0 0 / 1 / 1)",
        "rgb(0, 0, 0, 0, 0)",
        "rgb(0, 0, 0 / 1)",
        "rgb(a b c)",
        "rgb(0 0 0",
    ] {
        assert_eq!(parse_color(bad), None, "{bad}");
    }
}

// ── Hex: CSS Color 4 §5.2 ───────────────────────────────────────

/// CSS Color 4 §5.2: the fourth / eighth digits are the alpha.
#[test]
fn hex_alpha_is_kept() {
    assert_eq!(parse_color("#f008"), rgba(255, 0, 0, 0x88));
    assert_eq!(parse_color("#ff000080"), rgba(255, 0, 0, 0x80));
    assert_eq!(parse_color("#ff0000ff"), rgb(255, 0, 0));
}

// ── currentcolor: CSS Color 4 §6.4 ──────────────────────────────

/// §6.4: `currentcolor` (any case) parses, and serializes in lower
/// case.
#[test]
fn currentcolor_parses_and_serializes() {
    use crate::TuiStyle;
    use crate::property_dispatch::{serialize, set};
    for written in ["currentcolor", "currentColor", "CURRENTCOLOR"] {
        let mut style = TuiStyle::new();
        set("background-color", written, &mut style).unwrap();
        assert_eq!(
            serialize("background-color", &style).as_deref(),
            Some("currentcolor")
        );
    }
}

// ── hsl() / hsla(): CSS Color 4 §7 ──────────────────────────────

/// §7.1: the modern syntax — a hue (number of degrees or `<angle>`),
/// saturation and lightness as percentages or numbers, `/ alpha`.
#[test]
fn hsl_modern_syntax() {
    assert_eq!(parse_color("hsl(120deg 100% 50%)"), rgb(0, 255, 0));
    assert_eq!(parse_color("hsl(0 100% 50%)"), rgb(255, 0, 0));
    assert_eq!(parse_color("hsl(0 100 50)"), rgb(255, 0, 0));
    assert_eq!(parse_color("hsl(240 100% 50% / 0.5)"), rgba(0, 0, 255, 128));
    assert_eq!(parse_color("hsla(0.5turn 100% 50%)"), rgb(0, 255, 255));
    assert_eq!(parse_color("hsl(120 100% 25%)"), rgb(0, 128, 0));
    assert_eq!(parse_color("HSL(3.14159rad 100% 50%)"), rgb(0, 255, 255));
}

/// §7.1: the hue wraps; negative saturation clamps to 0%.
#[test]
fn hsl_hue_wraps_and_saturation_clamps() {
    assert_eq!(parse_color("hsl(-120 100% 50%)"), rgb(0, 0, 255));
    assert_eq!(parse_color("hsl(480 100% 50%)"), rgb(0, 255, 0));
    assert_eq!(parse_color("hsl(0 -50% 50%)"), rgb(128, 128, 128));
}

/// §4.4: `none` is a missing component, zero when used directly.
#[test]
fn hsl_none_components() {
    assert_eq!(parse_color("hsl(none 0% 50%)"), rgb(128, 128, 128));
    assert_eq!(parse_color("hsl(none 100% 50%)"), rgb(255, 0, 0));
    assert_eq!(parse_color("hsl(0 100% 50% / none)"), rgba(255, 0, 0, 0));
}

/// §7.1: math functions in the hue (an angle or a number) and the
/// other channels.
#[test]
fn hsl_math_functions() {
    assert_eq!(parse_color("hsl(calc(60deg * 2) 100% 50%)"), rgb(0, 255, 0));
    assert_eq!(parse_color("hsl(calc(100 + 20) 100% 50%)"), rgb(0, 255, 0));
    assert_eq!(parse_color("hsl(0 calc(50% * 2) 50%)"), rgb(255, 0, 0));
}

/// §7.1: the legacy syntax takes commas, percentages for saturation
/// and lightness, and no `none`.
#[test]
fn hsl_legacy_syntax() {
    assert_eq!(parse_color("hsl(120, 100%, 50%)"), rgb(0, 255, 0));
    assert_eq!(
        parse_color("hsla(120deg, 100%, 50%, 0.5)"),
        rgba(0, 255, 0, 128)
    );
    assert_eq!(parse_color("hsl(120, 100, 50)"), None);
    assert_eq!(parse_color("hsl(none, 100%, 50%)"), None);
    assert_eq!(parse_color("hsl(120, 100%)"), None);
}

// ── hwb(): CSS Color 4 §8 ───────────────────────────────────────

/// §8.1: hue, whiteness and blackness; whiteness + blackness ≥ 100%
/// is a gray.
#[test]
fn hwb_syntax() {
    assert_eq!(parse_color("hwb(0 0% 0%)"), rgb(255, 0, 0));
    assert_eq!(parse_color("hwb(120 0% 50%)"), rgb(0, 128, 0));
    assert_eq!(parse_color("hwb(0 60% 60%)"), rgb(128, 128, 128));
    assert_eq!(
        parse_color("hwb(240deg 20 20 / 50%)"),
        rgba(51, 51, 204, 128)
    );
    assert_eq!(parse_color("hwb(none none none)"), rgb(255, 0, 0));
    // No legacy syntax.
    assert_eq!(parse_color("hwb(0, 0%, 0%)"), None);
}
