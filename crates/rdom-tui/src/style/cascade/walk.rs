//! The cascade walk — `cascade_subtree`, which styles each element
//! (`element`; pseudo-elements: `pseudo`; rule matching: `matching`)
//! before its children and finishes it after them.
//!
//! `cascade_subtree` recurses into every element in the subtree,
//! computing a fresh `ComputedStyle` at each and writing it back.
//! Text/Comment/Fragment nodes have no `TuiExt` and get skipped
//! structurally (their element children are still visited).

use std::rc::Rc;

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::Position;
use crate::node::TuiNodeExt;
use crate::style::{ComputedStyle, PseudoElementTarget};

pub(super) use super::counters::CounterState;
use super::counters::{StoredOps, has_ops, takes_part};
use super::element::compute_element_style;
use super::inherit::layout_differs;
pub(super) use super::matching::Scratch;
use super::matching::{MatchedRules, Recorder, Rules, Slot};
use super::pseudo::compute_pseudo_style;
pub(super) use super::root_vars::merge_root_vars;
pub(super) use super::sheets::Sheets;

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
pub(super) fn compute_box<T>(
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
///
/// The walk recurses once per tree level, so this frame holds only what
/// must outlive the children's cascade — the element's styles behind
/// `Rc`s; computing them happens in [`style_element`] and
/// [`finish_element`], whose large frames are gone before the next
/// level starts (a style is kilobytes, and an element computes eight).
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

    let styled = match style_element(dom, sheets, id, parent_computed, counters, scratch, mode) {
        Styled::Kept {
            computed,
            parent_id,
        } => {
            return replay_kept(
                dom, sheets, id, &computed, parent_id, counters, scratch, mode,
            );
        }
        Styled::Fresh(styled) => styled,
    };

    // Recurse. Children inherit from our computed style. Aggregate
    // children's flags into our subtree flags.
    let mut flags = SubtreeFlags {
        has_positioned_pseudo: false,
        has_collapse: styled.computed.border_collapse == crate::layout::BorderCollapse::Collapse,
        has_counters: false,
    };
    let mut child = first_child(dom, id);
    while let Some(c) = child {
        // A `<details>` element's content inherits from its
        // `::details-content` slot.
        let slot = super::details::inherited_style(dom, id, c);
        flags.merge(cascade_subtree(
            dom,
            sheets,
            c,
            slot.as_deref().unwrap_or(&styled.computed),
            counters,
            scratch,
            mode,
        ));
        child = next_sibling(dom, c);
    }
    // A kept child leaves its own reads behind: only `::after`'s count.
    counters.take_read();
    finish_element(dom, sheets, id, styled, flags, counters, scratch)
}

/// [`style_element`]'s outcome. It lives on the stack for one call, so
/// the fresh variant's size (its match recorder's slots) costs nothing a
/// box would save, and boxing it would allocate per element.
#[allow(clippy::large_enum_variant)]
enum Styled {
    /// A restyle left the element's style unchanged: nothing it passes
    /// down changed, so its boxes and its subtree keep theirs.
    Kept {
        computed: Rc<ComputedStyle>,
        parent_id: Option<NodeId>,
    },
    /// The element's new style, its `::before`, and what its `::after`
    /// needs once the children are cascaded.
    Fresh(FreshElement),
}

/// An element styled before its children: what [`finish_element`]
/// needs after them.
struct FreshElement {
    computed: Rc<ComputedStyle>,
    computed_before: Option<Rc<ComputedStyle>>,
    recorded: Option<Rc<MatchedRules>>,
    recorder: Recorder,
    reads_counters: bool,
    /// A restyle: `::after` reuses the recorded matches too.
    restyle: bool,
}

