//! The animation types of every property of the dispatch table against
//! its specification's property definition ("Animation type:"), and the
//! value-level interpolation rules.

use super::*;
use crate::layout::{GapValue, Length, PaddingValue, Size, Visibility};
use AnimationType::{ByComputedValue as V, Discrete as D, NotAnimatable as N, RepeatableList as R};

/// What the specification says of a property name.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Spec {
    /// A longhand of this animation type.
    Longhand(AnimationType),
    /// A shorthand, an alias or a flow-relative property: "see
    /// individual properties" — it expands to longhands.
    Expands,
}
use Spec::{Expands as S, Longhand as L};

/// The property definition tables, by module. A shorthand's type is its
/// longhands'; a flow-relative property animates as its physical twin.
const SPEC: &[(&str, Spec)] = &[
    // CSS Color 4 §3.1, §13.1
    ("color", L(V)),
    ("opacity", L(V)),
    // CSS Backgrounds 3 §3
    ("background-color", L(V)),
    ("background", S),
    ("background-image", L(D)),
    ("background-position", L(R)),
    ("background-size", L(R)),
    ("background-repeat", L(D)),
    ("background-attachment", L(D)),
    ("background-origin", L(R)),
    ("background-clip", L(R)),
    // CSS Fonts 4 §2–§3, §6
    ("font-weight", L(V)),
    ("font-style", L(V)),
    ("font", S),
    ("font-size", L(V)),
    ("font-family", L(D)),
    ("font-stretch", L(V)),
    ("font-width", S),
    ("font-variant", L(D)),
    // CSS Text Decoration 4 §2–§4
    ("text-decoration", S),
    ("text-decoration-line", L(D)),
    ("text-decoration-style", L(D)),
    ("text-decoration-color", L(V)),
    ("text-decoration-thickness", L(V)),
    ("text-underline-offset", L(V)),
    ("text-underline-position", L(D)),
    ("text-decoration-skip-ink", L(D)),
    // CSS Display 3 §2, §4 (display: "see §2.9"; visibility: discrete,
    // with Web Animations' visibility rule)
    ("display", L(D)),
    ("visibility", L(D)),
    // CSS Flexbox 1 §5, §7; CSS Box Alignment 3 §5–§6
    ("flex-direction", L(D)),
    ("flex-wrap", L(D)),
    ("flex-flow", S),
    ("justify-content", L(D)),
    ("align-content", L(D)),
    ("align-items", L(D)),
    ("align-self", L(D)),
    ("justify-items", L(D)),
    ("justify-self", L(D)),
    ("place-content", S),
    ("place-items", S),
    ("place-self", S),
    ("flex", S),
    ("flex-grow", L(V)),
    ("flex-shrink", L(V)),
    ("flex-basis", L(V)),
    ("order", L(V)),
    // CSS Text 3 / 4
    ("white-space", S),
    ("white-space-collapse", L(D)),
    ("text-wrap-mode", L(D)),
    ("word-break", L(D)),
    ("overflow-wrap", L(D)),
    ("word-wrap", S),
    ("line-break", L(D)),
    ("hyphens", L(D)),
    ("tab-size", L(V)),
    ("text-transform", L(D)),
    ("text-indent", L(V)),
    ("text-align", S),
    ("text-align-all", L(D)),
    ("text-align-last", L(D)),
    ("text-justify", L(D)),
    ("text-wrap", S),
    ("text-wrap-style", L(D)),
    ("letter-spacing", L(V)),
    ("word-spacing", L(V)),
    // CSS Inline 3
    ("line-height", L(V)),
    ("vertical-align", L(V)),
    // CSS UI 4 §6–§7
    ("user-select", L(D)),
    ("pointer-events", L(D)),
    ("caret-color", L(V)),
    ("caret-text-color", L(V)),
    ("caret-shape", L(D)),
    ("caret-animation", L(D)),
    ("caret", S),
    ("accent-color", L(V)),
    ("appearance", L(D)),
    ("-webkit-appearance", S),
    ("field-sizing", L(D)),
    ("resize", L(D)),
    // CSS Overflow 3 / 4
    ("overflow", S),
    ("overflow-x", L(D)),
    ("overflow-y", L(D)),
    ("overflow-clip-margin", L(V)),
    ("text-overflow", L(D)),
    ("line-clamp", S),
    ("max-lines", L(V)),
    ("block-ellipsis", L(D)),
    ("continue", L(D)),
    ("-webkit-line-clamp", S),
    ("-webkit-box-orient", L(D)),
    // CSS Scrollbars 1, Overscroll 1, Scroll Snap 1, CSSOM View 1
    ("scrollbar-gutter", L(D)),
    ("scrollbar-width", L(D)),
    ("scrollbar-color", L(V)),
    ("outline", S),
    ("outline-style", L(D)),
    ("outline-width", L(V)),
    ("outline-color", L(V)),
    ("outline-offset", L(V)),
    ("cursor", L(D)),
    ("overscroll-behavior", S),
    ("overscroll-behavior-x", L(D)),
    ("overscroll-behavior-y", L(D)),
    ("scroll-padding", S),
    ("scroll-padding-top", L(V)),
    ("scroll-padding-right", L(V)),
    ("scroll-padding-bottom", L(V)),
    ("scroll-padding-left", L(V)),
    ("scroll-margin", S),
    ("scroll-margin-top", L(V)),
    ("scroll-margin-right", L(V)),
    ("scroll-margin-bottom", L(V)),
    ("scroll-margin-left", L(V)),
    ("scroll-snap-type", L(D)),
    ("scroll-snap-align", L(D)),
    ("scroll-snap-stop", L(D)),
    ("scroll-behavior", L(N)),
    // CSS Sizing 3 / 4, Box Sizing (UI 3)
    ("width", L(V)),
    ("height", L(V)),
    ("min-width", L(V)),
    ("max-width", L(V)),
    ("min-height", L(V)),
    ("max-height", L(V)),
    ("aspect-ratio", L(V)),
    ("box-sizing", L(D)),
    // CSS Values 5 §11
    ("interpolate-size", L(N)),
    ("contain-intrinsic-size", S),
    ("contain-intrinsic-width", L(V)),
    ("contain-intrinsic-height", L(V)),
    ("contain-intrinsic-inline-size", S),
    ("contain-intrinsic-block-size", S),
    // CSS Box Alignment 3 §8
    ("gap", S),
    ("row-gap", L(V)),
    ("column-gap", L(V)),
    // CSS Grid 2 §7–§8
    ("grid", S),
    ("grid-template", S),
    ("grid-template-columns", L(V)),
    ("grid-template-rows", L(V)),
    ("grid-template-areas", L(D)),
    ("grid-auto-columns", L(V)),
    ("grid-auto-rows", L(V)),
    ("grid-auto-flow", L(D)),
    ("grid-row-start", L(D)),
    ("grid-row-end", L(D)),
    ("grid-column-start", L(D)),
    ("grid-column-end", L(D)),
    ("grid-row", S),
    ("grid-column", S),
    ("grid-area", S),
    // CSS Box 4
    ("padding", S),
    ("padding-top", L(V)),
    ("padding-right", L(V)),
    ("padding-bottom", L(V)),
    ("padding-left", L(V)),
    ("margin", S),
    ("margin-top", L(V)),
    ("margin-right", L(V)),
    ("margin-bottom", L(V)),
    ("margin-left", L(V)),
    ("margin-trim", L(D)),
    // CSS Backgrounds 3 §4–§6
    ("border", S),
    ("border-top", S),
    ("border-right", S),
    ("border-bottom", S),
    ("border-left", S),
    ("border-style", S),
    ("border-top-style", L(D)),
    ("border-right-style", L(D)),
    ("border-bottom-style", L(D)),
    ("border-left-style", L(D)),
    ("border-color", S),
    ("border-top-color", L(V)),
    ("border-right-color", L(V)),
    ("border-bottom-color", L(V)),
    ("border-left-color", L(V)),
    ("border-width", S),
    ("border-top-width", L(V)),
    ("border-right-width", L(V)),
    ("border-bottom-width", L(V)),
    ("border-left-width", L(V)),
    ("border-radius", S),
    ("border-top-left-radius", L(V)),
    ("border-top-right-radius", L(V)),
    ("border-bottom-right-radius", L(V)),
    ("border-bottom-left-radius", L(V)),
    ("box-shadow", L(AnimationType::ShadowList)),
    // CSS 2.1 §17.6
    ("border-collapse", L(D)),
    ("border-spacing", L(V)),
    // CSS Generated Content 3, Lists 3
    ("content", L(D)),
    ("quotes", L(D)),
    ("list-style", S),
    ("list-style-type", L(D)),
    ("list-style-position", L(D)),
    ("list-style-image", L(D)),
    ("marker-side", L(D)),
    ("counter-reset", L(V)),
    ("counter-increment", L(V)),
    ("counter-set", L(V)),
    // CSS Position 3, CSS 2.1 §9.5 / §9.9
    ("position", L(D)),
    ("top", L(V)),
    ("right", L(V)),
    ("bottom", L(V)),
    ("left", L(V)),
    ("inset", S),
    ("z-index", L(V)),
    // CSS Position 4 §3.4
    ("overlay", L(D)),
    ("float", L(D)),
    ("clear", L(D)),
    // CSS Transitions 1 §2
    ("transition-property", L(N)),
    ("transition-duration", L(N)),
    ("transition-timing-function", L(N)),
    ("transition-delay", L(N)),
    ("transition-behavior", L(N)),
    ("transition", S),
    // CSS Animations 1 §4, CSS Animations 2 §3: not animatable
    ("animation-name", L(N)),
    ("animation-duration", L(N)),
    ("animation-timing-function", L(N)),
    ("animation-delay", L(N)),
    ("animation-iteration-count", L(N)),
    ("animation-direction", L(N)),
    ("animation-fill-mode", L(N)),
    ("animation-play-state", L(N)),
    ("animation-composition", L(N)),
    ("animation-timeline", L(N)),
    ("animation", S),
    // Scroll-driven Animations 1 §2–§4
    ("scroll-timeline-name", L(N)),
    ("scroll-timeline-axis", L(N)),
    ("view-timeline-name", L(N)),
    ("view-timeline-axis", L(N)),
    ("view-timeline-inset", L(V)),
    ("timeline-scope", L(N)),
    ("animation-range-start", L(N)),
    ("animation-range-end", L(N)),
    ("scroll-timeline", S),
    ("view-timeline", S),
    ("animation-range", S),
    // CSS Color Adjust 1 §2
    ("color-scheme", L(D)),
    // CSS Writing Modes 4 §2–§3
    ("direction", L(N)),
    ("writing-mode", L(N)),
];

