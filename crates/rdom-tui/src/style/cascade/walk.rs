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
pub(super) use super::counters::CounterState;
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

/// The counter state at the moment a full cascade would reach `target`:
/// replay the stored computed `counter-reset` / `counter-increment` of
/// every element before it in tree order (ancestors are entered, earlier
/// siblings and their subtrees are entered and left). Used by subtree
/// cascades, whose root can sit after arbitrary counter activity.
pub(super) fn counter_state_before(dom: &Dom<TuiExt>, target: NodeId) -> CounterState {
    let mut state = CounterState::default();
    let root = dom.root();
    if root == target {
        return state;
    }
    replay(dom, root, target, &mut state);
    state
}

/// Depth-first replay; returns `true` once `target` is reached (its own
/// ops are not applied — the cascade will apply them).
fn replay(dom: &Dom<TuiExt>, id: NodeId, target: NodeId, state: &mut CounterState) -> bool {
    if id == target {
        return true;
    }
    let parent_id = dom.node(id).parent_node().map(|p| p.id());
    if let Some(c) = dom.node(id).ext().and_then(|e| e.computed.as_ref()) {
        state.enter(parent_id, &c.counter_reset, &c.counter_increment);
    }
    let children: Vec<NodeId> = dom.node(id).child_nodes().map(|n| n.id()).collect();
    for child in children {
        if replay(dom, child, target, state) {
            return true;
        }
    }
    state.exit(id);
    false
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
    id: NodeId,
    parent_computed: &ComputedStyle,
    counters: &mut CounterState,
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
                child,
                parent_computed,
                counters,
            ));
        }
        counters.exit(id);
        return flags;
    }

    // Compute under a shared borrow. `::after` is computed after the
    // children (below): it sits after them in tree order, so a
    // `counter()` in it sees their increments.
    let (
        computed,
        computed_before,
        computed_backdrop,
        computed_selection,
        computed_scrollbar,
        computed_scrollbar_thumb_vertical,
        computed_scrollbar_thumb_horizontal,
    ) = {
        let parent_id = dom.node(id).parent_node().map(|p| p.id());
        let computed = compute_element_style(dom, sheets, id, parent_computed, parent_id, counters);
        let cb = compute_pseudo_style(
            dom,
            sheets,
            id,
            &computed,
            PseudoElementTarget::Before,
            counters,
        );
        let cbd = compute_pseudo_style(
            dom,
            sheets,
            id,
            &computed,
            PseudoElementTarget::Backdrop,
            counters,
        );
        let csel = compute_pseudo_style(
            dom,
            sheets,
            id,
            &computed,
            PseudoElementTarget::Selection,
            counters,
        );
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
        let (csb, csbt_v, csbt_h) = if needs_scrollbar {
            (
                compute_pseudo_style(
                    dom,
                    sheets,
                    id,
                    &computed,
                    PseudoElementTarget::Scrollbar,
                    counters,
                ),
                compute_pseudo_style_layered(
                    dom,
                    sheets,
                    id,
                    &computed,
                    &PseudoElementTarget::thumb_targets(true),
                    counters,
                ),
                compute_pseudo_style_layered(
                    dom,
                    sheets,
                    id,
                    &computed,
                    &PseudoElementTarget::thumb_targets(false),
                    counters,
                ),
            )
        } else {
            (None, None, None)
        };
        (computed, cb, cbd, csel, csb, csbt_v, csbt_h)
    };

    // Diff for layout invalidation. "No previous computed" counts as a
    // change (first cascade).
    let layout_changed = match dom.node(id).ext().and_then(|e| e.computed.as_ref()) {
        Some(prev) => layout_differs(prev, &computed),
        None => true,
    };

    // Write back.
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.computed = Some(std::rc::Rc::new(computed.clone()));
        ext.computed_backdrop = computed_backdrop;
        ext.computed_selection = computed_selection;
        ext.computed_scrollbar = computed_scrollbar;
        ext.computed_scrollbar_thumb_vertical = computed_scrollbar_thumb_vertical;
        ext.computed_scrollbar_thumb_horizontal = computed_scrollbar_thumb_horizontal;
        ext.style_dirty = false;
        if layout_changed {
            ext.layout_dirty = true;
        }
    }

    // Recurse. Children inherit from our computed style. Aggregate
    // children's flags into our subtree flags.
    let mut flags = SubtreeFlags {
        has_positioned_pseudo: false,
        has_collapse: computed.border_collapse == crate::layout::BorderCollapse::Collapse,
    };
    for child in child_ids {
        flags.merge(cascade_subtree(dom, sheets, child, &computed, counters));
    }

    // `::after` comes after the children in tree order.
    let computed_after = compute_pseudo_style(
        dom,
        sheets,
        id,
        &computed,
        PseudoElementTarget::After,
        counters,
    );
    counters.exit(id);
    let own_has_positioned_pseudo = computed_before
        .as_ref()
        .is_some_and(|c| c.position != Position::Static)
        || computed_after
            .as_ref()
            .is_some_and(|c| c.position != Position::Static);
    flags.has_positioned_pseudo |= own_has_positioned_pseudo;

    // Write the bottom-up aggregates.
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.computed_before = computed_before;
        ext.computed_after = computed_after;
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
    id: NodeId,
    parent: &ComputedStyle,
    parent_id: Option<NodeId>,
    counters: &mut CounterState,
) -> ComputedStyle {
    // Start from initial + inherit subset from parent. That includes
    // the custom-property map (an `Rc` clone; `apply_cascade_ladder`
    // copies on write only when this element declares `--*`).
    let mut working = ComputedStyle::initial();
    inherit_inheritable_from(&mut working, parent);

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

    // This element's `counter-reset` / `counter-increment` take effect
    // before its own generated content and its children are seen.
    counters.enter(
        parent_id,
        &working.counter_reset,
        &working.counter_increment,
    );

    // Host element's own `content` property. Normally `None`; authors
    // don't typically set `content` on a real element (CSS restricts it
    // to pseudo-elements) but we allow it for flexibility.
    let attr_lookup = |name: &str| dom.node(id).get_attribute(name).map(|s| s.to_string());
    let counter_lookup = |name: &str| counters.value(name);
    working.content = resolve_content_on(&working, &sorted, inline, &attr_lookup, &counter_lookup)
        .unwrap_or(None);

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
    counters: &CounterState,
) -> Option<ComputedStyle> {
    compute_pseudo_style_layered(dom, sheets, id, host_computed, &[target], counters)
}

