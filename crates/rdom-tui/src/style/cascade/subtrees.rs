//! Partial cascades: `cascade_subtrees` and `restyle_vars` over a set
//! of subtree roots.
//!
//! The roots are reduced to the outermost ones first — a root inside
//! another is cascaded with it — and kept in tree order (DOM §4.2.1).
//!
//! Counters (CSS Lists 3 §3.1) make an element's style depend on every
//! counter op before it in tree order, and every counter read after it
//! depend on its ops. A root whose subtree takes no part in counters —
//! creates, increments and reads none, before and after its cascade
//! (`TuiExt::tree_has_counters`) — is cascaded on its own. The others go
//! through one ordered walk from the document root that replays the
//! stored ops of the elements it does not recompute, skips every subtree
//! without counters, and cascades each root when it reaches it. When a
//! root's counter ops changed, the walk goes on past the last root and
//! recomputes the elements after it that take part in counters, so a
//! later `counter()` reads the new value.

use std::collections::HashSet;
use std::rc::Rc;

use rdom_core::{DocumentPosition, Dom, NodeId};

use super::PropertyRegistry;
use super::counters::{CounterState, StoredOps, takes_part};
use super::registered::document_registry;
use super::walk::{
    Mode, Scratch, Sheets, SubtreeFlags, cascade_subtree, first_child, merge_root_vars,
    next_sibling,
};
use crate::ext::TuiExt;
use crate::style::{ComputedStyle, Stylesheet, TuiStyle, VarMap};

/// Cascade (or, per `mode`, restyle) the subtrees at `roots`. Returns
/// the roots of every subtree it recomputed, in the order it did: the
/// outermost of `roots`, then the elements after them whose counters
/// moved.
pub(super) fn subtrees(
    dom: &mut Dom<TuiExt>,
    stylesheets: &[&Stylesheet],
    registry: Option<Rc<PropertyRegistry>>,
    roots: &[NodeId],
    mode: Mode,
) -> Vec<NodeId> {
    let roots = outermost(dom, roots);
    if roots.is_empty() {
        return roots;
    }
    let registry = registry.unwrap_or_else(|| document_registry(dom, stylesheets));
    let (mut done, stale) = walk_subtrees(dom, stylesheets, registry.clone(), roots, mode);
    // A reversed counter's initial value read the boxes after it as last
    // cascaded (CSS Lists 3 §4.2): re-cascade from those it moved — once,
    // as counter ops never depend on counter values.
    if !stale.is_empty() {
        let (again, _) = walk_subtrees(dom, stylesheets, registry, stale, Mode::Cascade);
        done.extend(again);
    }
    done
}

/// [`subtrees`]' one walk: the roots it recomputed, and the elements
/// whose reversed counter's initial value it found stale.
fn walk_subtrees(
    dom: &mut Dom<TuiExt>,
    stylesheets: &[&Stylesheet],
    registry: Rc<PropertyRegistry>,
    roots: Vec<NodeId>,
    mode: Mode,
) -> (Vec<NodeId>, Vec<NodeId>) {
    let roots = outermost(dom, &roots);
    let reads = super::media::begin(dom);
    super::container::begin(dom);
    let sheets = Sheets::new(stylesheets, registry, super::media::document_media(dom));
    super::note_first_rules(dom, &sheets);
    super::details::reclaim_content_boxes(dom);
    let merged_vars = merge_root_vars(dom, &sheets);
    let mut scratch = Scratch::default();
    let walked = walk_with(
        dom,
        stylesheets,
        &sheets,
        &merged_vars,
        roots,
        mode,
        &mut scratch,
    );
    scratch.flag_has_anchors(dom);
    super::media::note_reads(dom, reads);
    walked
}

/// [`walk_subtrees`] with its sheets and buffers made.
fn walk_with<'a>(
    dom: &mut Dom<TuiExt>,
    stylesheets: &[&Stylesheet],
    sheets: &Sheets<'a>,
    merged_vars: &VarMap,
    roots: Vec<NodeId>,
    mode: Mode,
    scratch: &mut Scratch<'a>,
) -> (Vec<NodeId>, Vec<NodeId>) {
    let mut cascade_alone = |dom: &mut Dom<TuiExt>, root: NodeId| {
        let parent_computed = parent_computed_for(dom, root, merged_vars);
        let mut counters = CounterState::default();
        let flags = cascade_subtree(
            dom,
            sheets,
            root,
            &parent_computed,
            &mut counters,
            &mut *scratch,
            mode,
        );
        bubble_subtree_flags(dom, root, flags);
        flags
    };
    if !uses_counters(dom, stylesheets, &roots) {
        for &root in &roots {
            cascade_alone(dom, root);
        }
        return (roots, Vec::new());
    }
    // Only roots inside the document take part: a detached subtree that
    // was marked dirty (the previous demo of a swap, a removed row)
    // renders nothing, and the walk from the document root would never
    // reach it.
    let document = dom.root();
    let mut done = Vec::with_capacity(roots.len());
    let mut ordered = Vec::new();
    let mut gained = Vec::new();
    for root in roots {
        if !(root == document
            || dom
                .compare_document_position(document, root)
                .contains(DocumentPosition::CONTAINED_BY))
        {
            continue;
        }
        if takes_part(dom, root) {
            ordered.push(root);
        } else if cascade_alone(dom, root).has_counters {
            // It gained a counter op or read: the counter values around
            // it are only right from the ordered walk, and the ops after
            // it moved.
            ordered.push(root);
            gained.push(root);
        } else {
            done.push(root);
        }
    }
    if ordered.is_empty() {
        return (done, Vec::new());
    }
    ordered.sort_by(|a, b| tree_order(dom, *a, *b));
    let mut walk = Ordered {
        sheets,
        merged_vars,
        path: ancestors(dom, &ordered),
        roots: &ordered,
        gained: &gained,
        next: 0,
        mode,
        scratch: &mut *scratch,
        recomputed: Vec::new(),
    };
    let mut counters = CounterState::exact();
    walk.visit(dom, document, &mut counters);
    let recomputed = walk.recomputed;
    done.extend(ordered);
    done.extend(recomputed);
    (done, counters.stale_reversed(dom))
}

