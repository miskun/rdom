//! `quotes` at computed-value time (CSS Generated Content 3 §2.1) and
//! the content language `quotes: auto` reads.

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::style::{ComputedStyle, Quotes};

/// The content language of `id` (HTML §3.2.6.2,
/// [`Dom::language`] — the lookup `:lang()` matches against): `None`
/// when there is none or it is empty ("unknown").
pub(super) fn content_language(dom: &Dom<TuiExt>, id: NodeId) -> Option<&str> {
    dom.language(id).filter(|lang| !lang.is_empty())
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
