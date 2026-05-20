//! CSS named colors — the 148 keywords from CSS Color Module Level 4 §6.1.
//!
//! Authoritative source:
//! <https://developer.mozilla.org/en-US/docs/Web/CSS/named-color>.
//! Every name maps to a fixed RGB value; the spec defines them as
//! case-insensitive aliases.
//!
//! Constants are exposed `pub const` so author code can refer to them
//! at compile time:
//!
//! ```
//! use rdom_style::{Color, color::named};
//! let accent: Color = named::DODGERBLUE;
//! ```
//!
//! Runtime lookup (e.g. from a CSS parser turning `color: rebeccapurple`
//! into a value) uses [`lookup`]:
//!
//! ```
//! use rdom_style::{Color, color::named};
//! assert_eq!(named::lookup("RebeccaPurple"), Some(Color::Rgb(102, 51, 153)));
//! assert_eq!(named::lookup("not-a-color"), None);
//! ```
//!
//! The `transparent` CSS keyword is NOT in this table — it maps to
//! `Color::Reset` and is handled at the parser level.
//!
//! Lookup cost: a sorted `&'static [(&'static str, Color)]` plus
//! `O(log n)` binary search with ASCII-case-insensitive compare. No
//! allocation, no `HashMap`, no `phf` dependency. 148 entries fit
//! comfortably in `.rodata` (~3 KB).
//!
//! ## A note on `dead_code`
//!
//! Every constant here is `pub` and part of the published API
//! surface, but clippy's `dead_code` lint flags `pub const`s when
//! they have no internal consumer. The UA stylesheet uses only a
//! handful of these (`DODGERBLUE`, `DARKGRAY`, etc.); the rest sit
//! waiting for downstream authors to reach for them. They are
//! intentionally exposed even when unused — exactly the same
//! contract as `std::f64::consts::PI` being available even in
//! programs that never use it. Module-level `allow(dead_code)`
//! reflects that intent rather than silencing a real defect.

#![allow(dead_code)]

use crate::Color;
use core::cmp::Ordering;

// ── CSS1 (16 keywords) ──────────────────────────────────────────────

pub const BLACK: Color = Color::Rgb(0, 0, 0);
pub const SILVER: Color = Color::Rgb(192, 192, 192);
pub const GRAY: Color = Color::Rgb(128, 128, 128);
pub const WHITE: Color = Color::Rgb(255, 255, 255);
pub const MAROON: Color = Color::Rgb(128, 0, 0);
pub const RED: Color = Color::Rgb(255, 0, 0);
pub const PURPLE: Color = Color::Rgb(128, 0, 128);
pub const FUCHSIA: Color = Color::Rgb(255, 0, 255);
pub const GREEN: Color = Color::Rgb(0, 128, 0);
pub const LIME: Color = Color::Rgb(0, 255, 0);
pub const OLIVE: Color = Color::Rgb(128, 128, 0);
pub const YELLOW: Color = Color::Rgb(255, 255, 0);
pub const NAVY: Color = Color::Rgb(0, 0, 128);
pub const BLUE: Color = Color::Rgb(0, 0, 255);
pub const TEAL: Color = Color::Rgb(0, 128, 128);
pub const AQUA: Color = Color::Rgb(0, 255, 255);

// ── CSS2 (1 keyword) ────────────────────────────────────────────────

pub const ORANGE: Color = Color::Rgb(255, 165, 0);

// ── CSS3 (130 keywords) ─────────────────────────────────────────────

