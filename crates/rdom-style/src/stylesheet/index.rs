//! [`RuleIndex`] — a stylesheet's rules bucketed by subject key, so the
//! cascade only runs the selector matcher on candidate rules.

use super::Rule;

/// Rules bucketed by the most selective simple selector of their
/// subject compound (`#id` > `.class` > `tag` > everything else), so a
/// cascade only tests the rules that can possibly match an element
/// instead of every rule of every sheet (`CASCADE-INITIAL-ALLOC-1`).
/// Built lazily by [`Stylesheet::rule_index`](super::Stylesheet::rule_index), dropped on mutation.
#[derive(Debug, Clone, Default)]
pub struct RuleIndex {
    by_id: std::collections::HashMap<String, Vec<u32>>,
    by_class: std::collections::HashMap<String, Vec<u32>>,
    by_tag: std::collections::HashMap<String, Vec<u32>>,
    /// Rules whose subject has no id / class / type key (`*`,
    /// `:not(…)`, `[attr]`, `:hover`, …): candidates for every element.
    universal: Vec<u32>,
}

impl RuleIndex {
    pub(super) fn build(rules: &[Rule]) -> Self {
        use rdom_core::selectors::SimpleSelector;
        let mut index = RuleIndex::default();
        for (i, rule) in rules.iter().enumerate() {
            let i = i as u32;
            let simples = rule
                .selector
                .0
                .first()
                .map(|c| c.subject.simples.as_slice())
                .unwrap_or(&[]);
            let id = simples.iter().find_map(|s| match s {
                SimpleSelector::Id(id) => Some(id),
                _ => None,
            });
            let class = simples.iter().find_map(|s| match s {
                SimpleSelector::Class(c) => Some(c),
                _ => None,
            });
            let tag = simples.iter().find_map(|s| match s {
                SimpleSelector::Type(t) => Some(t),
                _ => None,
            });
            if let Some(id) = id {
                index.by_id.entry(id.clone()).or_default().push(i);
            } else if let Some(class) = class {
                index.by_class.entry(class.clone()).or_default().push(i);
            } else if let Some(tag) = tag {
                // Exact case, like the matcher (`Type(t)` compares `tag != t`).
                index.by_tag.entry(tag.clone()).or_default().push(i);
            } else {
                index.universal.push(i);
            }
        }
        index
    }

    /// Indices (into `Stylesheet::rules()`, ascending, deduplicated) of
    /// every rule that can match an element with this tag, id and class
    /// list. A superset of the rules that do match; the caller still runs
    /// the full selector matcher on each.
    pub fn candidates<'a>(
        &self,
        tag: Option<&str>,
        id: Option<&str>,
        classes: impl Iterator<Item = &'a str>,
        out: &mut Vec<u32>,
    ) {
        out.clear();
        out.extend_from_slice(&self.universal);
        if let Some(tag) = tag
            && let Some(v) = self.by_tag.get(tag)
        {
            out.extend_from_slice(v);
        }
        if let Some(id) = id
            && let Some(v) = self.by_id.get(id)
        {
            out.extend_from_slice(v);
        }
        for class in classes {
            if let Some(v) = self.by_class.get(class) {
                out.extend_from_slice(v);
            }
        }
        out.sort_unstable();
        out.dedup();
    }
}
