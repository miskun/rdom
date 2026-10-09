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
    // The root fragment, the root element, matches these too
    // (C14G-ROOT-ELEMENT) — in the chain when a selector can read it so.
    let root_state = state.root_state;
    let chain = |dom: &Dom<TuiExt>, from: Option<NodeId>| {
        let mut chain: Vec<NodeId> = Vec::new();
        let mut cur = from;
        while let Some(id) = cur {
            let node = dom.node(id);
            if node.ext().is_some()
                || (root_state && crate::style::cascade::root::is_root_fragment(dom, id))
            {
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
    if state.has.fires(cause) {
        mark_has_anchors(dom, state, id, false);
    }
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
    // Non-element nodes (text/comment) don't have a TuiExt and don't
    // participate in the cascade directly. But their parent might — we
    // just skip them here. The root fragment is the root element
    // (C14G-ROOT-ELEMENT): its cascade is the whole tree's.
    if dom.node(id).ext().is_none() && !crate::style::cascade::root::is_root_fragment(dom, id) {
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
/// reads (the UA's `[dir]:dir(rtl)` rules among them). Find the auto
/// element whose contained text includes `from`'s: the nearest such
/// inclusive ancestor, unless an element whose text does not count for
/// its ancestors comes first — one with its own `ltr` / `rtl`, a
/// `<script>`, `<style>` or `<textarea>` — and mark it only when its
/// directionality is not the one it was last styled with
/// (`TuiExt::auto_direction`, `style::dir_auto`): an edit that leaves the
/// first strong character's direction restyles nothing. O(depth) plus
/// the host's directionality (its text up to the first strong
/// character), no allocation.
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
        let stops = is("ltr") || is("rtl") || matches!(tag, "script" | "style" | "textarea");
        if crate::style::dir_auto::is_auto_host(dom, id) {
            let now = dom.directionality(id);
            let known = dom
                .node_mut(id)
                .ext_mut()
                .and_then(|e| e.auto_direction.replace(now));
            if known != Some(now) {
                mark_state_dirty(dom, state, id, Cause::State);
            }
            return;
        }
        if stops {
            return;
        }
    }
}

/// Selectors 4 §4.5 (C11-HAS): a change at `from` can change the `:has()`
/// match of an anchor whose subtree holds it — an ancestor (`from`
/// itself too when `inclusive`: its child list changed) — or, when a
/// relative selector reaches siblings, an earlier sibling of `from` or of
/// an ancestor, at most `HasTriggers::sibling_reach` of them. Mark the
/// ones a cascade flagged as anchors (`TuiExt::has_anchor`), with
/// `Cause::Has` so a sibling combinator reading their `:has()` reaches
/// their siblings. Each (element, parent) is walked once per drain
/// (`DirtyState::has_walked`) — its ancestors' walks, and with unbounded
/// reach its earlier siblings', are then done too — so many changes under
/// one parent cost one walk of it. O(depth + reach) per change; nothing
/// before any cascade flagged an anchor. An anchor *after* `from` (a
/// sibling step nested in an argument, `.a:has(+ :is(.x ~ *))`) is the
/// sibling marking's (`mark_state_dirty`; `has_triggers` module doc).
pub(super) fn mark_has_anchors(
    dom: &mut Dom<TuiExt>,
    state: &mut DirtyState,
    from: NodeId,
    inclusive: bool,
) {
    if !crate::style::doc_flags::has_has_anchors(dom) {
        return;
    }
    let reach = state.has.sibling_reach();
    // The root fragment, the root element, keeps its flag on the
    // document (C14G-ROOT-ELEMENT).
    let root_anchor = crate::style::doc_flags::root_has_anchor(dom);
    let is_anchor = |dom: &Dom<TuiExt>, id: NodeId| {
        dom.node(id).ext().map_or_else(
            || root_anchor && crate::style::cascade::root::is_root_fragment(dom, id),
            |e| e.has_anchor,
        )
    };
    let mut cur = Some(from);
    let mut first = true;
    while let Some(id) = cur {
        probe::step();
        if (inclusive || !first) && is_anchor(dom, id) {
            mark_state_dirty(dom, state, id, Cause::Has);
        }
        let parent = dom.node(id).parent_node().map(|p| p.id());
        // Walked this drain under the same parent: its earlier siblings
        // and its ancestors are done.
        if !state.has_walked.insert((id, parent)) {
            break;
        }
        let mut sib = dom.node(id).previous_element_sibling().map(|s| s.id());
        let mut left = reach;
        while left > 0
            && let Some(s) = sib
        {
            probe::step();
            if is_anchor(dom, s) {
                mark_state_dirty(dom, state, s, Cause::Has);
            }
            // With unbounded reach, `s`'s own earlier siblings are this
            // run's: its walk is done (its ancestors are `id`'s).
            if reach == usize::MAX && !state.has_walked.insert((s, parent)) {
                break;
            }
            left = left.saturating_sub(usize::from(reach != usize::MAX));
            sib = dom.node(s).previous_element_sibling().map(|s| s.id());
        }
        first = false;
        cur = parent;
    }
}

/// Test-only: the elements `:has()` invalidation visited on this thread.
pub(crate) mod probe {
    #[cfg(test)]
    thread_local! {
        pub static STEPS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    }

    pub fn step() {
        #[cfg(test)]
        STEPS.with(|c| c.set(c.get() + 1));
    }

    #[cfg(test)]
    pub fn take() -> u64 {
        STEPS.with(|c| c.replace(0))
    }
}

/// Whether `id` is one of the HTML elements `tags`, ASCII
/// case-insensitively, as the table model reads them
/// (`rdom_core::table`).
fn is_html(dom: &Dom<TuiExt>, id: NodeId, tags: &[&str]) -> bool {
    dom.node(id)
        .tag_name()
        .is_some_and(|t| tags.iter().any(|w| t.eq_ignore_ascii_case(w)))
}

/// Whether a child-list change of `parent` adding `added` and removing
/// `removed` can move cells between an HTML table's columns (HTML
/// §4.9.12.1): an element coming or going among the children of a
/// `<table>`, a row group, a row or a `<colgroup>` — its rows, cells and
/// columns. Text, and anything inside a cell, moves none.
pub(super) fn moves_columns(
    dom: &Dom<TuiExt>,
    parent: NodeId,
    added: &[NodeId],
    removed: &[NodeId],
) -> bool {
    is_html(
        dom,
        parent,
        &["table", "thead", "tbody", "tfoot", "tr", "colgroup"],
    ) && added
        .iter()
        .chain(removed)
        .any(|&n| dom.node(n).node_type() == rdom_core::NodeType::Element)
}

/// A change at `id` that can move cells between an HTML table's columns
/// (Selectors 4 §16: a `span` / `colspan` / `rowspan`, or an element
/// child of a `<table>`, row group, row or `<colgroup>` coming or going,
/// [`moves_columns`]): `id`'s table — the `<table>` it is, or the nearest
/// above it within a table's structure — is restyled whole, every cell's
/// column selectors read anew.
pub(super) fn mark_column_change(dom: &mut Dom<TuiExt>, state: &mut DirtyState, id: NodeId) {
    const PARTS: &[&str] = &[
        "td", "th", "tr", "thead", "tbody", "tfoot", "col", "colgroup",
    ];
    let mut cur = Some(id);
    while let Some(n) = cur {
        if is_html(dom, n, &["table"]) {
            mark_style_dirty(dom, state, n);
            return;
        }
        if !is_html(dom, n, PARTS) {
            return;
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    }
}
