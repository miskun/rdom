//! Tests for the property dispatch table: the set → serialize → set
//! round-trip contract over every property, the field-table coverage
//! invariants, CSS-wide keyword handling, and per-property parsing /
//! serialization regressions.

use super::table::{Field, fields_of};
use super::*;
use crate::{TuiStyle, Value};

/// One canonical value per property — chosen to survive the
/// parser → serialize → parser round trip. The
/// `round_trip_every_property` test below iterates over this.
fn canonical_values() -> &'static [(&'static str, &'static str)] {
    &[
        ("color", "red"),
        ("background-color", "blue"),
        (
            "background",
            "url(\"a.png\") center / cover no-repeat green",
        ),
        ("background-image", "url(\"a.png\"), none"),
        ("background-position", "left 10% top"),
        ("background-size", "auto 50%"),
        ("background-repeat", "repeat-x"),
        ("background-attachment", "fixed"),
        ("background-origin", "content-box"),
        ("background-clip", "padding-box"),
        ("font-weight", "bold"),
        ("font-style", "italic"),
        ("font", "bold 12px serif"),
        ("font-size", "large"),
        ("font-family", "monospace"),
        ("font-stretch", "condensed"),
        ("font-width", "expanded"),
        ("font-variant", "small-caps"),
        ("text-decoration", "underline"),
        ("text-decoration-line", "overline"),
        ("text-decoration-style", "wavy"),
        ("text-decoration-color", "red"),
        ("text-decoration-thickness", "2"),
        ("text-underline-offset", "1"),
        ("text-underline-position", "under"),
        ("text-decoration-skip-ink", "none"),
        ("opacity", "0.5"),
        ("display", "inline"),
        ("flex-direction", "column"),
        ("flex-wrap", "wrap"),
        ("flex-flow", "column wrap"),
        ("justify-content", "safe center"),
        ("align-content", "space-around"),
        ("align-items", "last baseline"),
        ("align-self", "auto"),
        ("justify-items", "legacy center"),
        ("justify-self", "safe end"),
        ("place-content", "center space-between"),
        ("place-items", "end"),
        ("place-self", "center auto"),
        ("white-space", "pre"),
        ("user-select", "text"),
        ("pointer-events", "none"),
        ("visibility", "hidden"),
        ("caret-color", "transparent"),
        ("caret-text-color", "auto"),
        ("caret-shape", "underscore"),
        ("caret-animation", "manual"),
        ("caret", "rgb(1, 2, 3) manual bar"),
        ("accent-color", "rgb(4, 5, 6)"),
        ("appearance", "none"),
        ("-webkit-appearance", "menulist-button"),
        ("field-sizing", "content"),
        ("resize", "vertical"),
        ("overflow", "scroll"),
        ("overflow-x", "auto"),
        ("overflow-y", "hidden"),
        ("overflow-clip-margin", "content-box 2"),
        ("text-overflow", "ellipsis \">\""),
        ("line-clamp", "2"),
        ("max-lines", "3"),
        ("block-ellipsis", "auto"),
        ("continue", "collapse"),
        ("-webkit-line-clamp", "2"),
        ("-webkit-box-orient", "vertical"),
        ("scrollbar-gutter", "stable both-edges"),
        ("scrollbar-width", "thin"),
        ("scrollbar-color", "rgb(16, 32, 48) rgb(64, 80, 96)"),
        ("outline", "red dashed thick"),
        ("outline-style", "auto"),
        ("outline-width", "2px"),
        ("outline-color", "rgb(1, 2, 3)"),
        ("outline-offset", "-1"),
        ("cursor", "url(\"a.cur\") 2 3, url(\"b.png\"), pointer"),
        ("overscroll-behavior", "contain none"),
        ("overscroll-behavior-x", "none"),
        ("overscroll-behavior-y", "contain"),
        ("scroll-padding", "1 auto 10% 2"),
        ("scroll-padding-top", "1"),
        ("scroll-padding-right", "auto"),
        ("scroll-padding-bottom", "10%"),
        ("scroll-padding-left", "2"),
        ("scroll-margin", "1 -2"),
        ("scroll-margin-top", "1"),
        ("scroll-margin-right", "-2"),
        ("scroll-margin-bottom", "3"),
        ("scroll-margin-left", "4"),
        ("scroll-snap-type", "y mandatory"),
        ("scroll-snap-align", "start end"),
        ("scroll-snap-stop", "always"),
        ("scroll-behavior", "smooth"),
        ("width", "40"),
        ("height", "auto"),
        ("min-width", "10"),
        ("max-width", "100"),
        ("min-height", "5"),
        ("max-height", "50"),
        ("aspect-ratio", "16/9"),
        ("box-sizing", "border-box"),
        ("interpolate-size", "allow-keywords"),
        ("contain-intrinsic-size", "auto 10 none"),
        ("contain", "layout paint"),
        ("content-visibility", "auto"),
        ("will-change", "opacity, scroll-position"),
        ("translate", "1 50%"),
        ("rotate", "x 45deg"),
        ("scale", "2 0.5"),
        ("transform", "translateX(2) rotate(10deg)"),
        ("transform-origin", "0% 2"),
        ("transform-box", "content-box"),
        ("filter", "grayscale(0.5) drop-shadow(1 2 rgb(1, 2, 3))"),
        ("backdrop-filter", "invert(1) url(\"#f\")"),
        ("mix-blend-mode", "multiply"),
        ("isolation", "isolate"),
        ("background-blend-mode", "screen, difference"),
        ("clip-path", "inset(1 2 round 1) padding-box"),
        (
            "mask",
            "url(\"m.svg\") center / contain no-repeat border-box border-box add alpha",
        ),
        ("mask-image", "url(\"m.svg\"), none"),
        ("mask-mode", "alpha"),
        ("mask-repeat", "no-repeat"),
        ("mask-position", "center"),
        ("mask-clip", "no-clip"),
        ("mask-origin", "content-box"),
        ("mask-size", "contain"),
        ("mask-composite", "exclude"),
        ("mask-type", "alpha"),
        ("mask-border", "url(\"b.svg\") 30 / 1 / 0 round alpha"),
        ("mask-border-source", "none"),
        ("mask-border-slice", "30 fill"),
        ("mask-border-width", "1 auto"),
        ("mask-border-outset", "1"),
        ("mask-border-repeat", "round"),
        ("mask-border-mode", "luminance"),
        ("container", "card / size"),
        ("container-name", "a b"),
        ("container-type", "inline-size"),
        ("contain-intrinsic-width", "4"),
        ("contain-intrinsic-height", "auto 2"),
        ("contain-intrinsic-inline-size", "none"),
        ("contain-intrinsic-block-size", "auto none"),
        ("gap", "2"),
        ("row-gap", "1"),
        ("column-gap", "normal"),
        ("flex", "1"),
        ("flex-grow", "2"),
        ("flex-shrink", "1"),
        ("flex-basis", "content"),
        ("order", "-2"),
        ("grid", "auto-flow dense 1 / 2"),
        ("grid-template", "[a] \"x y\" 1 / 2 3"),
        (
            "grid-template-columns",
            "[a] 10 repeat(2, minmax(auto, 1fr)) [b]",
        ),
        ("grid-template-rows", "none"),
        ("grid-template-areas", "\"a b\" \". c\""),
        ("grid-auto-columns", "minmax(2, 1fr) auto"),
        ("grid-auto-rows", "fit-content(4)"),
        ("grid-auto-flow", "column dense"),
        ("grid-row-start", "span 2 a"),
        ("grid-row-end", "-1"),
        ("grid-column-start", "a"),
        ("grid-column-end", "2 b"),
        ("grid-row", "1 / span 2"),
        ("grid-column", "x"),
        ("grid-area", "a / 2 / auto / span 3"),
        ("padding", "1 2 3 4"),
        ("padding-top", "5"),
        ("padding-right", "6"),
        ("padding-bottom", "7"),
        ("padding-left", "8"),
        ("margin", "1 2 3 auto"),
        ("margin-top", "1"),
        ("margin-right", "2"),
        ("margin-bottom", "3"),
        ("margin-left", "auto"),
        ("margin-trim", "block-start inline-end"),
        ("border", "solid"),
        ("border-top", "solid"),
        ("border-right", "solid"),
        ("border-bottom", "solid"),
        ("border-left", "solid"),
        ("border-style", "dashed"),
        ("border-top-style", "double"),
        ("border-right-style", "hidden"),
        ("border-bottom-style", "dotted"),
        ("border-left-style", "ridge"),
        ("border-color", "rgb(10, 20, 30) red"),
        ("border-top-color", "red"),
        ("border-right-color", "blue"),
        ("border-bottom-color", "currentcolor"),
        ("border-left-color", "transparent"),
        ("border-width", "thin 2px"),
        ("border-top-width", "thick"),
        ("border-right-width", "1.5ch"),
        ("border-bottom-width", "0"),
        ("border-left-width", "calc(1 + 1)"),
        ("border-radius", "1 4px / 50%"),
        ("border-top-left-radius", "2"),
        ("border-top-right-radius", "1 3"),
        ("border-bottom-right-radius", "10%"),
        ("border-bottom-left-radius", "0"),
        ("box-shadow", "inset 1 2px 3 -1 red, 0 1"),
        ("border-collapse", "collapse"),
        ("border-spacing", "1 2"),
        ("table-layout", "fixed"),
        ("caption-side", "bottom"),
        ("empty-cells", "hide"),
        ("content", "\"hello\""),
        ("quotes", "\"<\" \">\""),
        ("list-style", "inside square"),
        ("list-style-type", "lower-roman"),
        ("list-style-position", "inside"),
        ("list-style-image", "url(\"a.png\")"),
        ("marker-side", "match-parent"),
        ("position", "absolute"),
        ("top", "10"),
        ("right", "20"),
        ("bottom", "auto"),
        ("left", "5"),
        ("z-index", "3"),
        ("overlay", "auto"),
        ("float", "inline-end"),
        ("clear", "both"),
        ("inset", "1 2 3 4"),
        ("transition-property", "color"),
        ("transition-duration", "200ms"),
        ("transition-timing-function", "ease-in-out"),
        ("transition-delay", "50ms"),
        ("transition-behavior", "allow-discrete"),
        ("transition", "width 300ms ease 0ms"),
        ("animation-name", "slide"),
        ("animation-duration", "200ms"),
        ("animation-timing-function", "linear"),
        ("animation-delay", "-50ms"),
        ("animation-iteration-count", "infinite"),
        ("animation-direction", "alternate"),
        ("animation-fill-mode", "both"),
        ("animation-play-state", "paused"),
        ("animation-composition", "add"),
        ("animation-timeline", "none"),
        ("animation", "300ms ease 0ms 1 normal none running slide"),
        ("scroll-timeline-name", "--s"),
        ("scroll-timeline-axis", "x"),
        ("view-timeline-name", "--v"),
        ("view-timeline-axis", "inline"),
        ("view-timeline-inset", "1 auto"),
        ("scroll-timeline", "--s x"),
        ("view-timeline", "--v block auto"),
        ("timeline-scope", "--s"),
        ("animation-range-start", "entry 10%"),
        ("animation-range-end", "exit 90%"),
        ("animation-range", "entry 0% exit 100%"),
        ("counter-reset", "chapter 0"),
        ("counter-increment", "chapter 1"),
        ("counter-set", "chapter 2"),
        ("color-scheme", "light dark"),
        ("white-space-collapse", "preserve-breaks"),
        ("text-wrap-mode", "nowrap"),
        ("word-break", "keep-all"),
        ("overflow-wrap", "anywhere"),
        ("word-wrap", "break-word"),
        ("line-break", "strict"),
        ("hyphens", "none"),
        ("tab-size", "4"),
        ("text-transform", "uppercase full-width"),
        ("text-indent", "-2 hanging"),
        ("text-align", "justify-all"),
        ("text-align-all", "center"),
        ("text-align-last", "right"),
        ("text-justify", "inter-character"),
        ("text-wrap", "nowrap balance"),
        ("text-wrap-style", "pretty"),
        ("letter-spacing", "2ch"),
        ("word-spacing", "1ch"),
        ("line-height", "2"),
        ("vertical-align", "middle"),
        ("direction", "rtl"),
        ("writing-mode", "vertical-rl"),
        ("inline-size", "6"),
        ("block-size", "auto"),
        ("min-inline-size", "1"),
        ("min-block-size", "2"),
        ("max-inline-size", "10"),
        ("max-block-size", "none"),
        ("overflow-block", "clip"),
        ("overflow-inline", "auto"),
        ("overscroll-behavior-block", "none"),
        ("overscroll-behavior-inline", "contain"),
        ("scroll-padding-block-start", "1"),
        ("scroll-padding-block-end", "auto"),
        ("scroll-padding-block", "1 2"),
        ("scroll-padding-inline-start", "3"),
        ("scroll-padding-inline-end", "4"),
        ("scroll-padding-inline", "5 6"),
        ("scroll-margin-block-start", "1"),
        ("scroll-margin-block-end", "-1"),
        ("scroll-margin-block", "1 2"),
        ("scroll-margin-inline-start", "3"),
        ("scroll-margin-inline-end", "4"),
        ("scroll-margin-inline", "5 6"),
        ("margin-block-start", "1"),
        ("margin-block-end", "auto"),
        ("margin-block", "1 2"),
        ("margin-inline-start", "3"),
        ("margin-inline-end", "-1"),
        ("margin-inline", "1 auto"),
        ("padding-block-start", "1"),
        ("padding-block-end", "2"),
        ("padding-block", "1 3"),
        ("padding-inline-start", "2"),
        ("padding-inline-end", "1"),
        ("padding-inline", "1 2"),
        ("inset-block-start", "1"),
        ("inset-block-end", "auto"),
        ("inset-block", "1 2"),
        ("inset-inline-start", "4"),
        ("inset-inline-end", "auto"),
        ("inset-inline", "1 2"),
        ("border-block-start", "solid"),
        ("border-block-end", "dashed red"),
        ("border-block", "double"),
        ("border-inline-start", "solid"),
        ("border-inline-end", "thick dotted blue"),
        ("border-inline", "solid"),
        ("border-block-start-color", "red"),
        ("border-block-start-style", "solid"),
        ("border-block-start-width", "thick"),
        ("border-block-end-color", "blue"),
        ("border-block-end-style", "double"),
        ("border-block-end-width", "thin"),
        ("border-block-color", "red blue"),
        ("border-block-style", "solid"),
        ("border-block-width", "thin thick"),
        ("border-inline-start-color", "red"),
        ("border-inline-start-style", "solid"),
        ("border-inline-start-width", "thick"),
        ("border-inline-end-color", "blue"),
        ("border-inline-end-style", "dashed"),
        ("border-inline-end-width", "0"),
        ("border-inline-color", "red blue"),
        ("border-inline-style", "solid double"),
        ("border-inline-width", "thin"),
        ("border-start-start-radius", "1"),
        ("border-start-end-radius", "2"),
        ("border-end-start-radius", "10%"),
        ("border-end-end-radius", "1 2"),
    ]
}

