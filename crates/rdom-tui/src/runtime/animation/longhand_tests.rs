//! C12-ANIMATABLE: every longhand rdom computes a value for animates as
//! its specification's "Animation type" says — one interpolation driven
//! per longhand, from cascaded values, against the dispatch table's list
//! (`rdom_style::animation`'s spec table checks the types themselves).

use rdom_style::animation::{AnimationType, Longhand};
use rdom_style::color::ColorScheme;

use crate::style::{ComputedStyle, Stylesheet};
use crate::{CascadeExt, TuiDom};

/// What 50 % progress gives.
#[derive(Debug, Clone, Copy)]
enum Mid {
    /// This value (CSS text), interpolated.
    Is(&'static str),
    /// A value strictly between the ends (a color, interpolated in Oklab).
    Between,
    /// The discrete step: the end value from 50 % on, the start below.
    Flips,
}
use Mid::{Between, Flips, Is};

/// `(longhand, from, to, at 50 %)`, one row per longhand with a computed
/// value.
const SAMPLES: &[(&str, &str, &str, Mid)] = &[
    ("color", "rgb(0, 0, 0)", "rgb(200, 200, 200)", Between),
    (
        "background-color",
        "rgb(0, 0, 0)",
        "rgb(200, 200, 200)",
        Between,
    ),
    ("background-clip", "border-box", "content-box", Flips),
    ("font-weight", "400", "800", Is("600")),
    ("font-style", "normal", "oblique 20deg", Is("oblique 10deg")),
    ("font-size", "10px", "20px", Is("15px")),
    ("font-family", "serif", "monospace", Flips),
    ("font-stretch", "50%", "150%", Is("100%")),
    ("font-variant", "normal", "small-caps", Flips),
    ("text-decoration-line", "none", "underline", Flips),
    ("text-decoration-style", "solid", "dotted", Flips),
    (
        "text-decoration-color",
        "rgb(0, 0, 0)",
        "rgb(200, 200, 200)",
        Between,
    ),
    ("text-decoration-thickness", "2", "4", Is("3")),
    ("text-underline-offset", "2", "4", Is("3")),
    ("text-underline-position", "auto", "under", Flips),
    ("text-decoration-skip-ink", "auto", "none", Flips),
    ("opacity", "0", "1", Is("0.5")),
    ("display", "block", "inline-block", Flips),
    ("flex-direction", "row", "column", Flips),
    ("flex-wrap", "nowrap", "wrap", Flips),
    ("justify-content", "start", "end", Flips),
    ("align-content", "start", "end", Flips),
    ("align-items", "start", "end", Flips),
    ("align-self", "start", "end", Flips),
    ("justify-items", "start", "end", Flips),
    ("justify-self", "start", "end", Flips),
    ("user-select", "auto", "none", Flips),
    ("pointer-events", "auto", "none", Flips),
    ("visibility", "hidden", "visible", Is("visible")),
    ("caret-color", "rgb(0, 0, 0)", "rgb(200, 200, 200)", Between),
    (
        "caret-text-color",
        "rgb(0, 0, 0)",
        "rgb(200, 200, 200)",
        Between,
    ),
    ("overflow-x", "visible", "hidden", Flips),
    ("overflow-y", "visible", "hidden", Flips),
    ("overflow-clip-margin", "2", "4", Is("3")),
    ("text-overflow", "clip", "ellipsis", Flips),
    ("max-lines", "2", "4", Is("3")),
    ("block-ellipsis", "no-ellipsis", "auto", Flips),
    ("continue", "auto", "discard", Flips),
    ("-webkit-box-orient", "horizontal", "vertical", Flips),
    ("scrollbar-gutter", "auto", "stable", Flips),
    ("scrollbar-width", "auto", "thin", Flips),
    (
        "scrollbar-color",
        "rgb(0, 0, 0) rgb(0, 0, 0)",
        "rgb(200, 200, 200) rgb(200, 200, 200)",
        Between,
    ),
    ("overscroll-behavior-x", "auto", "contain", Flips),
    ("overscroll-behavior-y", "auto", "contain", Flips),
    ("scroll-padding-top", "2", "4", Is("3")),
    ("scroll-padding-right", "2", "4", Is("3")),
    ("scroll-padding-bottom", "2", "4", Is("3")),
    ("scroll-padding-left", "2", "4", Is("3")),
    ("scroll-margin-top", "2", "4", Is("3")),
    ("scroll-margin-right", "2", "4", Is("3")),
    ("scroll-margin-bottom", "2", "4", Is("3")),
    ("scroll-margin-left", "2", "4", Is("3")),
    ("scroll-snap-type", "none", "x mandatory", Flips),
    ("scroll-snap-align", "none", "start", Flips),
    ("scroll-snap-stop", "normal", "always", Flips),
    ("width", "10", "20", Is("15")),
    ("height", "10%", "30%", Is("20%")),
    ("min-width", "10", "20", Is("15")),
    ("max-width", "10", "20", Is("15")),
    ("min-height", "10", "20", Is("15")),
    ("max-height", "10", "20", Is("15")),
    ("aspect-ratio", "1", "4", Is("2")),
    ("box-sizing", "content-box", "border-box", Flips),
    ("contain-intrinsic-width", "10", "20", Is("15")),
    ("contain-intrinsic-height", "10", "20", Is("15")),
    ("row-gap", "2", "4", Is("3")),
    ("column-gap", "2", "4", Is("3")),
    ("flex-grow", "0", "2", Is("1")),
    ("flex-shrink", "0", "2", Is("1")),
    ("flex-basis", "10", "20", Is("15")),
    ("order", "0", "4", Is("2")),
    ("grid-template-columns", "10 20", "20 40", Is("15 30")),
    ("grid-template-rows", "1fr 2", "3fr 4", Is("2fr 3")),
    ("grid-template-areas", "'a'", "'b'", Flips),
    ("grid-auto-columns", "10", "20", Is("15")),
    ("grid-auto-rows", "10", "20", Is("15")),
    ("grid-auto-flow", "row", "column", Flips),
    ("grid-row-start", "1", "2", Flips),
    ("grid-row-end", "1", "2", Flips),
    ("grid-column-start", "1", "2", Flips),
    ("grid-column-end", "1", "2", Flips),
    ("padding-top", "2", "4", Is("3")),
    ("padding-right", "2", "4", Is("3")),
    ("padding-bottom", "2", "4", Is("3")),
    ("padding-left", "2", "4", Is("3")),
    ("margin-top", "-2", "4", Is("1")),
    ("margin-right", "2", "4", Is("3")),
    ("margin-bottom", "2", "4", Is("3")),
    ("margin-left", "2", "4", Is("3")),
    ("margin-trim", "none", "block", Flips),
    ("border-top-style", "none", "solid", Flips),
    ("border-right-style", "none", "solid", Flips),
    ("border-bottom-style", "none", "solid", Flips),
    ("border-left-style", "none", "solid", Flips),
    (
        "border-top-color",
        "rgb(0, 0, 0)",
        "rgb(200, 200, 200)",
        Between,
    ),
    (
        "border-right-color",
        "rgb(0, 0, 0)",
        "rgb(200, 200, 200)",
        Between,
    ),
    (
        "border-bottom-color",
        "rgb(0, 0, 0)",
        "rgb(200, 200, 200)",
        Between,
    ),
    (
        "border-left-color",
        "rgb(0, 0, 0)",
        "rgb(200, 200, 200)",
        Between,
    ),
    ("border-top-width", "1", "3", Is("2")),
    ("border-right-width", "1", "3", Is("2")),
    ("border-bottom-width", "1", "3", Is("2")),
    ("border-left-width", "1", "3", Is("2")),
    ("border-top-left-radius", "2", "4", Is("3")),
    ("border-top-right-radius", "2", "4", Is("3")),
    ("border-bottom-right-radius", "2", "4", Is("3")),
    ("border-bottom-left-radius", "2", "4", Is("3")),
    (
        "box-shadow",
        "1 1 rgb(0, 0, 0)",
        "3 3 rgb(0, 0, 0)",
        Is("2 2 rgb(0, 0, 0)"),
    ),
    ("border-spacing", "2", "4", Is("3")),
    ("border-collapse", "separate", "collapse", Flips),
    ("content", "'a'", "'b'", Flips),
    ("quotes", "auto", "none", Flips),
    ("list-style-type", "disc", "square", Flips),
    ("list-style-position", "outside", "inside", Flips),
    ("list-style-image", "none", "url(a.png)", Flips),
    ("marker-side", "match-self", "match-parent", Flips),
    ("position", "static", "relative", Flips),
    ("top", "2", "4", Is("3")),
    ("right", "2", "4", Is("3")),
    ("bottom", "2", "4", Is("3")),
    ("left", "2", "4", Is("3")),
    ("z-index", "0", "4", Is("2")),
    ("float", "none", "left", Flips),
    ("clear", "none", "both", Flips),
    ("counter-reset", "a 0", "a 4", Is("a 2")),
    ("counter-increment", "a 0", "a 4", Is("a 2")),
    ("counter-set", "a 0", "a 4", Is("a 2")),
    ("color-scheme", "normal", "dark", Flips),
    ("white-space-collapse", "collapse", "preserve", Flips),
    ("text-wrap-mode", "wrap", "nowrap", Flips),
    ("word-break", "normal", "break-all", Flips),
    ("overflow-wrap", "normal", "anywhere", Flips),
    ("line-break", "auto", "strict", Flips),
    ("hyphens", "manual", "none", Flips),
    ("tab-size", "2", "4", Is("3")),
    ("text-transform", "none", "uppercase", Flips),
    ("text-indent", "2", "4", Is("3")),
    ("text-align-all", "start", "end", Flips),
    ("text-align-last", "auto", "end", Flips),
    ("text-justify", "auto", "none", Flips),
    ("text-wrap-style", "auto", "balance", Flips),
    ("letter-spacing", "2", "4", Is("3")),
    ("word-spacing", "2", "4", Is("3")),
    ("line-height", "2", "4", Is("3")),
    ("vertical-align", "2ch", "4ch", Is("3ch")),
];

/// `#t`'s computed style under `name: value`.
fn computed(name: &str, value: &str) -> Result<ComputedStyle, String> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.set_attribute(div, "id", "t").unwrap();
    dom.append_child(root, div).unwrap();
    let sheet = rdom_css::parse(&format!("#t {{ {name}: {value} }}"));
    if !sheet.warnings.is_empty() {
        return Err(format!("{name}: {value}: {:?}", sheet.warnings));
    }
    let sheet: Stylesheet = sheet.stylesheet;
    dom.cascade(&sheet);
    Ok((**dom.node(div).ext().unwrap().computed.as_ref().unwrap()).clone())
}

