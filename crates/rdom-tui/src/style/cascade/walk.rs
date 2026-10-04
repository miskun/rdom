//! The cascade walk — `cascade_subtree` + the per-element style
//! computation (pseudo-elements: `pseudo`; rule matching: `matching`).
//!
//! `cascade_subtree` recurses into every element in the subtree,
//! computing a fresh `ComputedStyle` at each and writing it back.
//! Text/Comment/Fragment nodes have no `TuiExt` and get skipped
//! structurally (their element children are still visited).

use std::rc::Rc;

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::Position;
use crate::style::{ComputedStyle, PseudoElementTarget, VarMap};

use super::apply::{finalize_bfc_formation, finalize_border_fg};
use super::content::resolve_content_on;
pub(super) use super::counters::CounterState;
use super::counters::StoredOps;
use super::inherit::{inherit_inheritable_from, layout_differs};
use super::ladder::{Declarations, apply_cascade_ladder, prepare};
pub(super) use super::matching::Scratch;
use super::matching::{MatchedRules, Recorder, Rules, Slot};
use super::pseudo::{before_targets, compute_pseudo_style};
pub(super) use super::sheets::Sheets;

/// Merge `root_vars` across all registered sheets into a single
/// `VarMap`. Later sheets win per var name — push order is the
/// last-wins tiebreaker. Allocates one fresh `Rc<HashMap>` per call;
/// callers compute this once per cascade pass (in `cascade_all` /
/// `cascade_subtrees_all`) and `Rc::clone` from there per element.
///
/// These are the `:root` custom properties, so their `attr()`s read the
/// element `:root` matches (CSS Values 5 §8.7, Selectors 4 §14.1): the
/// tree's root when it is an element; a fragment root has no
/// attributes (DIVERGENCES).
pub(super) fn merge_root_vars(dom: &Dom<TuiExt>, sheets: &Sheets<'_>) -> VarMap {
    let mut merged = std::collections::HashMap::new();
    for sheet in sheets.iter() {
        for (k, v) in sheet.vars() {
            merged.insert(k.clone(), v.clone());
        }
    }
    let root = dom.root();
    let root_attrs = |name: &str| dom.node(root).get_attribute(name);
    let attrs: Option<rdom_style::backend::AttrLookup<'_>> =
        (dom.node(root).node_type() == NodeType::Element).then_some(&root_attrs);
    // Their `var()`s substitute against each other (CSS Variables 1 §3).
    // Registered properties start at their initial value and are
    // validated as they resolve, before a dependent reads them
    // (Properties and Values 1 §2.1, §2.4); the root has no parent, so
    // an invalid one is its initial value.
    let registry = sheets.registry();
    if !registry.is_empty() {
        registry.seed_root(&mut merged);
    }
    let names: Vec<String> = merged.keys().cloned().collect();
    if registry.is_empty() {
        rdom_style::backend::resolve_custom_properties_on(
            &mut merged,
            names.iter().map(String::as_str),
            None,
            attrs,
        );
    } else {
        let no_parent = std::collections::HashMap::new();
        rdom_style::backend::resolve_custom_properties_on(
            &mut merged,
            names.iter().map(String::as_str),
            Some(&mut |name, value| registry.computed_value(name, value, &no_parent)),
            attrs,
        );
    }
    std::rc::Rc::new(merged)
}

/// The computed style a subtree root inherits from: its parent's, or
/// the initial style seeded with the sheet-level variables when the
/// parent is the fragment root.
pub(super) fn parent_computed_for(
    dom: &Dom<TuiExt>,
    root: NodeId,
    merged_vars: &VarMap,
) -> std::rc::Rc<ComputedStyle> {
    dom.node(root)
        .parent_node()
        .and_then(|p| p.ext().and_then(|e| e.computed.clone()))
        .unwrap_or_else(|| {
            let mut initial = ComputedStyle::initial();
            initial.vars = merged_vars.clone();
            std::rc::Rc::new(initial)
        })
}