/// The live roots of `roots` that no other root contains, deduplicated,
/// in their given order.
fn outermost(dom: &Dom<TuiExt>, roots: &[NodeId]) -> Vec<NodeId> {
    // A queued root can have been FREED between when it was marked dirty
    // and now: dropping one child fires `ChildListChanged`, whose
    // dirty-tracker handler marks every remaining sibling dirty (sibling
    // selectors), and one of those siblings may itself be dropped later
    // in the same teardown. A freed node has no subtree to cascade.
    let live: HashSet<NodeId> = roots.iter().copied().filter(|r| dom.contains(*r)).collect();
    let mut seen = HashSet::with_capacity(live.len());
    roots
        .iter()
        .copied()
        .filter(|r| live.contains(r) && seen.insert(*r))
        .filter(|r| {
            let mut up = dom.node(*r).parent_node().map(|p| p.id());
            while let Some(a) = up {
                if live.contains(&a) {
                    return false;
                }
                up = dom.node(a).parent_node().map(|p| p.id());
            }
            true
        })
        .collect()
}

/// The proper ancestors of `roots`: the elements an ordered walk enters
/// to reach them.
fn ancestors(dom: &Dom<TuiExt>, roots: &[NodeId]) -> HashSet<NodeId> {
    let mut path = HashSet::new();
    for root in roots {
        let mut up = dom.node(*root).parent_node().map(|p| p.id());
        while let Some(a) = up {
            if !path.insert(a) {
                break;
            }
            up = dom.node(a).parent_node().map(|p| p.id());
        }
    }
    path
}

/// The ordered walk over the roots whose subtrees take part in counters.
struct Ordered<'w, 'a> {
    sheets: &'w Sheets<'a>,
    merged_vars: &'w VarMap,
    /// The roots, in tree order, none inside another.
    roots: &'w [NodeId],
    /// The roots that gained counters cascaded alone: their ops moved.
    gained: &'w [NodeId],
    /// The roots' proper ancestors.
    path: HashSet<NodeId>,
    /// The next root the walk will reach.
    next: usize,
    mode: Mode,
    scratch: &'w mut Scratch<'a>,
    /// The elements after a moved counter op that were recomputed.
    recomputed: Vec<NodeId>,
}

impl Ordered<'_, '_> {
    /// Visit `id` in tree order. `false` when the walk is over: past the
    /// last root, with no counter value moved.
    fn visit(&mut self, dom: &mut Dom<TuiExt>, id: NodeId, counters: &mut CounterState) -> bool {
        #[cfg(test)]
        super::walk::probe::visit();
        let pending = self.next < self.roots.len();
        if pending && id == self.roots[self.next] {
            let parent_computed = parent_computed_for(dom, id, self.merged_vars);
            let flags = cascade_subtree(
                dom,
                self.sheets,
                id,
                &parent_computed,
                counters,
                self.scratch,
                self.mode,
            );
            bubble_subtree_flags(dom, id, flags);
            if self.gained.contains(&id) {
                counters.mark_changed();
            }
            self.next += 1;
            return true;
        }
        if !pending && !counters.is_changed() {
            return false;
        }
        let parent_id = dom.node(id).parent_node().map(|p| p.id());
        if pending && self.path.contains(&id) {
            // An element between roots keeps its computed styles; replay
            // their counter ops around the walk into its children.
            let ops = StoredOps::of(dom, id);
            let mut go_on = true;
            counters.replay_element(parent_id, id, &ops, |counters| {
                let mut child = first_child(dom, id);
                while let Some(c) = child {
                    if !self.visit(dom, c, counters) {
                        go_on = false;
                        break;
                    }
                    child = next_sibling(dom, c);
                }
            });
            return go_on;
        }
        // No root inside. Without counters it neither moves nor reads one.
        if !takes_part(dom, id) {
            return true;
        }
        if counters.is_changed() {
            // It may read a counter whose value moved: restyle it. No
            // selector's result changed for it, so its matches stand.
            let parent_computed = parent_computed_for(dom, id, self.merged_vars);
            let flags = cascade_subtree(
                dom,
                self.sheets,
                id,
                &parent_computed,
                counters,
                self.scratch,
                Mode::Restyle,
            );
            bubble_subtree_flags(dom, id, flags);
            self.recomputed.push(id);
            return true;
        }
        let ops = StoredOps::of(dom, id);
        counters.replay_element(parent_id, id, &ops, |c| c.replay_children(dom, id));
        true
    }
}