#[test]
fn property_names_matches_canonical_values_table() {
    // Sanity: every name in `property_names()` has a canonical
    // value, and vice versa.
    let names: Vec<&str> = property_names().to_vec();
    let canon: Vec<&str> = canonical_values().iter().map(|(n, _)| *n).collect();
    assert_eq!(
        names, canon,
        "property_names() and canonical_values() must enumerate the same set in the same order"
    );
}

#[test]
fn border_collapse_parses_both_keywords() {
    use crate::layout::BorderCollapse;
    let mut style = TuiStyle::new();
    set("border-collapse", "separate", &mut style).expect("separate parses");
    assert_eq!(
        style.border_collapse,
        Some(Value::Specified(BorderCollapse::Separate))
    );
    set("border-collapse", "collapse", &mut style).expect("collapse parses");
    assert_eq!(
        style.border_collapse,
        Some(Value::Specified(BorderCollapse::Collapse))
    );
}

#[test]
fn border_collapse_serializes_roundtrip() {
    let mut style = TuiStyle::new();
    set("border-collapse", "collapse", &mut style).unwrap();
    assert_eq!(
        serialize("border-collapse", &style).as_deref(),
        Some("collapse")
    );
    set("border-collapse", "separate", &mut style).unwrap();
    assert_eq!(
        serialize("border-collapse", &style).as_deref(),
        Some("separate")
    );
}

