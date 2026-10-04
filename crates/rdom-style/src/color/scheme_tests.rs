//! `color-scheme` (CSS Color Adjust 1 §2) and the terminal-background
//! scheme rule.

use super::*;
use crate::parse::tokenize;

fn list(css: &str) -> Option<ColorSchemeList> {
    ColorSchemeList::parse(&tokenize(css).unwrap())
}

/// §2: `normal | [ light | dark | <custom-ident> ]+ && only?`,
/// serialized as written (keywords lower case).
#[test]
fn color_scheme_grammar() {
    for (css, out) in [
        ("normal", "normal"),
        ("light", "light"),
        ("DARK", "dark"),
        ("light dark", "light dark"),
        ("dark light only", "dark light only"),
        ("only light", "light only"),
        ("light Brand", "light Brand"),
    ] {
        assert_eq!(list(css).map(|l| l.to_css()).as_deref(), Some(out), "{css}");
    }
    for bad in [
        "only",
        "normal light",
        "light only only",
        "light, dark",
        "12",
        "",
    ] {
        assert_eq!(list(bad), None, "{bad}");
    }
}

/// §2.1: the used scheme is the preferred one when the element
/// supports it (or says `normal`), else the first it supports.
#[test]
fn used_color_scheme() {
    use ColorScheme::{Dark, Light};
    let used = |css: &str, preferred| list(css).unwrap().used(preferred);
    assert_eq!(used("normal", Light), Light);
    assert_eq!(used("normal", Dark), Dark);
    assert_eq!(used("light", Dark), Light);
    assert_eq!(used("dark", Light), Dark);
    assert_eq!(used("light dark", Dark), Dark);
    assert_eq!(used("dark light", Light), Light);
    assert_eq!(used("brand", Light), Light);
    assert_eq!(used("brand dark", Light), Dark);
}

/// The terminal's background picks the scheme: light when black text
/// on it has more contrast than white text (WCAG 2 contrast).
#[test]
fn scheme_for_a_background() {
    use ColorScheme::{Dark, Light};
    assert_eq!(ColorScheme::for_background(Color::Rgb(0, 0, 0)), Dark);
    assert_eq!(
        ColorScheme::for_background(Color::Rgb(255, 255, 255)),
        Light
    );
    assert_eq!(
        ColorScheme::for_background(Color::Rgb(0x00, 0x2b, 0x36)),
        Dark
    );
    assert_eq!(
        ColorScheme::for_background(Color::Rgb(0xfd, 0xf6, 0xe3)),
        Light
    );
    assert_eq!(
        ColorScheme::for_background(Color::Rgb(0x30, 0x30, 0x30)),
        Dark
    );
    assert_eq!(ColorScheme::default(), Dark);
}
