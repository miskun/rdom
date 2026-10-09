//! Container queries in the cascade (CSS Conditional 5 §6.4–§6.6): an
//! element's query container, the size it answers with, and what the
//! cascade read of it.
//!
//! A query container's size is its content box after layout, which the
//! cascade of its descendants needs — so layout and the cascade
//! interleave (`render::layout_pass::container_pass`): the cascade reads
//! each container's size as last measured (none before its first layout:
//! a size query is then unknown, a container-relative unit the small
//! viewport's) and records, per container, the size it read
//! (`ContainerState`); after each layout the pass measures the queried
//! containers and re-cascades the subtree of every one whose size moved.
//! A document that queries no container records nothing, and the pass
//! costs one lookup.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use rdom_core::{Dom, NodeId, NodeType};
use rdom_style::conditional::{ContainerCondition, QueryContainer, Truth};
use rdom_style::{ConditionId, ConditionKind, Stylesheet};

use crate::ext::TuiExt;
use crate::style::ComputedStyle;

/// A container's size on the axes it answers size queries on (cells).
pub(crate) type AxisSizes = (Option<u16>, Option<u16>);

/// What the cascades read of the document's query containers.
#[derive(Debug, Default)]
struct ContainerState {
    /// An element has computed as a size container: only then does an
    /// element look for one for its container-relative units. Cleared by
    /// a whole-tree cascade, which sets it again if one still is.
    size_containers: Cell<bool>,
    /// Each queried container: the size the cascade read, and the size
    /// the last layout measured.
    queried: RefCell<HashMap<NodeId, Queried>>,
}

#[derive(Debug, Clone, Copy, Default)]
struct Queried {
    /// The size the cascade read (`None` before the first measurement).
    read: Option<AxisSizes>,
    /// The size the last layout gave it.
    measured: Option<AxisSizes>,
    /// A cascade read it since every element that could read it was last
    /// cascaded ([`begin_tree`], [`begin_subtrees`]): only a live entry
    /// is measured, and a dead one is forgotten after the cascade.
    live: bool,
}

/// Before a cascade: make sure the state exists (a `&Dom` cascade records
/// into it).
pub(crate) fn begin(dom: &mut Dom<TuiExt>) {
    if dom.document_data::<ContainerState>().is_none() {
        dom.set_document_data(ContainerState::default());
    }
}

/// Before a whole-tree cascade: every element is about to say again
/// whether it is a size container and which containers it reads, so the
/// flag is cleared and every queried container is dead until read.
pub(crate) fn begin_tree(dom: &Dom<TuiExt>) {
    let Some(state) = state(dom) else {
        return;
    };
    state.size_containers.set(false);
    for entry in state.queried.borrow_mut().values_mut() {
        entry.live = false;
    }
}

/// Before a cascade of the subtrees at `roots`: a queried container in one
/// of them (or one of them) has all its readers — its descendants and its
/// pseudo-elements — re-cascaded, so it is dead until one reads it again.
/// A container above a root keeps its readers outside the root alive.
pub(crate) fn begin_subtrees(dom: &Dom<TuiExt>, roots: &[NodeId]) {
    let Some(state) = state(dom) else {
        return;
    };
    for (&id, entry) in state.queried.borrow_mut().iter_mut() {
        if dom.contains(id) && roots.iter().any(|&r| dom.node(r).contains(id)) {
            entry.live = false;
        }
    }
}

fn state(dom: &Dom<TuiExt>) -> Option<&ContainerState> {
    dom.document_data::<ContainerState>()
}

/// Note that an element computed `computed`: a size container makes the
/// document look up containers for container-relative units.
pub(super) fn note_style(dom: &Dom<TuiExt>, computed: &ComputedStyle) {
    if computed.container_type.queries_inline()
        && let Some(state) = state(dom)
    {
        state.size_containers.set(true);
    }
}

/// Whether an element computed as a size container (tests).
#[cfg(test)]
pub(crate) fn has_size_containers(dom: &Dom<TuiExt>) -> bool {
    state(dom).is_some_and(|s| s.size_containers.get())
}

/// Whether any container's size was read by a cascade (the layout pass's
/// gate).
pub(crate) fn any_queried(dom: &Dom<TuiExt>) -> bool {
    state(dom).is_some_and(|s| !s.queried.borrow().is_empty())
}

/// The size `container` answers with, as last measured, recorded as read.
fn read_size(dom: &Dom<TuiExt>, container: NodeId) -> Option<AxisSizes> {
    let state = state(dom)?;
    let mut queried = state.queried.borrow_mut();
    let entry = queried.entry(container).or_default();
    entry.read = entry.measured;
    entry.live = true;
    entry.measured
}

