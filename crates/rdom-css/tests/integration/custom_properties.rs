//! §11.6 — Custom properties at :root.
//!
//! `:root { --name: value; }` is an ordinary rule: the cascade gives the
//! root its custom properties and every element inherits them (the root
//! fragment is the root element, C14G-ROOT-ELEMENT). The sheet-level map
//! (`Stylesheet::vars`) holds only what `define_var` puts there — the
//! parse-time `:root` mirror is gone.

use rdom_css::parse;

/// The value of `--name` on the root of a document cascaded with `css`,
/// as its element child inherits it.
fn root_var(css: &str, name: &str) -> Option<String> {
    let r = parse(css);
    root_var_of(&r.stylesheet, name)
}

fn root_var_of(sheet: &rdom_tui::Stylesheet, name: &str) -> Option<String> {
    use rdom_tui::CascadeExt;
    let mut dom: rdom_tui::TuiDom = rdom_tui::TuiDom::new();
    let root = dom.root();
    let html = dom.create_element("html");
    dom.append_child(root, html).unwrap();
    dom.cascade(sheet);
    let of = |id| {
        rdom_tui::style::cascade::computed_of(&dom, id)
            .vars
            .get(name)
            .map(|v| v.as_str().to_string())
    };
    let (on_root, inherited) = (of(root), of(html));
    assert_eq!(on_root, inherited, "the element inherits the root's");
    on_root
}

#[test]
fn root_defines_single_var() {
    let r = parse(":root { --accent: #3d90ce; }");
    assert!(r.warnings.is_empty(), "warnings: {:?}", r.warnings);
    assert_eq!(
        r.stylesheet.var("accent"),
        None,
        "no mirror: the sheet-level map is define_var's"
    );
    assert_eq!(
        root_var(":root { --accent: #3d90ce; }", "accent").as_deref(),
        Some("#3d90ce")
    );
}

#[test]
fn root_defines_multiple_vars() {
    let css = ":root { --accent: #3d90ce; --dim: #707070; }";
    assert!(parse(css).warnings.is_empty());
    assert_eq!(root_var(css, "accent").as_deref(), Some("#3d90ce"));
    assert_eq!(root_var(css, "dim").as_deref(), Some("#707070"));
}

#[test]
fn root_var_with_named_color() {
    assert!(parse(":root { --bg: red; }").warnings.is_empty());
    assert_eq!(
        root_var(":root { --bg: red; }", "bg").as_deref(),
        Some("red")
    );
}

#[test]
fn var_reference_after_root_definition() {
    // The reference is kept as tokens (CSS Variables 1 §3); the
    // cascade substitutes it per element.
    let r = parse(":root { --accent: #3d90ce; } button { color: var(--accent); }");
    assert!(r.warnings.is_empty(), "warnings: {:?}", r.warnings);
    assert_eq!(
        root_var_of(&r.stylesheet, "accent").as_deref(),
        Some("#3d90ce")
    );
    let button_rule = r
        .stylesheet
        .rules()
        .iter()
        .find(|x| x.source_text == "button")
        .expect("button rule present");
    assert!(button_rule.style.fg.is_none());
    assert_eq!(button_rule.style.pending[0].name, "color");
}

#[test]
fn later_root_rule_overrides_earlier_var() {
    // Last-wins (matches CSS cascade for declarations on the
    // same selector).
    let css = ":root { --accent: #aaa; } :root { --accent: #bbb; }";
    assert!(parse(css).warnings.is_empty());
    assert_eq!(root_var(css, "accent").as_deref(), Some("#bbb"));
}

#[test]
fn root_ignores_unknown_property() {
    // `:root { background: red; }` — `background` (without -color)
    // is not in the M1 property table; emits UnknownProperty.
    // Custom property still registers.
    assert_eq!(
        root_var(":root { background: red; --accent: blue; }", "accent").as_deref(),
        Some("blue")
    );
}

#[test]
fn var_with_fallback_color_is_pending() {
    let r = parse("a { color: var(--missing, #ff0000); }");
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let style = &r.stylesheet.rules()[0].style;
    assert!(style.fg.is_none());
    assert!(style.pending[0].has_substitution);
}

/// `CSS-VARS-SCOPE-1`: a custom property under any selector stays on
/// the rule (the cascade scopes it per element); nothing warns, and
/// nothing publishes through the sheet-level map.
#[test]
fn custom_property_under_any_selector_stays_on_the_rule() {
    let r = parse(".dark { --accent: red; color: blue }");
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let rule = &r.stylesheet.rules()[0];
    assert_eq!(
        rule.style.custom_properties,
        vec![rdom_tui::style::CustomDeclaration {
            name: "accent".into(),
            value: "red".into(),
            important: false,
        }]
    );
    assert!(
        rule.style.fg.is_some(),
        "the rest of the block still applies"
    );
    assert_eq!(
        r.stylesheet.var("accent"),
        None,
        "the sheet-level map is define_var's"
    );
}

