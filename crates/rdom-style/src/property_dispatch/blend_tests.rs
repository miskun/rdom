//! Dispatch tests for the compositing properties (C15-BLEND; Compositing
//! and Blending 1 §3.2, §3.4, §5.2, Compositing 2 §9): `mix-blend-mode`,
//! `isolation`, `background-blend-mode`.

use super::*;
use crate::TuiStyle;

fn round(name: &str, value: &str) -> Option<String> {
    let mut style = TuiStyle::new();
    set(name, value, &mut style).ok()?;
    serialize(name, &style)
}

const MODES: &[&str] = &[
    "normal",
    "multiply",
    "screen",
    "overlay",
    "darken",
    "lighten",
    "color-dodge",
    "color-burn",
    "hard-light",
    "soft-light",
    "difference",
    "exclusion",
    "hue",
    "saturation",
    "color",
    "luminosity",
];

/// §3.2: `mix-blend-mode: <blend-mode> | plus-darker | plus-lighter`
/// (Compositing 2 adds the two); §5.2: `isolation: auto | isolate`; §3.4:
/// `background-blend-mode: <blend-mode>#`. None inherit.
#[test]
fn compositing_properties_parse() {
    for m in MODES.iter().chain(&["plus-darker", "plus-lighter"]) {
        assert_eq!(round("mix-blend-mode", m).as_deref(), Some(*m));
        assert_eq!(
            round("mix-blend-mode", &m.to_uppercase()).as_deref(),
            Some(*m)
        );
    }
    for bad in ["multiply screen", "add", "", "multiply,"] {
        assert_eq!(round("mix-blend-mode", bad), None, "{bad}");
    }
    for k in ["auto", "isolate"] {
        assert_eq!(round("isolation", k).as_deref(), Some(k));
    }
    assert_eq!(round("isolation", "none"), None);
    for m in MODES {
        assert_eq!(round("background-blend-mode", m).as_deref(), Some(*m));
    }
    assert_eq!(
        round("background-blend-mode", "multiply, Screen").as_deref(),
        Some("multiply, screen")
    );
    for bad in ["plus-lighter", "multiply screen", ""] {
        assert_eq!(round("background-blend-mode", bad), None, "{bad}");
    }
    for name in ["mix-blend-mode", "isolation", "background-blend-mode"] {
        assert!(!inherits(name));
    }
}
