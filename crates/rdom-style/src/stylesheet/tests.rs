//! `Stylesheet` tests: selector-text preprocessing, the builder, the
//! rule index, and cross-builder scenarios.

use super::selector_text::{extract_pseudo_suffix, split_top_level_commas};
use super::*;
use crate::{Color, TuiStyle};

// ── split_top_level_commas ───────────────────────────────────────

#[test]
fn split_empty() {
    assert!(split_top_level_commas("").is_empty());
    assert!(split_top_level_commas("   ").is_empty());
}

#[test]
fn split_no_commas() {
    assert_eq!(split_top_level_commas("div.foo"), vec!["div.foo"]);
}

#[test]
fn split_simple_list() {
    assert_eq!(split_top_level_commas("a, b, c"), vec!["a", " b", " c"]);
}

#[test]
fn split_preserves_not_parens() {
    // :not(a, b) has an internal comma that must not split.
    assert_eq!(
        split_top_level_commas("div:not(a, b), span"),
        vec!["div:not(a, b)", " span"]
    );
}

#[test]
fn split_preserves_attribute_brackets() {
    // Attribute values with quoted commas (rare but valid).
    assert_eq!(
        split_top_level_commas(r#"[data-x="a,b"], .foo"#),
        vec![r#"[data-x="a,b"]"#, " .foo"]
    );
}

// ── extract_pseudo_suffix ────────────────────────────────────────

/// `extract_pseudo_suffix` with an owned core, so assertions compare
/// against string literals.
fn extract(selector: &str) -> Result<(String, PseudoElementTarget), String> {
    extract_pseudo_suffix(selector).map(|(core, pseudo)| (core.into_owned(), pseudo))
}

#[test]
fn extract_no_pseudo() {
    assert_eq!(
        extract("div.foo").unwrap(),
        ("div.foo".into(), PseudoElementTarget::None)
    );
}

#[test]
fn extract_before() {
    assert_eq!(
        extract("tree-item::before").unwrap(),
        ("tree-item".into(), PseudoElementTarget::Before)
    );
}

#[test]
fn extract_after() {
    assert_eq!(
        extract("dialog .close::after").unwrap(),
        ("dialog .close".into(), PseudoElementTarget::After)
    );
}

#[test]
fn extract_tolerates_trailing_whitespace() {
    assert_eq!(
        extract("h1::before   ").unwrap(),
        ("h1".into(), PseudoElementTarget::Before)
    );
}

/// Selectors 4 §5.2: a compound without a type selector has an
/// implicit `*`, so a pseudo-element with nothing (or only a combinator
/// or whitespace) before it attaches to `*` — `::before` is `*::before`,
/// `div ::before` is `div *::before`, `div > ::after` is `div > *::after`
/// (C5G-BARE-PSEUDO).
#[test]
fn extract_bare_pseudo_attaches_to_an_implicit_universal() {
    let core = |s: &str| extract(s).unwrap();
    assert_eq!(core("::before"), ("*".into(), PseudoElementTarget::Before));
    assert_eq!(core("::after"), ("*".into(), PseudoElementTarget::After));
    assert_eq!(
        core("div ::before"),
        ("div *".into(), PseudoElementTarget::Before)
    );
    assert_eq!(
        core("div > ::after"),
        ("div > *".into(), PseudoElementTarget::After)
    );
    assert_eq!(
        core("div>::after"),
        ("div>*".into(), PseudoElementTarget::After)
    );
    assert_eq!(
        core("h1 + ::before"),
        ("h1 + *".into(), PseudoElementTarget::Before)
    );
    assert_eq!(
        core("h1 ~::before"),
        ("h1 ~*".into(), PseudoElementTarget::Before)
    );
    assert_eq!(
        core("::placeholder"),
        ("*".into(), PseudoElementTarget::Placeholder)
    );
    assert_eq!(
        core("::selection"),
        ("*".into(), PseudoElementTarget::Selection)
    );
    assert_eq!(
        core("::scrollbar"),
        ("*".into(), PseudoElementTarget::Scrollbar)
    );
    assert_eq!(
        core("::scrollbar-thumb:vertical"),
        ("*".into(), PseudoElementTarget::ScrollbarThumbVertical)
    );
    // An escaped space is part of the identifier: `.a\ ` is a compound.
    assert_eq!(
        core(".a\\ ::before"),
        (".a\\ ".into(), PseudoElementTarget::Before)
    );
}

/// Nested, a bare pseudo-element is relative to the parent: `.a {
/// ::before {} }` is `.a *::before` (CSS Nesting 1 §2) — the same
/// selector as the explicit `& ::before`.
#[test]
fn a_nested_bare_pseudo_element_is_relative() {
    let parent = StyleSelector::parse(".a").unwrap();
    let bare = StyleSelector::parse_nested("::before", &parent).unwrap();
    let explicit = StyleSelector::parse_nested("& *::before", &parent).unwrap();
    let mut a = Stylesheet::bare();
    a.add_style_rule(&bare, TuiStyle::new(), RuleContext::default());
    let mut b = Stylesheet::bare();
    b.add_style_rule(&explicit, TuiStyle::new(), RuleContext::default());
    assert_eq!(a.rules()[0].pseudo, PseudoElementTarget::Before);
    assert_eq!(a.rules()[0].selector, b.rules()[0].selector);
}

/// The Tailwind preflight / modern-normalize reset, `*, ::before,
/// ::after`, parses into three items — none of them dropped
/// (C5G-BARE-PSEUDO).
#[test]
fn the_bare_pseudo_reset_list_parses() {
    let sel = StyleSelector::parse("*, ::before, ::after").expect("the reset parses");
    let mut sheet = Stylesheet::bare();
    sheet.add_style_rule(&sel, TuiStyle::new(), RuleContext::default());
    let pseudos: Vec<_> = sheet.rules().iter().map(|r| r.pseudo).collect();
    assert_eq!(
        pseudos,
        vec![
            PseudoElementTarget::None,
            PseudoElementTarget::Before,
            PseudoElementTarget::After
        ]
    );
}

#[test]
fn extract_rejects_unsupported_pseudo_element() {
    assert!(extract("p::grammar-error").is_err());
}

// ── Legacy single-colon pseudo-elements (C10-LEGACY-COLON) ─────────

/// Selectors 4 §15 (CSS 2.1 compatibility): `:before`, `:after`,
/// `:first-line` and `:first-letter` are the pseudo-elements of the same
/// name written with one colon, in any ASCII case.
#[test]
fn legacy_single_colon_pseudo_elements_are_pseudo_elements() {
    assert_eq!(
        extract("p:before").unwrap(),
        ("p".into(), PseudoElementTarget::Before)
    );
    assert_eq!(
        extract("li.x:AFTER").unwrap(),
        ("li.x".into(), PseudoElementTarget::After)
    );
    assert_eq!(
        extract("a:hover:before").unwrap(),
        ("a:hover".into(), PseudoElementTarget::Before)
    );
    assert_eq!(
        extract("p:first-line").unwrap(),
        ("p".into(), PseudoElementTarget::FirstLine)
    );
    assert_eq!(
        extract("p:first-letter").unwrap(),
        ("p".into(), PseudoElementTarget::FirstLetter)
    );
    assert_eq!(
        extract(":before").unwrap(),
        ("*".into(), PseudoElementTarget::Before),
        "a bare legacy pseudo-element attaches to the implicit `*`"
    );
}

/// The double-colon `::first-line` / `::first-letter` (CSS Pseudo 4 §2)
/// parse as their pseudo-elements, and pseudo-element names are ASCII
/// case-insensitive (Selectors 4 §4.1).
#[test]
fn first_line_and_first_letter_parse_and_names_ignore_case() {
    assert_eq!(
        extract("p::first-line").unwrap(),
        ("p".into(), PseudoElementTarget::FirstLine)
    );
    assert_eq!(
        extract("p::First-Letter").unwrap(),
        ("p".into(), PseudoElementTarget::FirstLetter)
    );
    assert_eq!(
        extract("p::BEFORE").unwrap(),
        ("p".into(), PseudoElementTarget::Before)
    );
}

/// An escaped colon is part of an identifier (CSS Syntax 3 §4.3.7):
/// `.a\:before` is the class `a:before`, not a pseudo-element.
#[test]
fn an_escaped_colon_is_no_legacy_pseudo_element() {
    assert_eq!(
        extract(r".a\:before").unwrap(),
        (r".a\:before".into(), PseudoElementTarget::None)
    );
}

/// The legacy spelling carries a pseudo-element's specificity (0,0,1),
/// the same rule as its double-colon twin.
#[test]
fn a_legacy_pseudo_element_has_pseudo_element_specificity() {
    let legacy = StyleSelector::parse("p:before").unwrap();
    let modern = StyleSelector::parse("p::before").unwrap();
    let mut a = Stylesheet::bare();
    a.add_style_rule(&legacy, TuiStyle::new(), RuleContext::default());
    let mut b = Stylesheet::bare();
    b.add_style_rule(&modern, TuiStyle::new(), RuleContext::default());
    assert_eq!(a.rules()[0].specificity, b.rules()[0].specificity);
    assert_eq!(a.rules()[0].selector, b.rules()[0].selector);
    assert_eq!(a.rules()[0].pseudo, PseudoElementTarget::Before);
}

#[test]
fn extract_selection() {
    assert_eq!(
        extract("p::selection").unwrap(),
        ("p".into(), PseudoElementTarget::Selection)
    );
    assert_eq!(
        extract("article .body::selection").unwrap(),
        ("article .body".into(), PseudoElementTarget::Selection)
    );
}

#[test]
fn extract_rejects_multiple_pseudo_suffixes() {
    assert!(extract("p::before::after").is_err());
}

#[test]
fn extract_scrollbar() {
    // `::scrollbar-thumb` must split before `::scrollbar` — if
    // the parser stripped `::scrollbar` first the leftover
    // would be `-thumb` (invalid). Order matters.
    assert_eq!(
        extract("*::scrollbar-thumb").unwrap(),
        ("*".into(), PseudoElementTarget::ScrollbarThumb)
    );
    assert_eq!(
        extract("*::scrollbar").unwrap(),
        ("*".into(), PseudoElementTarget::Scrollbar)
    );
    assert_eq!(
        extract(".sidebar::scrollbar-thumb").unwrap(),
        (".sidebar".into(), PseudoElementTarget::ScrollbarThumb)
    );
}

// ── Stylesheet builder ───────────────────────────────────────────

/// `CASCADE-INITIAL-ALLOC-1`: for every UA rule (plus a few author
/// shapes), an element carrying the subject's tag / id / classes gets
/// that rule back from the index — the index never hides a match.
#[test]
fn rule_index_never_drops_a_matching_rule() {
    use rdom_core::selectors::SimpleSelector;
    let sheet = Stylesheet::new()
        .rule_unchecked("#hero.big span", TuiStyle::new())
        .rule_unchecked("*:hover", TuiStyle::new())
        .rule_unchecked("[role=tree] > li.leaf", TuiStyle::new())
        .rule_unchecked(":not(.x)", TuiStyle::new())
        .rule_unchecked("DIV.Mixed", TuiStyle::new());
    let index = sheet.rule_index();
    let mut out = Vec::new();
    for (i, rule) in sheet.rules().iter().enumerate() {
        let simples = &rule.selector.0[0].subject.simples;
        let tag = simples.iter().find_map(|s| match s {
            SimpleSelector::Type(t) => Some(t.as_str()),
            _ => None,
        });
        let id = simples.iter().find_map(|s| match s {
            SimpleSelector::Id(v) => Some(v.as_str()),
            _ => None,
        });
        let classes: Vec<&str> = simples
            .iter()
            .filter_map(|s| match s {
                SimpleSelector::Class(c) => Some(c.as_str()),
                _ => None,
            })
            .collect();
        index.candidates(tag, id, classes.iter().copied(), &mut out);
        assert!(
            out.contains(&(i as u32)),
            "rule {i} `{}` missing",
            rule.source_text
        );
        assert!(out.windows(2).all(|w| w[0] < w[1]), "sorted, deduplicated");
    }
    // Case is exact on both sides, like the matcher: a `DIV` rule is a
    // candidate for a `DIV` element and not for a `div` one.
    index.candidates(Some("DIV"), None, ["Mixed"].into_iter(), &mut out);
    assert!(
        out.iter()
            .any(|&i| sheet.rules()[i as usize].source_text == "DIV.Mixed")
    );
    index.candidates(Some("div"), None, ["mixed"].into_iter(), &mut out);
    assert!(
        !out.iter()
            .any(|&i| sheet.rules()[i as usize].source_text == "DIV.Mixed")
    );
    // Nothing keyed leaks onto an element it cannot match.
    index.candidates(Some("zzz"), None, std::iter::empty(), &mut out);
    assert!(out.iter().all(|&i| {
        let simples = &sheet.rules()[i as usize].selector.0[0].subject.simples;
        !simples
            .iter()
            .any(|s| matches!(s, SimpleSelector::Id(_) | SimpleSelector::Class(_)))
    }));
}

/// `C1G-SCOPE-COST`: a subject whose only keys are inside `:is()` — a
/// nested rule under a list parent, `.a, .b { &:hover {} }`, is
/// `:is(.a, .b):hover` (CSS Nesting 1 §2) — is indexed under each
/// argument's key, so it is a candidate for elements carrying one of
/// them and not for every element; one argument without a key keeps
/// the rule a candidate for all.
#[test]
fn is_arguments_key_the_rule_index() {
    let sheet = Stylesheet::bare()
        .rule_unchecked(":is(.a, #x, p > b):hover", TuiStyle::new())
        .rule_unchecked(":is(.a, *):focus", TuiStyle::new());
    let index = sheet.rule_index();
    let mut out = Vec::new();
    let empty = || std::iter::empty::<&str>();
    index.candidates(Some("span"), None, ["a"].into_iter(), &mut out);
    assert!(out.contains(&0), "class argument");
    index.candidates(Some("span"), Some("x"), empty(), &mut out);
    assert!(out.contains(&0), "id argument");
    index.candidates(Some("b"), None, empty(), &mut out);
    assert!(out.contains(&0), "the argument's subject, `b`");
    index.candidates(Some("span"), None, ["c"].into_iter(), &mut out);
    assert!(!out.contains(&0), "no argument's key: not a candidate");
    assert!(out.contains(&1), "an unkeyed argument keeps it universal");
}

#[test]
fn bare_has_no_rules() {
    assert_eq!(Stylesheet::bare().rules().len(), 0);
}

#[test]
fn rule_adds_author_rule() {
    let s = Stylesheet::bare()
        .rule("div.hero", TuiStyle::new().fg(Color::Rgb(255, 0, 0)))
        .unwrap();
    assert_eq!(s.rules().len(), 1);
    assert_eq!(s.rules()[0].origin, RuleOrigin::Author);
    assert_eq!(s.rules()[0].source_text, "div.hero");
}

#[test]
fn rule_computes_specificity() {
    let s = Stylesheet::bare()
        .rule("#main", TuiStyle::new().fg(Color::Rgb(255, 0, 0)))
        .unwrap()
        .rule("div", TuiStyle::new().fg(Color::Rgb(0, 0, 255)))
        .unwrap();
    assert!(s.rules()[0].specificity > s.rules()[1].specificity);
}

#[test]
fn rule_assigns_monotonic_source_idx() {
    let s = Stylesheet::bare()
        .rule("a", TuiStyle::new())
        .unwrap()
        .rule("b", TuiStyle::new())
        .unwrap()
        .rule("c", TuiStyle::new())
        .unwrap();
    let idxs: Vec<u32> = s.rules().iter().map(|r| r.source_idx).collect();
    assert_eq!(idxs, vec![0, 1, 2]);
}

#[test]
fn rule_expands_selector_list() {
    let s = Stylesheet::bare()
        .rule("a, b, c.foo", TuiStyle::new().fg(Color::Rgb(255, 0, 0)))
        .unwrap();
    assert_eq!(s.rules().len(), 3);
    assert_eq!(s.rules()[0].source_text, "a");
    assert_eq!(s.rules()[1].source_text, "b");
    assert_eq!(s.rules()[2].source_text, "c.foo");
    // Each one parses as a single complex selector, so specificities differ.
    // a + b are equal (type 1 each); c.foo is higher.
    assert_eq!(s.rules()[0].specificity, s.rules()[1].specificity);
    assert!(s.rules()[2].specificity > s.rules()[0].specificity);
}

#[test]
fn rule_handles_pseudo_element() {
    let s = Stylesheet::bare()
        .rule(
            "tree-item::before",
            TuiStyle::new().fg(Color::Rgb(255, 0, 0)),
        )
        .unwrap();
    assert_eq!(s.rules()[0].pseudo, PseudoElementTarget::Before);
    // source_text preserves the full selector for debug/devtools —
    // the pseudo suffix has already been handled by `pseudo`.
    assert_eq!(s.rules()[0].source_text, "tree-item::before");
}

#[test]
fn rule_handles_mixed_pseudo_list() {
    let s = Stylesheet::bare()
        .rule(
            "a, b::before, c::after",
            TuiStyle::new().fg(Color::Rgb(255, 0, 0)),
        )
        .unwrap();
    assert_eq!(s.rules().len(), 3);
    assert_eq!(s.rules()[0].pseudo, PseudoElementTarget::None);
    assert_eq!(s.rules()[1].pseudo, PseudoElementTarget::Before);
    assert_eq!(s.rules()[2].pseudo, PseudoElementTarget::After);
}

#[test]
fn rule_pseudo_specificity_adds_type_bump() {
    let s = Stylesheet::bare()
        .rule("a", TuiStyle::new())
        .unwrap()
        .rule("a::before", TuiStyle::new())
        .unwrap();
    // Both share the `a` type count; `a::before` has one more.
    assert!(s.rules()[1].specificity > s.rules()[0].specificity);
}

#[test]
fn rule_rejects_invalid_selector() {
    let err = Stylesheet::bare()
        .rule("div..foo", TuiStyle::new())
        .unwrap_err();
    assert!(err.msg.contains("empty class selector") || err.msg.contains("class"));
    assert_eq!(err.source, "div..foo");
}

#[test]
fn rule_rejects_empty_selector_string() {
    assert!(Stylesheet::bare().rule("", TuiStyle::new()).is_err());
    assert!(Stylesheet::bare().rule("   ", TuiStyle::new()).is_err());
}

#[test]
fn rule_rejects_empty_item_in_list() {
    assert!(Stylesheet::bare().rule("a, , b", TuiStyle::new()).is_err());
}

#[test]
fn rule_unchecked_panics_on_error() {
    let result =
        std::panic::catch_unwind(|| Stylesheet::bare().rule_unchecked("div..foo", TuiStyle::new()));
    assert!(result.is_err());
}

#[test]
fn rule_unchecked_succeeds_on_valid() {
    let s = Stylesheet::bare().rule_unchecked("div", TuiStyle::new());
    assert_eq!(s.rules().len(), 1);
}

#[test]
fn define_var_stores_value() {
    let s = Stylesheet::bare()
        .define_var("accent", "#ff0000")
        .define_var("muted", "#888");
    assert_eq!(s.var("accent"), Some("#ff0000"));
    assert_eq!(s.var("muted"), Some("#888"));
    assert!(s.var("nope").is_none());
}

#[test]
fn define_var_overwrites() {
    let s = Stylesheet::bare().define_var("x", "1").define_var("x", "2");
    assert_eq!(s.var("x"), Some("2"));
}

#[test]
fn define_var_mut_accumulates_in_a_loop() {
    // The borrow-by-value fluent `define_var` forces consumers to
    // either chain inline or `mem::take` the sheet to pass it
    // through. `define_var_mut` takes `&mut self` and returns
    // `&mut Self` so accumulation in a loop Just Works.
    let mut s = Stylesheet::bare();
    for (name, value) in [("a", "#111"), ("b", "#222"), ("c", "#333")] {
        s.define_var_mut(name, value);
    }
    assert_eq!(s.var("a"), Some("#111"));
    assert_eq!(s.var("b"), Some("#222"));
    assert_eq!(s.var("c"), Some("#333"));
}

#[test]
fn define_var_mut_chains_via_returned_ref() {
    let mut s = Stylesheet::bare();
    s.define_var_mut("a", "1")
        .define_var_mut("b", "2")
        .define_var_mut("c", "3");
    assert_eq!(s.var("a"), Some("1"));
    assert_eq!(s.var("b"), Some("2"));
    assert_eq!(s.var("c"), Some("3"));
}

#[test]
fn define_var_mut_overwrites_like_define_var() {
    let mut s = Stylesheet::bare();
    s.define_var_mut("x", "1");
    s.define_var_mut("x", "2");
    assert_eq!(s.var("x"), Some("2"));
}

// ── Cross-builder scenarios ──────────────────────────────────────

#[test]
fn ua_rule_comes_before_author_rule_in_source_order() {
    let s = Stylesheet::new()
        .rule(".my-button", TuiStyle::new().fg(Color::Rgb(255, 0, 0)))
        .unwrap();
    // All UA rules precede any author rule. Phase E extended the
    // UA default set, so the author rule lands at the tail.
    let last = s.rules().last().unwrap();
    assert_eq!(last.origin, RuleOrigin::Author);
    for r in s.rules().iter().take(s.rules().len() - 1) {
        assert_eq!(r.origin, RuleOrigin::UserAgent);
    }
}

#[test]
fn complex_selector_preserved_in_rule() {
    let s = Stylesheet::bare()
        .rule("div.a > span", TuiStyle::new())
        .unwrap();
    let complex = &s.rules()[0].selector.0[0];
    // Subject is span; one ancestor (div.a with Child combinator)
    assert_eq!(complex.ancestors.len(), 1);
}

#[test]
fn not_with_internal_comma_not_split() {
    let s = Stylesheet::bare()
        .rule(":not(a, b)", TuiStyle::new())
        .unwrap();
    // One rule (not two).
    assert_eq!(s.rules().len(), 1);
}

#[test]
fn source_idx_continues_across_multiple_rule_calls_with_lists() {
    let s = Stylesheet::bare()
        .rule("a, b", TuiStyle::new()) // idxs 0, 1
        .unwrap()
        .rule("c, d, e", TuiStyle::new()) // idxs 2, 3, 4
        .unwrap();
    let idxs: Vec<u32> = s.rules().iter().map(|r| r.source_idx).collect();
    assert_eq!(idxs, vec![0, 1, 2, 3, 4]);
}

#[test]
fn ua_rule_specificity_is_tiny() {
    let s = Stylesheet::new();
    let ua = &s.rules()[0];
    // [disabled] is 1 class-level, 0 ids, 0 types.
    assert_eq!(ua.specificity.id, 0);
    assert_eq!(ua.specificity.class_attr_pseudo, 1);
    assert_eq!(ua.specificity.type_pseudo_el, 0);
}

#[test]
fn default_is_empty() {
    // Default::default is the `bare` constructor, not `new`.
    assert_eq!(Stylesheet::default().rules().len(), 0);
}

#[test]
fn error_display_shows_position() {
    let err = Stylesheet::bare()
        .rule("div..foo", TuiStyle::new())
        .unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("div..foo"));
}

// ── ::placeholder (CSS Pseudo-Elements 4 §4.3) ──────────────────────

#[test]
fn extract_placeholder() {
    assert_eq!(
        extract("input::placeholder").unwrap(),
        ("input".into(), PseudoElementTarget::Placeholder)
    );
}

/// Only the properties that apply to `::first-line` apply to
/// `::placeholder` (CSS Pseudo-Elements 4 §4.3): in rdom that is color,
/// background, font weight / style, `text-decoration`, `opacity` and
/// custom properties. Anything else (here `width`, `content`,
/// `display`) is dropped from the rule when it is built, with its
/// `!important` bit. A `::placeholder` rule is a pseudo-element rule
/// for specificity.
#[test]
fn placeholder_rules_keep_only_first_line_properties() {
    use crate::Content;
    use crate::layout::TextDecoration;
    use crate::layout::{Display, Size};
    let style = TuiStyle::new()
        .fg_important(Color::Rgb(255, 0, 0))
        .bg(Color::Rgb(0, 0, 255))
        .bold(true)
        .italic(true)
        .opacity(0.5)
        .text_decoration(TextDecoration::Underline)
        .width_important(Size::Fixed(3))
        .display(Display::Block)
        .content(Content::Str("x".into()));
    let sheet = Stylesheet::bare().rule_unchecked("input::placeholder", style.clone());
    let rule = &sheet.rules()[0];
    assert_eq!(rule.pseudo, PseudoElementTarget::Placeholder);
    let kept = &rule.style;
    assert_eq!(kept.fg, style.fg);
    assert_eq!(kept.bg, style.bg);
    assert_eq!(kept.font, style.font);
    assert_eq!(kept.opacity, style.opacity);
    assert_eq!(kept.text_decoration, style.text_decoration);
    assert_eq!(kept.width, None);
    assert_eq!(kept.display, None);
    assert_eq!(kept.content, None);
    assert!(kept.important.contains(crate::ImportantMask::FG));
    assert!(!kept.important.contains(crate::ImportantMask::WIDTH));

    let before = Stylesheet::bare().rule_unchecked("input::before", TuiStyle::new());
    assert_eq!(rule.specificity, before.rules()[0].specificity);
}

/// `C2G-STATELESS-REGISTRY`: a sheet's version stays while it is
/// unchanged, and every mutation — and a clone — gives a new one.
#[test]
fn version_is_renewed_by_mutation_and_clone() {
    let mut sheet = Stylesheet::bare();
    let v = sheet.version();
    assert_eq!(sheet.version(), v, "reading changes nothing");
    sheet.add_rule("p", TuiStyle::new()).unwrap();
    let after_rule = sheet.version();
    assert_ne!(after_rule, v);
    sheet.define_var_mut("x", "1");
    assert_ne!(sheet.version(), after_rule);
    let clone = sheet.clone();
    assert_ne!(clone.version(), sheet.version(), "a clone is another sheet");
    let before = sheet.version();
    sheet.register_property(crate::PropertyRegistration {
        name: "w".into(),
        syntax: crate::PropertySyntax::Universal,
        inherits: false,
        initial_value: None,
    });
    assert_ne!(sheet.version(), before);
}

// ── ::marker (C10-LIST-ITEM) ────────────────────────────────────────

/// CSS Pseudo-Elements 4 §3.1: `li::marker` targets the marker; CSS
/// Lists 3 §3.2: only `color`, the font properties, `white-space`,
/// `content`, `direction` and the animation / transition properties
/// apply to it — a rule keeps those and drops the rest, `!important`
/// bits included.
#[test]
fn marker_rules_keep_only_the_marker_properties() {
    assert_eq!(
        extract("li::marker").unwrap(),
        ("li".into(), PseudoElementTarget::Marker)
    );
    let sheet = rdom_css_like(
        "li::marker",
        &[
            ("color", "red"),
            ("font-weight", "bold"),
            ("white-space", "pre"),
            ("content", "\"x\""),
            ("direction", "rtl"),
            ("transition-duration", "1s"),
            ("padding", "3"),
            ("text-transform", "uppercase"),
            ("display", "block"),
        ],
    );
    let rule = &sheet.rules()[0];
    let s = &rule.style;
    assert!(s.fg.is_some() && s.font.weight.is_some());
    assert!(s.text.white_space_collapse.is_some() && s.content.is_some());
    assert!(s.text_direction.is_some() && s.transition_duration.is_some());
    assert!(
        s.padding.top.is_none(),
        "padding does not apply to ::marker"
    );
    assert!(s.text.text_transform.is_none());
    assert!(s.display.is_none());
}

/// A sheet with one `selector` rule declaring `decls` (through the
/// property dispatch, as the CSS parser would).
fn rdom_css_like(selector: &str, decls: &[(&str, &str)]) -> Stylesheet {
    let mut style = TuiStyle::new();
    for (name, value) in decls {
        crate::property_dispatch::set(name, value, &mut style).unwrap();
    }
    let sel = StyleSelector::parse(selector).unwrap();
    let mut sheet = Stylesheet::bare();
    sheet.add_style_rule(&sel, style, RuleContext::default());
    sheet
}