/// `:root` inside a selector list is one rule for every selector; the
/// declaration rides on each rule, and the root takes it like a lone
/// `:root` rule's.
#[test]
fn root_in_a_selector_list_keeps_the_declaration_on_the_rule() {
    let r = parse(":root, body { --accent: red }");
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    assert!(
        r.stylesheet
            .rules()
            .iter()
            .all(|rule| rule.style.custom_property_value("accent") == Some("red"))
    );
    assert_eq!(root_var_of(&r.stylesheet, "accent").as_deref(), Some("red"));
}

/// A `style="--x: …"` attribute declares the property on that element.
#[test]
fn inline_custom_property_is_kept() {
    let r = rdom_css::parse_inline("--accent: red !important; color: blue");
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    assert!(r.style.fg.is_some());
    let d = &r.style.custom_properties[0];
    assert_eq!(
        (d.name.as_str(), d.value.as_str(), d.important),
        ("accent", "red", true)
    );
}

// ── `:root` in cascade order (C1G-ROOT-SEED, C14G-ROOT-ELEMENT) ─────
//
// The root's custom properties are the cascade's answer for the root:
// CSS Cascade 5 §6.4 (unlayered beats layered for normal declarations,
// later layers beat earlier ones, reversed for `!important`) and Cascade
// 4 §6.4 (important beats normal, then order of appearance — imported
// rules come at the import's position).

fn mirrored(css: &str) -> Option<String> {
    let loader = |url: &str| match url {
        "x.css" => Ok(":root { --c: red }".to_string()),
        other => Err(format!("no {other}")),
    };
    let r = rdom_css::parse_with_loader(css, &loader);
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    root_var_of(&r.stylesheet, "c")
}

#[test]
fn unlayered_root_beats_a_layered_one() {
    assert_eq!(
        mirrored(":root { --c: blue } @layer a { :root { --c: red } }").as_deref(),
        Some("blue")
    );
}

#[test]
fn a_later_layer_beats_an_earlier_one() {
    assert_eq!(
        mirrored("@layer b, a; @layer a { :root { --c: red } } @layer b { :root { --c: green } }")
            .as_deref(),
        Some("red")
    );
}

#[test]
fn important_beats_a_later_normal_declaration() {
    assert_eq!(
        mirrored(":root { --c: blue !important } :root { --c: red }").as_deref(),
        Some("blue")
    );
    assert_eq!(
        mirrored("@layer a { :root { --c: red !important } } :root { --c: blue !important }")
            .as_deref(),
        Some("red"),
        "important layered beats important unlayered"
    );
}

#[test]
fn imported_roots_sit_at_the_import() {
    assert_eq!(
        mirrored("@import 'x.css'; :root { --c: blue }").as_deref(),
        Some("blue")
    );
    assert_eq!(
        mirrored("@import 'x.css' layer(l); :root { --c: blue }").as_deref(),
        Some("blue")
    );
    assert_eq!(
        mirrored("@import 'x.css' layer(l);").as_deref(),
        Some("red")
    );
}

// ── As written (C5G-CUSTOM-SERIALIZE) ──────────────────────────────

/// CSS Variables 1 §2: a custom property's value is its token sequence
/// as written — whitespace between tokens, a comment and the case of an
/// identifier included, leading / trailing whitespace trimmed — and
/// CSSOM reads that text back. A declaration holding `var()` is kept as
/// written until the cascade (§3), so it reads back the same way.
#[test]
fn custom_properties_and_var_declarations_keep_their_text() {
    let r = rdom_css::parse_inline(
        "--x:  1 - 2 ; --y: A  /* c */ b; --z: calc( 1px+2px ); color: var(--c ,  RED ) !important",
    );
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let value = |n: &str| r.style.custom_property_value(n).map(str::to_string);
    assert_eq!(value("x").as_deref(), Some("1 - 2"));
    assert_eq!(value("y").as_deref(), Some("A  /* c */ b"));
    assert_eq!(value("z").as_deref(), Some("calc( 1px+2px )"));
    assert_eq!(
        rdom_style::property_dispatch::serialize("color", &r.style).as_deref(),
        Some("var(--c ,  RED )")
    );
}