/// Web Animations 1 §5.1: every property of the dispatch table has the
/// animation type its definition gives — a longhand its own, a
/// shorthand or a flow-relative property its longhands' (and expands to
/// at least one).
#[test]
fn every_property_has_its_specified_animation_type() {
    use crate::property_dispatch::property_names;
    let logical =
        |n: &str| crate::property_dispatch::physical_names(n, TextDirection::Ltr).is_some();
    for &name in property_names() {
        let spec = SPEC.iter().find(|(n, _)| *n == name).map(|(_, s)| *s);
        let spec = spec.unwrap_or_else(|| {
            assert!(logical(name), "{name}: missing from the spec table");
            S
        });
        match spec {
            L(kind) => assert_eq!(animation_type(name), Some(kind), "{name}"),
            S => {
                assert_eq!(animation_type(name), None, "{name} is no longhand");
                assert!(
                    !transition_longhands(name, TextDirection::Ltr).is_empty(),
                    "{name} expands to no longhand"
                );
            }
        }
    }
    for (name, _) in SPEC {
        assert!(property_names().contains(name), "{name} is in the table");
    }
    assert_eq!(
        Longhand::all().count(),
        SPEC.iter().filter(|(_, s)| matches!(s, L(_))).count(),
        "one longhand per spec longhand"
    );
}