fn at(l: Longhand, a: &ComputedStyle, b: &ComputedStyle, p: f64) -> ComputedStyle {
    let mut out = b.clone();
    l.interpolate(a, b, p, ColorScheme::Dark, &mut out);
    out
}

/// Web Animations 1 §5.3: each longhand's computed values combine by its
/// animation type — interpolated at 50 %, or the discrete step there.
#[test]
fn every_longhand_drives_one_interpolation() {
    let mut errors = Vec::new();
    for l in Longhand::all().filter(|l| l.is_animatable()) {
        if let Err(e) = drive(l) {
            errors.push(e);
        }
    }
    for (name, ..) in SAMPLES {
        match Longhand::from_name(name) {
            Some(l) if l.is_animatable() => {}
            _ => errors.push(format!("{name}: not an animatable longhand")),
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

fn drive(l: Longhand) -> Result<(), String> {
    let Some(&(name, from, to, mid)) = SAMPLES.iter().find(|s| s.0 == l.name()) else {
        return Err(format!("{}: no sample", l.name()));
    };
    let (a, b) = (computed(name, from)?, computed(name, to)?);
    let check = |ok: bool, what: &str| {
        if ok {
            Ok(())
        } else {
            Err(format!("{name}: {what}"))
        }
    };
    check(l.differs(&a, &b), "the samples differ")?;
    let half = at(l, &a, &b, 0.5);
    match mid {
        Is(css) => {
            check(l.interpolable(&a, &b), "interpolates")?;
            let want = computed(name, css)?;
            check(!l.differs(&half, &want), &format!("50 % is {css}"))
        }
        Between => {
            check(l.interpolable(&a, &b), "interpolates")?;
            check(
                l.differs(&half, &a) && l.differs(&half, &b),
                "50 % is between the ends",
            )
        }
        Flips => {
            check(!l.interpolable(&a, &b), "is discrete")?;
            check(
                l.animation_type() != AnimationType::ByComputedValue,
                "is no by-value type",
            )?;
            check(!l.differs(&half, &b), "50 % is the end value")?;
            check(
                !l.differs(&at(l, &a, &b, 0.49), &a),
                "49 % is the start value",
            )
        }
    }
}
