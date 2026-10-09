//! §11.12 — Round-trip with the fluent builder. The CSS parser
//! is an alternate front door; given equivalent input both
//! surfaces produce stylesheets that compare structurally
//! identical (up to UA defaults). These tests lock that property
//! in so future drift is caught early.

use rdom_css::{from_css, from_css_strict};
use rdom_tui::layout::{Display, Padding, PaddingValue, Size};
use rdom_tui::{Color, Stylesheet, TuiColor, TuiStyle};

#[test]
fn single_rule_round_trip() {
    let from_builder = Stylesheet::new()
        .rule(
            "button",
            TuiStyle::new()
                .fg(Color::Rgb(255, 0, 0))
                .bg(Color::Rgb(0, 0, 0)),
        )
        .unwrap();
    let from_parser = from_css("button { color: red; background-color: black; }");

    let b_rules = author_rules(&from_builder);
    let p_rules = author_rules(&from_parser);
    assert_eq!(b_rules.len(), p_rules.len());
    for (b, p) in b_rules.iter().zip(p_rules.iter()) {
        assert_eq!(b.source_text, p.source_text);
        assert_eq!(b.style, p.style);
    }
}

#[test]
fn multiple_rules_round_trip() {
    let from_builder = Stylesheet::new()
        .rule("screen", TuiStyle::new().display(Display::Block).gap(2))
        .unwrap()
        .rule("title", TuiStyle::new().bold(true))
        .unwrap()
        .rule("hint", TuiStyle::new().fg(Color::Rgb(128, 128, 128)))
        .unwrap();

    let css = "screen { display: block; gap: 2; }
               title { font-weight: bold; }
               hint { color: gray; }";
    let from_parser = from_css(css);

    assert_eq!(
        author_rules(&from_builder).len(),
        author_rules(&from_parser).len()
    );
    for (b, p) in author_rules(&from_builder)
        .iter()
        .zip(author_rules(&from_parser).iter())
    {
        assert_eq!(b.source_text, p.source_text);
        assert_eq!(b.style, p.style);
    }
}

#[test]
fn padding_shorthand_round_trip() {
    let from_builder = Stylesheet::new()
        .rule(
            "p",
            TuiStyle::new().padding(Padding {
                top: PaddingValue::Cells(1),
                right: PaddingValue::Cells(2),
                bottom: PaddingValue::Cells(3),
                left: PaddingValue::Cells(4),
            }),
        )
        .unwrap();
    let from_parser = from_css("p { padding: 1 2 3 4; }");
    assert_eq!(
        author_rules(&from_builder)[0].style,
        author_rules(&from_parser)[0].style
    );
}

/// `define_var` and a parsed `:root` rule give the root the same value
/// (the cascade's: the parser no longer mirrors `:root` into the sheet's
/// map, C14G-ROOT-ELEMENT).
#[test]
fn var_definition_round_trip() {
    use rdom_tui::CascadeExt;
    let root_var = |sheet: &Stylesheet| {
        let mut dom: rdom_tui::TuiDom = rdom_tui::TuiDom::new();
        dom.cascade(sheet);
        rdom_tui::style::cascade::computed_of(&dom, dom.root())
            .vars
            .get("accent")
            .map(|v| v.as_str().to_string())
    };
    let from_builder = Stylesheet::new().define_var("accent", "#3d90ce");
    let from_parser = from_css(":root { --accent: #3d90ce; }");
    assert_eq!(root_var(&from_builder).as_deref(), Some("#3d90ce"));
    assert_eq!(root_var(&from_builder), root_var(&from_parser));
}

#[test]
fn var_reference_round_trip() {
    let from_builder = Stylesheet::new()
        .rule("button", TuiStyle::new().fg(TuiColor::var("accent")))
        .unwrap();
    let from_parser = from_css("button { color: var(--accent); }");
    // The builder's typed `TuiColor::Var` and the parser's pending
    // tokens (CSS Variables 1 §3) serialize alike.
    let text = |sheet: &Stylesheet| {
        rdom_css::property_dispatch::serialize("color", &author_rules(sheet)[0].style)
            .map(|s| s.replace(' ', ""))
    };
    assert_eq!(text(&from_builder), Some("var(--accent)".to_string()));
    assert_eq!(text(&from_parser), text(&from_builder));
}

#[test]
fn from_css_includes_ua_defaults() {
    // from_css is the convenience that gives you a working sheet —
    // UA defaults included (so `Stylesheet::new()` semantics).
    let s = from_css("");
    assert!(
        s.rules().len() >= 100,
        "expected UA defaults, got {} rules",
        s.rules().len()
    );
}

#[test]
fn from_css_strict_succeeds_on_clean_input() {
    let result = from_css_strict("a { width: 5; }");
    assert!(result.is_ok());
}

#[test]
fn from_css_strict_errors_on_invalid() {
    let result = from_css_strict("a { unknown-prop: 5; }");
    assert!(result.is_err());
}

#[test]
fn rule_with_size_value_round_trip() {
    let from_builder = Stylesheet::new()
        .rule(
            "col",
            TuiStyle::new()
                .width(Size::Flex(1.0))
                .height(Size::Fixed(3)),
        )
        .unwrap();
    let from_parser = from_css("col { width: 1fr; height: 3; }");
    assert_eq!(
        author_rules(&from_builder)[0].style,
        author_rules(&from_parser)[0].style
    );
}

fn author_rules(s: &Stylesheet) -> Vec<&rdom_tui::style::Rule> {
    s.rules()
        .iter()
        .filter(|r| r.origin == rdom_tui::style::RuleOrigin::Author)
        .collect()
}
