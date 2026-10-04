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
        let mut index = RuleIndex::default();
        for (i, rule) in rules.iter().enumerate() {
            let i = i as u32;
            let keys = rule
                .selector
                .0
                .first()
                .and_then(|c| compound_keys(&c.subject.simples));
            match keys {
                Some(keys) => {
                    for key in keys {
                        let (map, name) = match key {
                            Key::Id(id) => (&mut index.by_id, id),
                            Key::Class(class) => (&mut index.by_class, class),
                            // Exact case, like the matcher (`Type(t)`
                            // compares `tag != t`).
                            Key::Tag(tag) => (&mut index.by_tag, tag),
                        };
                        let bucket = map.entry(name.to_string()).or_default();
                        if bucket.last() != Some(&i) {
                            bucket.push(i);
                        }
                    }
                }
                None => index.universal.push(i),
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

/// One bucket a rule is filed under.
enum Key<'s> {
    Id(&'s str),
    Class(&'s str),
    Tag(&'s str),
}

/// The keys an element must carry one of for a compound to match it:
/// the compound's own most selective simple selector (an id, else a
/// class, else a type), or else — when it holds `:is()` — the keys of every
/// argument's subject (an element matching `:is(.a, .b)` carries `a` or
/// `b`). `None` when some argument has none: the rule is a candidate
/// for every element.
fn compound_keys(simples: &[rdom_core::selectors::SimpleSelector]) -> Option<Vec<Key<'_>>> {
    use rdom_core::selectors::SimpleSelector;
    let own = simples
        .iter()
        .find_map(|s| match s {
            SimpleSelector::Id(id) => Some(Key::Id(id)),
            _ => None,
        })
        .or_else(|| {
            simples.iter().find_map(|s| match s {
                SimpleSelector::Class(c) => Some(Key::Class(c)),
                _ => None,
            })
        })
        .or_else(|| {
            simples.iter().find_map(|s| match s {
                SimpleSelector::Type(t) => Some(Key::Tag(t)),
                _ => None,
            })
        });
    if let Some(key) = own {
        return Some(vec![key]);
    }
    // Any `:is()` whose every argument is keyed bounds the compound.
    simples.iter().find_map(|s| match s {
        SimpleSelector::Is(list) if !list.0.is_empty() => {
            let mut keys = Vec::new();
            for item in &list.0 {
                keys.extend(compound_keys(&item.subject.simples)?);
            }
            Some(keys)
        }
        _ => None,
    })
}
