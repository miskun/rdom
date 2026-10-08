//! The marking primitives the observer uses: an element's subtree, its
//! siblings when a sibling combinator can read the change, the ancestor
//! chains of an interaction state, the hosts that read text.

use rdom_core::{Dom, NodeId};

use super::DirtyState;
use crate::ext::TuiExt;
use crate::style::sibling_triggers::Cause;

/// Mark `from` and each of its ancestors that carries a non-empty
/// `placeholder` attribute — the elements whose `:placeholder-shown`
/// reads the text below them. O(depth), no allocation.
pub(super) fn mark_placeholder_hosts(dom: &mut Dom<TuiExt>, state: &mut DirtyState, from: NodeId) {
    let mut cur = Some(from);
    while let Some(id) = cur {
        let node = dom.node(id);
        cur = node.parent_node().map(|p| p.id());
        if node
            .get_attribute("placeholder")
            .is_some_and(|v| !v.is_empty())
        {
            mark_state_dirty(dom, state, id, Cause::State);
        }
    }
}

/// An ancestor-matched state (`:hover`, `:active`, `:focus-within`)
/// moved from `prev` to `next`: the elements whose match flipped are
/// the two inclusive ancestor chains minus their common part. Each
/// side's topmost flipped element is marked — its subtree cascade
/// covers the rest of that side's chain, and `mark_state_dirty` reaches
/// its siblings for `+` / `~` — so moving between two children of a
/// hovered `<li>` restyles the two children, not the `<li>`.
/// O(depth), two small allocations.
pub(super) fn mark_chain_change(
    dom: &mut Dom<TuiExt>,
    state: &mut DirtyState,
    prev: Option<NodeId>,
    next: Option<NodeId>,
) {
    let chain = |dom: &Dom<TuiExt>, from: Option<NodeId>| {
        let mut chain: Vec<NodeId> = Vec::new();
        let mut cur = from;
        while let Some(id) = cur {
            let node = dom.node(id);
            if node.ext().is_some() {
                chain.push(id);
            }
            cur = node.parent_node().map(|p| p.id());
        }
        chain
    };
    let mut old = chain(dom, prev);
    let mut new = chain(dom, next);
    // Both chains end at the same root when both nodes are connected:
    // drop the shared tail, whose match does not change.
    while !old.is_empty() && old.last() == new.last() {
        old.pop();
        new.pop();
    }
    for top in [old.last(), new.last()].into_iter().flatten() {
        mark_state_dirty(dom, state, *top, Cause::State);
    }
}

/// `id`'s own selector state changed (`cause`): mark its subtree, and —
/// when some `+` / `~` in the sheets can read that kind of change
/// (`SiblingTriggers::fires`) — its parent's element children, whose
/// match can read `id`'s state through the sibling combinator. The
/// children are marked once per parent per drain (`sibling_marked`).
pub(super) fn mark_state_dirty(
    dom: &mut Dom<TuiExt>,
    state: &mut DirtyState,
    id: NodeId,
    cause: Cause<'_>,
) {
    mark_style_dirty(dom, state, id);
    if !state.siblings.fires(cause) {
        return;
    }
    let Some(parent) = dom.node(id).parent_node().map(|p| p.id()) else {
        return;
    };
    if !state.sibling_marked.insert(parent) {
        return;
    }
    let mut sib = dom.node(parent).first_element_child().map(|c| c.id());
    while let Some(s) = sib {
        mark_style_dirty(dom, state, s);
        sib = dom.node(s).next_element_sibling().map(|c| c.id());
    }
}

/// Mark `id`'s subtree as dirty. Sets `style_dirty=true` on the node
/// itself and pushes it to the roots worklist — unless an ancestor is
/// already a queued root (the ancestor's cascade will re-cascade us).
pub(super) fn mark_style_dirty(dom: &mut Dom<TuiExt>, state: &mut DirtyState, id: NodeId) {
    // Non-element nodes (text/comment/fragment root) don't have a TuiExt
    // and don't participate in the cascade directly. But their parent
    // might — we just skip them here.
    if dom.node(id).ext().is_none() {
        return;
    }

    // Walk ancestors. If any ancestor is a queued root, its cascade
    // covers us — don't push to roots, but still flip our flag for
    // completeness. The test is roots-set membership, not the
    // ancestor's `style_dirty` flag: a flag set without a queued root
    // (a direct `TuiExt` write, a stale flag) would otherwise swallow
    // every root below it (`P7G-SETTER-MUTATION-1`).
    let mut ancestor_dirty = false;
    let mut cur = dom.node(id).parent_node().map(|p| p.id());
    while let Some(a) = cur {
        if state.roots_set.contains(&a) {
            ancestor_dirty = true;
            break;
        }
        cur = dom.node(a).parent_node().map(|p| p.id());
    }

    // Flip self.
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.style_dirty = true;
    }

    if !ancestor_dirty {
        // If `id` is already in the roots list, don't push it twice.
        if state.roots_set.insert(id) {
            state.roots.push(id);
        }
    }
}

/// HTML §3.2.6.4: the text below a `dir=auto` element — or a `<bdi>`
/// without a valid `dir` — decides its directionality, which `:dir()`
/// reads (the UA's `[dir]:dir(rtl)` rules among them). Mark the auto
/// element whose contained text includes `from`'s: the nearest such
/// inclusive ancestor, unless an element whose text does not count for
/// its ancestors comes first — one with its own `ltr` / `rtl`, a
/// `<script>`, `<style>` or `<textarea>`. O(depth), no allocation.
pub(super) fn mark_auto_direction_host(
    dom: &mut Dom<TuiExt>,
    state: &mut DirtyState,
    from: NodeId,
) {
    let mut cur = Some(from);
    while let Some(id) = cur {
        let node = dom.node(id);
        cur = node.parent_node().map(|p| p.id());
        let Some(tag) = node.tag_name() else {
            continue;
        };
        let dir = node.get_attribute("dir");
        let is = |k: &str| dir.is_some_and(|d| d.eq_ignore_ascii_case(k));
        let auto = is("auto") || (tag == "bdi" && !is("ltr") && !is("rtl"));
        if auto {
            mark_state_dirty(dom, state, id, Cause::State);
            return;
        }
        if is("ltr") || is("rtl") || matches!(tag, "script" | "style" | "textarea") {
            return;
        }
    }
}
