//! CAPTURE phase 3 (`specs/CAPTURE.md` §Derivation chain): on every acid
//! page, every element's style explanation names exactly the rules the
//! cascade matched for its own box — read through an independent path,
//! the cascade's recorded matches (`App::matched_rules`) — and every
//! winner is a declaration the cascade applied.

use std::collections::BTreeSet;

use rdom_showcase::demos::acid;
use rdom_style::property_dispatch;
use rdom_tui::core_api::NodeType;
use rdom_tui::{PseudoElementTarget, Rule};

use crate::acid::compare::open_page;

/// Whether `rule` declares anything (a longhand or a custom property).
fn declares(rule: &Rule) -> bool {
    !rule.style.custom_properties.is_empty()
        || property_dispatch::property_names()
            .iter()
            .any(|n| property_dispatch::serialize(n, &rule.style).is_some())
}

#[test]
fn every_acid_elements_explanation_names_the_rules_the_cascade_matched() {
    let mut failures = Vec::new();
    let mut explained = 0;
    for page in 1..=acid::page_count() {
        let mut app = open_page(page, None);
        app.advance(0).unwrap();
        let dom = app.dom();
        let elements: Vec<_> = dom
            .descendants(dom.root())
            .filter(|&n| dom.node(n).node_type() == NodeType::Element)
            .collect();
        for id in elements {
            let Some(trace) = app.explain_style(id) else {
                continue;
            };
            explained += 1;
            let named: BTreeSet<(String, String)> = trace
                .declarations()
                .iter()
                .filter_map(|d| d.rule.as_ref())
                .map(|r| (r.selector.clone(), format!("{:?}", r.specificity)))
                .collect();
            let matched: BTreeSet<(String, String)> = app
                .matched_rules(id)
                .into_iter()
                .map(|(_, r)| r)
                .filter(|r| r.pseudo == PseudoElementTarget::None && declares(r))
                .map(|r| (r.source_text.clone(), format!("{:?}", r.specificity)))
                .collect();
            if named != matched {
                let missing: Vec<_> = matched.difference(&named).collect();
                let extra: Vec<_> = named.difference(&matched).collect();
                failures.push(format!(
                    "page {page} {id:?} <{}>: matched but not explained {missing:?}; explained but not matched {extra:?}",
                    dom.node(id).tag_name().unwrap_or_default()
                ));
            }
        }
    }
    assert!(explained > 500, "only {explained} elements explained");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