/// CSS Transitions 1 §2.1: a shorthand covers its longhands; a
/// flow-relative name its physical twin under the element's direction.
#[test]
fn transition_names_expand_to_longhands() {
    let names = |n: &str, d| -> Vec<&str> {
        transition_longhands(n, d)
            .iter()
            .map(|l| l.name())
            .collect()
    };
    assert_eq!(
        names("padding", TextDirection::Ltr),
        [
            "padding-top",
            "padding-right",
            "padding-bottom",
            "padding-left"
        ]
    );
    assert_eq!(names("gap", TextDirection::Ltr), ["row-gap", "column-gap"]);
    assert_eq!(
        names("inset", TextDirection::Ltr),
        ["top", "right", "bottom", "left"]
    );
    assert_eq!(
        names("margin-inline-start", TextDirection::Ltr),
        ["margin-left"]
    );
    assert_eq!(
        names("margin-inline-start", TextDirection::Rtl),
        ["margin-right"]
    );
    assert_eq!(names("block-size", TextDirection::Ltr), ["height"]);
    assert_eq!(names("font-width", TextDirection::Ltr), ["font-stretch"]);
    assert_eq!(names("display", TextDirection::Ltr), ["display"]);
    assert!(names("nope", TextDirection::Ltr).is_empty());
}

