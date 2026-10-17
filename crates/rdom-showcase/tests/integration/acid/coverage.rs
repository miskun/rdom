//! ACID-COVERAGE (`specs/ACID.md` ground rule 4): the acid page uses every
//! CSS feature rdom dispatches, so a new one cannot skip it.
//!
//! "Used" means it styled something (C16G-ACID-RIGOUR). The tiles' rules
//! are read from what the cascade matched while their pages were shown —
//! each stage-1 page as painted, and each stage-2 step at every
//! checkpoint ([`Used::record`], from `App::matched_rules`): an author
//! rule counts only when its selector matched an element of the page
//! under conditions that held (`@media`, `@supports`, `@container`). So:
//!
//! - a **property** is used when a matched author rule declares it — or a
//!   `@keyframes` block a matched rule's `animation-name` names, or a
//!   `@position-try` rule its `position-try-fallbacks` names — and is
//!   written as a valid declaration of its own name in the tiles' CSS
//!   (`rdom_css::parse_inline`, so a shorthand counts only where it is
//!   written); a `style` attribute's declaration counts as written, its
//!   element being on the page;
//! - a **pseudo-class** — keyword or functional (`:not()`, `:is()`,
//!   `:where()`, `:has()`, `:nth-*()`, `:nth-*col()`, `:lang()`, `:dir()`)
//!   — is used when a matched rule's selector holds it, or a rule's
//!   pseudo-element takes it (`::before:hover`);
//! - a **pseudo-element** is used when a matched rule targets it;
//! - an **at-rule** is used when a matched rule sits in it (`@media`,
//!   `@supports`, `@container`, `@layer`, `@scope`, `@starting-style`) or
//!   names it (`@keyframes`, `@position-try`, `@counter-style`), when a
//!   matched rule declares a custom property an `@property` registers, and
//!   — `@import` — when it opens a rule in the text.
//!
//! What must be used is read from the crates that dispatch it:
//! `property_dispatch::property_names()`, `selectors::pseudo_class_names()`
//! (each parsed to its `PseudoClass`) with [`FUNCTIONAL`],
//! `pseudo_element_names()` (each parsed to its `PseudoElementTarget`) and
//! `rdom_css::at_rule_names()`.

use std::collections::{BTreeSet, HashSet};
use std::mem::{Discriminant, discriminant};

use rdom_css::property_dispatch::{declares, property_names, serialize};
use rdom_showcase::demos::acid;
use rdom_style::ConditionKind;
use rdom_tui::core_api::selectors::{
    self, ComplexSelector, NthKind, PseudoClass, SelectorList, SimpleSelector,
};
use rdom_tui::render::Backend;
use rdom_tui::runtime::app::App;
use rdom_tui::{
    NodeType, PseudoElementTarget, Rule, RuleOrigin, Stylesheet, TuiDom, UserActionState,
};

/// The functional pseudo-classes besides `:dir()` (which
/// `pseudo_class_names()` lists): every name the selector parser
/// dispatches to its own grammar.
const FUNCTIONAL: &[&str] = &[
    "not",
    "is",
    "where",
    "has",
    "nth-child",
    "nth-last-child",
    "nth-of-type",
    "nth-last-of-type",
    "nth-col",
    "nth-last-col",
    "lang",
];

/// What the tiles' rules styled while their pages were shown (module
/// doc).
#[derive(Default)]
pub struct Used {
    pub properties: BTreeSet<String>,
    pub classes: HashSet<Discriminant<PseudoClass>>,
    pub functional: BTreeSet<&'static str>,
    pub elements: HashSet<Discriminant<PseudoElementTarget>>,
    pub at_rules: BTreeSet<&'static str>,
    /// The rules already read, by address — within one `App`, whose
    /// sheets outlive every record of it.
    seen: HashSet<usize>,
}

impl Used {
    /// Read the author rules the last cascade of `app` matched, for every
    /// element of its document.
    pub fn record<B: Backend>(&mut self, app: &App<B>) {
        let dom = app.dom();
        let mut matched: Vec<(&Stylesheet, &Rule)> = Vec::new();
        for id in dom.descendants(dom.root()).collect::<Vec<_>>() {
            for (sheet, rule) in app.matched_rules(id) {
                if rule.origin == RuleOrigin::Author
                    && self.seen.insert(rule as *const Rule as usize)
                {
                    matched.push((sheet, rule));
                }
            }
        }
        let mut sheets: Vec<&Stylesheet> = Vec::new();
        for (sheet, _) in &matched {
            if !sheets.iter().any(|s| std::ptr::eq(*s, *sheet)) {
                sheets.push(sheet);
            }
        }
        // `@starting-style` rules style no element's box — only the
        // starting style of one that has no before-change style (CSS
        // Transitions 2 §3) — so the cascade records no match of them:
        // one counts when its selector matches an element of the page.
        let ids: Vec<_> = dom.descendants(dom.root()).collect();
        for &sheet in &sheets {
            for rule in sheet.rules() {
                if rule.starting_style
                    && rule.origin == RuleOrigin::Author
                    && !self.seen.contains(&(rule as *const Rule as usize))
                    && ids.iter().any(|&id| dom.matches_list(id, &rule.selector))
                {
                    self.seen.insert(rule as *const Rule as usize);
                    matched.push((sheet, rule));
                }
            }
        }
        for (sheet, rule) in matched {
            self.rule(sheet, rule, &sheets);
        }
    }

