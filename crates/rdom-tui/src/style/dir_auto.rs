//! The elements whose directionality their text decides (HTML §3.2.6.4:
//! `dir=auto`, and a `<bdi>` without a valid `dir`), and the
//! directionality the cascade last styled each with — so a text edit
//! below one restyles it only when its first strong character's
//! direction flips (`dirty_tracker::marks::mark_auto_direction_host`).

use rdom_core::{Directionality, Dom, NodeId};

use crate::ext::TuiExt;

/// Whether `id` is an element whose directionality comes from its text.
pub(crate) fn is_auto_host(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    let node = dom.node(id);
    let Some(tag) = node.tag_name() else {
        return false;
    };
    let dir = node.get_attribute("dir");
    let is = |k: &str| dir.is_some_and(|d| d.eq_ignore_ascii_case(k));
    is("auto") || (tag == "bdi" && !is("ltr") && !is("rtl"))
}

/// The directionality to record for `id` when it is cascaded: its
/// current one if it is an auto host, else `None`.
pub(crate) fn styled_direction(dom: &Dom<TuiExt>, id: NodeId) -> Option<Directionality> {
    is_auto_host(dom, id).then(|| dom.directionality(id))
}
