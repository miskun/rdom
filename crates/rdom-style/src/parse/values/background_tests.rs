//! Tests for the background grammars (CSS Backgrounds 3 §3).

use super::*;
use crate::parse::token::tokenize;

fn toks(s: &str) -> Vec<Token> {
    tokenize(s).expect("tokenizes")
}

fn position(s: &str) -> Option<Vec<String>> {
    parse_background_position(&toks(s))
}

// ── <bg-position> (§3.6) ───────────────────────────────────────────

/// One to four values: a lone keyword or offset, an x / y pair, the
/// keyword pair in either order, and side keywords with offsets.
#[test]
fn position_accepts_the_values_grammar_forms() {
    for ok in [
        "center",
        "top",
        "10%",
        "0 0",
        "left top",
        "top left",
        "center bottom",
        "10% 3",
        "-2 50%",
        "left 10% top",
        "right 2 bottom 10%",
        "center top 1",
        "bottom 1 right",
    ] {
        assert!(position(ok).is_some(), "{ok} should parse");
    }
}

/// Two keywords of one axis, an offset after a vertical keyword in the
/// two-value form, and offsets without a side keyword in the long form
/// are invalid.
#[test]
fn position_rejects_ill_formed_values() {
    for bad in [
        "left right",
        "top bottom",
        "top 10%",
        "left 1 right 2",
        "1 2 3",
        "center center 1",
        "left 1 2",
        "red",
    ] {
        assert!(position(bad).is_none(), "{bad} should be rejected");
    }
}

// ── <bg-size> (§3.9) ───────────────────────────────────────────────

#[test]
fn size_grammar() {
    for ok in ["cover", "contain", "auto", "auto 50%", "10 2", "100%"] {
        assert!(parse_background_size(&toks(ok)).is_some(), "{ok}");
    }
    for bad in ["cover contain", "-1", "auto auto auto", "left"] {
        assert!(parse_background_size(&toks(bad)).is_none(), "{bad}");
    }
}

// ── <repeat-style> (§3.4) ──────────────────────────────────────────

#[test]
fn repeat_grammar() {
    let r = |s: &str| parse_background_repeat(&toks(s));
    assert_eq!(
        r("repeat-x"),
        Some(vec![BackgroundRepeat {
            x: RepeatStyle::Repeat,
            y: RepeatStyle::NoRepeat
        }])
    );
    assert_eq!(
        r("space round"),
        Some(vec![BackgroundRepeat {
            x: RepeatStyle::Space,
            y: RepeatStyle::Round
        }])
    );
    assert_eq!(
        r("no-repeat"),
        Some(vec![BackgroundRepeat {
            x: RepeatStyle::NoRepeat,
            y: RepeatStyle::NoRepeat
        }])
    );
    assert!(r("repeat-x repeat").is_none());
    assert!(r("repeat repeat repeat").is_none());
}

// ── <bg-image> (§3.3) ──────────────────────────────────────────────

/// `url()` serializes with a string argument, quoted or not as written
/// (CSSOM §6.7.2); gradients are kept; other functions are not images.
#[test]
fn image_grammar() {
    let i = |s: &str| parse_background_image(&toks(s));
    assert_eq!(
        i("url(a/b.png)"),
        Some(vec!["url(\"a/b.png\")".to_string()])
    );
    assert_eq!(
        i("url('x y.png')"),
        Some(vec!["url(\"x y.png\")".to_string()])
    );
    assert_eq!(i("none, none").map(|v| v.len()), Some(2));
    assert!(i("linear-gradient(red, blue)").is_some());
    assert!(i("rgb(1, 2, 3)").is_none());
    assert!(i("url(a.png) url(b.png)").is_none());
}

// ── `background` (§3.10) ───────────────────────────────────────────

/// A layer's sub-values in any order; omitted ones at their initial
/// values.
#[test]
fn shorthand_layer_in_any_order() {
    let b = parse_background(&toks("no-repeat red fixed url(a.png) center / cover")).unwrap();
    assert_eq!(b.color, parse_color(&toks("red")));
    let l = &b.layers[0];
    assert_eq!(l.image, "url(\"a.png\")");
    assert_eq!(l.position, "center");
    assert_eq!(l.size, "cover");
    assert_eq!(l.repeat.x, RepeatStyle::NoRepeat);
    assert_eq!(l.attachment, BackgroundAttachment::Fixed);
    assert_eq!((l.origin, l.clip), (INITIAL_ORIGIN, INITIAL_CLIP));
}

/// `/ <bg-size>` needs a position before it; a sub-value given twice is
/// invalid; the color belongs to the final layer only.
#[test]
fn shorthand_rejects_ill_formed_layers() {
    for bad in [
        "/ cover",
        "red blue",
        "url(a.png) url(b.png)",
        "fixed scroll",
        "red, url(a.png)",
        "url(a.png),",
        "border-box padding-box content-box",
        "center / left",
    ] {
        assert!(parse_background(&toks(bad)).is_none(), "{bad}");
    }
}