pub const ALICEBLUE: Color = Color::Rgb(240, 248, 255);
pub const ANTIQUEWHITE: Color = Color::Rgb(250, 235, 215);
pub const AQUAMARINE: Color = Color::Rgb(127, 255, 212);
pub const AZURE: Color = Color::Rgb(240, 255, 255);
pub const BEIGE: Color = Color::Rgb(245, 245, 220);
pub const BISQUE: Color = Color::Rgb(255, 228, 196);
pub const BLANCHEDALMOND: Color = Color::Rgb(255, 235, 205);
pub const BLUEVIOLET: Color = Color::Rgb(138, 43, 226);
pub const BROWN: Color = Color::Rgb(165, 42, 42);
pub const BURLYWOOD: Color = Color::Rgb(222, 184, 135);
pub const CADETBLUE: Color = Color::Rgb(95, 158, 160);
pub const CHARTREUSE: Color = Color::Rgb(127, 255, 0);
pub const CHOCOLATE: Color = Color::Rgb(210, 105, 30);
pub const CORAL: Color = Color::Rgb(255, 127, 80);
pub const CORNFLOWERBLUE: Color = Color::Rgb(100, 149, 237);
pub const CORNSILK: Color = Color::Rgb(255, 248, 220);
pub const CRIMSON: Color = Color::Rgb(220, 20, 60);
pub const CYAN: Color = Color::Rgb(0, 255, 255);
pub const DARKBLUE: Color = Color::Rgb(0, 0, 139);
pub const DARKCYAN: Color = Color::Rgb(0, 139, 139);
pub const DARKGOLDENROD: Color = Color::Rgb(184, 134, 11);
pub const DARKGRAY: Color = Color::Rgb(169, 169, 169);
pub const DARKGREEN: Color = Color::Rgb(0, 100, 0);
pub const DARKGREY: Color = Color::Rgb(169, 169, 169);
pub const DARKKHAKI: Color = Color::Rgb(189, 183, 107);
pub const DARKMAGENTA: Color = Color::Rgb(139, 0, 139);
pub const DARKOLIVEGREEN: Color = Color::Rgb(85, 107, 47);
pub const DARKORANGE: Color = Color::Rgb(255, 140, 0);
pub const DARKORCHID: Color = Color::Rgb(153, 50, 204);
pub const DARKRED: Color = Color::Rgb(139, 0, 0);
pub const DARKSALMON: Color = Color::Rgb(233, 150, 122);
pub const DARKSEAGREEN: Color = Color::Rgb(143, 188, 143);
pub const DARKSLATEBLUE: Color = Color::Rgb(72, 61, 139);
pub const DARKSLATEGRAY: Color = Color::Rgb(47, 79, 79);
pub const DARKSLATEGREY: Color = Color::Rgb(47, 79, 79);
pub const DARKTURQUOISE: Color = Color::Rgb(0, 206, 209);
pub const DARKVIOLET: Color = Color::Rgb(148, 0, 211);
pub const DEEPPINK: Color = Color::Rgb(255, 20, 147);
pub const DEEPSKYBLUE: Color = Color::Rgb(0, 191, 255);
pub const DIMGRAY: Color = Color::Rgb(105, 105, 105);
pub const DIMGREY: Color = Color::Rgb(105, 105, 105);
pub const DODGERBLUE: Color = Color::Rgb(30, 144, 255);
pub const FIREBRICK: Color = Color::Rgb(178, 34, 34);
pub const FLORALWHITE: Color = Color::Rgb(255, 250, 240);
pub const FORESTGREEN: Color = Color::Rgb(34, 139, 34);
pub const GAINSBORO: Color = Color::Rgb(220, 220, 220);
pub const GHOSTWHITE: Color = Color::Rgb(248, 248, 255);
pub const GOLD: Color = Color::Rgb(255, 215, 0);
pub const GOLDENROD: Color = Color::Rgb(218, 165, 32);
pub const GREENYELLOW: Color = Color::Rgb(173, 255, 47);
pub const GREY: Color = Color::Rgb(128, 128, 128);
pub const HONEYDEW: Color = Color::Rgb(240, 255, 240);
pub const HOTPINK: Color = Color::Rgb(255, 105, 180);
pub const INDIANRED: Color = Color::Rgb(205, 92, 92);
pub const INDIGO: Color = Color::Rgb(75, 0, 130);
pub const IVORY: Color = Color::Rgb(255, 255, 240);
pub const KHAKI: Color = Color::Rgb(240, 230, 140);
pub const LAVENDER: Color = Color::Rgb(230, 230, 250);
pub const LAVENDERBLUSH: Color = Color::Rgb(255, 240, 245);
pub const LAWNGREEN: Color = Color::Rgb(124, 252, 0);
pub const LEMONCHIFFON: Color = Color::Rgb(255, 250, 205);
pub const LIGHTBLUE: Color = Color::Rgb(173, 216, 230);
pub const LIGHTCORAL: Color = Color::Rgb(240, 128, 128);
pub const LIGHTCYAN: Color = Color::Rgb(224, 255, 255);
pub const LIGHTGOLDENRODYELLOW: Color = Color::Rgb(250, 250, 210);
pub const LIGHTGRAY: Color = Color::Rgb(211, 211, 211);
pub const LIGHTGREEN: Color = Color::Rgb(144, 238, 144);
pub const LIGHTGREY: Color = Color::Rgb(211, 211, 211);
pub const LIGHTPINK: Color = Color::Rgb(255, 182, 193);
pub const LIGHTSALMON: Color = Color::Rgb(255, 160, 122);
pub const LIGHTSEAGREEN: Color = Color::Rgb(32, 178, 170);
pub const LIGHTSKYBLUE: Color = Color::Rgb(135, 206, 250);
pub const LIGHTSLATEGRAY: Color = Color::Rgb(119, 136, 153);
pub const LIGHTSLATEGREY: Color = Color::Rgb(119, 136, 153);
pub const LIGHTSTEELBLUE: Color = Color::Rgb(176, 196, 222);
pub const LIGHTYELLOW: Color = Color::Rgb(255, 255, 224);
pub const LIMEGREEN: Color = Color::Rgb(50, 205, 50);
pub const LINEN: Color = Color::Rgb(250, 240, 230);
pub const MAGENTA: Color = Color::Rgb(255, 0, 255);
pub const MEDIUMAQUAMARINE: Color = Color::Rgb(102, 205, 170);
pub const MEDIUMBLUE: Color = Color::Rgb(0, 0, 205);
pub const MEDIUMORCHID: Color = Color::Rgb(186, 85, 211);
pub const MEDIUMPURPLE: Color = Color::Rgb(147, 112, 219);
pub const MEDIUMSEAGREEN: Color = Color::Rgb(60, 179, 113);
pub const MEDIUMSLATEBLUE: Color = Color::Rgb(123, 104, 238);
pub const MEDIUMSPRINGGREEN: Color = Color::Rgb(0, 250, 154);
pub const MEDIUMTURQUOISE: Color = Color::Rgb(72, 209, 204);
pub const MEDIUMVIOLETRED: Color = Color::Rgb(199, 21, 133);
pub const MIDNIGHTBLUE: Color = Color::Rgb(25, 25, 112);
pub const MINTCREAM: Color = Color::Rgb(245, 255, 250);
pub const MISTYROSE: Color = Color::Rgb(255, 228, 225);
pub const MOCCASIN: Color = Color::Rgb(255, 228, 181);
pub const NAVAJOWHITE: Color = Color::Rgb(255, 222, 173);
pub const OLDLACE: Color = Color::Rgb(253, 245, 230);
pub const OLIVEDRAB: Color = Color::Rgb(107, 142, 35);
pub const ORANGERED: Color = Color::Rgb(255, 69, 0);
pub const ORCHID: Color = Color::Rgb(218, 112, 214);
pub const PALEGOLDENROD: Color = Color::Rgb(238, 232, 170);
pub const PALEGREEN: Color = Color::Rgb(152, 251, 152);
pub const PALETURQUOISE: Color = Color::Rgb(175, 238, 238);
pub const PALEVIOLETRED: Color = Color::Rgb(219, 112, 147);
pub const PAPAYAWHIP: Color = Color::Rgb(255, 239, 213);
pub const PEACHPUFF: Color = Color::Rgb(255, 218, 185);
pub const PERU: Color = Color::Rgb(205, 133, 63);
pub const PINK: Color = Color::Rgb(255, 192, 203);
pub const PLUM: Color = Color::Rgb(221, 160, 221);
pub const POWDERBLUE: Color = Color::Rgb(176, 224, 230);
pub const ROSYBROWN: Color = Color::Rgb(188, 143, 143);
pub const ROYALBLUE: Color = Color::Rgb(65, 105, 225);
pub const SADDLEBROWN: Color = Color::Rgb(139, 69, 19);
pub const SALMON: Color = Color::Rgb(250, 128, 114);
pub const SANDYBROWN: Color = Color::Rgb(244, 164, 96);
pub const SEAGREEN: Color = Color::Rgb(46, 139, 87);
pub const SEASHELL: Color = Color::Rgb(255, 245, 238);
pub const SIENNA: Color = Color::Rgb(160, 82, 45);
pub const SKYBLUE: Color = Color::Rgb(135, 206, 235);
pub const SLATEBLUE: Color = Color::Rgb(106, 90, 205);
pub const SLATEGRAY: Color = Color::Rgb(112, 128, 144);
pub const SLATEGREY: Color = Color::Rgb(112, 128, 144);
pub const SNOW: Color = Color::Rgb(255, 250, 250);
pub const SPRINGGREEN: Color = Color::Rgb(0, 255, 127);
pub const STEELBLUE: Color = Color::Rgb(70, 130, 180);
pub const TAN: Color = Color::Rgb(210, 180, 140);
pub const THISTLE: Color = Color::Rgb(216, 191, 216);
pub const TOMATO: Color = Color::Rgb(255, 99, 71);
pub const TURQUOISE: Color = Color::Rgb(64, 224, 208);
pub const VIOLET: Color = Color::Rgb(238, 130, 238);
pub const WHEAT: Color = Color::Rgb(245, 222, 179);
pub const WHITESMOKE: Color = Color::Rgb(245, 245, 245);
pub const YELLOWGREEN: Color = Color::Rgb(154, 205, 50);