    /// Add `other`'s findings.
    pub fn merge(&mut self, other: &Used) {
        self.properties.extend(other.properties.iter().cloned());
        self.classes.extend(other.classes.iter().copied());
        self.functional.extend(other.functional.iter().copied());
        self.elements.extend(other.elements.iter().copied());
        self.at_rules.extend(other.at_rules.iter().copied());
    }

    fn declared(&mut self, style: &rdom_tui::TuiStyle) {
        for name in property_names() {
            if declares(style, name) {
                self.properties.insert((*name).to_string());
            }
        }
    }

    fn rule(&mut self, sheet: &Stylesheet, rule: &Rule, sheets: &[&Stylesheet]) {
        self.declared(&rule.style);
        list_classes(&rule.selector, &mut self.classes, &mut self.functional);
        state_classes(rule.pseudo_state, &mut self.classes);
        self.elements.insert(discriminant(&rule.pseudo));
        if rule.layer.is_some() {
            self.at_rules.insert("layer");
        }
        if rule.scope.is_some() {
            self.at_rules.insert("scope");
        }
        if rule.starting_style {
            self.at_rules.insert("starting-style");
        }
        for c in sheet.condition_chain(rule.condition) {
            match c.kind {
                ConditionKind::Media(_) => self.at_rules.insert("media"),
                ConditionKind::Supports(_) => self.at_rules.insert("supports"),
                ConditionKind::Container(_) => self.at_rules.insert("container"),
                _ => false,
            };
        }
        let names = |property: &str| -> Vec<String> {
            serialize(property, &rule.style)
                .map(|v| {
                    v.split([',', ' '])
                        .map(|w| w.trim().to_string())
                        .filter(|w| !w.is_empty())
                        .collect()
                })
                .unwrap_or_default()
        };
        for name in names("animation-name") {
            for k in sheets.iter().flat_map(|s| s.keyframes()) {
                if *k.name == *name {
                    self.at_rules.insert("keyframes");
                    for frame in &k.keyframes {
                        self.declared(&frame.style);
                    }
                }
            }
        }
        for name in names("position-try-fallbacks") {
            for t in sheets.iter().flat_map(|s| s.position_try_rules()) {
                if *t.name == *name {
                    self.at_rules.insert("position-try");
                    self.declared(t.declarations());
                }
            }
        }
        let counters: Vec<String> = ["list-style-type", "list-style", "content"]
            .iter()
            .flat_map(|p| serialize(p, &rule.style))
            .collect();
        for c in sheets.iter().flat_map(|s| s.counter_styles()) {
            if counters.iter().any(|v| v.contains(&*c.name)) {
                self.at_rules.insert("counter-style");
            }
        }
        // A registered custom property declared, or read through `var()`.
        for r in sheets.iter().flat_map(|s| s.registered_properties()) {
            let name = r.name.trim_start_matches("--");
            let reads = rule.style.pending.iter().any(|d| {
                d.has_substitution
                    && serialize(&d.name, &rule.style).is_some_and(|v| v.contains(&*r.name))
            });
            if reads || rule.style.custom_property_value(name).is_some() {
                self.at_rules.insert("property");
            }
        }
    }
}

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