/// The size `container`'s box gives it now, on the axes its
/// `container-type` answers on; `None` without a box.
fn measure(dom: &Dom<TuiExt>, container: NodeId) -> Option<AxisSizes> {
    let node = dom.node(container);
    let ext = node.ext()?;
    let computed = ext.computed.as_deref()?;
    if computed.display == crate::layout::Display::None {
        return None;
    }
    let ty = computed.container_type;
    let r = ext.content_layout;
    Some((
        ty.queries_inline().then_some(r.width),
        ty.queries_block().then_some(r.height),
    ))
}

/// After a layout: measure every queried container and return those whose
/// size moved from the one the cascade read — the roots to re-cascade.
/// A container no longer in the document, or that no cascade reads any
/// more (dead), is forgotten.
pub(crate) fn stale(dom: &Dom<TuiExt>) -> Vec<NodeId> {
    let Some(state) = state(dom) else {
        return Vec::new();
    };
    let mut queried = state.queried.borrow_mut();
    queried.retain(|&id, e| e.live && dom.contains(id) && dom.node(id).is_connected());
    let mut out = Vec::new();
    for (&id, entry) in queried.iter_mut() {
        entry.measured = measure(dom, id);
        if entry.measured != entry.read {
            out.push(id);
        }
    }
    out.sort_unstable();
    out
}

/// The size `container`'s readers were last cascaded at, and the size the
/// last layout measured ([`stale`]); `None` for a container not queried.
pub(crate) fn sizes(
    dom: &Dom<TuiExt>,
    container: NodeId,
) -> Option<(Option<AxisSizes>, Option<AxisSizes>)> {
    let queried = state(dom)?.queried.borrow();
    queried.get(&container).map(|e| (e.read, e.measured))
}

/// Keep `container`'s readers as they were last cascaded, though its size
/// moved: the layout pass found it cycling (`container_pass`), so its
/// measured size counts as read.
pub(crate) fn freeze(dom: &Dom<TuiExt>, container: NodeId) {
    if let Some(state) = state(dom)
        && let Some(e) = state.queried.borrow_mut().get_mut(&container)
    {
        e.read = e.measured;
    }
}

/// The element ancestors of `id`, nearest first — from `id` itself for a
/// pseudo-element (its originating element is its nearest ancestor for
/// container queries, CSS Conditional 5 §6.4).
fn ancestors(dom: &Dom<TuiExt>, id: NodeId, inclusive: bool) -> impl Iterator<Item = NodeId> + '_ {
    let start = if inclusive {
        Some(id)
    } else {
        dom.node(id).parent_node().map(|p| p.id())
    };
    std::iter::successors(start, move |&n| dom.node(n).parent_node().map(|p| p.id()))
        .filter(move |&n| dom.node(n).node_type() == NodeType::Element)
}

fn computed(dom: &Dom<TuiExt>, id: NodeId) -> Option<&ComputedStyle> {
    dom.node(id).ext()?.computed.as_deref()
}

/// Whether every `@container` condition enclosing a rule of `sheet` under
/// `condition` holds for `id` (a pseudo-element of it when `pseudo`):
/// each evaluated against its query container (§6.4) — unknown, as when
/// there is none, is false.
pub(super) fn holds(
    dom: &Dom<TuiExt>,
    id: NodeId,
    pseudo: bool,
    sheet: &Stylesheet,
    condition: Option<ConditionId>,
) -> bool {
    sheet
        .condition_chain(condition)
        .all(|rule| match &rule.kind {
            ConditionKind::Container(query) => query
                .conditions()
                .iter()
                .any(|c| evaluate(dom, id, pseudo, c).holds()),
            _ => true,
        })
}

/// One `<container-condition>` for `id` (§6.4): the nearest ancestor with
/// its name and of the type its features need is the query container.
fn evaluate(dom: &Dom<TuiExt>, id: NodeId, pseudo: bool, condition: &ContainerCondition) -> Truth {
    #[cfg(test)]
    probe::EVALUATIONS.with(|c| c.set(c.get() + 1));
    let needs_size = condition.needs_size();
    let needs_scroll = condition.needs_scroll_state();
    let found = ancestors(dom, id, pseudo).find_map(|a| {
        let style = computed(dom, a)?;
        let ty = style.container_type;
        let fits = condition.name().is_none_or(|n| style.container_name.has(n))
            && (!needs_size || ty.queries_inline())
            && (!needs_scroll || ty.scroll_state);
        fits.then_some((a, style))
    });
    let Some((container, style)) = found else {
        return Truth::Unknown;
    };
    let (width, height) = if needs_size {
        read_size(dom, container).unwrap_or((None, None))
    } else {
        (None, None)
    };
    // The var map is keyed without the `--`; a running transition's or
    // animation's value is the computed one (`animated_vars`, CSS
    // Transitions 1 §3), as `var()` reads it.
    let vars = style.animated_vars.as_ref().unwrap_or(&style.vars);
    let custom = |name: &str| {
        vars.get(name.strip_prefix("--").unwrap_or(name))
            .map(|v| v.as_str().to_string())
            .filter(|v| !v.trim().is_empty())
    };
    condition.evaluate(&QueryContainer::new(
        width.map(f64::from),
        height.map(f64::from),
        &custom,
    ))
}

