//! `TuiColor` — either a concrete `Color` or a `var(--name)` reference.
//!
//! Sits on the input side of the cascade (inside `TuiStyle`). The
//! cascade resolves every `TuiColor` into a concrete `Color` via
//! `resolve_tui_color()` before writing into `ComputedStyle.fg` /
//! `.bg` / `.border_fg`, so layout and paint never see a `Var`.
//!
//! ## `var()` resolution
//!
//! Given `TuiColor::Var { name, fallback }`:
//!
//! 1. Look up `name` in the vars map. If found AND parses as a color,
//!    that's the result.
//! 2. Otherwise, recursively resolve the `fallback` (which may itself
//!    be a `Var { ... }` — chains are supported).
//! 3. If neither yields a concrete color, use the property's
//!    "inherit" fallback (passed in by the caller — parent's computed
//!    value for that property).
//!
//! The string-to-Color parser accepts:
//!
//! - `#rgb` / `#rgba` / `#rrggbb` / `#rrggbbaa`: hex literals,
//!   expanded to `Color::Rgb`. Alpha (4- and 8-digit forms) is
//!   validated but dropped — terminal cells paint opaque.
//! - Named ANSI colors (`red`, `blue`, `gray`, `lightcyan`, ...)
//! - `reset` → `Color::Reset` (terminal default)
//! - Decimal `0..=255` → `Color::Indexed`
//!
//! Anything else returns `None` and the cascade uses the fallback
//! chain.

use crate::Color;

/// Input-side color on `TuiStyle`. Either a literal or a `var()` ref.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TuiColor {
    /// A literal terminal color — `#ff0000`, `red`, `Color::Indexed(204)`.
    Literal(Color),
    /// Reference to a custom property (`var(--name)` or `var(--name,
    /// fallback)`). Resolved during cascade against `ComputedStyle.vars`.
    Var {
        name: String,
        /// Nested fallback `TuiColor` if `name` is unresolved. `None`
        /// means "let the cascade use the property's own inherit /
        /// initial fallback".
        fallback: Option<Box<TuiColor>>,
    },
}

impl TuiColor {
    /// `var(--name)` with no fallback.
    pub fn var(name: impl Into<String>) -> Self {
        Self::Var {
            name: name.into(),
            fallback: None,
        }
    }

    /// `var(--name, fallback)`. The `fallback` is itself a `TuiColor`,
    /// so `.var_with("accent", TuiColor::Literal(Color::Rgb(255, 0, 0)))` works,
    /// as does chaining `.var_with("accent", TuiColor::var("fallback"))`.
    pub fn var_with(name: impl Into<String>, fallback: TuiColor) -> Self {
        Self::Var {
            name: name.into(),
            fallback: Some(Box::new(fallback)),
        }
    }

    /// Is this a `Var(...)`? Helper for tests + devtools.
    pub fn is_var(&self) -> bool {
        matches!(self, TuiColor::Var { .. })
    }
}

impl From<Color> for TuiColor {
    fn from(c: Color) -> Self {
        Self::Literal(c)
    }
}

/// Parse a string into a concrete `Color` using the full CSS color
/// grammar — named keywords, hex (3/4/6/8 digit, with or without
/// `#`), indexed (0–255), `rgb()`, and `rgba()`. Used by the cascade
/// to resolve custom-property (`var(--*)`) string values stored on
/// the stylesheet.
///
/// Tokenizes input and dispatches through `parse::values::parse_color`
/// — the single canonical color grammar in `rdom-style`. Returns
/// `None` for unparseable input *or* for parse results that aren't
/// a `TuiColor::Literal` (e.g. a nested `var(--*)` inside a stored
/// var value — vars-in-vars stay unsupported in v0.1.0).
///
/// Simple cases (single named-ident or hex token) take the fast
/// path via `parse_simple_color` directly; the full grammar handles
/// the function-call cases (`rgb()`, `rgba()`).
pub fn parse_color(input: &str) -> Option<Color> {
    let s = input.trim();
    if s.is_empty() {
        return None;
    }
    // Fast path: simple inputs (named / hex with `#` / indexed)
    // that don't need a tokenizer.
    if let Some(c) = parse_simple_color(s) {
        return Some(c);
    }
    // Full grammar via tokenizer — handles `rgb()`, `rgba()`, and
    // any future grammar additions.
    let tokens = crate::parse::tokenize(s).ok()?;
    match crate::parse::values::parse_color(&tokens)? {
        TuiColor::Literal(c) => Some(c),
        // Nested `var()` in a stored var value is unsupported in
        // v0.1.0; the cascade returns `None` here and falls
        // through to the inherit fallback.
        TuiColor::Var { .. } => None,
    }
}

