//! Dispatch tests for the backgrounds (CSS Backgrounds 3 §3): the
//! `background` shorthand and its longhands — set, serialize, the
//! shorthand's reset of omitted sub-values, and `!important` routing.

use super::*;
use crate::layout::{BackgroundAttachment, BackgroundRepeat, RepeatStyle, VisualBox};
use crate::{Color, TuiColor, TuiStyle, Value};

fn specified<T: Clone>(v: &Option<Value<T>>) -> T {
    match v {
        Some(Value::Specified(x)) => x.clone(),
        other => panic!("not specified: {}", other.is_some()),
    }
}

/// Backgrounds 3 §3.10: `background: url(x.png) red` is an image layer
/// with the color red — the image parses and is stored; the color is
/// `background-color`.
#[test]
fn background_shorthand_takes_an_image_and_a_color() {
    let mut style = TuiStyle::new();
    set("background", "url(x.png) red", &mut style).unwrap();
    assert_eq!(
        specified(&style.bg),
        TuiColor::Literal(Color::Rgb(255, 0, 0))
    );
    assert_eq!(
        specified(&style.background_image),
        vec!["url(\"x.png\")".to_string()]
    );
    assert_eq!(
        serialize("background", &style).as_deref(),
        Some("url(\"x.png\") red")
    );
}

/// §3.10: every sub-value of a layer, in any order; one box sets
/// `background-origin` and `background-clip`, two set them in that
/// order.
#[test]
fn background_shorthand_sets_every_longhand() {
    let mut style = TuiStyle::new();
    set(
        "background",
        "url(a.png) center / cover no-repeat fixed padding-box content-box rgb(1, 2, 3)",
        &mut style,
    )
    .unwrap();
    assert_eq!(specified(&style.background_position), vec!["center"]);
    assert_eq!(specified(&style.background_size), vec!["cover"]);
    assert_eq!(
        specified(&style.background_repeat),
        vec![BackgroundRepeat {
            x: RepeatStyle::NoRepeat,
            y: RepeatStyle::NoRepeat
        }]
    );
    assert_eq!(
        specified(&style.background_attachment),
        vec![BackgroundAttachment::Fixed]
    );
    assert_eq!(
        specified(&style.background_origin),
        vec![VisualBox::PaddingBox]
    );
    assert_eq!(
        specified(&style.background_clip),
        vec![VisualBox::ContentBox]
    );
    let mut one_box = TuiStyle::new();
    set("background", "content-box", &mut one_box).unwrap();
    assert_eq!(
        specified(&one_box.background_origin),
        vec![VisualBox::ContentBox]
    );
    assert_eq!(
        specified(&one_box.background_clip),
        vec![VisualBox::ContentBox]
    );
}

/// §3.10: comma-separated layers; only the final layer may carry the
/// color.
#[test]
fn background_shorthand_layers() {
    let mut style = TuiStyle::new();
    set(
        "background",
        "url(a.png) no-repeat, linear-gradient(red, blue) blue",
        &mut style,
    )
    .unwrap();
    assert_eq!(specified(&style.background_image).len(), 2);
    assert_eq!(specified(&style.background_clip).len(), 2);
    assert_eq!(
        specified(&style.bg),
        TuiColor::Literal(Color::Rgb(0, 0, 255))
    );
    assert_eq!(
        set("background", "red, url(a.png)", &mut TuiStyle::new()),
        Err(DispatchError::InvalidValue)
    );
    assert_eq!(
        set("background", "red blue", &mut TuiStyle::new()),
        Err(DispatchError::InvalidValue)
    );
}

/// §3.10: the shorthand resets every sub-value it omits — `background:
/// red` after `background-image: url(a.png)` leaves no image;
/// `background: none` is no image and a `transparent` color.
#[test]
fn background_shorthand_resets_omitted_sub_values() {
    let mut style = TuiStyle::new();
    set("background-image", "url(a.png)", &mut style).unwrap();
    set("background-clip", "content-box", &mut style).unwrap();
    set("background", "red", &mut style).unwrap();
    assert_eq!(specified(&style.background_image), vec!["none"]);
    assert_eq!(
        specified(&style.background_clip),
        vec![VisualBox::BorderBox]
    );
    assert_eq!(serialize("background", &style).as_deref(), Some("red"));

    let mut none = TuiStyle::new();
    set("background", "none", &mut none).unwrap();
    assert_eq!(specified(&none.bg), TuiColor::Literal(Color::TRANSPARENT));
    assert_eq!(serialize("background", &none).as_deref(), Some("none"));
}