#[test]
fn margin_shorthand_one_value_applies_to_all_sides() {
    use crate::layout::{Margin, MarginValue};
    let mut style = TuiStyle::new();
    set("margin", "5", &mut style).expect("1-value shorthand parses");
    assert_eq!(
        style.margin,
        specified_sides(Margin {
            top: MarginValue::Cells(5),
            right: MarginValue::Cells(5),
            bottom: MarginValue::Cells(5),
            left: MarginValue::Cells(5),
        })
    );
}

#[test]
fn margin_shorthand_two_values_split_vertical_horizontal() {
    use crate::layout::{Margin, MarginValue};
    let mut style = TuiStyle::new();
    set("margin", "1 2", &mut style).expect("2-value shorthand parses");
    assert_eq!(
        style.margin,
        specified_sides(Margin {
            top: MarginValue::Cells(1),
            right: MarginValue::Cells(2),
            bottom: MarginValue::Cells(1),
            left: MarginValue::Cells(2),
        })
    );
}

#[test]
fn margin_shorthand_three_values_top_horiz_bottom() {
    use crate::layout::{Margin, MarginValue};
    let mut style = TuiStyle::new();
    set("margin", "1 2 3", &mut style).expect("3-value shorthand parses");
    assert_eq!(
        style.margin,
        specified_sides(Margin {
            top: MarginValue::Cells(1),
            right: MarginValue::Cells(2),
            bottom: MarginValue::Cells(3),
            left: MarginValue::Cells(2),
        })
    );
}

#[test]
fn margin_shorthand_four_values_each_side() {
    use crate::layout::{Margin, MarginValue};
    let mut style = TuiStyle::new();
    set("margin", "1 2 3 4", &mut style).expect("4-value shorthand parses");
    assert_eq!(
        style.margin,
        specified_sides(Margin {
            top: MarginValue::Cells(1),
            right: MarginValue::Cells(2),
            bottom: MarginValue::Cells(3),
            left: MarginValue::Cells(4),
        })
    );
}

#[test]
fn margin_accepts_negative_values() {
    use crate::layout::Margin;
    let mut style = TuiStyle::new();
    set("margin", "-5", &mut style).expect("negative values parse");
    assert_eq!(style.margin, specified_sides(Margin::all_cells(-5)));
}

#[test]
fn margin_auto_keyword_parses() {
    use crate::layout::{Margin, MarginValue};
    let mut style = TuiStyle::new();
    // `0 auto`: top/bottom = 0, left/right = auto. Classic
    // horizontal centering for block-level boxes — semantic
    // wired in M5.3b.
    set("margin", "0 auto", &mut style).expect("0 auto parses");
    assert_eq!(
        style.margin,
        specified_sides(Margin {
            top: MarginValue::Cells(0),
            right: MarginValue::Auto,
            bottom: MarginValue::Cells(0),
            left: MarginValue::Auto,
        })
    );
}