// ── CSS4 (1 keyword) ────────────────────────────────────────────────

pub const REBECCAPURPLE: Color = Color::Rgb(102, 51, 153);

// ── Sorted lookup table for runtime resolution ──────────────────────

/// `(lowercase_name, color)` pairs, **sorted by name** so [`lookup`]
/// can use binary search. CSS name comparison is ASCII-case-
/// insensitive; the table stores the canonical lowercase form, and
/// [`lookup`] compares case-insensitively.
const NAMED: &[(&str, Color)] = &[
    ("aliceblue", ALICEBLUE),
    ("antiquewhite", ANTIQUEWHITE),
    ("aqua", AQUA),
    ("aquamarine", AQUAMARINE),
    ("azure", AZURE),
    ("beige", BEIGE),
    ("bisque", BISQUE),
    ("black", BLACK),
    ("blanchedalmond", BLANCHEDALMOND),
    ("blue", BLUE),
    ("blueviolet", BLUEVIOLET),
    ("brown", BROWN),
    ("burlywood", BURLYWOOD),
    ("cadetblue", CADETBLUE),
    ("chartreuse", CHARTREUSE),
    ("chocolate", CHOCOLATE),
    ("coral", CORAL),
    ("cornflowerblue", CORNFLOWERBLUE),
    ("cornsilk", CORNSILK),
    ("crimson", CRIMSON),
    ("cyan", CYAN),
    ("darkblue", DARKBLUE),
    ("darkcyan", DARKCYAN),
    ("darkgoldenrod", DARKGOLDENROD),
    ("darkgray", DARKGRAY),
    ("darkgreen", DARKGREEN),
    ("darkgrey", DARKGREY),
    ("darkkhaki", DARKKHAKI),
    ("darkmagenta", DARKMAGENTA),
    ("darkolivegreen", DARKOLIVEGREEN),
    ("darkorange", DARKORANGE),
    ("darkorchid", DARKORCHID),
    ("darkred", DARKRED),
    ("darksalmon", DARKSALMON),
    ("darkseagreen", DARKSEAGREEN),
    ("darkslateblue", DARKSLATEBLUE),
    ("darkslategray", DARKSLATEGRAY),
    ("darkslategrey", DARKSLATEGREY),
    ("darkturquoise", DARKTURQUOISE),
    ("darkviolet", DARKVIOLET),
    ("deeppink", DEEPPINK),
    ("deepskyblue", DEEPSKYBLUE),
    ("dimgray", DIMGRAY),
    ("dimgrey", DIMGREY),
    ("dodgerblue", DODGERBLUE),
    ("firebrick", FIREBRICK),
    ("floralwhite", FLORALWHITE),
    ("forestgreen", FORESTGREEN),
    ("fuchsia", FUCHSIA),
    ("gainsboro", GAINSBORO),
    ("ghostwhite", GHOSTWHITE),
    ("gold", GOLD),
    ("goldenrod", GOLDENROD),
    ("gray", GRAY),
    ("green", GREEN),
    ("greenyellow", GREENYELLOW),
    ("grey", GREY),
    ("honeydew", HONEYDEW),
    ("hotpink", HOTPINK),
    ("indianred", INDIANRED),
    ("indigo", INDIGO),
    ("ivory", IVORY),
    ("khaki", KHAKI),
    ("lavender", LAVENDER),
    ("lavenderblush", LAVENDERBLUSH),
    ("lawngreen", LAWNGREEN),
    ("lemonchiffon", LEMONCHIFFON),
    ("lightblue", LIGHTBLUE),
    ("lightcoral", LIGHTCORAL),
    ("lightcyan", LIGHTCYAN),
    ("lightgoldenrodyellow", LIGHTGOLDENRODYELLOW),
    ("lightgray", LIGHTGRAY),
    ("lightgreen", LIGHTGREEN),
    ("lightgrey", LIGHTGREY),
    ("lightpink", LIGHTPINK),
    ("lightsalmon", LIGHTSALMON),
    ("lightseagreen", LIGHTSEAGREEN),
    ("lightskyblue", LIGHTSKYBLUE),
    ("lightslategray", LIGHTSLATEGRAY),
    ("lightslategrey", LIGHTSLATEGREY),
    ("lightsteelblue", LIGHTSTEELBLUE),
    ("lightyellow", LIGHTYELLOW),
    ("lime", LIME),
    ("limegreen", LIMEGREEN),
    ("linen", LINEN),
    ("magenta", MAGENTA),
    ("maroon", MAROON),
    ("mediumaquamarine", MEDIUMAQUAMARINE),
    ("mediumblue", MEDIUMBLUE),
    ("mediumorchid", MEDIUMORCHID),
    ("mediumpurple", MEDIUMPURPLE),
    ("mediumseagreen", MEDIUMSEAGREEN),
    ("mediumslateblue", MEDIUMSLATEBLUE),
    ("mediumspringgreen", MEDIUMSPRINGGREEN),
    ("mediumturquoise", MEDIUMTURQUOISE),
    ("mediumvioletred", MEDIUMVIOLETRED),
    ("midnightblue", MIDNIGHTBLUE),
    ("mintcream", MINTCREAM),
    ("mistyrose", MISTYROSE),
    ("moccasin", MOCCASIN),
    ("navajowhite", NAVAJOWHITE),
    ("navy", NAVY),
    ("oldlace", OLDLACE),
    ("olive", OLIVE),
    ("olivedrab", OLIVEDRAB),
    ("orange", ORANGE),
    ("orangered", ORANGERED),
    ("orchid", ORCHID),
    ("palegoldenrod", PALEGOLDENROD),
    ("palegreen", PALEGREEN),
    ("paleturquoise", PALETURQUOISE),
    ("palevioletred", PALEVIOLETRED),
    ("papayawhip", PAPAYAWHIP),
    ("peachpuff", PEACHPUFF),
    ("peru", PERU),
    ("pink", PINK),
    ("plum", PLUM),
    ("powderblue", POWDERBLUE),
    ("purple", PURPLE),
    ("rebeccapurple", REBECCAPURPLE),
    ("red", RED),
    ("rosybrown", ROSYBROWN),
    ("royalblue", ROYALBLUE),
    ("saddlebrown", SADDLEBROWN),
    ("salmon", SALMON),
    ("sandybrown", SANDYBROWN),
    ("seagreen", SEAGREEN),
    ("seashell", SEASHELL),
    ("sienna", SIENNA),
    ("silver", SILVER),
    ("skyblue", SKYBLUE),
    ("slateblue", SLATEBLUE),
    ("slategray", SLATEGRAY),
    ("slategrey", SLATEGREY),
    ("snow", SNOW),
    ("springgreen", SPRINGGREEN),
    ("steelblue", STEELBLUE),
    ("tan", TAN),
    ("teal", TEAL),
    ("thistle", THISTLE),
    ("tomato", TOMATO),
    ("turquoise", TURQUOISE),
    ("violet", VIOLET),
    ("wheat", WHEAT),
    ("white", WHITE),
    ("whitesmoke", WHITESMOKE),
    ("yellow", YELLOW),
    ("yellowgreen", YELLOWGREEN),
];

