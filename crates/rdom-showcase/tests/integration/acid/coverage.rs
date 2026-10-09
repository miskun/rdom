//! ACID-COVERAGE (`specs/ACID.md` ground rule 4): the acid page uses every
//! CSS feature rdom dispatches, so a new one cannot skip it.
//!
//! "Used" is mechanical: the tiles' own CSS — each tile's sheet, its late
//! sheet, the `<style>` elements and `style` attributes in its markup and
//! the sheets those import (not the page frame's rules, which only place
//! the tiles) — is parsed and walked:
//!
//! - a **property** is used when a declaration of it parses on its own
//!   without a warning (`rdom_css::parse_inline`), wherever it stands — a
//!   style rule, a nested rule, a keyframe, a `@position-try` rule, a
//!   `style` attribute;
//! - a **pseudo-class** is used when a parsed rule's selector holds it —
//!   inside `:not()` / `:is()` / `:where()` / `:has()` / `of S` too — or a
//!   rule's pseudo-element takes it (`::before:hover`);
//! - a **pseudo-element** is used when a parsed rule targets it;
//! - an **at-rule** is used when it opens a rule anywhere in the text.
//!
//! What must be used is read from the crates that dispatch it:
//! `property_dispatch::property_names()`, `selectors::pseudo_class_names()`
//! (each parsed to its `PseudoClass`), `PseudoElementTarget::named()` and
//! `rdom_css::at_rule_names()`.

use std::collections::{BTreeSet, HashSet};
use std::mem::{Discriminant, discriminant};

use rdom_showcase::demos::acid;
use rdom_tui::core_api::selectors::{
    self, ComplexSelector, PseudoClass, SelectorList, SimpleSelector,
};
use rdom_tui::{NodeType, PseudoElementTarget, TuiDom, UserActionState};

/// Properties exempt from the coverage rule, each with its reason. Kept as
/// short as possible: a property that draws nothing in a terminal is still
/// used on the page (its tile says so in its derivation); only one with no
/// static meaning at all belongs here.
const EXEMPT_PROPERTIES: &[(&str, &str)] = &[];

/// Every CSS text the tiles carry, and the inline declarations of their
/// markup's `style` attributes.
struct PageCss {
    sheets: Vec<String>,
    inline: Vec<String>,
}

fn page_css() -> PageCss {
    let mut sheets: Vec<String> = Vec::new();
    let mut inline: Vec<String> = Vec::new();
    for t in acid::TILES {
        sheets.push(t.css.to_string());
        sheets.push(t.late_css.to_string());
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        rdom_parser::parse_into(&mut dom, t.markup, root).expect("tile markup parses");
        for id in dom.descendants(root).collect::<Vec<_>>() {
            let node = dom.node(id);
            if node.node_type() != NodeType::Element {
                continue;
            }
            if node.tag_name() == Some("style") {
                sheets.push(node.text_content());
            }
            if let Some(style) = node.get_attribute("style") {
                inline.push(style.to_string());
            }
        }
    }
    for imports in acid::tiles::IMPORTS {
        sheets.extend(imports.iter().map(|(_, text)| text.to_string()));
    }
    PageCss { sheets, inline }
}

// ── A small CSS walker: declarations and at-rules ────────────────────

/// `text` without its comments.
fn strip_comments(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find("/*") {
        out.push_str(&rest[..i]);
        rest = rest[i + 2..].split_once("*/").map_or("", |(_, r)| r);
    }
    out.push_str(rest);
    out
}