#[test]
fn margin_longhand_combines_with_previous_shorthand() {
    // Setting a longhand after a shorthand updates just that side.
    use crate::layout::{Margin, MarginValue};
    let mut style = TuiStyle::new();
    set("margin", "5", &mut style).unwrap();
    set("margin-top", "10", &mut style).unwrap();
    assert_eq!(
        style.margin,
        specified_sides(Margin {
            top: MarginValue::Cells(10),
            right: MarginValue::Cells(5),
            bottom: MarginValue::Cells(5),
            left: MarginValue::Cells(5),
        })
    );
}

#[test]
fn min_width_auto_parses_and_round_trips() {
    // M5.1.b: `min-width: auto` is the CSS keyword that opts a flex
    // item into intrinsic min-content protection. The dispatch
    // accepts it both directions of the round trip.
    let mut style = TuiStyle::new();
    set("min-width", "auto", &mut style).expect("auto parses");
    assert_eq!(serialize("min-width", &style).as_deref(), Some("auto"));

    let mut style = TuiStyle::new();
    set("min-height", "auto", &mut style).expect("auto parses");
    assert_eq!(serialize("min-height", &style).as_deref(), Some("auto"));
}

#[test]
fn min_width_numeric_still_round_trips_after_auto_support() {
    // Regression: adding `auto` must not break the existing
    // numeric path that M5.1.a shipped.
    let mut style = TuiStyle::new();
    set("min-width", "10", &mut style).expect("numeric parses");
    assert_eq!(serialize("min-width", &style).as_deref(), Some("10"));
}

#[test]
fn set_unknown_property_errs() {
    let mut style = TuiStyle::new();
    assert_eq!(
        set("not-a-property", "x", &mut style),
        Err(DispatchError::UnknownProperty)
    );
}

#[test]
fn set_invalid_value_errs() {
    let mut style = TuiStyle::new();
    assert_eq!(
        set("color", "definitely-not-a-color", &mut style),
        Err(DispatchError::InvalidValue)
    );
}

#[test]
fn serialize_unset_property_is_none() {
    let style = TuiStyle::new();
    for (name, _) in canonical_values() {
        assert!(
            serialize(name, &style).is_none(),
            "serialize({name}, unset) should be None"
        );
    }
}

#[test]
fn serialize_unknown_property_is_none() {
    let style = TuiStyle::new();
    assert!(serialize("bogus", &style).is_none());
}

/// CALC-PADMARG-1 closing test. Percent-bearing padding/margin
/// `calc()` declarations must survive set → serialize → set.
/// The canonical_values table doesn't cover this directly because
/// it pairs each property with one canonical form; calc-bearing
/// values are an additional shape over the same property.
#[test]
fn padding_and_margin_calc_round_trip() {
    let cases = [
        ("padding-top", "calc(50% + 1)"),
        ("padding-left", "calc(25% - 2)"),
        ("margin-right", "calc(10% + 3)"),
        ("margin-bottom", "calc(100% / 2)"),
    ];
    for (name, value) in cases {
        let mut style_a = TuiStyle::new();
        set(name, value, &mut style_a)
            .unwrap_or_else(|e| panic!("first set({name:?}, {value:?}) errored: {e:?}"));
        let serialized = serialize(name, &style_a)
            .unwrap_or_else(|| panic!("serialize({name:?}) returned None"));
        let mut style_b = TuiStyle::new();
        set(name, &serialized, &mut style_b)
            .unwrap_or_else(|e| panic!("round-trip set({name:?}, {serialized:?}) errored: {e:?}",));
        assert_eq!(
            style_a, style_b,
            "{name}: calc round-trip diverged. original={value:?}, serialized={serialized:?}"
        );
    }
}

// ── HARDENING-2026-09 Batch 2 ────────────────────────────────────

/// CSS-wide keywords (CSS Cascade 4 §7): `inherit` and `initial`
/// are valid for every property; `unset` is `inherit` for inherited
/// properties and `initial` otherwise.
/// `STYLE-PROPERTY-TABLES-1`: the property → field table is total
/// over the property list, every field is reachable from some
/// property, and every `ImportantMask` bit is owned by a field.
#[test]
fn field_table_covers_every_property_field_and_mask_bit() {
    for name in property_names() {
        assert!(fields_of(name).is_some(), "{name} has no fields");
        assert!(property_mask(name).is_some(), "{name} has no mask");
    }
    assert!(fields_of("bogus").is_none());
    for field in Field::ALL {
        assert!(
            property_names()
                .iter()
                .any(|n| fields_of(n).unwrap().contains(field)),
            "{field:?} is not owned by any property"
        );
    }
    let owned = Field::ALL
        .iter()
        .fold(crate::ImportantMask::empty(), |m, f| m | f.mask());
    assert_eq!(owned, crate::ImportantMask::all());
}

/// C4G-IMPORTANT-BITSET — CSS Cascade 4 §6.4: importance belongs to
/// each declaration, so every longhand field the table dispatches to
/// owns its own `!important` bit, distinct from every other field's —
/// `transition-duration: 1s !important` must not make
/// `transition-delay` important. The bit is the field's row in the
/// table, so a new property gets one without a hand-picked number.
#[test]
fn every_dispatched_field_has_a_distinct_important_bit() {
    for (i, a) in Field::ALL.iter().enumerate() {
        assert!(!a.mask().is_empty(), "{a:?} has no bit");
        for b in &Field::ALL[i + 1..] {
            assert!(
                (a.mask() & b.mask()).is_empty(),
                "{a:?} and {b:?} share a bit"
            );
        }
    }
    for name in property_names() {
        let fields = fields_of(name).unwrap();
        let mask = property_mask(name).unwrap();
        for f in fields {
            assert!(mask.contains(f.mask()), "{name} lacks {f:?}'s bit");
        }
    }
}

/// `display` writes the derived `flow` too; removing it must not
/// leave the flow behind.
#[test]
fn removing_display_clears_the_derived_flow() {
    let mut style = TuiStyle::new();
    set("display", "flex", &mut style).unwrap();
    assert!(style.flow.is_some());
    assert!(remove("display", &mut style));
    assert!(style.display.is_none() && style.flow.is_none());
    assert_eq!(
        property_mask("display"),
        Some(
            crate::ImportantMask::DISPLAY
                | crate::ImportantMask::FLOW
                | crate::ImportantMask::LIST_ITEM
                | crate::ImportantMask::WEBKIT_BOX
        )
    );
}

