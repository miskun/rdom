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

// ── lab() / lch() / oklab() / oklch(): CSS Color 4 §9 ───────────

/// `color` parses to an opaque sRGB color within one step per channel
/// of `(r, g, b)` (the reference values are rounded).
#[track_caller]
fn assert_near(css: &str, (r, g, b): (u8, u8, u8)) {
    let Some(Color::Rgb(pr, pg, pb)) = parse_color(css) else {
        panic!("{css} → {:?}", parse_color(css));
    };
    let close = |x: u8, y: u8| x.abs_diff(y) <= 1;
    assert!(
        close(pr, r) && close(pg, g) && close(pb, b),
        "{css} → rgb({pr}, {pg}, {pb}), expected rgb({r}, {g}, {b})"
    );
}

/// §9.2: CIE Lab (D50) — the sRGB primaries and white / black.
#[test]
fn lab_converts_to_srgb() {
    assert_near("lab(54.29% 80.80 69.89)", (255, 0, 0));
    assert_near("lab(87.82 -79.27 80.99)", (0, 255, 0));
    assert_near("lab(29.57 68.29 -112.03)", (0, 0, 255));
    assert_near("lab(100 0 0)", (255, 255, 255));
    assert_near("lab(0% 0 0)", (0, 0, 0));
    // §9.1: a / b percentages are of 125; L clamps to 0–100.
    assert_near("lab(54.29% 64.64% 55.912%)", (255, 0, 0));
    assert_near("lab(150 0 0)", (255, 255, 255));
    assert_eq!(
        parse_color("lab(50 0 0 / 50%)").map(|c| c.alpha()),
        Some(128)
    );
}

/// §9.3: LCH — chroma (100% = 150, negative clamps to 0) and hue.
#[test]
fn lch_converts_to_srgb() {
    assert_near("lch(54.29% 106.84 40.85)", (255, 0, 0));
    assert_near("lch(54.29% 106.84 40.85deg)", (255, 0, 0));
    assert_near("lch(50% -10 0)", lab_gray());
    assert_near("lch(50% 0 none)", lab_gray());
}

/// `lab(50 0 0)`: a mid gray.
fn lab_gray() -> (u8, u8, u8) {
    (119, 119, 119)
}

/// §9.4: Oklab and Oklch — L 0–1 (100% = 1), a / b / C 100% = 0.4.
#[test]
fn oklab_and_oklch_convert_to_srgb() {
    assert_near("oklab(0.62796 0.22486 0.12585)", (255, 0, 0));
    assert_near("oklab(62.796% 56.215% 31.4625%)", (255, 0, 0));
    assert_near("oklab(0.86644 -0.23389 0.1795)", (0, 255, 0));
    assert_near("oklab(0.45201 -0.03246 -0.31153)", (0, 0, 255));
    assert_near("oklch(0.62796 0.25768 29.2339)", (255, 0, 0));
    assert_near("oklch(44.027% 0.1603 303.37)", (102, 51, 153));
    assert_near("oklch(1 0 0)", (255, 255, 255));
}

// ── color(): CSS Color 4 §10 ────────────────────────────────────

/// §10: the predefined RGB spaces and XYZ, numbers or percentages.
#[test]
fn color_function_predefined_spaces() {
    assert_near("color(srgb 1 0 0)", (255, 0, 0));
    assert_near("color(srgb 100% 50% 0%)", (255, 128, 0));
    assert_near("color(srgb-linear 0.2 0.2 0.2)", (124, 124, 124));
    assert_near("color(display-p3 0.5 0.5 0.5)", (128, 128, 128));
    assert_near("color(display-p3 1 1 1)", (255, 255, 255));
    assert_near("color(a98-rgb 1 1 1)", (255, 255, 255));
    assert_near("color(prophoto-rgb 1 1 1)", (255, 255, 255));
    assert_near("color(rec2020 0 0 0)", (0, 0, 0));
    assert_near("color(rec2020 1 1 1)", (255, 255, 255));
    assert_near("color(xyz-d65 0.41239 0.21264 0.01933)", (255, 0, 0));
    assert_near("color(xyz 0.41239 0.21264 0.01933)", (255, 0, 0));
    assert_near("color(xyz-d50 0.43607 0.22249 0.01392)", (255, 0, 0));
    assert_eq!(parse_color("color(srgb 1 0 0 / 0.5)"), rgba(255, 0, 0, 128));
    assert_near("color(srgb none 0 0)", (0, 0, 0));
    assert_eq!(parse_color("color(nope 1 0 0)"), None);
    assert_eq!(parse_color("color(srgb 1 0)"), None);
    assert_eq!(parse_color("color(srgb, 1, 0, 0)"), None);
}