/// The longhands parse their `#` lists and serialize them back; their
/// grammars reject what the spec does.
#[test]
fn background_longhands_parse_and_serialize() {
    for (name, value) in [
        ("background-image", "url(\"a.png\"), none"),
        ("background-position", "left 10% top, center"),
        ("background-size", "auto 50%, cover"),
        ("background-repeat", "repeat-x, space round"),
        ("background-attachment", "scroll, local"),
        ("background-origin", "border-box, content-box"),
        ("background-clip", "padding-box"),
    ] {
        let mut style = TuiStyle::new();
        set(name, value, &mut style).unwrap_or_else(|e| panic!("{name}: {value}: {e:?}"));
        assert_eq!(serialize(name, &style).as_deref(), Some(value), "{name}");
    }
    for (name, bad) in [
        ("background-image", "rgb(1, 2, 3)"),
        ("background-position", "left right"),
        ("background-size", "-1"),
        ("background-repeat", "repeat-x repeat"),
        ("background-attachment", "sticky"),
        ("background-origin", "margin-box"),
        // Backgrounds 4's `text`: a background cannot show through a
        // glyph's shape in a cell (DIVERGENCES §1).
        ("background-clip", "text"),
    ] {
        assert_eq!(
            set(name, bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{name}: {bad}"
        );
    }
}

/// `!important` on the shorthand covers every longhand it sets, and
/// removing it clears them.
#[test]
fn background_shorthand_owns_every_longhand() {
    let all = property_mask("background").unwrap();
    for name in [
        "background-color",
        "background-image",
        "background-position",
        "background-size",
        "background-repeat",
        "background-attachment",
        "background-origin",
        "background-clip",
    ] {
        assert!(all.contains(property_mask(name).unwrap()), "{name}");
    }
    let mut style = TuiStyle::new();
    set("background", "url(a.png) red", &mut style).unwrap();
    assert!(remove("background", &mut style));
    assert!(style.background_image.is_none() && style.bg.is_none());
}

// ── `box-shadow` (§6.1) ────────────────────────────────────────────

/// §6.1: `none`, or shadows front to back — the lengths together (two
/// to four: offsets, a non-negative blur, a spread of any sign), the
/// color and `inset` before or after them; an omitted color is
/// `currentcolor`, an omitted blur or spread 0.
#[test]
fn box_shadow_grammar() {
    use crate::layout::{BoxShadow, PaintLength};
    let mut style = TuiStyle::new();
    set("box-shadow", "red 1 -2px 3 inset, 0 1px", &mut style).unwrap();
    let list = specified(&style.box_shadow);
    assert_eq!(
        list[0],
        BoxShadow {
            inset: true,
            offset_x: PaintLength::Cells(1.0),
            offset_y: PaintLength::Px(-2.0),
            blur: PaintLength::Cells(3.0),
            spread: PaintLength::Cells(0.0),
            color: TuiColor::Literal(Color::Rgb(255, 0, 0)),
        }
    );
    assert_eq!(list[1].color, TuiColor::CurrentColor);
    assert_eq!(
        serialize("box-shadow", &style).as_deref(),
        Some("inset 1 -2px 3 red, 0 1px")
    );
    set("box-shadow", "none", &mut style).unwrap();
    assert_eq!(specified(&style.box_shadow), vec![]);
    assert_eq!(serialize("box-shadow", &style).as_deref(), Some("none"));
    set("box-shadow", "1 1 0 -1 blue", &mut style).unwrap();
    assert_eq!(
        serialize("box-shadow", &style).as_deref(),
        Some("1 1 0 -1 blue")
    );
    for bad in [
        "1",
        "1 2 3 4 5",
        "1 2 -3",
        "1 red 2",
        "inset inset 1 2",
        "1 2 red blue",
        "1 2 10%",
        "none, 1 2",
    ] {
        assert_eq!(
            set("box-shadow", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad}"
        );
    }
}

/// C4G-NUMBER-RANGE — CSS Syntax 3 §4.3.13 with CSS Values 4 §5.1: an
/// integer literal past the implementation's range is clamped, not
/// rejected, so a huge spread parses (and paints clamped, C4G-SHADOW-CLAMP)
/// — clamped where the length consumes it, the token keeping the literal
/// whole (C5G-INT-CLAMP-SITE); a dimension keeps its value.
#[test]
fn box_shadow_takes_an_integer_past_the_range() {
    use crate::layout::PaintLength;
    let mut style = TuiStyle::new();
    set("box-shadow", "0 0 0 9999999999 red", &mut style).unwrap();
    assert_eq!(
        specified(&style.box_shadow)[0].spread,
        PaintLength::Cells(i32::MAX as f32)
    );
    set(
        "box-shadow",
        "0 0 0 99999999999999999999999 red",
        &mut style,
    )
    .unwrap();
    assert_eq!(
        specified(&style.box_shadow)[0].spread,
        PaintLength::Cells(i32::MAX as f32)
    );
    set("box-shadow", "99999999999ch 0 red", &mut style).unwrap();
    let PaintLength::Cells(x) = specified(&style.box_shadow)[0].offset_x else {
        panic!("a cell length");
    };
    assert!(x >= i32::MAX as f32, "{x}");
}

/// C4G-NUMBER-RANGE — one rule for rdom's unitless cell length (DIVERGENCES
/// §1): any `<number>` is a length in cells, a fraction included, exactly
/// as `calc(<number>)` already was — so `1.5` and `calc(1.5)` parse alike
/// in every cell-length property, rounded onto the grid where the property
/// stores whole cells.
#[test]
fn unitless_fractions_are_cell_lengths_everywhere() {
    for (name, value) in [
        ("box-shadow", "1.5 -0.5 red"),
        ("border-radius", "1.5"),
        ("border-width", "1.5"),
        ("width", "1.5"),
        ("min-width", "1.5"),
        ("max-height", "1.5"),
        ("gap", "1.5"),
        ("padding", "1.5 0.5"),
        ("margin", "-1.5"),
        ("top", "-0.5"),
        ("border-spacing", "1.5"),
        ("flex", "1 1 2.5"),
    ] {
        let mut bare = TuiStyle::new();
        set(name, value, &mut bare).unwrap_or_else(|e| panic!("{name}: {value} → {e:?}"));
        let calc = value
            .split(' ')
            .map(|v| match v.strip_prefix('-') {
                Some(n) if !v.contains('.') || name == "flex" => format!("-{n}"),
                Some(n) => format!("calc(-1 * {n})"),
                None if v.contains('.') => format!("calc({v})"),
                None => v.to_string(),
            })
            .collect::<Vec<_>>()
            .join(" ");
        let mut via_calc = TuiStyle::new();
        set(name, &calc, &mut via_calc).unwrap_or_else(|e| panic!("{name}: {calc} → {e:?}"));
        assert_eq!(bare, via_calc, "{name}: {value} vs {calc}");
    }
}