/// If a partial cascade introduced a positioned pseudo or a
/// `border-collapse: collapse` element anywhere in `root`'s subtree,
/// bubble `true` up through the ancestors so the document-level
/// early-exit checks don't stale-`false`. Never bubbles `false` — that
/// would require seeing every ancestor's other subtrees.
pub(super) fn bubble_subtree_flags(dom: &mut Dom<TuiExt>, root: NodeId, flags: SubtreeFlags) {
    if !(flags.has_positioned_pseudo || flags.has_collapse) {
        return;
    }
    let mut cur = dom.node(root).parent_node().map(|p| p.id());
    while let Some(p) = cur {
        if let Some(ext) = dom.node_mut(p).ext_mut() {
            if flags.has_positioned_pseudo {
                ext.tree_has_positioned_pseudo = true;
            }
            if flags.has_collapse {
                ext.tree_has_collapse = true;
            }
        }
        cur = dom.node(p).parent_node().map(|n| n.id());
    }
}

/// Pre-order walk from `id` that cascades each of `roots` (sorted in
/// tree order) when it reaches it and replays the stored counter ops of
/// every element in between, so counters are exact for all roots in one
/// pass. Roots nested inside an earlier root are covered by it and
/// skipped. Stops after the last root.
#[allow(clippy::too_many_arguments)]
pub(super) fn cascade_roots_in_order<'a>(
    dom: &mut Dom<TuiExt>,
    sheets: &Sheets<'a>,
    merged_vars: &VarMap,
    roots: &[NodeId],
    next: &mut usize,
    id: NodeId,
    counters: &mut CounterState,
    scratch: &mut Scratch<'a>,
    mode: Mode,
) {
    if *next >= roots.len() {
        return;
    }
    if id == roots[*next] {
        let parent_computed = parent_computed_for(dom, id, merged_vars);
        let flags = cascade_subtree(dom, sheets, id, &parent_computed, counters, scratch, mode);
        bubble_subtree_flags(dom, id, flags);
        *next += 1;
        // Roots inside this subtree were just cascaded with it.
        while *next < roots.len()
            && dom
                .compare_document_position(id, roots[*next])
                .contains(rdom_core::DocumentPosition::CONTAINED_BY)
        {
            *next += 1;
        }
        return;
    }
    let parent_id = dom.node(id).parent_node().map(|p| p.id());
    // An element between roots keeps its computed styles; replay their
    // counter ops around the walk into its children.
    let ops = StoredOps::of(dom, id);
    let children: Vec<NodeId> = dom.node(id).child_nodes().map(|n| n.id()).collect();
    counters.replay_element(parent_id, id, &ops, |counters| {
        for child in children {
            cascade_roots_in_order(
                dom,
                sheets,
                merged_vars,
                roots,
                next,
                child,
                counters,
                scratch,
                mode,
            );
            if *next >= roots.len() {
                break;
            }
        }
    });
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

/// How a subtree walk gets each element's rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    /// Match every element's selectors, and record the matches.
    Cascade,
    /// Reuse the matches recorded under the same sheets (matching where
    /// there are none), and keep the subtree of an element whose style
    /// comes out unchanged: nothing it passes down changed. For a
    /// restyle no selector's result can change in — a registered custom
    /// property's animated value moving (`C1G-PROPERTY-RESTYLE`).
    Restyle,
}

/// What computing one element's boxes reads: the tree, the sheets, the
/// element, and the pass's counters and buffers.
pub(super) struct ElementCx<'w, 'a> {
    pub dom: &'w Dom<TuiExt>,
    pub sheets: &'w Sheets<'a>,
    pub id: NodeId,
    pub counters: &'w mut CounterState,
    pub scratch: &'w mut Scratch<'a>,
}