/// `text` without its comments — a `/*` inside a string is text.
fn strip_comments(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '"' | '\'' => {
                let q = chars[i];
                out.push(q);
                i += 1;
                while i < chars.len() && chars[i] != q {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        out.push(chars[i]);
                        i += 1;
                    }
                    out.push(chars[i]);
                    i += 1;
                }
                if i < chars.len() {
                    out.push(q);
                    i += 1;
                }
            }
            '/' if chars.get(i + 1) == Some(&'*') => {
                i += 2;
                while i < chars.len() && !(chars[i] == '*' && chars.get(i + 1) == Some(&'/')) {
                    i += 1;
                }
                i += 2;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
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

fn list_classes(
    list: &SelectorList,
    out: &mut HashSet<Discriminant<PseudoClass>>,
    functional: &mut BTreeSet<&'static str>,
) {
    for complex in &list.0 {
        complex_classes(complex, out, functional);
    }
}

fn complex_classes(
    complex: &ComplexSelector,
    out: &mut HashSet<Discriminant<PseudoClass>>,
    functional: &mut BTreeSet<&'static str>,
) {
    let compounds =
        std::iter::once(&complex.subject).chain(complex.ancestors.iter().map(|(_, c)| c));
    for compound in compounds {
        for simple in &compound.simples {
            match simple {
                SimpleSelector::Pseudo(p) => {
                    out.insert(discriminant(p));
                }
                SimpleSelector::Not(l) => {
                    functional.insert("not");
                    list_classes(l, out, functional);
                }
                SimpleSelector::Is(l) => {
                    functional.insert("is");
                    list_classes(l, out, functional);
                }
                SimpleSelector::Where(l) => {
                    functional.insert("where");
                    list_classes(l, out, functional);
                }
                SimpleSelector::Has(relative) => {
                    functional.insert("has");
                    for r in relative {
                        complex_classes(&r.selector, out, functional);
                    }
                }
                SimpleSelector::Lang(_) => {
                    functional.insert("lang");
                }
                SimpleSelector::Nth(nth) => {
                    functional.insert(match nth.kind {
                        NthKind::Child => "nth-child",
                        NthKind::LastChild => "nth-last-child",
                        NthKind::OfType => "nth-of-type",
                        NthKind::LastOfType => "nth-last-of-type",
                    });
                    if let Some(of) = &nth.of {
                        list_classes(of, out, functional);
                    }
                }
                SimpleSelector::NthColumn(c) => {
                    functional.insert(if c.last { "nth-last-col" } else { "nth-col" });
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

/// The target of the pseudo-element `name` (`pseudo_element_names`), as
/// a rule ending in it parses.
fn element_named(name: &str) -> PseudoElementTarget {
    let text = match name {
        "highlight" => "p::highlight(x) {}".to_string(),
        name => format!("p::{name} {{}}"),
    };
    let sheet = rdom_css::from_css_strict(&text).expect("a listed pseudo-element parses");
    sheet.rules()[0].pseudo.clone()
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
    // What the tiles write: each valid declaration's property, each
    // at-rule opening a rule.
    let (mut written, mut at_text) = (BTreeSet::new(), BTreeSet::new());
    for text in &css.sheets {
        walk(&strip_comments(text), &mut written, &mut at_text);
    }
    let mut inline = BTreeSet::new();
    for text in &css.inline {
        for item in text.split(';') {
            declaration(item.trim(), &mut inline);
        }
    }
    // What styled something: every page as painted, every step's
    // checkpoints.
    let mut used = Used::default();
    for page in 1..=acid::page_count() {
        match super::compare::painted_page(page) {
            Ok(p) => used.merge(&p.used),
            Err(e) => panic!("{e}"),
        }
    }
    for r in super::interactive::results() {
        used.merge(&r.used);
    }
    let properties: BTreeSet<&String> = written
        .iter()
        .filter(|p| used.properties.contains(*p))
        .chain(inline.iter())
        .collect();
    let exempt: HashSet<&str> = EXEMPT_PROPERTIES.iter().map(|(n, _)| *n).collect();
    let mut gaps = Gaps::default();
    for name in property_names() {
        if !properties.contains(&(*name).to_string()) && !exempt.contains(name) {
            gaps.properties.push((*name).to_string());
        }
    }
    for name in selectors::pseudo_class_names() {
        if !used.classes.contains(&discriminant(&class_named(name))) {
            gaps.pseudo_classes.push(format!(":{name}"));
        }
    }
    for name in FUNCTIONAL {
        if !used.functional.contains(name) {
            gaps.pseudo_classes.push(format!(":{name}()"));
        }
    }
    for name in rdom_tui::pseudo_element_names() {
        if !used.elements.contains(&discriminant(&element_named(name))) {
            gaps.pseudo_elements.push(format!("::{name}"));
        }
    }
    for name in rdom_css::at_rule_names() {
        let seen = used.at_rules.contains(name) || (name == "import" && at_text.contains(name));
        if !seen {
            gaps.at_rules.push(format!("@{name}"));
        }
    }
    gaps
}

/// Each functional pseudo-class listed parses (the list names the
/// parser's grammars, and an unparsable name would never be used).
#[test]
fn the_functional_pseudo_classes_parse() {
    for name in FUNCTIONAL {
        let arg = match *name {
            "not" | "is" | "where" | "has" => "p",
            "lang" => "en",
            _ => "1",
        };
        let text = format!(":{name}({arg})");
        assert!(selectors::parse(&text).is_ok(), "{text}");
    }
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
             @keyframes k { from { opacity: 0 } } .d { --x: 1; width: frob; content: \"a;b{/*\" } \
             .e { height: 1 } /* \"*/ .f { padding: 1 }",
        ),
        &mut properties,
        &mut at_rules,
    );
    assert_eq!(
        properties.iter().map(String::as_str).collect::<Vec<_>>(),
        ["color", "content", "height", "margin", "opacity", "padding"]
    );
    assert_eq!(
        at_rules.iter().map(String::as_str).collect::<Vec<_>>(),
        ["keyframes", "media"]
    );
}
