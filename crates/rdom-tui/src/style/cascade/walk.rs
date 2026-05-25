//! The cascade walk — `cascade_subtree` + the per-element /
//! per-pseudo-element style computation.
//!
//! `cascade_subtree` recurses into every element in the subtree,
//! computing a fresh `ComputedStyle` at each and writing it back.
//! Text/Comment/Fragment nodes have no `TuiExt` and get skipped
//! structurally (their element children are still visited).

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::Position;
use crate::style::{ComputedStyle, PseudoElementTarget, Rule, Stylesheet, VarMap};

use super::apply::{apply_cascade_ladder, finalize_bfc_formation, finalize_border_fg};
use super::content::resolve_content_on;
use super::inherit::{inherit_inheritable_from, layout_differs};

/// Merge `root_vars` across all registered sheets into a single
/// `VarMap`. Later sheets win per var name — push order is the
/// last-wins tiebreaker. Allocates one fresh `Rc<HashMap>` per call;
/// callers compute this once per cascade pass (in `cascade_all` /
/// `cascade_subtrees_all`) and `Rc::clone` from there per element.
pub(super) fn merge_root_vars(sheets: &[&Stylesheet]) -> VarMap {
    let mut merged = std::collections::HashMap::new();
    for sheet in sheets {
        for (k, v) in sheet.vars() {
            merged.insert(k.clone(), v.clone());
        }
    }
    std::rc::Rc::new(merged)
}

/// Bottom-up flags aggregated up the tree during cascade. Each
/// flag mirrors a `TuiExt` field that layout / paint use to skip
/// walks when nothing in the subtree needs them.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct SubtreeFlags {
    pub has_positioned_pseudo: bool,
    pub has_collapse: bool,
}

impl SubtreeFlags {
    fn merge(&mut self, other: SubtreeFlags) {
        self.has_positioned_pseudo |= other.has_positioned_pseudo;
        self.has_collapse |= other.has_collapse;
    }
}

/// Walk `id`'s subtree. Returns the subtree's aggregated
/// [`SubtreeFlags`] — currently `has_positioned_pseudo`
/// (positioned `::before` / `::after`) and `has_collapse`
/// (`border-collapse: collapse` anywhere). Each flag is written to
/// the element's `TuiExt` so layout / paint can do an O(1) check
/// at the root and skip whole walks when nothing relevant is in
/// play. See `TuiExt` docs for the incremental-cascade
/// conservatism rules.
pub(super) fn cascade_subtree(
    dom: &mut Dom<TuiExt>,
    sheets: &[&Stylesheet],
    merged_vars: &VarMap,
    id: NodeId,
    parent_computed: &ComputedStyle,
) -> SubtreeFlags {
    // Collect child ids up-front; mutations below don't change structure
    // but borrow rules need shared → exclusive swap.
    let child_ids: Vec<NodeId> = dom.node(id).child_nodes().map(|n| n.id()).collect();

    // Non-element nodes (text / comment / fragment): still recurse so
    // their element children get cascaded — the root is a Fragment by
    // default — but don't compute style for them (TuiExt only carries
    // Element data). The aggregate still bubbles up through them so
    // the layout/paint check at dom.root() picks it up.
    let is_element = dom.node(id).node_type() == NodeType::Element;
    if !is_element {
        let mut flags = SubtreeFlags::default();
        for child in child_ids {
            flags.merge(cascade_subtree(
                dom,
                sheets,
                merged_vars,
                child,
                parent_computed,
            ));
        }
        return flags;
    }

    // Compute under a shared borrow.
    let (
        computed,
        computed_before,
        computed_after,
        computed_backdrop,
        computed_selection,
        computed_scrollbar,
        computed_scrollbar_thumb,
    ) = {
        let computed = compute_element_style(dom, sheets, merged_vars, id, parent_computed);
        let cb = compute_pseudo_style(dom, sheets, id, &computed, PseudoElementTarget::Before);
        let ca = compute_pseudo_style(dom, sheets, id, &computed, PseudoElementTarget::After);
        let cbd = compute_pseudo_style(dom, sheets, id, &computed, PseudoElementTarget::Backdrop);
        let csel = compute_pseudo_style(dom, sheets, id, &computed, PseudoElementTarget::Selection);
        // Scrollbar pseudos only computed for elements that actually
        // have non-`Visible` overflow on at least one axis — saves a
        // selector-matching pass per element on the (very common)
        // non-scrollable case.
        let needs_scrollbar = !matches!(
            computed.overflow_x,
            crate::layout::Overflow::Visible | crate::layout::Overflow::Hidden
        ) || !matches!(
            computed.overflow_y,
            crate::layout::Overflow::Visible | crate::layout::Overflow::Hidden
        );
        let (csb, csbt) = if needs_scrollbar {
            (
                compute_pseudo_style(dom, sheets, id, &computed, PseudoElementTarget::Scrollbar),
                compute_pseudo_style(
                    dom,
                    sheets,
                    id,
                    &computed,
                    PseudoElementTarget::ScrollbarThumb,
                ),
            )
        } else {
            (None, None)
        };
        (computed, cb, ca, cbd, csel, csb, csbt)
    };

    let own_has_positioned_pseudo = computed_before
        .as_ref()
        .is_some_and(|c| c.position != Position::Static)
        || computed_after
            .as_ref()
            .is_some_and(|c| c.position != Position::Static);

    // Diff for layout invalidation. "No previous computed" counts as a
    // change (first cascade).
    let layout_changed = match dom.node(id).ext().and_then(|e| e.computed.as_ref()) {
        Some(prev) => layout_differs(prev, &computed),
        None => true,
    };

    // Write back.
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.computed = Some(computed.clone());
        ext.computed_before = computed_before;
        ext.computed_after = computed_after;
        ext.computed_backdrop = computed_backdrop;
        ext.computed_selection = computed_selection;
        ext.computed_scrollbar = computed_scrollbar;
        ext.computed_scrollbar_thumb = computed_scrollbar_thumb;
        ext.style_dirty = false;
        if layout_changed {
            ext.layout_dirty = true;
        }
    }

    // Recurse. Children inherit from our computed style. Aggregate
    // children's flags into our subtree flags.
    let mut flags = SubtreeFlags {
        has_positioned_pseudo: own_has_positioned_pseudo,
        has_collapse: computed.border_collapse == crate::layout::BorderCollapse::Collapse,
    };
    for child in child_ids {
        flags.merge(cascade_subtree(dom, sheets, merged_vars, child, &computed));
    }

    // Write the bottom-up aggregates.
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.tree_has_positioned_pseudo = flags.has_positioned_pseudo;
        ext.tree_has_collapse = flags.has_collapse;
    }
    flags
}