/// The query containers `id`'s container-relative units resolve against
/// (§6.6): the nearest size container on the inline axis and the nearest
/// `size` one on the block axis — each `None` when there is none (the
/// small viewport's then). Nothing is looked up in a document with no
/// size container.
pub(super) fn unit_containers(
    dom: &Dom<TuiExt>,
    id: NodeId,
    pseudo: bool,
) -> (Option<NodeId>, Option<NodeId>) {
    if !state(dom).is_some_and(|s| s.size_containers.get()) {
        return (None, None);
    }
    let mut inline = None;
    let mut block = None;
    for a in ancestors(dom, id, pseudo) {
        let Some(style) = computed(dom, a) else {
            continue;
        };
        if inline.is_none() && style.container_type.queries_inline() {
            inline = Some(a);
        }
        if block.is_none() && style.container_type.queries_block() {
            block = Some(a);
        }
        if inline.is_some() && block.is_some() {
            break;
        }
    }
    (inline, block)
}

/// `cx` with the sizes of `containers` (from [`unit_containers`]) for the
/// container-relative units, as last measured — 0 for a container not
/// measured yet, so a unit resolved against it counts as a read
/// ([`note_unit_reads`]) and the layout pass re-cascades once it is
/// measured. No read is recorded here.
pub(super) fn with_unit_sizes(
    dom: &Dom<TuiExt>,
    cx: rdom_style::calc::UnitContext,
    (inline, block): (Option<NodeId>, Option<NodeId>),
) -> rdom_style::calc::UnitContext {
    let measured = |c: NodeId| {
        state(dom)?
            .queried
            .borrow()
            .get(&c)
            .and_then(|e| e.measured)
    };
    let inline_size = inline.map(|c| measured(c).and_then(|s| s.0).map_or(0.0, f64::from));
    let block_size = block.map(|c| measured(c).and_then(|s| s.1).map_or(0.0, f64::from));
    cx.with_container(inline_size, block_size)
}

/// After resolving an element's units with [`with_unit_sizes`] since
/// `before` (`rdom_style::calc::container_reads`): when a
/// container-relative unit read a container, record its containers as
/// read.
pub(super) fn note_unit_reads(
    dom: &Dom<TuiExt>,
    before: u64,
    (inline, block): (Option<NodeId>, Option<NodeId>),
) {
    if rdom_style::calc::container_reads() == before {
        return;
    }
    for c in [inline, block].into_iter().flatten() {
        read_size(dom, c);
    }
}

/// The sheets a cascade without an `App` ran with, kept for the layout
/// pass to re-cascade query containers with (clones, made only when a
/// container was queried, and again only when a sheet changes).
struct HeadlessInputs {
    key: Vec<u64>,
    sheets: std::rc::Rc<[std::rc::Rc<Stylesheet>]>,
    registry: std::rc::Rc<super::PropertyRegistry>,
}

/// After a cascade outside an `App` (`CascadeExt`): when it queried a
/// container, keep its sheets for the layout pass.
pub(crate) fn remember_inputs(dom: &mut Dom<TuiExt>, stylesheets: &[&Stylesheet]) {
    if !(any_queried(dom) || crate::style::content_visibility::any_auto(dom))
        || crate::runtime::style_flush::published(dom).is_some()
    {
        return;
    }
    let key: Vec<u64> = stylesheets.iter().map(|s| s.version()).collect();
    if dom
        .document_data::<HeadlessInputs>()
        .is_some_and(|i| i.key == key)
    {
        return;
    }
    let registry = super::registered::document_registry(dom, stylesheets);
    let sheets = stylesheets
        .iter()
        .map(|s| std::rc::Rc::new((*s).clone()))
        .collect();
    dom.set_document_data(HeadlessInputs {
        key,
        sheets,
        registry,
    });
}

/// The sheets and registry to re-cascade a query container's subtree
/// with, and the `App`'s dirty tracker when one runs the document.
pub(crate) fn inputs(dom: &Dom<TuiExt>) -> Option<crate::runtime::style_flush::CascadeInputs> {
    if let Some(inputs) = crate::runtime::style_flush::published(dom) {
        return Some(inputs);
    }
    let i = dom.document_data::<HeadlessInputs>()?;
    Some(crate::runtime::style_flush::CascadeInputs {
        sheets: i.sheets.clone(),
        registry: i.registry.clone(),
        tracker: None,
    })
}

/// Test-only counters.
#[cfg(test)]
pub(crate) mod probe {
    thread_local! {
        pub static EVALUATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }
}