/// Compute one box of the element in `cx` from `cached` (else by
/// matching, recorded into `recorder`).
fn compute_box<T>(
    cx: &mut ElementCx<'_, '_>,
    slot: Slot,
    cached: Option<&MatchedRules>,
    recorder: &mut Recorder,
    compute: impl FnOnce(&mut ElementCx<'_, '_>, Rules<'_>) -> T,
) -> T {
    let rules = cached.map_or(Rules::Match, |m| m.rules(slot));
    let out = compute(cx, rules);
    if let Rules::Match = rules {
        recorder.record(slot, cx.scratch);
    }
    out
}

/// Walk `id`'s subtree. Returns the subtree's aggregated
/// [`SubtreeFlags`] — currently `has_positioned_pseudo`
/// (positioned `::before` / `::after`) and `has_collapse`
/// (`border-collapse: collapse` anywhere). Each flag is written to
/// the element's `TuiExt` so layout / paint can do an O(1) check
/// at the root and skip whole walks when nothing relevant is in
/// play. See `TuiExt` docs for the incremental-cascade
/// conservatism rules.
pub(super) fn cascade_subtree<'a>(
    dom: &mut Dom<TuiExt>,
    sheets: &Sheets<'a>,
    id: NodeId,
    parent_computed: &ComputedStyle,
    counters: &mut CounterState,
    scratch: &mut Scratch<'a>,
    mode: Mode,
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
                scratch,
                mode,
            ));
        }
        counters.exit(id);
        return flags;
    }

    // The matches recorded under these sheets: reused by a restyle,
    // compared against (to keep them without allocating) by a cascade.
    let recorded = dom
        .node(id)
        .ext()
        .and_then(|e| e.matched.clone())
        .filter(|m| m.is_for(sheets));
    let cached = recorded.as_deref().filter(|_| mode == Mode::Restyle);

    let mut recorder = Recorder::new(recorded.clone(), mode == Mode::Restyle);
    let parent_id = dom.node(id).parent_node().map(|p| p.id());

    let computed = {
        let mut cx = ElementCx {
            dom: &*dom,
            sheets,
            id,
            counters: &mut *counters,
            scratch: &mut *scratch,
        };
        compute_box(
            &mut cx,
            Slot::Element,
            cached,
            &mut recorder,
            |cx, rules| compute_element_style(cx, parent_computed, parent_id, rules),
        )
    };
    let previous = dom.node(id).ext().and_then(|e| e.computed.clone());
    if mode == Mode::Restyle && previous.as_deref() == Some(&computed) {
        // Nothing this element passes down changed: its boxes and its
        // subtree keep their styles.
        if let Some(ext) = dom.node_mut(id).ext_mut() {
            ext.matched = Some(recorder.finish(sheets));
        }
        // Its own ops were applied computing it; its boxes and its
        // subtree are replayed.
        let ops = StoredOps::pseudos_of(dom, id);
        counters.replay_element(parent_id, id, &ops, |c| c.replay_children(dom, id));
        let ext = dom.node(id).ext();
        return SubtreeFlags {
            has_positioned_pseudo: ext.is_some_and(|e| e.tree_has_positioned_pseudo),
            has_collapse: ext.is_some_and(|e| e.tree_has_collapse),
        };
    }

    // Compute under a shared borrow. `::after` is computed after the
    // children (below): it sits after them in tree order, so a
    // `counter()` in it sees their increments.
    let (
        computed_before,
        computed_backdrop,
        computed_selection,
        computed_scrollbar,
        computed_scrollbar_thumb_vertical,
        computed_scrollbar_thumb_horizontal,
    ) = {
        let mut cx = ElementCx {
            dom: &*dom,
            sheets,
            id,
            counters: &mut *counters,
            scratch: &mut *scratch,
        };
        let mut pseudo = |cx: &mut ElementCx<'_, '_>, slot, targets: &[PseudoElementTarget]| {
            compute_box(cx, slot, cached, &mut recorder, |cx, rules| {
                compute_pseudo_style(cx, &computed, targets, rules)
            })
        };
        let cb = pseudo(&mut cx, Slot::Before, before_targets(dom, id));
        let cbd = pseudo(&mut cx, Slot::Backdrop, &[PseudoElementTarget::Backdrop]);
        let csel = pseudo(&mut cx, Slot::Selection, &[PseudoElementTarget::Selection]);
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
                pseudo(&mut cx, Slot::Scrollbar, &[PseudoElementTarget::Scrollbar]),
                pseudo(
                    &mut cx,
                    Slot::ThumbVertical,
                    &PseudoElementTarget::thumb_targets(true),
                ),
                pseudo(
                    &mut cx,
                    Slot::ThumbHorizontal,
                    &PseudoElementTarget::thumb_targets(false),
                ),
            )
        } else {
            (None, None, None)
        };
        (cb, cbd, csel, csb, csbt_v, csbt_h)
    };

    // Diff for layout invalidation. "No previous computed" counts as a
    // change (first cascade).
    let layout_changed = match &previous {
        Some(prev) => layout_differs(prev, &computed),
        None => true,
    };

    // Write back.
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.computed = Some(std::rc::Rc::new(computed.clone()));
        ext.computed_backdrop = computed_backdrop.map(Rc::new);
        ext.computed_selection = computed_selection.map(Rc::new);
        ext.computed_scrollbar = computed_scrollbar.map(Rc::new);
        ext.computed_scrollbar_thumb_vertical = computed_scrollbar_thumb_vertical.map(Rc::new);
        ext.computed_scrollbar_thumb_horizontal = computed_scrollbar_thumb_horizontal.map(Rc::new);
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
        flags.merge(cascade_subtree(
            dom, sheets, child, &computed, counters, scratch, mode,
        ));
    }

    // `::after` comes after the children in tree order.
    let computed_after = {
        let mut cx = ElementCx {
            dom: &*dom,
            sheets,
            id,
            counters: &mut *counters,
            scratch: &mut *scratch,
        };
        compute_box(&mut cx, Slot::After, cached, &mut recorder, |cx, rules| {
            compute_pseudo_style(cx, &computed, &[PseudoElementTarget::After], rules)
        })
    };
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
        ext.computed_before = computed_before.map(std::rc::Rc::new);
        ext.computed_after = computed_after.map(std::rc::Rc::new);
        ext.tree_has_positioned_pseudo = flags.has_positioned_pseudo;
        ext.tree_has_collapse = flags.has_collapse;
        ext.matched = Some(recorder.finish(sheets));
    }
    flags
}

