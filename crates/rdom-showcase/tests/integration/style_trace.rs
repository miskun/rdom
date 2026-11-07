//! CAPTURE phase 3 (`specs/CAPTURE.md` §Derivation chain): on every acid
//! page, every element's style explanation names the same rules — by
//! selector, specificity and location — as the cascade's recorded matches
//! for its own box (`App::matched_rules`, an independent path), counting
//! the rules that declare something. That the explanation's blocks,
//! values and order are exactly what the cascade applied is
//! `rdom-tui`'s `explain_tests` (the cascade's own record of its ladder).

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
                .map(|r| {
                    (
                        r.selector.clone(),
                        format!("{:?} {:?}", r.specificity, r.location),
                    )
                })
                .collect();
            let matched: BTreeSet<(String, String)> = app
                .matched_rules(id)
                .into_iter()
                .map(|(_, r)| r)
                .filter(|r| r.pseudo == PseudoElementTarget::None && declares(r))
                .map(|r| {
                    (
                        r.source_text.clone(),
                        format!("{:?} {:?}", r.specificity, r.location),
                    )
                })
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