#[test]
fn easing_functions_serialize() {
    let mut style = TuiStyle::new();
    set(
        "transition-timing-function",
        "cubic-bezier(0.1, 0.7, 1, 0.1), steps(4, jump-none), step-end, linear",
        &mut style,
    )
    .unwrap();
    assert_eq!(
        serialize("transition-timing-function", &style).as_deref(),
        Some("cubic-bezier(0.1, 0.7, 1, 0.1), steps(4, jump-none), steps(1, jump-end), linear")
    );
}

/// `CALC-GAP-1`: `gap` accepts `calc()` with percentages and bare
/// percentages, kept symbolic until layout; constant calc folds.
#[test]
fn gap_accepts_calc_and_percent() {
    use crate::calc::CalcExpr;
    use crate::layout::GapValue;
    let mut style = TuiStyle::new();
    set("gap", "calc(50% - 1)", &mut style).unwrap();
    assert!(matches!(
        style.row_gap,
        Some(Value::Specified(GapValue::Calc(_)))
    ));
    assert_eq!(serialize("gap", &style).as_deref(), Some("calc(50% - 1)"));
    set("gap", "calc(2 * 3)", &mut style).unwrap();
    assert_eq!(style.row_gap, Some(Value::Specified(GapValue::Cells(6))));
    set("gap", "10%", &mut style).unwrap();
    assert_eq!(
        style.row_gap,
        Some(Value::Specified(GapValue::calc(CalcExpr::Percent(10.0))))
    );
    assert_eq!(
        set("gap", "-1", &mut style),
        Err(DispatchError::InvalidValue)
    );
    assert_eq!(GapValue::calc(CalcExpr::Percent(10.0)).resolve(30), 3);
}

#[test]
fn css_wide_keywords_parse_for_every_property() {
    for (name, _) in canonical_values() {
        let mut style = TuiStyle::new();
        set(name, "inherit", &mut style).unwrap_or_else(|e| panic!("{name}: inherit {e:?}"));
        let mut style = TuiStyle::new();
        set(name, "initial", &mut style).unwrap_or_else(|e| panic!("{name}: initial {e:?}"));
        let mut style = TuiStyle::new();
        set(name, "unset", &mut style).unwrap_or_else(|e| panic!("{name}: unset {e:?}"));
    }
    let mut style = TuiStyle::new();
    set("color", "inherit", &mut style).unwrap();
    assert_eq!(style.fg, Some(Value::Inherit));
    set("width", "initial", &mut style).unwrap();
    assert_eq!(style.width, Some(Value::Initial));
    // `unset`: color inherits, width does not.
    set("color", "unset", &mut style).unwrap();
    assert_eq!(style.fg, Some(Value::Inherit));
    set("width", "unset", &mut style).unwrap();
    assert_eq!(style.width, Some(Value::Initial));
    // Case-insensitive.
    set("color", "INHERIT", &mut style).unwrap();
    assert_eq!(style.fg, Some(Value::Inherit));
}

#[test]
fn css_wide_keywords_serialize_as_themselves() {
    let mut style = TuiStyle::new();
    set("color", "inherit", &mut style).unwrap();
    assert_eq!(serialize("color", &style).as_deref(), Some("inherit"));
    set("width", "initial", &mut style).unwrap();
    assert_eq!(serialize("width", &style).as_deref(), Some("initial"));
}

/// A shorthand serializes as a CSS-wide keyword only when *every*
/// field it owns holds that same keyword; mixed fields fall through
/// to the normal per-field serialization (or `None`).
#[test]
fn css_wide_serialization_requires_all_owned_fields_to_agree() {
    let mut style = TuiStyle::new();
    set("overflow-x", "inherit", &mut style).unwrap();
    set("overflow-y", "hidden", &mut style).unwrap();
    assert_ne!(serialize("overflow", &style).as_deref(), Some("inherit"));
    assert_eq!(serialize("overflow-x", &style).as_deref(), Some("inherit"));

    let mut style = TuiStyle::new();
    set("top", "inherit", &mut style).unwrap();
    set("left", "1", &mut style).unwrap();
    assert_ne!(serialize("inset", &style).as_deref(), Some("inherit"));

    let mut style = TuiStyle::new();
    set("overflow", "initial", &mut style).unwrap();
    assert_eq!(serialize("overflow", &style).as_deref(), Some("initial"));
    assert_eq!(serialize("overflow-y", &style).as_deref(), Some("initial"));
}

/// `aspect-ratio` was missed by the checked-`u16` sweep; since C2-RATIO
/// its terms are `<number>`s (CSS Values 4 §5.7), so a large term is
/// kept as written, never wrapped.
#[test]
fn aspect_ratio_large_terms_are_kept_not_wrapped() {
    let mut style = TuiStyle::new();
    set("aspect-ratio", "70000 / 1", &mut style).unwrap();
    assert_eq!(
        serialize("aspect-ratio", &style).as_deref(),
        Some("70000 / 1")
    );
}

/// CSS Color 4 §11.1: out-of-range opacity is valid and clamps.
#[test]
fn negative_opacity_clamps_to_zero() {
    let mut style = TuiStyle::new();
    set("opacity", "-0.5", &mut style).unwrap();
    assert_eq!(serialize("opacity", &style).as_deref(), Some("0"));
}

/// `pointer-events: auto | none` (CSS Pointer Events / SVG 1.1 §16.6
/// subset): parses, serializes, and inherits.
#[test]
fn pointer_events_parses_serializes_and_inherits() {
    let mut style = TuiStyle::new();
    set("pointer-events", "none", &mut style).unwrap();
    assert_eq!(serialize("pointer-events", &style).as_deref(), Some("none"));
    set("pointer-events", "auto", &mut style).unwrap();
    assert_eq!(serialize("pointer-events", &style).as_deref(), Some("auto"));
    assert_eq!(
        set("pointer-events", "visiblePainted", &mut TuiStyle::new()),
        Err(DispatchError::InvalidValue),
        "SVG-only values are not supported in a cell grid"
    );
    assert!(inherits("pointer-events"));
    assert!(remove("pointer-events", &mut style));
    assert_eq!(serialize("pointer-events", &style), None);
}