/// [`compute_pseudo_style`] over several targets: rules for any of
/// `targets` match, and at equal specificity a rule for a later target
/// wins (the axis-specific `::scrollbar-thumb:vertical` layers over the
/// axis-neutral `::scrollbar-thumb`). Content fallback and the
/// `Some`-ness rule are those of the first target.
fn compute_pseudo_style_layered(
    dom: &Dom<TuiExt>,
    sheets: &[&Stylesheet],
    id: NodeId,
    host_computed: &ComputedStyle,
    targets: &[PseudoElementTarget],
    counters: &CounterState,
) -> Option<ComputedStyle> {
    let target = targets[0];
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
    let mut matching: Vec<(usize, usize, &Rule)> = Vec::new();
    for (sheet_idx, sheet) in sheets.iter().enumerate() {
        for rule in sheet.rules() {
            if let Some(rank) = targets.iter().position(|t| *t == rule.pseudo)
                && dom.matches_list(id, &rule.selector)
            {
                matching.push((rank, sheet_idx, rule));
            }
        }
    }
    matching.sort_by_key(|(rank, sheet_idx, r)| (r.specificity, *rank, *sheet_idx, r.source_idx));
    let sorted: Vec<&Rule> = matching.iter().map(|(_, _, r)| *r).collect();

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
    let counter_lookup = |name: &str| counters.value(name);
    let declared = resolve_content_on(&working, &sorted, None, &attr_lookup, &counter_lookup);
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
        | PseudoElementTarget::ScrollbarThumbVertical
        | PseudoElementTarget::ScrollbarThumbHorizontal
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
