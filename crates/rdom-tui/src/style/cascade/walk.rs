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

use super::apply::finalize_bfc_formation;
use super::content::resolve_content_on;
pub(super) use super::counters::CounterState;
use super::counters::{StoredOps, has_ops, takes_part};
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
        registry.seed_root(&mut merged, sheets.viewport());
    }
    let names: Vec<String> = merged.keys().cloned().collect();
    let mut cx = rdom_style::backend::SubstitutionContext::new();
    if let Some(attrs) = attrs {
        cx = cx.with_attrs(attrs);
    }
    let no_parent = std::collections::HashMap::new();
    let mut computed =
        |name: &str, value| registry.computed_value(name, value, &no_parent, sheets.viewport());
    if !registry.is_empty() {
        cx = cx.with_computed(&mut computed);
    }
    // An invalid one is the guaranteed-invalid value (removed); the
    // cascade does not report why.
    let _invalid = rdom_style::backend::resolve_custom_properties(
        &mut merged,
        names.iter().map(String::as_str),
        cx,
    );
    std::rc::Rc::new(merged)
}

/// Bottom-up flags aggregated up the tree during cascade. Each
/// flag mirrors a `TuiExt` field that layout / paint use to skip
/// walks when nothing in the subtree needs them.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct SubtreeFlags {
    pub has_positioned_pseudo: bool,
    pub has_collapse: bool,
    /// `TuiExt::tree_has_counters`.
    pub has_counters: bool,
}

impl SubtreeFlags {
    fn merge(&mut self, other: SubtreeFlags) {
        self.has_positioned_pseudo |= other.has_positioned_pseudo;
        self.has_collapse |= other.has_collapse;
        self.has_counters |= other.has_counters;
    }

    /// The flags `id` recorded at its last cascade.
    fn stored(dom: &Dom<TuiExt>, id: NodeId) -> Self {
        let ext = dom.node(id).ext();
        SubtreeFlags {
            has_positioned_pseudo: ext.is_some_and(|e| e.tree_has_positioned_pseudo),
            has_collapse: ext.is_some_and(|e| e.tree_has_collapse),
            has_counters: ext.is_some_and(|e| e.tree_has_counters),
        }
    }
}

/// `id`'s first child.
pub(super) fn first_child(dom: &Dom<TuiExt>, id: NodeId) -> Option<NodeId> {
    dom.node(id).first_child().map(|n| n.id())
}

/// `id`'s next sibling. The walks step through children with these
/// rather than collecting them: a cascade changes no tree structure.
pub(super) fn next_sibling(dom: &Dom<TuiExt>, id: NodeId) -> Option<NodeId> {
    dom.node(id).next_sibling().map(|n| n.id())
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
    #[cfg(test)]
    probe::visit();
    // Non-element nodes (text / comment / fragment): still recurse so
    // their element children get cascaded — the root is a Fragment by
    // default — but don't compute style for them (TuiExt only carries
    // Element data). The aggregate still bubbles up through them so
    // the layout/paint check at dom.root() picks it up.
    let is_element = dom.node(id).node_type() == NodeType::Element;
    if !is_element {
        let mut flags = SubtreeFlags::default();
        let mut child = first_child(dom, id);
        while let Some(c) = child {
            flags.merge(cascade_subtree(
                dom,
                sheets,
                c,
                parent_computed,
                counters,
                scratch,
                mode,
            ));
            child = next_sibling(dom, c);
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
    // Track this element's counter reads (`TuiExt::reads_counters`).
    counters.take_read();

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
    // Counter values before this element moved and its boxes read one:
    // they must be recomputed even when the element's style is unchanged.
    let reads_moved_counters =
        counters.is_changed() && dom.node(id).ext().is_some_and(|e| e.reads_counters);
    if mode == Mode::Restyle && previous.as_deref() == Some(&computed) && !reads_moved_counters {
        // Nothing this element passes down changed: its boxes and its
        // subtree keep their styles.
        if let Some(ext) = dom.node_mut(id).ext_mut() {
            ext.matched = Some(recorder.finish(sheets));
        }
        // Its own ops were applied computing it; its boxes and its
        // subtree are replayed — except, once counter values moved, the
        // children that take part in counters: they may read one.
        let ops = StoredOps::pseudos_of(dom, id);
        let mut flags = SubtreeFlags::stored(dom, id);
        if counters.is_changed() && flags.has_counters {
            counters.replay_element(parent_id, id, &ops, |counters| {
                let mut child = first_child(dom, id);
                while let Some(c) = child {
                    if takes_part(dom, c) {
                        flags.merge(cascade_subtree(
                            dom, sheets, c, &computed, counters, scratch, mode,
                        ));
                    }
                    child = next_sibling(dom, c);
                }
            });
        } else {
            counters.replay_element(parent_id, id, &ops, |c| c.replay_children(dom, id));
        }
        return flags;
    }
    counters.note_ops(previous.as_deref(), Some(&computed));

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
    let mut reads_counters = counters.take_read();
    // `::before` comes before the children: a changed op there moves
    // their counters.
    counters.note_ops(
        dom.node(id)
            .ext()
            .and_then(|e| e.computed_before.as_deref()),
        computed_before.as_ref(),
    );

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
        has_counters: false,
    };
    let mut child = first_child(dom, id);
    while let Some(c) = child {
        flags.merge(cascade_subtree(
            dom, sheets, c, &computed, counters, scratch, mode,
        ));
        child = next_sibling(dom, c);
    }
    // A kept child leaves its own reads behind: only `::after`'s count.
    counters.take_read();

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
    reads_counters |= counters.take_read();
    counters.exit(id);
    let own_has_positioned_pseudo = computed_before
        .as_ref()
        .is_some_and(|c| c.position != Position::Static)
        || computed_after
            .as_ref()
            .is_some_and(|c| c.position != Position::Static);
    flags.has_positioned_pseudo |= own_has_positioned_pseudo;

    flags.has_counters |= reads_counters
        || has_ops(&computed)
        || computed_before.as_ref().is_some_and(has_ops)
        || computed_after.as_ref().is_some_and(has_ops);

    // Write the bottom-up aggregates.
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        counters.note_ops(ext.computed_after.as_deref(), computed_after.as_ref());
        ext.computed_before = computed_before.map(std::rc::Rc::new);
        ext.computed_after = computed_after.map(std::rc::Rc::new);
        ext.tree_has_positioned_pseudo = flags.has_positioned_pseudo;
        ext.tree_has_collapse = flags.has_collapse;
        ext.tree_has_counters = flags.has_counters;
        ext.reads_counters = reads_counters;
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
        sheets.viewport(),
    );
    let decls = decls.with(substituted.as_ref());
    let preferred = sheets.color_scheme();
    let colors = apply_cascade_ladder(&mut working, plan, decls, parent, preferred);
    // `currentcolor` takes the element's final `color`, `light-dark()`
    // its final `color-scheme`.
    colors.finalize(&mut working, parent.fg, preferred);

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
    let counter_lookup = |name: &str| counters.value(name);
    working.content = resolve_content_on(&working, plan, decls, &counter_lookup).unwrap_or(None);

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

/// Test-only: how many nodes the cascade's walks visited on this thread
/// (cascaded, or walked to replay their counter ops).
#[cfg(test)]
pub(super) mod probe {
    thread_local! {
        static VISITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    pub fn visit() {
        VISITS.with(|c| c.set(c.get() + 1));
    }

    pub fn take() -> usize {
        VISITS.with(|c| c.replace(0))
    }
}