/// `scroll-behavior: auto | smooth` (CSSOM View §12.1): parses,
/// serializes, does not inherit, and owns its own `!important` bit.
#[test]
fn scroll_behavior_parses_serializes_and_does_not_inherit() {
    let mut style = TuiStyle::new();
    set("scroll-behavior", "smooth", &mut style).unwrap();
    assert_eq!(
        style.scroll_behavior,
        Some(Value::Specified(crate::layout::ScrollBehavior::Smooth))
    );
    assert_eq!(
        serialize("scroll-behavior", &style).as_deref(),
        Some("smooth")
    );
    set("scroll-behavior", "AUTO", &mut style).unwrap();
    assert_eq!(
        serialize("scroll-behavior", &style).as_deref(),
        Some("auto")
    );
    assert_eq!(
        set("scroll-behavior", "instant", &mut TuiStyle::new()),
        Err(DispatchError::InvalidValue),
        "`instant` is a ScrollBehavior IDL value, not a property keyword"
    );
    assert!(!inherits("scroll-behavior"));
    assert_eq!(
        property_mask("scroll-behavior"),
        Some(crate::ImportantMask::SCROLL_BEHAVIOR)
    );
    set("scroll-behavior", "inherit", &mut style).unwrap();
    assert_eq!(style.scroll_behavior, Some(Value::Inherit));
    assert!(remove("scroll-behavior", &mut style));
    assert_eq!(serialize("scroll-behavior", &style), None);
}

/// `background` shorthand with only a color is `background-color`
/// (CSS Backgrounds 3 §3.10); the full shorthand is
/// `background_tests.rs`.
#[test]
fn background_shorthand_sets_background_color() {
    let mut style = TuiStyle::new();
    set("background", "red", &mut style).unwrap();
    assert_eq!(
        serialize("background-color", &style).as_deref(),
        Some("red")
    );
    assert_eq!(serialize("background", &style).as_deref(), Some("red"));
    assert!(
        property_mask("background")
            .unwrap()
            .contains(property_mask("background-color").unwrap())
    );
}

/// Named colors serialize back to a CSS name, never to a non-CSS
/// one: `lightcoral` used to come back as `lightred`, which then
/// failed to parse.
#[test]
fn named_colors_round_trip_through_css_names() {
    for name in [
        "lightcoral",
        "rebeccapurple",
        "gray",
        "red",
        "cyan",
        "aliceblue",
    ] {
        let mut a = TuiStyle::new();
        set("color", name, &mut a).unwrap();
        let out = serialize("color", &a).unwrap();
        assert!(!out.starts_with("rgb("), "{name} → {out}");
        let mut b = TuiStyle::new();
        set("color", &out, &mut b).unwrap_or_else(|e| panic!("{name} → {out}: {e:?}"));
        assert_eq!(a, b);
    }
    let mut c = TuiStyle::new();
    set("color", "rgb(1, 2, 3)", &mut c).unwrap();
    assert_eq!(serialize("color", &c).as_deref(), Some("rgb(1, 2, 3)"));
}

/// `transparent` serializes as itself and reads back.
#[test]
fn transparent_round_trips() {
    let mut a = TuiStyle::new();
    set("color", "transparent", &mut a).unwrap();
    assert_eq!(serialize("color", &a).as_deref(), Some("transparent"));
}

/// CSS Color 4 §15.2 (CSSOM): a translucent sRGB color serializes in
/// the legacy `rgba()` form, alpha with the fewest decimals that
/// round-trip, and reads back as the same color.
#[test]
fn translucent_colors_serialize_as_rgba() {
    let mut a = TuiStyle::new();
    set("background-color", "rgb(1 2 3 / 50%)", &mut a).unwrap();
    let out = serialize("background-color", &a).unwrap();
    assert_eq!(out, "rgba(1, 2, 3, 0.5)");
    let mut b = TuiStyle::new();
    set("background-color", &out, &mut b).unwrap();
    assert_eq!(a, b);
}

/// Serialization must re-parse to the same tree: sub-expressions
/// keep their parentheses.
#[test]
fn calc_serialization_keeps_parentheses() {
    for src in [
        "calc((50% + 2) * 2)",
        "calc(100% - (2 + 3))",
        "calc(10 / (2 * 5))",
        "calc((10 - 2) - 3)",
    ] {
        let mut a = TuiStyle::new();
        set("width", src, &mut a).unwrap();
        let out = serialize("width", &a).unwrap();
        let mut b = TuiStyle::new();
        set("width", &out, &mut b).unwrap_or_else(|e| panic!("{src} → {out}: {e:?}"));
        assert_eq!(a, b, "{src} → {out}");
    }
    let mut s = TuiStyle::new();
    set("width", "calc((50% + 2) * 2)", &mut s).unwrap();
    assert_eq!(
        serialize("width", &s).as_deref(),
        Some("calc((50% + 2) * 2)")
    );
}

/// Division by zero is valid (CSS Values 4 §10.9: IEEE-754, `10 / 0`
/// is +∞), and the width clamps to its range; it used to be rejected at
/// parse time (C2G-CALC-SEMANTICS).
#[test]
fn calc_division_by_zero_is_infinite_not_invalid() {
    let mut s = TuiStyle::new();
    set("width", "calc(10 / 0)", &mut s).unwrap();
    assert_eq!(
        s.width,
        Some(Value::Specified(crate::layout::Size::Fixed(u16::MAX)))
    );
    set("width", "calc(10 / 0.0)", &mut TuiStyle::new()).unwrap();
    set("width", "calc(10 / 2)", &mut TuiStyle::new()).unwrap();
}

/// `flex: <grow> <shrink> <basis>` accepts the canonical `0%` basis
/// and fractional factors.
#[test]
fn flex_shorthand_accepts_percent_basis_and_fractional_factors() {
    for src in ["1 1 0%", "1 0.5 auto", "2 1 0", "1 1 10%"] {
        set("flex", src, &mut TuiStyle::new()).unwrap_or_else(|e| panic!("{src}: {e:?}"));
    }
    assert_eq!(
        set("flex", "1 1 foo", &mut TuiStyle::new()),
        Err(DispatchError::InvalidValue)
    );
}

/// The headline spec test for step 25. Every property name in
/// the dispatch table must survive a set → serialize → set
/// round trip, with the resulting `TuiStyle` byte-equal to the
/// first set's `TuiStyle`.
#[test]
fn round_trip_every_property() {
    for (name, value) in canonical_values() {
        let mut style_a = TuiStyle::new();
        set(name, value, &mut style_a).unwrap_or_else(|e| {
            panic!("first set({name:?}, {value:?}) errored: {e:?}");
        });
        let serialized = serialize(name, &style_a)
            .unwrap_or_else(|| panic!("serialize({name:?}) returned None"));
        let mut style_b = TuiStyle::new();
        set(name, &serialized, &mut style_b).unwrap_or_else(|e| {
            panic!(
                "round-trip set({name:?}, {serialized:?}) errored: {e:?} — \
                     serializer produced unparsable form"
            );
        });
        assert_eq!(
            style_a, style_b,
            "{name}: round-trip diverged. original={value:?}, serialized={serialized:?}"
        );
    }
}