/// §13.2: an out-of-gamut color maps into sRGB by reducing its OKLCh
/// chroma — the lightness and hue are kept, not each channel clipped.
#[test]
fn out_of_gamut_colors_are_gamut_mapped() {
    // Display P3 red keeps its hue: still a red, no channel past red's.
    let Some(Color::Rgb(r, g, b)) = parse_color("color(display-p3 1 0 0)") else {
        panic!()
    };
    assert_eq!(r, 255);
    assert!(g < 40 && b < 40, "rgb({r}, {g}, {b})");
    // A chroma far past sRGB at oklch L 0.7, hue 150 (green).
    let Some(Color::Rgb(r, g, b)) = parse_color("oklch(0.7 0.4 150)") else {
        panic!()
    };
    assert!(g > r && g > b, "rgb({r}, {g}, {b})");
    let [l, _, h] = crate::color::oklch_of(parse_color("oklch(0.7 0.4 150)").unwrap());
    assert!((l - 0.7).abs() < 0.02, "L {l}");
    assert!((h - 150.0).abs() < 5.0, "h {h}");
    // Clipping each channel would change the lightness; mapping keeps it.
    let origin = crate::color::oklch_of(Color::Rgb(255, 128, 128));
    let [l, _, _] = crate::color::oklch_of(parse_color("color(srgb 1.2 0.5 0.5)").unwrap());
    assert!(l > origin[0], "mapped L {l} ≤ {}", origin[0]);
}

// ── color-mix(): CSS Color 5 §2 ─────────────────────────────────

/// §2.1: two colors mixed in the named space, by default half each.
#[test]
fn color_mix_in_rectangular_spaces() {
    assert_eq!(
        parse_color("color-mix(in srgb, red, blue)"),
        rgb(128, 0, 128)
    );
    assert_eq!(
        parse_color("color-mix(in srgb, red 25%, blue)"),
        rgb(64, 0, 191)
    );
    assert_eq!(
        parse_color("color-mix(in srgb, 25% red, blue)"),
        rgb(64, 0, 191)
    );
    assert_eq!(
        parse_color("color-mix(in srgb, red, blue 75%)"),
        rgb(64, 0, 191)
    );
    assert_eq!(
        parse_color("color-mix(in srgb, red calc(25%), blue)"),
        rgb(64, 0, 191)
    );
    assert_eq!(
        parse_color("color-mix(in oklab, red, blue)"),
        rgb(140, 83, 162)
    );
    assert_eq!(
        parse_color("color-mix(in srgb-linear, black, white)"),
        rgb(188, 188, 188)
    );
    assert_eq!(parse_color("color-mix(in xyz, red, red)"), rgb(255, 0, 0));
    assert_eq!(
        parse_color("COLOR-MIX(IN SRGB, red, blue)"),
        rgb(128, 0, 128)
    );
}

/// Color 5: without an interpolation method the mix is in Oklab.
#[test]
fn color_mix_defaults_to_oklab() {
    assert_eq!(parse_color("color-mix(red, blue)"), rgb(140, 83, 162));
}

/// §2.2: percentages that do not sum to 100% scale; below 100% the
/// result's alpha scales too; a zero sum or one outside 0–100% is
/// invalid.
#[test]
fn color_mix_percentage_normalization() {
    assert_eq!(
        parse_color("color-mix(in srgb, red 20%, blue 20%)"),
        rgba(128, 0, 128, 102)
    );
    assert_eq!(
        parse_color("color-mix(in srgb, red 60%, blue 60%)"),
        rgb(128, 0, 128)
    );
    assert_eq!(parse_color("color-mix(in srgb, red 0%, blue 0%)"), None);
    assert_eq!(parse_color("color-mix(in srgb, red 150%, blue)"), None);
    assert_eq!(parse_color("color-mix(in srgb, red -5%, blue)"), None);
}

/// Color 4 §12.4: the hue interpolation methods of a polar space.
#[test]
fn color_mix_hue_methods() {
    assert_eq!(
        parse_color("color-mix(in hsl, red, lime)"),
        rgb(255, 255, 0)
    );
    assert_eq!(
        parse_color("color-mix(in hsl shorter hue, red, lime)"),
        rgb(255, 255, 0)
    );
    assert_eq!(
        parse_color("color-mix(in hsl longer hue, red, lime)"),
        rgb(0, 0, 255)
    );
    assert_eq!(
        parse_color("color-mix(in hsl increasing hue, lime, red)"),
        rgb(0, 0, 255)
    );
    assert_eq!(
        parse_color("color-mix(in hsl decreasing hue, red, lime)"),
        rgb(0, 0, 255)
    );
    // A hue method needs a polar space.
    assert_eq!(
        parse_color("color-mix(in srgb longer hue, red, lime)"),
        None
    );
}