/// Look up a CSS named color. ASCII-case-insensitive per the CSS
/// spec — `RebeccaPurple`, `rebeccapurple`, and `REBECCAPURPLE` all
/// resolve to the same RGB. Returns `None` for unknown names and
/// for the special keyword `transparent` (which maps to
/// `Color::Reset` at the parser level, NOT here).
pub fn lookup(name: &str) -> Option<Color> {
    NAMED
        .binary_search_by(|&(canonical, _)| cmp_lowercase(canonical, name))
        .ok()
        .map(|i| NAMED[i].1)
}

/// Compare two ASCII strings ignoring case. The canonical table
/// stores names already lowercased so `a` is treated as-is and `b`
/// is the user's input (which may be mixed case).
fn cmp_lowercase(a: &str, b: &str) -> Ordering {
    let mut ab = a.bytes();
    let mut bb = b.bytes();
    loop {
        match (ab.next(), bb.next()) {
            (Some(x), Some(y)) => {
                let yl = y.to_ascii_lowercase();
                match x.cmp(&yl) {
                    Ordering::Equal => continue,
                    other => return other,
                }
            }
            (Some(_), None) => return Ordering::Greater,
            (None, Some(_)) => return Ordering::Less,
            (None, None) => return Ordering::Equal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Count check pins the table size — any future addition /
    /// removal updates this number deliberately.
    #[test]
    fn table_has_148_entries() {
        assert_eq!(NAMED.len(), 148);
    }

    /// Sorted check — required for binary search correctness. If
    /// this fails, the table was edited but not re-sorted.
    #[test]
    fn table_is_sorted_alphabetically() {
        for window in NAMED.windows(2) {
            assert!(
                window[0].0 < window[1].0,
                "table not sorted: {:?} should come before {:?}",
                window[0].0,
                window[1].0
            );
        }
    }

    #[test]
    fn lookup_canonical_lowercase() {
        assert_eq!(lookup("dodgerblue"), Some(DODGERBLUE));
        assert_eq!(lookup("rebeccapurple"), Some(REBECCAPURPLE));
        assert_eq!(lookup("aliceblue"), Some(ALICEBLUE));
        assert_eq!(lookup("yellowgreen"), Some(YELLOWGREEN));
    }

    #[test]
    fn lookup_is_case_insensitive() {
        assert_eq!(lookup("DodgerBlue"), Some(DODGERBLUE));
        assert_eq!(lookup("DODGERBLUE"), Some(DODGERBLUE));
        assert_eq!(lookup("RebeccaPurple"), Some(REBECCAPURPLE));
        assert_eq!(lookup("REBECCAPURPLE"), Some(REBECCAPURPLE));
    }

    #[test]
    fn lookup_returns_none_for_unknown() {
        assert_eq!(lookup("notacolor"), None);
        assert_eq!(lookup(""), None);
        assert_eq!(lookup("bluish"), None);
    }

    /// `transparent` is intentionally NOT in this table — the parser
    /// layer maps it to `Color::Reset`. Documenting the negative
    /// here so a future change that adds it here doesn't slip past
    /// review.
    #[test]
    fn transparent_keyword_not_in_table() {
        assert_eq!(lookup("transparent"), None);
        assert_eq!(lookup("Transparent"), None);
    }

    #[test]
    fn css1_keywords_resolve() {
        // Sanity spot-check on the original 16.
        assert_eq!(lookup("black"), Some(Color::Rgb(0, 0, 0)));
        assert_eq!(lookup("white"), Some(Color::Rgb(255, 255, 255)));
        assert_eq!(lookup("aqua"), Some(Color::Rgb(0, 255, 255)));
    }

    #[test]
    fn spelling_pairs_resolve_to_same_rgb() {
        // gray ↔ grey, darkgray ↔ darkgrey, etc.
        assert_eq!(lookup("gray"), lookup("grey"));
        assert_eq!(lookup("darkgray"), lookup("darkgrey"));
        assert_eq!(lookup("dimgray"), lookup("dimgrey"));
        assert_eq!(lookup("lightgray"), lookup("lightgrey"));
        assert_eq!(lookup("slategray"), lookup("slategrey"));
        assert_eq!(lookup("darkslategray"), lookup("darkslategrey"));
        assert_eq!(lookup("lightslategray"), lookup("lightslategrey"));
    }

    #[test]
    fn hex_aliases_resolve_to_same_rgb() {
        // aqua == cyan (#00ffff), fuchsia == magenta (#ff00ff).
        assert_eq!(lookup("aqua"), lookup("cyan"));
        assert_eq!(lookup("fuchsia"), lookup("magenta"));
    }
}