/// Per-element cascade: start from initial + inheritance, collect
/// matching rules, apply the ladder, resolve `content`, finalize
/// `border_fg`.
fn compute_element_style(
    dom: &Dom<TuiExt>,
    sheets: &[&Stylesheet],
    merged_vars: &VarMap,
    id: NodeId,
    parent: &ComputedStyle,
) -> ComputedStyle {
    // Start from initial + inherit subset from parent.
    let mut working = ComputedStyle::initial();
    inherit_inheritable_from(&mut working, parent);
    // Vars are precomputed once per cascade pass — Rc::clone is
    // cheap (one atomic-or-non-atomic refcount bump, depending on
    // Rc vs Arc), so every element shares the same `VarMap`
    // allocation.
    working.vars = std::rc::Rc::clone(merged_vars);

    // Collect matching non-pseudo-element rules across all sheets.
    // Track each rule's sheet index so cascade order is
    // (specificity, sheet_idx, source_idx) — later sheets win
    // same-specificity contests just like later rules in a single
    // sheet do.
    let mut matching: Vec<(usize, &Rule)> = Vec::new();
    for (sheet_idx, sheet) in sheets.iter().enumerate() {
        for rule in sheet.rules() {
            if rule.pseudo == PseudoElementTarget::None && dom.matches_list(id, &rule.selector) {
                matching.push((sheet_idx, rule));
            }
        }
    }
    matching.sort_by_key(|(sheet_idx, r)| (r.specificity, *sheet_idx, r.source_idx));
    let sorted: Vec<&Rule> = matching.iter().map(|(_, r)| *r).collect();

    // Inline style on this element (may be empty).
    let inline = dom.node(id).ext().map(|e| &e.inline_style);

    apply_cascade_ladder(&mut working, &sorted, inline, parent);

    // Host element's own `content` property. Normally `None`; authors
    // don't typically set `content` on a real element (CSS restricts it
    // to pseudo-elements) but we allow it for flexibility.
    let attr_lookup = |name: &str| dom.node(id).get_attribute(name).map(|s| s.to_string());
    working.content = resolve_content_on(&working, &sorted, inline, &attr_lookup).unwrap_or(None);

    // border_fg falls back to working.fg when no rule declared it
    // (property catalog: initial = "inherits fg"). Implemented as a
    // post-pass rather than during apply_color because the author may
    // set fg AFTER border_fg in the rule (same specificity), and we
    // need the *final* fg value as the fallback.
    finalize_border_fg(&mut working, &sorted, inline);
    // BFC formation predicate (CSS 2.1 §9.4.1). Computed AFTER the
    // cascade ladder so it reads the final values of `flow`,
    // `display`, `overflow_*`, `position`. Used by the block-layout
    // margin-collapse pass — landing here in phase 1 so phase 5 has
    // it ready to consume.
    finalize_bfc_formation(&mut working);

    working
}