/// CSS UI 4 §6.1: `user-select` is not inherited (initial `auto`), so
/// `unset` means `initial`; the used value of `auto` is resolved by the
/// renderer against the parent's used value.
#[test]
fn user_select_is_not_inherited_and_unset_means_initial() {
    assert!(!inherits("user-select"));
    let mut style = TuiStyle::new();
    set("user-select", "unset", &mut style).unwrap();
    assert_eq!(style.user_select, Some(Value::Initial));
}

/// CSS property names are ASCII case-insensitive (custom properties
/// excepted): every dispatch entry point folds the name, so CSSOM's
/// `setProperty("COLOR", …)` / `getPropertyValue("Color")` /
/// `removeProperty("COLOR")` agree with the block parser.
#[test]
fn property_names_are_ascii_case_insensitive() {
    let mut style = TuiStyle::new();
    set("COLOR", "red", &mut style).unwrap();
    assert!(serialize("color", &style).is_some());
    assert_eq!(serialize("Color", &style), serialize("color", &style));
    assert_eq!(property_mask("COLOR"), property_mask("color"));
    assert!(inherits("COLOR"));
    assert!(remove("CoLoR", &mut style));
    assert!(style.fg.is_none());
    set("--Foo", "1", &mut style).unwrap();
    assert_eq!(serialize("--foo", &style), None);
    assert_eq!(serialize("--Foo", &style).as_deref(), Some("1"));
}

/// CSS Cascade 4 §7.3: `revert` is a CSS-wide keyword, valid for every
/// property (ASCII case-insensitive), stored for the cascade to roll
/// back and serialized as written.
#[test]
fn revert_is_a_css_wide_keyword() {
    for &name in property_names() {
        let mut style = TuiStyle::new();
        set(name, "REVERT", &mut style).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(serialize(name, &style).as_deref(), Some("revert"), "{name}");
    }
}

/// CSS Cascade 4 §3.2: `all` sets every property except `direction`,
/// `unicode-bidi` and custom properties to a CSS-wide keyword. Driven
/// by the property table: every name in it takes the keyword (`unset`
/// resolved per property), `!important` covers them all, and anything
/// but a CSS-wide keyword is invalid.
#[test]
fn all_shorthand_sets_every_property_in_the_table() {
    let mut style = TuiStyle::new();
    style.set_custom_property("x", "1", false);
    set("ALL", "unset", &mut style).unwrap();
    // `direction` is in the table but not in `all` (C5-WRITING).
    let in_all = || property_names().iter().filter(|n| **n != "direction");
    // A shorthand over an inherited and non-inherited longhands
    // (`line-clamp`'s `block-ellipsis` inherits) has no one keyword:
    // CSSOM serializes it as the empty string (C8-LINE-CLAMP).
    let mixed = ["line-clamp", "-webkit-line-clamp"];
    for &name in in_all() {
        let want = if inherits(name) { "inherit" } else { "initial" };
        let want = (!mixed.contains(&name)).then_some(want);
        assert_eq!(serialize(name, &style).as_deref(), want, "{name}");
    }
    assert_eq!(serialize("direction", &style), None);
    assert_eq!(style.custom_property_value("x"), Some("1"));

    let mut style = TuiStyle::new();
    set("all", "revert", &mut style).unwrap();
    for &name in in_all() {
        assert_eq!(serialize(name, &style).as_deref(), Some("revert"), "{name}");
    }
    assert_eq!(serialize("all", &style).as_deref(), Some("revert"));

    let every = in_all().fold(crate::ImportantMask::empty(), |m, n| {
        m | property_mask(n).unwrap()
    });
    assert_eq!(property_mask("all"), Some(every));
    assert!(remove("all", &mut style));
    assert!(
        property_names()
            .iter()
            .all(|n| serialize(n, &style).is_none())
    );
    assert_eq!(
        set("all", "red", &mut style),
        Err(DispatchError::InvalidValue)
    );
}

/// CSS Cascade 5 §7.4: `revert-layer` is a CSS-wide keyword too.
#[test]
fn revert_layer_is_a_css_wide_keyword() {
    for &name in property_names() {
        let mut style = TuiStyle::new();
        set(name, "Revert-Layer", &mut style).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(
            serialize(name, &style).as_deref(),
            Some("revert-layer"),
            "{name}"
        );
    }
}

/// C2G-MAX-NONE — CSS Sizing 3 §5.2: `max-width` / `max-height: none`
/// (ASCII case-insensitive) is a declared value — `Some(None)` — that
/// serializes back as `none`.
#[test]
fn max_size_none_is_a_declared_value() {
    for name in ["max-width", "max-height"] {
        let mut s = TuiStyle::new();
        set(name, "None", &mut s).unwrap();
        let field = if name == "max-width" {
            &s.max_width
        } else {
            &s.max_height
        };
        assert_eq!(
            field,
            &Some(Value::Specified(crate::layout::MaxSize::None)),
            "{name}"
        );
        assert_eq!(serialize(name, &s).as_deref(), Some("none"), "{name}");
        set(name, "12", &mut s).unwrap();
        assert_eq!(serialize(name, &s).as_deref(), Some("12"), "{name}");
    }
}