/// Split a block's contents (or a sheet) into its items: `(prelude,
/// Some(block))` for a rule, `(text, None)` for a declaration or a
/// statement — at depth 0, strings and parentheses respected.
fn items(text: &str) -> Vec<(String, Option<String>)> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let (mut start, mut i, mut parens) = (0, 0, 0i32);
    while i < chars.len() {
        match chars[i] {
            '"' | '\'' => {
                let q = chars[i];
                i += 1;
                while i < chars.len() && chars[i] != q {
                    i += if chars[i] == '\\' { 2 } else { 1 };
                }
            }
            '(' => parens += 1,
            ')' => parens -= 1,
            ';' if parens == 0 => {
                out.push((chars[start..i].iter().collect(), None));
                start = i + 1;
            }
            '{' if parens == 0 => {
                let prelude: String = chars[start..i].iter().collect();
                let (mut depth, mut j) = (1, i + 1);
                while j < chars.len() && depth > 0 {
                    match chars[j] {
                        '{' => depth += 1,
                        '}' => depth -= 1,
                        '"' | '\'' => {
                            let q = chars[j];
                            j += 1;
                            while j < chars.len() && chars[j] != q {
                                j += if chars[j] == '\\' { 2 } else { 1 };
                            }
                        }
                        _ => {}
                    }
                    j += 1;
                }
                let block: String = chars[i + 1..j.saturating_sub(1)].iter().collect();
                out.push((prelude, Some(block)));
                start = j;
                i = j;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    out.push((chars[start..].iter().collect(), None));
    out
}

/// Walk `text`'s items: every declaration that parses on its own counts
/// its property in `properties`; every at-rule its name in `at_rules`.
fn walk(text: &str, properties: &mut BTreeSet<String>, at_rules: &mut BTreeSet<String>) {
    for (head, block) in items(text) {
        let head = head.trim();
        if let Some(name) = head.strip_prefix('@') {
            let name: String = name
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect();
            at_rules.insert(name.to_ascii_lowercase());
        }
        match block {
            Some(block) => walk(&block, properties, at_rules),
            None => declaration(head, properties),
        }
    }
}

/// Count `text` in `properties` when it is a declaration that parses.
fn declaration(text: &str, properties: &mut BTreeSet<String>) {
    let Some((name, _)) = text.split_once(':') else {
        return;
    };
    let name = name.trim().to_ascii_lowercase();
    if name.is_empty() || name.starts_with("--") || name.starts_with('@') {
        return;
    }
    let parsed = rdom_css::parse_inline(text);
    if parsed.warnings.is_empty() {
        properties.insert(name);
    }
}

// ── Selectors ────────────────────────────────────────────────────────

fn list_classes(list: &SelectorList, out: &mut HashSet<Discriminant<PseudoClass>>) {
    for complex in &list.0 {
        complex_classes(complex, out);
    }
}

fn complex_classes(complex: &ComplexSelector, out: &mut HashSet<Discriminant<PseudoClass>>) {
    let compounds =
        std::iter::once(&complex.subject).chain(complex.ancestors.iter().map(|(_, c)| c));
    for compound in compounds {
        for simple in &compound.simples {
            match simple {
                SimpleSelector::Pseudo(p) => {
                    out.insert(discriminant(p));
                }
                SimpleSelector::Not(l) | SimpleSelector::Is(l) | SimpleSelector::Where(l) => {
                    list_classes(l, out)
                }
                SimpleSelector::Has(relative) => {
                    for r in relative {
                        complex_classes(&r.selector, out);
                    }
                }
                SimpleSelector::Nth(nth) => {
                    if let Some(of) = &nth.of {
                        list_classes(of, out);
                    }
                }
                _ => {}
            }
        }
    }
}

/// The pseudo-classes a `::pseudo:state` rule's state names.
fn state_classes(state: UserActionState, out: &mut HashSet<Discriminant<PseudoClass>>) {
    for (flag, class) in [
        (UserActionState::HOVER, PseudoClass::Hover),
        (UserActionState::ACTIVE, PseudoClass::Active),
        (UserActionState::FOCUS, PseudoClass::Focus),
        (UserActionState::FOCUS_VISIBLE, PseudoClass::FocusVisible),
        (UserActionState::FOCUS_WITHIN, PseudoClass::FocusWithin),
    ] {
        if state.contains(flag) {
            out.insert(discriminant(&class));
        }
    }
}

/// The `PseudoClass` the selector parser makes of `name`.
fn class_named(name: &str) -> PseudoClass {
    let text = if name == "dir" {
        ":dir(ltr)".to_string()
    } else {
        format!(":{name}")
    };
    let list = selectors::parse(&text).expect("a listed pseudo-class parses");
    match list.0[0].subject.simples.as_slice() {
        [SimpleSelector::Pseudo(p)] => *p,
        other => panic!("{text} parsed to {other:?}"),
    }
}

/// What the acid page leaves unused.
#[derive(Default)]
struct Gaps {
    properties: Vec<String>,
    pseudo_classes: Vec<String>,
    pseudo_elements: Vec<String>,
    at_rules: Vec<String>,
}

fn gaps() -> Gaps {
    let css = page_css();
    let (mut properties, mut at_rules) = (BTreeSet::new(), BTreeSet::new());
    let mut classes = HashSet::new();
    let mut elements = HashSet::new();
    for text in &css.sheets {
        let text = strip_comments(text);
        walk(&text, &mut properties, &mut at_rules);
        for rule in rdom_css::parse(&text).stylesheet.rules() {
            list_classes(&rule.selector, &mut classes);
            state_classes(rule.pseudo_state, &mut classes);
            elements.insert(discriminant(&rule.pseudo));
        }
    }
    for text in &css.inline {
        for item in text.split(';') {
            declaration(item.trim(), &mut properties);
        }
    }
    let exempt: HashSet<&str> = EXEMPT_PROPERTIES.iter().map(|(n, _)| *n).collect();
    let mut gaps = Gaps::default();
    for name in rdom_css::property_dispatch::property_names() {
        if !properties.contains(*name) && !exempt.contains(name) {
            gaps.properties.push((*name).to_string());
        }
    }
    for name in selectors::pseudo_class_names() {
        if !classes.contains(&discriminant(&class_named(name))) {
            gaps.pseudo_classes.push(format!(":{name}"));
        }
    }
    let mut seen = HashSet::new();
    for target in PseudoElementTarget::named() {
        let kind = discriminant(&target);
        if seen.insert(kind) && !elements.contains(&kind) {
            gaps.pseudo_elements.push(format!("{target:?}"));
        }
    }
    for name in rdom_css::at_rule_names() {
        if !at_rules.contains(name) {
            gaps.at_rules.push(format!("@{name}"));
        }
    }
    gaps
}

/// Every property rdom dispatches (bar the commented exemptions), every
/// pseudo-class and pseudo-element its selectors parse and every at-rule
/// it evaluates is used by some tile (ACID.md ground rule 4).
#[test]
fn the_acid_page_uses_every_css_feature_rdom_dispatches() {
    let g = gaps();
    let total =
        g.properties.len() + g.pseudo_classes.len() + g.pseudo_elements.len() + g.at_rules.len();
    assert!(
        total == 0,
        "the acid page does not use {total} features — give each a place in a tile, with its derived cells:\n\
         properties ({}): {}\npseudo-classes ({}): {}\npseudo-elements ({}): {}\nat-rules ({}): {}",
        g.properties.len(),
        g.properties.join(" "),
        g.pseudo_classes.len(),
        g.pseudo_classes.join(" "),
        g.pseudo_elements.len(),
        g.pseudo_elements.join(" "),
        g.at_rules.len(),
        g.at_rules.join(" "),
    );
}

/// The exemptions name dispatched properties, each once, with a reason.
#[test]
fn coverage_exemptions_are_dispatched_properties_with_reasons() {
    let names = rdom_css::property_dispatch::property_names();
    let mut seen = HashSet::new();
    for (name, reason) in EXEMPT_PROPERTIES {
        assert!(names.contains(name), "{name} is not a dispatched property");
        assert!(seen.insert(*name), "{name} exempted twice");
        assert!(!reason.trim().is_empty(), "{name} needs a reason");
    }
}

/// The walker's own cases: declarations in rules, nested rules, at-rules
/// and keyframes count; an invalid declaration and a custom property do
/// not.
#[test]
fn the_coverage_walker_reads_declarations_and_at_rules() {
    let (mut properties, mut at_rules) = (BTreeSet::new(), BTreeSet::new());
    walk(
        &strip_comments(
            ".a { color: red; /* width: 1; */ .b { height: 2 } } @media (width > 1) { .c { margin: 1 } } \
             @keyframes k { from { opacity: 0 } } .d { --x: 1; width: frob; content: \"a;b{\" }",
        ),
        &mut properties,
        &mut at_rules,
    );
    assert_eq!(
        properties.iter().map(String::as_str).collect::<Vec<_>>(),
        ["color", "content", "height", "margin", "opacity"]
    );
    assert_eq!(
        at_rules.iter().map(String::as_str).collect::<Vec<_>>(),
        ["keyframes", "media"]
    );
}