/// Parse the *simple* color subset — named keywords, hex (`#` or
/// bare nibbles), indexed 0–255. Returns `None` for inputs the
/// simple parser can't handle (anything with parentheses, commas,
/// or `var()` syntax — those need the full grammar via
/// [`parse_color`]).
///
/// `pub(crate)` so `parse::values::parse_color_at` can use it for
/// Ident/HexColor tokens without recursing back through the
/// full-grammar dispatch in [`parse_color`].
pub(crate) fn parse_simple_color(input: &str) -> Option<Color> {
    let s = input.trim();
    if s.is_empty() {
        return None;
    }
    // Hex literal with `#` prefix.
    if let Some(hex) = s.strip_prefix('#') {
        return parse_hex(hex);
    }
    // Decimal 0..=255 → indexed.
    if s.chars().all(|c| c.is_ascii_digit()) {
        return s.parse::<u8>().ok().map(Color::Indexed);
    }
    // Named.
    //
    // - `reset` → terminal default fg/bg, an rdom-specific keyword.
    // - `transparent` → `Color::Reset`. Terminals have no alpha; CSS
    //   `transparent` means "let what's behind show through", which
    //   is the same semantics as our `Reset` slot.
    // - Everything else falls through to the 148 CSS named-color
    //   table (`color::named`), which now owns every keyword that
    //   the pre-T6 ANSI match used to claim. `lightblue` resolves
    //   to CSS `#ADD8E6` (pale), `red` to `#FF0000` (full), etc.
    let lower = s.to_ascii_lowercase();
    if matches!(lower.as_str(), "reset" | "transparent") {
        return Some(Color::Reset);
    }
    crate::color::named::lookup(&lower)
}

fn parse_hex(hex: &str) -> Option<Color> {
    match hex.len() {
        3 => {
            // `#rgb` — double each nibble.
            let r = hex_digit(hex.as_bytes()[0])?;
            let g = hex_digit(hex.as_bytes()[1])?;
            let b = hex_digit(hex.as_bytes()[2])?;
            Some(Color::Rgb(r * 17, g * 17, b * 17))
        }
        4 => {
            // `#rgba` — short form with alpha. Alpha (4th nibble)
            // is dropped; rdom-tui paints opaque cells.
            let r = hex_digit(hex.as_bytes()[0])?;
            let g = hex_digit(hex.as_bytes()[1])?;
            let b = hex_digit(hex.as_bytes()[2])?;
            hex_digit(hex.as_bytes()[3])?; // validate alpha nibble
            Some(Color::Rgb(r * 17, g * 17, b * 17))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Color::Rgb(r, g, b))
        }
        8 => {
            // `#rrggbbaa` — long form with alpha. Alpha (last
            // byte) is dropped.
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            u8::from_str_radix(&hex[6..8], 16).ok()?; // validate alpha
            Some(Color::Rgb(r, g, b))
        }
        _ => None,
    }
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Resolve `color` against `vars`. `inherit_fallback` is the
/// property's fallback value (typically the parent's computed color)
/// used when every var lookup and explicit fallback fails.
///
/// This is the single entrypoint used by the cascade; the resolution
/// is pure — no mutation of the vars map.
pub fn resolve_tui_color(
    color: &TuiColor,
    vars: &std::collections::HashMap<String, String>,
    inherit_fallback: Color,
) -> Color {
    match color {
        TuiColor::Literal(c) => *c,
        TuiColor::Var { name, fallback } => {
            // 1. Lookup in vars.
            if let Some(v) = vars.get(name)
                && let Some(c) = parse_color(v)
            {
                return c;
            }
            // 2. Explicit fallback chain.
            if let Some(fb) = fallback {
                return resolve_tui_color(fb, vars, inherit_fallback);
            }
            // 3. Cascade's inherit fallback.
            inherit_fallback
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

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
    fn hex4_rgba_drops_alpha() {
        // `#rgba` — short form with alpha. The 4th nibble is
        // validated but discarded; rdom-tui paints opaque cells.
        assert_eq!(parse_color("#f00f"), Some(Color::Rgb(0xff, 0, 0)));
        assert_eq!(parse_color("#f008"), Some(Color::Rgb(0xff, 0, 0)));
    }

    #[test]
    fn hex8_rrggbbaa_drops_alpha() {
        // `#rrggbbaa` — long form with alpha. Alpha byte
        // discarded.
        assert_eq!(parse_color("#ff0000ff"), Some(Color::Rgb(0xff, 0, 0)));
        assert_eq!(parse_color("#ff000080"), Some(Color::Rgb(0xff, 0, 0)));
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
        );
        assert_eq!(c, Color::Rgb(255, 0, 0));
    }

    #[test]
    fn var_resolves_via_map() {
        let mut vars = HashMap::new();
        vars.insert("accent".into(), "#ff0000".into());
        let c = resolve_tui_color(&TuiColor::var("accent"), &vars, Color::Reset);
        assert_eq!(c, Color::Rgb(255, 0, 0));
    }

    #[test]
    fn var_missing_falls_back_to_inherit() {
        let vars = HashMap::new();
        let c = resolve_tui_color(&TuiColor::var("missing"), &vars, Color::Rgb(0, 0, 255));
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
        let c = resolve_tui_color(&expr, &vars, Color::Reset);
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
        let c = resolve_tui_color(&expr, &vars, Color::Reset);
        assert_eq!(c, Color::Rgb(0, 128, 0));
    }

    #[test]
    fn var_primary_hit_skips_fallback() {
        let mut vars = HashMap::new();
        vars.insert("a".into(), "cyan".into());
        let expr = TuiColor::var_with("a", TuiColor::Literal(Color::Rgb(255, 0, 0)));
        let c = resolve_tui_color(&expr, &vars, Color::Reset);
        assert_eq!(c, Color::Rgb(0, 255, 255));
    }

    // ── CSS named keywords + transparent (T2) ────────────────────────

    /// `transparent` resolves to `Color::Reset` — terminals have no
    /// alpha; "let what's behind show through" is the same slot.
    #[test]
    fn transparent_keyword_resolves_to_reset() {
        assert_eq!(parse_color("transparent"), Some(Color::Reset));
        assert_eq!(parse_color("Transparent"), Some(Color::Reset));
        assert_eq!(parse_color("TRANSPARENT"), Some(Color::Reset));
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
}
