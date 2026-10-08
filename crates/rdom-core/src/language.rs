//! The language of a node (HTML §3.2.6.2) and the language-range
//! matching `:lang()` uses (Selectors 4 §7.2, RFC 4647 §3.3.2).

use crate::dom::Dom;
use crate::node::NodeData;
use crate::node_id::NodeId;

impl<Ext> Dom<Ext> {
    /// HTML §3.2.6.2 "the language of a node": the `xml:lang` or `lang`
    /// attribute (in that order) of `id` or of its nearest ancestor
    /// element carrying one. `Some("")` when that attribute is empty — the
    /// language is explicitly unknown — and `None` when no inclusive
    /// ancestor has one (rdom has no document-wide default language).
    pub fn language(&self, id: NodeId) -> Option<&str> {
        let mut cur = Some(id);
        while let Some(c) = cur {
            let node = self.get_node(c)?;
            if let NodeData::Element { attrs, .. } = &node.data
                && let Some(lang) = attrs.get("xml:lang").or_else(|| attrs.get("lang"))
            {
                return Some(lang);
            }
            cur = node.parent;
        }
        None
    }
}

/// Selectors 4 §7.2: whether the language tag `lang` matches the language
/// `range` by RFC 4647 §3.3.2 extended filtering, ASCII case-insensitively.
/// An unknown language (`""`) is matched by the empty range only; `*`
/// as the first subtag matches any known language.
pub(crate) fn lang_range_matches(range: &str, lang: &str) -> bool {
    if range.is_empty() || lang.is_empty() {
        return range.is_empty() && lang.is_empty();
    }
    let mut range = range.split('-');
    let mut tag = lang.split('-');
    // §3.3.2 step 2: the first subtags match, or the range's is `*`.
    let (Some(first_range), Some(first_tag)) = (range.next(), tag.next()) else {
        return false;
    };
    if first_range != "*" && !first_range.eq_ignore_ascii_case(first_tag) {
        return false;
    }
    let mut tag = tag.peekable();
    // Step 3: each further range subtag is found in order; a wildcard
    // is skipped, a singleton in the tag stops the search.
    for want in range {
        if want == "*" {
            continue;
        }
        loop {
            let Some(have) = tag.next() else {
                return false;
            };
            if have.eq_ignore_ascii_case(want) {
                break;
            }
            if have.len() == 1 {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::lang_range_matches;

    /// RFC 4647 §3.3.2's own examples: `de-*-DE` (and `de-DE`) match
    /// `de-DE`, `de-de`, `de-Latn-DE`, `de-Latf-DE`, `de-DE-x-goethe`,
    /// `de-Latn-DE-1996`, `de-Deva-DE`, and not `de`, `de-x-DE`,
    /// `de-Deva`.
    #[test]
    fn extended_filtering_follows_rfc_4647() {
        for range in ["de-*-DE", "de-DE"] {
            for tag in [
                "de-DE",
                "de-de",
                "de-Latn-DE",
                "de-Latf-DE",
                "de-DE-x-goethe",
                "de-Latn-DE-1996",
                "de-Deva-DE",
            ] {
                assert!(lang_range_matches(range, tag), "{range} ~ {tag}");
            }
            for tag in ["de", "de-x-DE", "de-Deva"] {
                assert!(!lang_range_matches(range, tag), "{range} !~ {tag}");
            }
        }
    }
}