/// The computed style a subtree root inherits from: its parent's — a
/// `<details>` element's content slot's for its content — or the initial
/// style seeded with the sheet-level variables when the parent is the
/// fragment root.
pub(super) fn parent_computed_for(
    dom: &Dom<TuiExt>,
    root: NodeId,
    merged_vars: &VarMap,
) -> Rc<ComputedStyle> {
    let parent = dom.node(root).parent_node().map(|p| p.id());
    // A `<details>` element's content inherits from its slot.
    if let Some(slot) = parent.and_then(|p| super::details::inherited_style(dom, p, root)) {
        return slot;
    }
    dom.node(root)
        .parent_node()
        .and_then(|p| p.ext().and_then(|e| e.computed.clone()))
        .unwrap_or_else(|| {
            let mut initial = ComputedStyle::initial();
            initial.vars = merged_vars.clone();
            Rc::new(initial)
        })
}

/// If a partial cascade introduced a positioned pseudo, a
/// `border-collapse: collapse` element or a counter anywhere in `root`'s
/// subtree, bubble `true` up through the ancestors so the document-level
/// early-exit checks don't stale-`false`. Never bubbles `false` — that
/// would require seeing every ancestor's other subtrees.
fn bubble_subtree_flags(dom: &mut Dom<TuiExt>, root: NodeId, flags: SubtreeFlags) {
    if !(flags.has_positioned_pseudo || flags.has_collapse || flags.has_counters) {
        return;
    }
    let mut cur = dom.node(root).parent_node().map(|p| p.id());
    while let Some(p) = cur {
        if let Some(ext) = dom.node_mut(p).ext_mut() {
            ext.tree_has_positioned_pseudo |= flags.has_positioned_pseudo;
            ext.tree_has_collapse |= flags.has_collapse;
            ext.tree_has_counters |= flags.has_counters;
        }
        cur = dom.node(p).parent_node().map(|n| n.id());
    }
}

/// Can anything create, increment or read a counter (CSS Lists 3 §3)?
/// Then every subtree root depends on the elements before it in tree
/// order. Counter ops come from the sheets' rules and from `style`
/// attributes (CSS Style Attributes §3): those already cascaded are in
/// the document's `tree_has_counters`, and a root's subtree may hold new
/// ones.
fn uses_counters(dom: &Dom<TuiExt>, stylesheets: &[&Stylesheet], roots: &[NodeId]) -> bool {
    stylesheets
        .iter()
        .any(|s| s.rules().iter().any(|r| style_uses_counters(&r.style)))
        || dom
            .node(dom.root())
            .child_nodes()
            .any(|c| c.ext().is_some_and(|e| e.tree_has_counters))
        || roots.iter().any(|&r| inline_uses_counters(dom, r))
}

/// Does a `style` attribute in the subtree at `id` use counters?
fn inline_uses_counters(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    let mut stack = vec![id];
    while let Some(id) = stack.pop() {
        let node = dom.node(id);
        if node
            .ext()
            .and_then(|e| e.inline_style.as_deref())
            .is_some_and(style_uses_counters)
        {
            return true;
        }
        stack.extend(node.child_nodes().map(|c| c.id()));
    }
    false
}

/// Does `style` create, increment or read a counter, or hold a `<quote>`
/// item (the quote depth runs in tree order like a counter, CSS
/// Generated Content 3 §2.2)? A `var()`
/// declaration counts when it is one of those properties (or `all`):
/// only its substitution can tell.
fn style_uses_counters(style: &TuiStyle) -> bool {
    style.counter_reset.is_some()
        || style.counter_increment.is_some()
        || style.pending.iter().any(|d| {
            d.has_substitution
                && matches!(
                    d.name.as_str(),
                    "counter-reset" | "counter-increment" | "content" | "all"
                )
        })
        || style
            .content
            .as_ref()
            .and_then(|c| c.as_specified())
            .is_some_and(|c| c.uses_counters() || c.uses_quotes())
}

/// Tree order (DOM §4.2.1) for two live nodes; equal only for the same node.
fn tree_order(dom: &Dom<TuiExt>, a: NodeId, b: NodeId) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    if a == b {
        return Ordering::Equal;
    }
    let pos = dom.compare_document_position(a, b);
    if pos.contains(DocumentPosition::FOLLOWING) {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}