/// Compute the element `id`'s style, and — unless a restyle keeps it —
/// its `::before` and the pseudo-elements that do not depend on its
/// children, writing what it can back. Not inlined into the recursion:
/// its frame holds every style it computes.
#[inline(never)]
fn style_element<'a>(
    dom: &mut Dom<TuiExt>,
    sheets: &Sheets<'a>,
    id: NodeId,
    parent_computed: &ComputedStyle,
    counters: &mut CounterState,
    scratch: &mut Scratch<'a>,
    mode: Mode,
) -> Styled {
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
    // A box-less element's children take their parent box — so whether
    // they are flex or grid items, blockified (CSS Display 3 §2.5 /
    // §2.7) — from above it: its own style staying the same keeps theirs
    // only while no element between it and that box changed the answer
    // in this restyle (C7G-MINOR). An element that changes it is noted
    // for the walk below it.
    let item_parent = |c: &ComputedStyle| {
        (
            c.display == crate::layout::Display::Contents,
            c.flow.is_flex_or_grid(),
        )
    };
    if mode == Mode::Restyle
        && previous
            .as_deref()
            .is_some_and(|p| item_parent(p) != item_parent(&computed))
    {
        scratch.items_changed.push(id);
    }
    // CSS Values 4 §6.1.1: every `rlh` reads the root element's line
    // height, absolute at computed-value time — a restyle that moves it
    // reaches them under elements whose own style stays.
    if mode == Mode::Restyle
        && dom.document_element().id() == id
        && previous
            .as_deref()
            .is_some_and(|p| p.text.line_height.rows() != computed.text.line_height.rows())
    {
        scratch.root_line_height_moved = true;
    }
    let keeps_subtree = (computed.display != crate::layout::Display::Contents
        || !items_changed_above(dom, id, &scratch.items_changed))
        && !scratch.root_line_height_moved;
    if mode == Mode::Restyle
        && let Some(previous) = previous.as_ref().filter(|p| ***p == computed)
        && !reads_moved_counters
        && keeps_subtree
    {
        if let Some(ext) = dom.node_mut(id).ext_mut() {
            ext.matched = Some(recorder.finish(sheets));
        }
        return Styled::Kept {
            computed: previous.clone(),
            parent_id,
        };
    }
    counters.note_ops(previous.as_deref(), Some(&computed));
    if computed.list_item {
        crate::style::doc_flags::note_list_item(dom);
    }

    // Compute under a shared borrow. `::after` is computed after the
    // children (`finish_element`): it sits after them in tree order, so
    // a `counter()` in it sees their increments.
    let early = {
        let mut cx = ElementCx {
            dom: &*dom,
            sheets,
            id,
            counters: &mut *counters,
            scratch: &mut *scratch,
        };
        super::early_pseudos::compute(&mut cx, &computed, cached, &mut recorder)
    };
    let reads_counters = counters.take_read();
    // `::marker` and `::before` come before the children: a changed op
    // there moves their counters.
    counters.note_ops(
        dom.node(id)
            .ext()
            .and_then(|e| e.computed_marker().map(|s| &**s)),
        early.marker.as_ref(),
    );
    counters.note_ops(
        dom.node(id)
            .ext()
            .and_then(|e| e.computed_before.as_deref()),
        early.before.as_ref(),
    );

    // Diff for layout invalidation. "No previous computed" counts as a
    // change (first cascade).
    let layout_changed = match &previous {
        Some(prev) => layout_differs(prev, &computed),
        None => true,
    };

    // Write back; `::before` waits for the children (`finish_element`).
    let computed = Rc::new(computed);
    let mut early = early;
    let before = early.before.take();
    let auto_direction = crate::style::dir_auto::styled_direction(dom, id);
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.computed = Some(computed.clone());
        ext.auto_direction = auto_direction;
        early.write(ext);
        ext.style_dirty = false;
        if layout_changed {
            ext.layout_dirty = true;
        }
    }
    super::details::sync_content_box(dom, id);
    Styled::Fresh(FreshElement {
        computed,
        computed_before: before.map(Rc::new),
        recorded,
        recorder,
        reads_counters,
        restyle: mode == Mode::Restyle,
    })
}

/// Whether the restyle changed the answer the box-less element `id`'s
/// children read for their blockification: whether an element between
/// it and its box parent, or the box parent (CSS Display 3 §2.5), is
/// among `changed` — which the top-down walk filled before reaching `id`.
fn items_changed_above(dom: &Dom<TuiExt>, id: NodeId, changed: &[NodeId]) -> bool {
    if changed.is_empty() {
        return false;
    }
    // Box parents (`render::box_tree::slot`): slotted content's is its
    // `::details-content` box.
    let mut cur = super::details::box_parent(dom, id);
    while let Some(id) = cur {
        let n = dom.node(id);
        if n.node_type() != NodeType::Element {
            return false;
        }
        if changed.contains(&id) {
            return true;
        }
        if !n
            .computed()
            .is_some_and(|c| c.display == crate::layout::Display::Contents)
        {
            return false;
        }
        cur = super::details::box_parent(dom, id);
    }
    false
}