/// Color 4 §12.2–§12.3: a missing component takes the other color's;
/// an achromatic color's hue is powerless; alpha is premultiplied.
#[test]
fn color_mix_missing_powerless_and_alpha() {
    assert_eq!(
        parse_color("color-mix(in srgb, rgb(none 0 0), rgb(255 0 0))"),
        rgb(255, 0, 0)
    );
    let Some(Color::Rgb(r, g, b)) = parse_color("color-mix(in oklch, white, blue)") else {
        panic!()
    };
    assert!(b > r && b > g, "rgb({r}, {g}, {b}) keeps blue's hue");
    assert_eq!(
        parse_color("color-mix(in srgb, transparent, red)"),
        rgba(255, 0, 0, 128)
    );
}

/// Colors nest, and a malformed mix is invalid.
#[test]
fn color_mix_nests_and_rejects_malformed() {
    assert_eq!(
        parse_color("color-mix(in srgb, color-mix(in srgb, red, blue), white)"),
        rgb(191, 128, 191)
    );
    for bad in [
        "color-mix(in srgb, red)",
        "color-mix(in srgb red, blue)",
        "color-mix(in nope, red, blue)",
        "color-mix(in srgb, red, blue, lime)",
        "color-mix(srgb, red, blue)",
        "color-mix(in srgb, red 10% 20%, blue)",
        "color-mix(in srgb, reset, blue)",
    ] {
        assert_eq!(parse_color(bad), None, "{bad}");
    }
}

/// Color 5 §2: a mix holding `currentcolor` is computed at
/// computed-value time, against the element's color; it serializes
/// as written.
#[test]
fn color_mix_with_currentcolor_waits_for_the_element() {
    use crate::{ColorContext, TuiColor};
    let mix = TuiColor::parse("color-mix(in srgb, currentColor, blue)").unwrap();
    assert!(mix.depends_on_element());
    let vars = std::collections::HashMap::new();
    assert_eq!(
        mix.resolve(&vars, &ColorContext::new(Color::Rgb(255, 0, 0))),
        rgb(128, 0, 128)
    );
    let mut style = crate::TuiStyle::new();
    crate::property_dispatch::set(
        "background-color",
        "color-mix(in srgb, currentColor 25%, blue)",
        &mut style,
    )
    .unwrap();
    let text = crate::property_dispatch::serialize("background-color", &style).unwrap();
    let mut again = crate::TuiStyle::new();
    crate::property_dispatch::set("background-color", &text, &mut again).unwrap();
    assert_eq!(style, again, "{text}");
}

// ── Relative colors: CSS Color 5 §4 ─────────────────────────────

/// §4.1: `from <color>` binds the origin's channels, converted to the
/// function's space, to keywords the channels may use — alone or in
/// math functions — with `alpha` for its alpha.
#[test]
fn relative_rgb() {
    assert_eq!(parse_color("rgb(from red r g b)"), rgb(255, 0, 0));
    assert_eq!(parse_color("rgb(from red b g r)"), rgb(0, 0, 255));
    assert_eq!(parse_color("rgba(from red R G B)"), rgb(255, 0, 0));
    assert_eq!(
        parse_color("rgb(from rgb(10 20 30) calc(r * 2) g b / 0.5)"),
        rgba(20, 20, 30, 128)
    );
    assert_eq!(
        parse_color("rgb(from rgb(0 0 0 / 50%) r g b / calc(alpha * 2))"),
        rgb(0, 0, 0)
    );
    assert_eq!(parse_color("rgb(from red r g b / alpha)"), rgb(255, 0, 0));
    assert_eq!(
        parse_color("rgb(from hsl(120 100% 50%) r g b)"),
        rgb(0, 255, 0)
    );
    assert_eq!(parse_color("rgb(from red none g b)"), rgb(0, 0, 0));
}

