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
        ("background", "green"),
        ("border-color", "rgb(10, 20, 30)"),
        ("font-weight", "bold"),
        ("font-style", "italic"),
        ("text-decoration", "underline"),
        ("opacity", "0.5"),
        ("display", "inline"),
        ("flex-direction", "column"),
        ("white-space", "pre"),
        ("user-select", "text"),
        ("pointer-events", "none"),
        ("caret-color", "transparent"),
        ("caret-text-color", "auto"),
        ("overflow", "scroll"),
        ("overflow-x", "auto"),
        ("overflow-y", "hidden"),
        ("scrollbar-gutter", "stable"),
        ("width", "40"),
        ("height", "auto"),
        ("min-width", "10"),
        ("max-width", "100"),
        ("min-height", "5"),
        ("max-height", "50"),
        ("aspect-ratio", "16/9"),
        ("gap", "2"),
        ("flex", "1"),
        ("flex-shrink", "1"),
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
        ("border-collapse", "collapse"),
        ("content", "\"hello\""),
        ("position", "absolute"),
        ("top", "10"),
        ("right", "20"),
        ("bottom", "auto"),
        ("left", "5"),
        ("z-index", "3"),
        ("inset", "1 2 3 4"),
        ("transition-property", "color"),
        ("transition-duration", "200ms"),
        ("transition-timing-function", "ease-in-out"),
        ("transition-delay", "50ms"),
        ("transition", "width 300ms ease 0ms"),
        ("counter-reset", "chapter 0"),
        ("counter-increment", "chapter 1"),
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
        Some(Value::Specified(Margin {
            top: MarginValue::Cells(5),
            right: MarginValue::Cells(5),
            bottom: MarginValue::Cells(5),
            left: MarginValue::Cells(5),
        }))
    );
}

#[test]
fn margin_shorthand_two_values_split_vertical_horizontal() {
    use crate::layout::{Margin, MarginValue};
    let mut style = TuiStyle::new();
    set("margin", "1 2", &mut style).expect("2-value shorthand parses");
    assert_eq!(
        style.margin,
        Some(Value::Specified(Margin {
            top: MarginValue::Cells(1),
            right: MarginValue::Cells(2),
            bottom: MarginValue::Cells(1),
            left: MarginValue::Cells(2),
        }))
    );
}

#[test]
fn margin_shorthand_three_values_top_horiz_bottom() {
    use crate::layout::{Margin, MarginValue};
    let mut style = TuiStyle::new();
    set("margin", "1 2 3", &mut style).expect("3-value shorthand parses");
    assert_eq!(
        style.margin,
        Some(Value::Specified(Margin {
            top: MarginValue::Cells(1),
            right: MarginValue::Cells(2),
            bottom: MarginValue::Cells(3),
            left: MarginValue::Cells(2),
        }))
    );
}

#[test]
fn margin_shorthand_four_values_each_side() {
    use crate::layout::{Margin, MarginValue};
    let mut style = TuiStyle::new();
    set("margin", "1 2 3 4", &mut style).expect("4-value shorthand parses");
    assert_eq!(
        style.margin,
        Some(Value::Specified(Margin {
            top: MarginValue::Cells(1),
            right: MarginValue::Cells(2),
            bottom: MarginValue::Cells(3),
            left: MarginValue::Cells(4),
        }))
    );
}

#[test]
fn margin_accepts_negative_values() {
    use crate::layout::Margin;
    let mut style = TuiStyle::new();
    set("margin", "-5", &mut style).expect("negative values parse");
    assert_eq!(style.margin, Some(Value::Specified(Margin::all_cells(-5))));
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
        Some(Value::Specified(Margin {
            top: MarginValue::Cells(0),
            right: MarginValue::Auto,
            bottom: MarginValue::Cells(0),
            left: MarginValue::Auto,
        }))
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
        Some(Value::Specified(Margin {
            top: MarginValue::Cells(10),
            right: MarginValue::Cells(5),
            bottom: MarginValue::Cells(5),
            left: MarginValue::Cells(5),
        }))
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
        Some(crate::ImportantMask::DISPLAY | crate::ImportantMask::FLOW)
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
        style.gap,
        Some(Value::Specified(GapValue::Calc(_)))
    ));
    assert_eq!(serialize("gap", &style).as_deref(), Some("calc(50% - 1)"));
    set("gap", "calc(2 * 3)", &mut style).unwrap();
    assert_eq!(style.gap, Some(Value::Specified(GapValue::Cells(6))));
    set("gap", "10%", &mut style).unwrap();
    assert_eq!(
        style.gap,
        Some(Value::Specified(GapValue::Calc(Box::new(
            CalcExpr::Percent(10.0)
        ))))
    );
    assert_eq!(
        set("gap", "-1", &mut style),
        Err(DispatchError::InvalidValue)
    );
    assert_eq!(
        GapValue::Calc(Box::new(CalcExpr::Percent(10.0))).resolve(30),
        3
    );
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

/// `aspect-ratio` was missed by the checked-`u16` sweep.
#[test]
fn aspect_ratio_out_of_range_is_rejected_not_wrapped() {
    assert_eq!(
        set("aspect-ratio", "70000 / 1", &mut TuiStyle::new()),
        Err(DispatchError::InvalidValue)
    );
    set("aspect-ratio", "16 / 9", &mut TuiStyle::new()).unwrap();
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

/// `background` shorthand with only a color is `background-color`.
#[test]
fn background_shorthand_sets_background_color() {
    let mut style = TuiStyle::new();
    set("background", "red", &mut style).unwrap();
    assert_eq!(
        serialize("background-color", &style).as_deref(),
        Some("red")
    );
    assert_eq!(serialize("background", &style).as_deref(), Some("red"));
    assert_eq!(
        property_mask("background"),
        property_mask("background-color")
    );
    // Anything beyond a color (images, positions) is unsupported.
    assert_eq!(
        set("background", "url(x.png) red", &mut TuiStyle::new()),
        Err(DispatchError::InvalidValue)
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

/// Division by a literal zero is invalid at parse time (CSS Values 4
/// §10.9), not a silent 0 at layout time.
#[test]
fn calc_division_by_literal_zero_is_rejected() {
    assert_eq!(
        set("width", "calc(10 / 0)", &mut TuiStyle::new()),
        Err(DispatchError::InvalidValue)
    );
    assert_eq!(
        set("width", "calc(10 / 0.0)", &mut TuiStyle::new()),
        Err(DispatchError::InvalidValue)
    );
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