/// Per-element cascade: start from initial + inheritance, collect
/// matching rules, apply the ladder, resolve `content`, finalize
/// `border_fg`.
fn compute_element_style(
    cx: &mut ElementCx<'_, '_>,
    parent: &ComputedStyle,
    parent_id: Option<NodeId>,
    rules: Rules<'_>,
) -> ComputedStyle {
    let (dom, sheets, id) = (cx.dom, cx.sheets, cx.id);
    // Start from initial + inherit subset from parent. That includes
    // the custom-property map (an `Rc` clone; `apply_cascade_ladder`
    // copies on write only when this element declares `--*`).
    let mut working = ComputedStyle::initial();
    inherit_inheritable_from(&mut working, parent);

    // Collect matching non-pseudo-element rules across all sheets.
    // Cascade order is (specificity, scope proximity, sheet_idx,
    // source_idx) — later sheets win same-specificity contests just
    // like later rules in a single sheet do.
    cx.scratch
        .gather(dom, sheets, id, &[PseudoElementTarget::None], rules);
    let Scratch {
        sorted,
        ranks,
        plan,
        ..
    } = &*cx.scratch;
    let counters = &mut *cx.counters;

    // Inline style on this element (may be empty).
    let inline = dom.node(id).ext().and_then(|e| e.inline_style.as_deref());

    let decls = Declarations::new(sorted, ranks, inline);
    // Running transitions of registered custom properties
    // (`runtime::animation`).
    let transitions = dom
        .node(id)
        .ext()
        .and_then(|e| e.presentation.as_deref())
        .and_then(|p| p.custom_properties.as_ref());
    // `attr()` reads this element's attributes (CSS Values 5 §8.7).
    let attrs = |name: &str| dom.node(id).get_attribute(name);
    let substituted = prepare(
        &mut working,
        plan,
        decls,
        sheets.registry(),
        transitions,
        &attrs,
    );
    let decls = decls.with(substituted.as_ref());
    apply_cascade_ladder(&mut working, plan, decls, parent);

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
    working.content =
        resolve_content_on(&working, plan, decls, &attr_lookup, &counter_lookup).unwrap_or(None);

    // border_fg falls back to working.fg when no rule declared it
    // (property catalog: initial = "inherits fg"). Implemented as a
    // post-pass rather than during apply_color because the author may
    // set fg AFTER border_fg in the rule (same specificity), and we
    // need the *final* fg value as the fallback.
    finalize_border_fg(&mut working, decls);
    // BFC formation predicate (CSS 2.1 §9.4.1). Computed AFTER the
    // cascade ladder so it reads the final values of `flow`,
    // `display`, `overflow_*`, `position`. Used by the block-layout
    // margin-collapse pass — landing here in phase 1 so phase 5 has
    // it ready to consume.
    finalize_bfc_formation(&mut working);
    // Viewport-percentage lengths are absolute at computed-value time
    // (CSS Values 4 §6.1.2).
    working.resolve_viewport_units(sheets.viewport());

    working
}