/// Pseudo-element computation. Returns `None` if the pseudo-element
/// should not render (no matching rules AND no legacy
/// `before_content` / `after_content` text set AND no `content`
/// resolved).
fn compute_pseudo_style(
    dom: &Dom<TuiExt>,
    sheets: &[&Stylesheet],
    id: NodeId,
    host_computed: &ComputedStyle,
    target: PseudoElementTarget,
) -> Option<ComputedStyle> {
    if target == PseudoElementTarget::None {
        return None;
    }

    // Pseudo-elements inherit from the host's computed style (per spec),
    // not from the host's parent.
    let mut working = ComputedStyle::initial();
    inherit_inheritable_from(&mut working, host_computed);
    // Pseudo-elements share the host's vars (which came from the
    // merged stylesheet roots).
    working.vars = host_computed.vars.clone();

    // Collect matching rules for this pseudo across all sheets, with
    // sheet_idx as the secondary tiebreaker.
    let mut matching: Vec<(usize, &Rule)> = Vec::new();
    for (sheet_idx, sheet) in sheets.iter().enumerate() {
        for rule in sheet.rules() {
            if rule.pseudo == target && dom.matches_list(id, &rule.selector) {
                matching.push((sheet_idx, rule));
            }
        }
    }
    matching.sort_by_key(|(sheet_idx, r)| (r.specificity, *sheet_idx, r.source_idx));
    let sorted: Vec<&Rule> = matching.iter().map(|(_, r)| *r).collect();

    // Pseudo-elements don't have their own inline_style on `TuiExt`.
    apply_cascade_ladder(&mut working, &sorted, None, host_computed);

    // Border_fg fallback (same rule as for host elements).
    finalize_border_fg(&mut working, &sorted, None);
    finalize_bfc_formation(&mut working);

    // Resolve content:
    //   - None  = no `content:` declaration at all → use legacy fallback
    //   - Some(None) = `content: none;` declared → suppress (NO fallback)
    //   - Some(Some(s)) = content resolved to string
    // Pseudo-elements read attributes from the HOST element — `attr(label)`
    // on `optgroup::before` looks up the `<optgroup>`'s `label` attribute.
    let attr_lookup = |name: &str| dom.node(id).get_attribute(name).map(|s| s.to_string());
    let declared = resolve_content_on(&working, &sorted, None, &attr_lookup);
    let fallback = dom.node(id).ext().and_then(|e| match target {
        PseudoElementTarget::Before => e.before_content.clone(),
        PseudoElementTarget::After => e.after_content.clone(),
        // `::backdrop` and `::selection` have no legacy
        // `before_content`-style field — they're purely
        // style-driven. No fallback content.
        // `::backdrop`, `::selection`, `::scrollbar`, and
        // `::scrollbar-thumb` have no legacy `before_content`-style
        // field — they're purely style-driven. No fallback content.
        PseudoElementTarget::Backdrop
        | PseudoElementTarget::Selection
        | PseudoElementTarget::Scrollbar
        | PseudoElementTarget::ScrollbarThumb
        | PseudoElementTarget::None => None,
    });
    let final_content = match declared {
        Some(explicit) => explicit, // declared (even as None) → use as-is
        None => fallback,           // undeclared → legacy fallback
    };

    // Skip entirely if the pseudo-element has nothing to contribute.
    if sorted.is_empty() && final_content.is_none() {
        return None;
    }
    working.content = final_content;
    Some(working)
}