fn style() -> ComputedStyle {
    ComputedStyle::initial()
}

fn at(l: &str, a: &ComputedStyle, b: &ComputedStyle, p: f64) -> ComputedStyle {
    let mut out = b.clone();
    Longhand::from_name(l)
        .unwrap()
        .interpolate(a, b, p, ColorScheme::Dark, &mut out);
    out
}

/// Whole cells (DIVERGENCES §1): a cell length interpolates exactly and
/// rounds onto the grid half to even, as `calc()` does; a length mixed
/// with a percentage keeps both parts (CSS Values 4 §3.4.3).
#[test]
fn lengths_interpolate_in_whole_cells_and_mix_with_percentages() {
    let (mut a, mut b) = (style(), style());
    a.height = Size::Fixed(0);
    b.height = Size::Fixed(5);
    assert_eq!(at("height", &a, &b, 0.5).height, Size::Fixed(2), "2.5 → 2");
    assert_eq!(at("height", &a, &b, 0.7).height, Size::Fixed(4), "3.5 → 4");
    a.width = Size::Percent(10.0);
    b.width = Size::Percent(30.0);
    assert_eq!(at("width", &a, &b, 0.5).width, Size::Percent(20.0));
    a.padding.top = PaddingValue::Cells(2);
    b.padding.top = PaddingValue::calc(crate::calc::CalcExpr::Percent(50.0));
    let mixed = at("padding-top", &a, &b, 0.5).padding.top;
    let PaddingValue::Calc(e) = mixed else {
        panic!("a calc mix: {mixed:?}")
    };
    assert_eq!(e.linear_parts(), Some((1.0, 25.0)));
    a.top = Length::Cells(-4);
    b.top = Length::Cells(4);
    assert_eq!(at("top", &a, &b, 0.25).top, Length::Cells(-2));
}

/// Web Animations 1 §5.3.1: a pair that is not interpolable — `auto`
/// against a length, a keyword `gap` — steps at 50 %, and is reported so.
#[test]
fn a_pair_that_does_not_interpolate_steps_at_the_midpoint() {
    let (mut a, mut b) = (style(), style());
    a.height = Size::Auto;
    b.height = Size::Fixed(9);
    let h = Longhand::from_name("height").unwrap();
    assert!(!h.interpolable(&a, &b));
    assert_eq!(at("height", &a, &b, 0.49).height, Size::Auto);
    assert_eq!(at("height", &a, &b, 0.5).height, Size::Fixed(9));
    a.row_gap = GapValue::Normal;
    b.row_gap = GapValue::Cells(2);
    assert!(!Longhand::from_name("row-gap").unwrap().interpolable(&a, &b));
    b.height = Size::Fixed(3);
    a.height = Size::Fixed(1);
    assert!(h.interpolable(&a, &b));
}

/// A discrete longhand never interpolates (CSS Transitions 2 §2 gates it
/// on `allow-discrete`), flips at 50 % with its companions, and
/// `visibility` keeps its own rule (CSS Display 3 §4).
#[test]
fn discrete_longhands_flip_at_the_midpoint_with_their_companions() {
    use crate::layout::{Display, Flow};
    let (mut a, mut b) = (style(), style());
    a.display = Display::Block;
    a.flow = Flow::Block;
    b.display = Display::InlineBlock;
    b.flow = Flow::Flex;
    let d = Longhand::from_name("display").unwrap();
    assert!(!d.interpolable(&a, &b));
    let mid = at("display", &a, &b, 0.5);
    assert_eq!((mid.display, mid.flow), (Display::InlineBlock, Flow::Flex));
    let early = at("display", &a, &b, 0.2);
    assert_eq!((early.display, early.flow), (Display::Block, Flow::Block));

    a.visibility = Visibility::Hidden;
    b.visibility = Visibility::Visible;
    let v = Longhand::from_name("visibility").unwrap();
    assert!(v.interpolable(&a, &b));
    assert_eq!(
        at("visibility", &a, &b, 0.1).visibility,
        Visibility::Visible
    );
    assert_eq!(at("visibility", &a, &b, 0.0).visibility, Visibility::Hidden);
}