/// §4.2 – §4.4: the hue functions and the Lab family.
#[test]
fn relative_hsl_hwb_lab() {
    assert_eq!(
        parse_color("hsl(from red calc(h + 120) s l)"),
        rgb(0, 255, 0)
    );
    assert_eq!(
        parse_color("hsl(from blue h s calc(l * 0.5))"),
        rgb(0, 0, 128)
    );
    assert_eq!(parse_color("hwb(from red h w b)"), rgb(255, 0, 0));
    assert_near("lab(from red l a b)", (255, 0, 0));
    assert_near("oklab(from red l a b)", (255, 0, 0));
    let Some(Color::Rgb(r, g, b)) = parse_color("oklch(from red l 0 h)") else {
        panic!()
    };
    assert!(
        r.abs_diff(g) <= 1 && g.abs_diff(b) <= 1,
        "a gray: rgb({r}, {g}, {b})"
    );
    let Some(Color::Rgb(r, _, b)) = parse_color("oklch(from red l c calc(h + 180))") else {
        panic!()
    };
    assert!(b > r, "the complement of red is bluish: {r} / {b}");
}

/// §4.5: `color(from <color> <space> …)`, channels `r g b` or `x y z`.
#[test]
fn relative_color_function() {
    assert_eq!(parse_color("color(from red srgb r g b)"), rgb(255, 0, 0));
    assert_eq!(
        parse_color("color(from red srgb calc(r * 0.5) g b)"),
        rgb(128, 0, 0)
    );
    assert_near("color(from red xyz x y z)", (255, 0, 0));
    assert_near("color(from red display-p3 r g b)", (255, 0, 0));
}

/// Relative colors take only the modern syntax, a known keyword per
/// channel and a valid origin.
#[test]
fn relative_colors_reject_malformed() {
    for bad in [
        "rgb(from red r, g, b)",
        "rgb(from red)",
        "rgb(from red x g b)",
        "rgb(from red r g)",
        "rgb(from notacolor r g b)",
        "rgb(from r g b)",
        "color(from red r g b)",
        "hsl(from red r s l)",
    ] {
        assert_eq!(parse_color(bad), None, "{bad}");
    }
}

/// A relative color from `currentcolor` is computed for the element.
#[test]
fn relative_color_from_currentcolor_waits_for_the_element() {
    use crate::{ColorContext, TuiColor};
    let c = TuiColor::parse("rgb(from currentcolor r g b / 50%)").unwrap();
    assert!(c.depends_on_element());
    let vars = std::collections::HashMap::new();
    assert_eq!(
        c.resolve(&vars, &ColorContext::new(Color::Rgb(255, 0, 0))),
        rgba(255, 0, 0, 128)
    );
}

// ── System colors: CSS Color 4 §6.2 ─────────────────────────────

/// §6.2: the system colors parse case-insensitively and serialize as
/// their keyword; they map onto the terminal and the UA palette.
#[test]
fn system_colors_parse_and_resolve() {
    use crate::{ColorContext, TuiColor};
    let vars = std::collections::HashMap::new();
    let cx = ColorContext::new(Color::Rgb(1, 2, 3));
    let resolve = |css: &str| TuiColor::parse(css).and_then(|c| c.resolve(&vars, &cx));
    // The terminal's own colors.
    assert_eq!(resolve("Canvas"), Some(Color::Reset));
    assert_eq!(resolve("canvastext"), Some(Color::Reset));
    // The UA palette.
    assert_eq!(resolve("LinkText"), rgb(30, 144, 255));
    assert_eq!(resolve("VISITEDTEXT"), rgb(30, 144, 255));
    assert_eq!(resolve("Mark"), rgb(255, 255, 0));
    assert_eq!(resolve("MarkText"), rgb(0, 0, 0));
    assert_eq!(resolve("GrayText"), rgb(0x7F, 0x86, 0x8B));
    assert_eq!(resolve("Highlight"), rgb(0x39, 0x4B, 0x7E));
    assert_eq!(resolve("AccentColor"), rgb(30, 144, 255));
    // A deprecated keyword maps to its replacement (§6.2.1).
    assert_eq!(
        TuiColor::parse("ButtonHighlight"),
        TuiColor::parse("ButtonFace")
    );
    let mut style = crate::TuiStyle::new();
    crate::property_dispatch::set("color", "CanvasText", &mut style).unwrap();
    assert_eq!(
        crate::property_dispatch::serialize("color", &style).as_deref(),
        Some("canvastext")
    );
}

/// Inside a color function a system color needs a definite color:
/// `Canvas` / `CanvasText` take the canvas model's (black / white).
#[test]
fn system_colors_inside_functions() {
    assert_eq!(
        parse_color("color-mix(in srgb, Canvas, white)"),
        rgb(128, 128, 128)
    );
    assert_eq!(
        parse_color("color-mix(in srgb, CanvasText, black)"),
        rgb(128, 128, 128)
    );
    assert_eq!(
        parse_color("rgb(from LinkText r g b / 50%)"),
        rgba(30, 144, 255, 128)
    );
}