/// A restyle kept the element `id`'s style (`computed`): its boxes and
/// its subtree are replayed — except, once counter values moved, the
/// children that take part in counters: they may read one.
#[allow(clippy::too_many_arguments)]
fn replay_kept<'a>(
    dom: &mut Dom<TuiExt>,
    sheets: &Sheets<'a>,
    id: NodeId,
    computed: &ComputedStyle,
    parent_id: Option<NodeId>,
    counters: &mut CounterState,
    scratch: &mut Scratch<'a>,
    mode: Mode,
) -> SubtreeFlags {
    // Its own ops were applied computing it.
    let ops = StoredOps::pseudos_of(dom, id);
    let mut flags = SubtreeFlags::stored(dom, id);
    if counters.is_changed() && flags.has_counters {
        counters.replay_element(parent_id, id, &ops, |counters| {
            let mut child = first_child(dom, id);
            while let Some(c) = child {
                if takes_part(dom, c) {
                    let slot = super::details::inherited_style(dom, id, c);
                    let parent = slot.as_deref().unwrap_or(computed);
                    flags.merge(cascade_subtree(
                        dom, sheets, c, parent, counters, scratch, mode,
                    ));
                }
                child = next_sibling(dom, c);
            }
        });
    } else {
        counters.replay_element(parent_id, id, &ops, |c| c.replay_children(dom, id));
    }
    flags
}

/// After the element `id`'s children: its `::after` (which sees their
/// counter increments) and the bottom-up aggregates, written back. Not
/// inlined into the recursion, as [`style_element`] is not.
#[inline(never)]
fn finish_element<'a>(
    dom: &mut Dom<TuiExt>,
    sheets: &Sheets<'a>,
    id: NodeId,
    styled: FreshElement,
    mut flags: SubtreeFlags,
    counters: &mut CounterState,
    scratch: &mut Scratch<'a>,
) -> SubtreeFlags {
    let FreshElement {
        computed,
        computed_before,
        recorded,
        mut recorder,
        mut reads_counters,
        restyle,
    } = styled;
    let cached = recorded.as_deref().filter(|_| restyle);
    // `::after` comes after the children in tree order, and its own
    // marker after it (CSS Pseudo-Elements 4 §4).
    let (computed_after, after_marker) = {
        let mut cx = ElementCx {
            dom: &*dom,
            sheets,
            id,
            counters: &mut *counters,
            scratch: &mut *scratch,
        };
        let after = compute_box(&mut cx, Slot::After, cached, &mut recorder, |cx, rules| {
            compute_pseudo_style(cx, &computed, &[PseudoElementTarget::After], rules)
        });
        let marker =
            super::early_pseudos::after_marker(&mut cx, after.as_ref(), cached, &mut recorder);
        (after, marker)
    };
    reads_counters |= counters.take_read();
    counters.exit(id);
    let own_has_positioned_pseudo = computed_before
        .as_deref()
        .is_some_and(|c| c.position != Position::Static)
        || computed_after
            .as_ref()
            .is_some_and(|c| c.position != Position::Static);
    flags.has_positioned_pseudo |= own_has_positioned_pseudo;

    flags.has_counters |= reads_counters
        || has_ops(&computed)
        || dom
            .node(id)
            .ext()
            .and_then(|e| e.computed_marker().map(|s| &**s))
            .is_some_and(has_ops)
        || computed_before.as_deref().is_some_and(has_ops)
        || computed_after.as_ref().is_some_and(has_ops);

    // Write the bottom-up aggregates.
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        counters.note_ops(ext.computed_after.as_deref(), computed_after.as_ref());
        ext.computed_before = computed_before;
        ext.computed_after = computed_after.map(std::rc::Rc::new);
        let marker = after_marker.map(std::rc::Rc::new);
        ext.update_pseudo(marker.is_some(), |p| p.after_marker = marker);
        ext.tree_has_positioned_pseudo = flags.has_positioned_pseudo;
        ext.tree_has_collapse = flags.has_collapse;
        ext.tree_has_counters = flags.has_counters;
        ext.reads_counters = reads_counters;
        ext.matched = Some(recorder.finish(sheets));
    }
    super::details::mirror_flags(dom, id);
    flags
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
