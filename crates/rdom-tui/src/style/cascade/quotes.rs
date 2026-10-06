//! `quotes` at computed-value time (CSS Generated Content 3 §2.1) and
//! the content language `quotes: auto` reads.

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::style::{ComputedStyle, Quotes};

/// The content language of `id` (HTML §3.2.6.2): the `lang` attribute
/// of the node or its nearest ancestor that has one (`xml:lang` first,
/// as HTML orders them); `None` when there is none or it is empty
/// ("unknown").
pub(super) fn content_language(dom: &Dom<TuiExt>, id: NodeId) -> Option<&str> {
    let mut cur = Some(dom.node(id));
    while let Some(node) = cur {
        if node.node_type() == NodeType::Element
            && let Some(lang) = node
                .get_attribute("xml:lang")
                .or_else(|| node.get_attribute("lang"))
        {
            return (!lang.is_empty()).then_some(lang);
        }
        cur = node.parent_node();
    }
    None
}

/// `quotes: match-parent` computes to the parent's value (§2.1): its
/// pairs, `none`, or — `auto` — the pairs of the parent's language, so
/// the child keeps the parent's marks whatever its own language.
/// The root element (no parent element) computes `match-parent` to
/// `auto`.
pub(super) fn finalize_quotes(
    working: &mut ComputedStyle,
    parent: &ComputedStyle,
    dom: &Dom<TuiExt>,
    parent_id: Option<NodeId>,
) {
    if working.quotes != Quotes::MatchParent {
        return;
    }
    working.quotes = match parent_id {
        Some(p) if dom.node(p).node_type() == NodeType::Element => {
            parent.quotes.explicit(content_language(dom, p))
        }
        _ => Quotes::Auto,
    };
}
