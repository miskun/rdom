//! Dispatch tests for the list properties (CSS Lists 3 §3; C10-LIST-ITEM):
//! `list-style-type`, `list-style-position`, `list-style-image`, the
//! `list-style` shorthand and `marker-side`.

use super::*;
use crate::TuiStyle;

/// Set `name: css` on a fresh style; its serialization back.
fn round_trip(name: &str, css: &str) -> Result<String, DispatchError> {
    let mut style = TuiStyle::new();
    set(name, css, &mut style)?;
    Ok(serialize(name, &style).expect("serializes"))
}

/// §3.4: `list-style-type: <counter-style> | <string> | none`; §3.5:
/// `list-style-position: inside | outside`; §3.3: `list-style-image:
/// <image> | none` (kept, inert); CSS Lists 3 §3.6: `marker-side:
/// match-self | match-parent`. All inherited.
#[test]
fn the_list_longhands_take_their_grammar() {
    for (name, css, out) in [
        ("list-style-type", "disc", "disc"),
        ("list-style-type", "Upper-Roman", "upper-roman"),
        ("list-style-type", "thumbs", "thumbs"),
        ("list-style-type", r#""→ ""#, r#""→ ""#),
        ("list-style-type", "none", "none"),
        (
            "list-style-type",
            r#"symbols(cyclic "*")"#,
            r#"symbols(cyclic "*")"#,
        ),
        ("list-style-position", "INSIDE", "inside"),
        ("list-style-position", "outside", "outside"),
        ("list-style-image", "none", "none"),
        ("list-style-image", "url(dot.png)", r#"url("dot.png")"#),
        ("marker-side", "match-parent", "match-parent"),
        ("marker-side", "match-self", "match-self"),
    ] {
        assert_eq!(round_trip(name, css).as_deref(), Ok(out), "{name}: {css}");
        assert!(inherits(name), "{name} inherits");
    }
    for (name, css) in [
        ("list-style-type", "inherit none"),
        ("list-style-type", r#""a" "b""#),
        ("list-style-type", "default"),
        ("list-style-position", "left"),
        ("list-style-image", "dot"),
        ("marker-side", "left"),
    ] {
        assert!(
            round_trip(name, css).is_err(),
            "{name}: {css} must be rejected"
        );
    }
}

/// §3.6: `list-style: <'list-style-position'> || <'list-style-image'> ||
/// <'list-style-type'>`, any order; `none` sets whichever of image and
/// type no other value set (both when neither did); an omitted longhand
/// is reset to its initial value.
#[test]
fn the_list_style_shorthand() {
    let longhands = |css: &str| {
        let mut style = TuiStyle::new();
        set("list-style", css, &mut style).unwrap_or_else(|e| panic!("{css}: {e:?}"));
        (
            serialize("list-style-position", &style).unwrap(),
            serialize("list-style-image", &style).unwrap(),
            serialize("list-style-type", &style).unwrap(),
        )
    };
    let t = |a: &str, b: &str, c: &str| (a.to_string(), b.to_string(), c.to_string());
    assert_eq!(longhands("square inside"), t("inside", "none", "square"));
    assert_eq!(longhands("inside square"), t("inside", "none", "square"));
    assert_eq!(longhands("none"), t("outside", "none", "none"));
    assert_eq!(longhands("none square"), t("outside", "none", "square"));
    assert_eq!(
        longhands("url(a.png) none"),
        t("outside", r#"url("a.png")"#, "none")
    );
    assert_eq!(longhands("inside"), t("inside", "none", "disc"));
    assert_eq!(longhands(r#""- " inside"#), t("inside", "none", r#""- ""#));
    for css in [
        "none none none",
        "inside outside",
        "square disc",
        "url(a) url(b)",
    ] {
        assert!(
            round_trip("list-style", css).is_err(),
            "{css} must be rejected"
        );
    }
    assert_eq!(
        round_trip("list-style", "square inside").as_deref(),
        Ok("inside square")
    );
    assert_eq!(round_trip("list-style", "none").as_deref(), Ok("none"));
    assert_eq!(round_trip("list-style", "disc").as_deref(), Ok("disc"));
}