/// C2G-FLEX-SHORTHAND, C6-FLEX-LONGHANDS — CSS Flexbox §7.2: `flex:
/// none | [ <'flex-grow'> <'flex-shrink'>? || <'flex-basis'> ]`. `none`
/// is `0 0 auto`, `auto` `1 1 auto`; an omitted grow is 1, an omitted
/// shrink 1, an omitted basis `0%` (as engines, C6G-FLEX-BASIS-ZERO); the basis may come first; a unitless
/// zero not preceded by two factors is a factor. The shorthand sets its
/// three longhands and nothing else (`width` / `height` stay).
#[test]
fn flex_shorthand_full_grammar() {
    use crate::calc::CalcExpr;
    use crate::layout::FlexBasis;
    let pct = |p: f64| FlexBasis::calc(CalcExpr::Percent(p));
    let cases: [(&str, f32, f32, FlexBasis); 14] = [
        ("none", 0.0, 0.0, FlexBasis::Auto),
        ("auto", 1.0, 1.0, FlexBasis::Auto),
        ("2", 2.0, 1.0, pct(0.0)),
        ("2 3", 2.0, 3.0, pct(0.0)),
        ("0 1 auto", 0.0, 1.0, FlexBasis::Auto),
        ("1 0", 1.0, 0.0, pct(0.0)),
        ("1 30%", 1.0, 1.0, pct(30.0)),
        ("30% 2", 2.0, 1.0, pct(30.0)),
        ("2 0.5 10", 2.0, 0.5, FlexBasis::Cells(10)),
        ("0 1 0", 0.0, 1.0, FlexBasis::Cells(0)),
        ("auto 3", 3.0, 1.0, FlexBasis::Auto),
        ("content", 1.0, 1.0, FlexBasis::Content),
        ("0 0 0", 0.0, 0.0, FlexBasis::Cells(0)),
        // C4G-NUMBER-RANGE: a unitless fraction is a cell length, so a
        // third number is the basis (rounded onto the grid).
        ("1 2 2.5", 1.0, 2.0, FlexBasis::Cells(2)),
    ];
    for (src, grow, shrink, basis) in cases {
        let mut s = TuiStyle::new();
        set("flex", src, &mut s).unwrap_or_else(|e| panic!("{src}: {e:?}"));
        assert_eq!(s.flex_grow, Some(Value::Specified(grow)), "{src}");
        assert_eq!(s.flex_shrink, Some(Value::Specified(shrink)), "{src}");
        assert_eq!(s.flex_basis, Some(Value::Specified(basis)), "{src}");
        assert_eq!((s.width.clone(), s.height.clone()), (None, None), "{src}");
        // The serialization reads back as the same declaration.
        let text = serialize("flex", &s).unwrap_or_else(|| panic!("{src} serializes"));
        let mut again = TuiStyle::new();
        set("flex", &text, &mut again).unwrap();
        assert_eq!(again, s, "{src} → {text}");
    }
    for bad in [
        "1 2 3 4",
        "1 1 foo",
        "-1",
        "auto auto",
        "1 -2",
        "none 1",
        "1 2 auto 3",
    ] {
        assert_eq!(
            set("flex", bad, &mut TuiStyle::new()),
            Err(DispatchError::InvalidValue),
            "{bad}"
        );
    }
}

/// C4G-REEXPORTS — CSS Sizing 3 §5.2: `min-*` / `max-*` take a
/// `<length-percentage>`. A bare percentage is stored in the shape
/// `width` gives it (`Size::Percent`), which the `percent` constructors
/// build; a math function stays `Calc`. Serialization is unchanged.
#[test]
fn min_max_percentages_share_the_size_shape() {
    use crate::layout::{MaxSize, MinSize, Size};
    let mut style = TuiStyle::new();
    set("width", "50%", &mut style).unwrap();
    set("min-width", "50%", &mut style).unwrap();
    set("max-width", "50%", &mut style).unwrap();
    assert_eq!(style.width, Some(Value::Specified(Size::percent(50.0))));
    assert_eq!(
        style.min_width,
        Some(Value::Specified(MinSize::percent(50.0)))
    );
    assert_eq!(
        style.max_width,
        Some(Value::Specified(MaxSize::percent(50.0)))
    );
    assert_eq!(serialize("min-width", &style).as_deref(), Some("50%"));
    assert_eq!(serialize("max-width", &style).as_deref(), Some("50%"));
    set("min-height", "calc(50% + 1)", &mut style).unwrap();
    assert!(matches!(
        style.min_height,
        Some(Value::Specified(MinSize::Calc(_)))
    ));
}

/// C4G-SERIALIZE — CSS Variables 1 §2.1: a custom property's value may
/// be any tokens except a `<bad-url-token>`; a `<url-token>` is kept and
/// reads back as itself.
#[test]
fn custom_property_rejects_a_bad_url() {
    let mut style = TuiStyle::new();
    assert_eq!(
        set("--x", "url(a b)", &mut style),
        Err(DispatchError::InvalidValue)
    );
    set("--x", "url(0001.png) -1px calc(1px - 2px)", &mut style).unwrap();
    assert_eq!(
        serialize("--x", &style).as_deref(),
        Some("url(0001.png) -1px calc(1px - 2px)")
    );
}

/// Every side of `sides` declared as specified.
fn specified_sides<T>(
    sides: impl Into<crate::layout::Sides<T>>,
) -> crate::layout::Sides<Option<Value<T>>> {
    sides.into().map(|v| Some(Value::Specified(v)))
}

// ─── set_from_source's importance (C6G-FRONTEND-API) ────────────────

/// `set_from_source` takes the declaration's `!important` as
/// `set_custom_source` does (CSS Cascade 4 §6.4: importance is per
/// declaration) — for a plain property, a custom one, a `var()` value
/// and an inline-axis flow-relative one alike — so a front end does not
/// follow it with `set_important` in the right order.
#[test]
fn set_from_source_takes_important() {
    use crate::parse::token::tokenize;
    let mut s = TuiStyle::new();
    let v = tokenize("red").unwrap();
    set_from_source("color", &v, Some("red"), true, &mut s).unwrap();
    assert!(is_important("color", &s));
    let v = tokenize("1").unwrap();
    set_from_source("--x", &v, None, true, &mut s).unwrap();
    assert!(
        s.custom_properties
            .iter()
            .any(|d| d.name == "x" && d.important)
    );
    let v = tokenize("var(--x)").unwrap();
    set_from_source("margin-top", &v, Some("var(--x)"), true, &mut s).unwrap();
    assert!(
        s.pending
            .iter()
            .any(|d| d.name == "margin-top" && d.important)
    );
    set_from_source(
        "margin-inline-start",
        &tokenize("2").unwrap(),
        None,
        true,
        &mut s,
    )
    .unwrap();
    assert!(is_important("margin-inline-start", &s));
    set_from_source("padding-top", &tokenize("2").unwrap(), None, false, &mut s).unwrap();
    assert!(!is_important("padding-top", &s));
}

/// C8-PARSE-ERROR: a dispatch error is a `std::error::Error`.
#[test]
fn a_dispatch_error_is_a_std_error() {
    let err: Box<dyn std::error::Error> = Box::new(DispatchError::UnknownProperty);
    assert_eq!(err.to_string(), "unknown CSS property");
    let err: Box<dyn std::error::Error> = Box::new(DispatchError::InvalidValue);
    assert_eq!(err.to_string(), "invalid value for property");
}
