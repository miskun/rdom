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