/// The derived parts of a value move with it: the used border with a
/// border width, the bold modifier with the weight.
#[test]
fn companions_follow_the_value() {
    use crate::layout::{BorderStyle, BorderWidth, PaintLength};
    let (mut a, mut b) = (style(), style());
    for s in [&mut a, &mut b] {
        s.border_style.top = BorderStyle::Solid;
    }
    a.border_width.top = BorderWidth::Length(PaintLength::Cells(0.0));
    b.border_width.top = BorderWidth::Length(PaintLength::Cells(1.0));
    b.border = b.border_style.with_widths(&b.border_width);
    let start = at("border-top-width", &a, &b, 0.0);
    assert_eq!(
        start.border.top,
        BorderStyle::None,
        "a zero width draws none"
    );
    let mid = at("border-top-width", &a, &b, 0.5);
    assert_eq!(
        mid.border.top,
        BorderStyle::Solid,
        "a width draws its style"
    );

    a.font.weight = crate::layout::FontWeight::Number(400.0);
    b.font.weight = crate::layout::FontWeight::Number(800.0);
    let mid = at("font-weight", &a, &b, 0.5);
    assert_eq!(mid.font.weight, crate::layout::FontWeight::Number(600.0));
    assert!(mid.modifiers.contains(crate::Modifier::BOLD));
}

/// Not-animatable longhands never interpolate and are not animatable.
#[test]
fn not_animatable_longhands_never_transition() {
    for name in [
        "direction",
        "writing-mode",
        "scroll-behavior",
        "transition-delay",
    ] {
        let l = Longhand::from_name(name).unwrap();
        assert!(!l.is_animatable(), "{name}");
        assert!(!l.interpolable(&style(), &style()), "{name}");
    }
    assert!(
        !Longhand::from_name("background-image")
            .unwrap()
            .has_computed_value()
    );
}

/// CSS Values 5 §11: under `interpolate-size: allow-keywords` a sizing
/// keyword and a length interpolate as
/// `calc-size(keyword, size * (1 - p) + length * p)`; under `numeric-only`
/// (the initial value) they do not. Two different keywords never do; a
/// `calc-size()` and a length always do.
#[test]
fn interpolate_size_lets_auto_interpolate_through_calc_size() {
    use crate::layout::{CalcSize, CalcSizeBasis, InterpolateSize};
    let w = Longhand::from_name("width").unwrap();
    let (mut a, mut b) = (style(), style());
    a.width = Size::Auto;
    b.width = Size::Fixed(10);
    assert!(!w.interpolable(&a, &b), "numeric-only");
    b.interpolate_size = InterpolateSize::AllowKeywords;
    assert!(w.interpolable(&a, &b));
    let Size::CalcSize(c) = at("width", &a, &b, 0.5).width else {
        panic!("{:?}", at("width", &a, &b, 0.5).width)
    };
    assert_eq!((c.basis.clone(), c.factor), (CalcSizeBasis::Auto, 0.5));
    assert_eq!(c.resolve(40, 0), 25, "half of 40 and half of 10");
    assert_eq!(
        at("width", &a, &b, 0.0).width,
        Size::Auto,
        "the ends are themselves"
    );
    assert_eq!(at("width", &a, &b, 1.0).width, Size::Fixed(10));

    b.width = Size::Intrinsic(crate::layout::IntrinsicSize::MinContent);
    assert!(!w.interpolable(&a, &b), "two keywords");

    b.interpolate_size = InterpolateSize::NumericOnly;
    a.width = Size::CalcSize(std::sync::Arc::new(CalcSize::new(
        CalcSizeBasis::Auto,
        1.0,
        crate::calc::CalcExpr::Number(2.0),
    )));
    b.width = Size::Fixed(10);
    assert!(w.interpolable(&a, &b), "a calc-size() with a length");
}
