//! HTML §6.12.2's popover algorithms: check popover validity, show
//! popover, hide popover, hide all popovers until, close entire popover
//! list, topmost popover ancestor, nearest inclusive open popover.
//!
//! A popover's *visibility state* is its top-layer membership
//! (`TopLayerKind::Popover`). The document's *showing auto* and *showing
//! hint popover lists* are the top layer's popovers, in top-layer order,
//! by the mode each was shown in ([`Popovers::opened`]) — so an element
//! that leaves the document (and with it the top layer, rdom-core's
//! removing steps) leaves its list too. The per-element state HTML keeps
//! (opened mode, invoker, previously focused element, the "showing or
//! hiding" flag) is document data here.

use std::collections::{HashMap, HashSet};

use rdom_core::{DomError, EventDetail, NodeId, ToggleDetail, ToggleState, TopLayerKind};

use super::{PopoverState, is_showing, popover_state};
use crate::{TuiDom, TuiEvent};

/// The popover state of one document (HTML's per-element popover state,
/// kept off `TuiExt`).
#[derive(Debug, Default)]
pub(crate) struct Popovers {
    /// The mode each showing popover was shown in ("opened in popover
    /// mode"): its stack.
    opened: HashMap<NodeId, PopoverState>,
    /// "Popover invoker".
    invoker: HashMap<NodeId, NodeId>,
    /// "Previously focused element", restored on hide.
    previously_focused: HashMap<NodeId, NodeId>,
    /// "Popover showing or hiding": the popovers inside their own show /
    /// hide algorithm, whose nested runs fire no events.
    showing_or_hiding: HashSet<NodeId>,
}

fn data(dom: &mut TuiDom) -> &mut Popovers {
    if dom.document_data::<Popovers>().is_none() {
        dom.set_document_data(Popovers::default());
    }
    dom.document_data_mut::<Popovers>()
        .expect("the popover data was just set")
}

fn opened(dom: &TuiDom, id: NodeId) -> Option<PopoverState> {
    dom.document_data::<Popovers>()?.opened.get(&id).copied()
}

/// The mode a showing popover was shown in (its stack).
pub(super) fn opened_mode(dom: &TuiDom, id: NodeId) -> Option<PopoverState> {
    opened(dom, id).filter(|_| is_showing(dom, id))
}

pub(super) fn invoker_of(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
    if !is_showing(dom, id) {
        return None;
    }
    dom.document_data::<Popovers>()?.invoker.get(&id).copied()
}

/// The showing popovers opened in `mode`, bottom to top: the document's
/// showing auto (or hint) popover list.
pub(super) fn showing_list(dom: &TuiDom, mode: PopoverState) -> Vec<NodeId> {
    dom.top_layer()
        .iter()
        .copied()
        .filter(|&id| is_showing(dom, id) && opened(dom, id) == Some(mode))
        .collect()
}

/// "Check popover validity", throwing: the element has a `popover`
/// attribute, is connected and is not a modal dialog.
pub(super) fn check_attribute(dom: &TuiDom, id: NodeId) -> rdom_core::Result<()> {
    check(dom, id, is_showing(dom, id), true).map(|_| ())
}

/// HTML "check popover validity" for `expect_showing`: `Ok(false)` when
/// the element is not in the expected visibility state or (without
/// `throw`) not a valid popover; the exceptions as errors with `throw`.
fn check(dom: &TuiDom, id: NodeId, expect_showing: bool, throw: bool) -> rdom_core::Result<bool> {
    let fail = |e: DomError| if throw { Err(e) } else { Ok(false) };
    if popover_state(dom, id).is_none() {
        return fail(DomError::NotSupported(
            "popover: the element has no `popover` attribute",
        ));
    }
    if is_showing(dom, id) != expect_showing {
        return Ok(false);
    }
    if !dom.contains(id) || !dom.node(id).is_connected() {
        return fail(DomError::InvalidState(
            "popover: the element is not connected",
        ));
    }
    if dom.top_layer_kind(id) == Some(TopLayerKind::ModalDialog) {
        return fail(DomError::InvalidState(
            "popover: the element is a modal dialog",
        ));
    }
    Ok(true)
}

/// Clears an element's "popover showing or hiding" flag when its
/// outermost algorithm run returns — on every path, panics included.
struct ShowingOrHiding {
    id: NodeId,
    nested: bool,
}

impl ShowingOrHiding {
    /// Set the flag; `nested` when it was set already.
    fn enter(dom: &mut TuiDom, id: NodeId) -> Self {
        let nested = !data(dom).showing_or_hiding.insert(id);
        Self { id, nested }
    }

    fn leave(self, dom: &mut TuiDom) {
        if !self.nested {
            data(dom).showing_or_hiding.remove(&self.id);
        }
    }
}

/// Fire a non-bubbling `beforetoggle` / `toggle` at `id`; returns whether
/// it went uncanceled.
fn fire(
    dom: &mut TuiDom,
    id: NodeId,
    ty: &str,
    open: bool,
    cancelable: bool,
    source: Option<NodeId>,
) -> bool {
    let (old, new) = if open {
        (ToggleState::Closed, ToggleState::Open)
    } else {
        (ToggleState::Open, ToggleState::Closed)
    };
    let mut ev = TuiEvent::new(ty);
    ev.event.bubbles = false;
    ev.event.cancelable = cancelable;
    ev.event.detail =
        EventDetail::Toggle(Box::new(ToggleDetail::new(old, new).with_source(source)));
    crate::tui_event::dispatch_to_live(dom, id, &mut ev) && !ev.event.default_prevented()
}

/// HTML §6.12.2 "show popover" given `throw` and `source`.
pub(super) fn show(
    dom: &mut TuiDom,
    id: NodeId,
    throw: bool,
    source: Option<NodeId>,
) -> rdom_core::Result<()> {
    if !check(dom, id, false, throw)? {
        return Ok(());
    }
    let flag = ShowingOrHiding::enter(dom, id);
    let result = show_flagged(dom, id, throw, source, !flag.nested);
    flag.leave(dom);
    result
}

fn show_flagged(
    dom: &mut TuiDom,
    id: NodeId,
    throw: bool,
    source: Option<NodeId>,
    fire_events: bool,
) -> rdom_core::Result<()> {
    if !fire(dom, id, "beforetoggle", true, true, source) || !check(dom, id, false, throw)? {
        return Ok(());
    }
    let original = popover_state(dom, id).expect("checked above");
    let auto = showing_list(dom, PopoverState::Auto);
    let hint = showing_list(dom, PopoverState::Hint);
    let auto_ancestor = topmost_popover_ancestor(dom, id, &auto, source, true);
    let hint_ancestor = topmost_popover_ancestor(dom, id, &hint, source, true);
    let stack = match original {
        PopoverState::Auto => {
            close_entire_list(dom, PopoverState::Hint, false, fire_events);
            hide_all_until(dom, auto_ancestor, false, fire_events);
            PopoverState::Auto
        }
        PopoverState::Hint => {
            if hint_ancestor.is_some() {
                hide_all_until(dom, hint_ancestor, false, fire_events);
                PopoverState::Hint
            } else {
                close_entire_list(dom, PopoverState::Hint, false, fire_events);
                if auto_ancestor.is_some() {
                    hide_all_until(dom, auto_ancestor, false, fire_events);
                    PopoverState::Auto
                } else {
                    PopoverState::Hint
                }
            }
        }
        PopoverState::Manual => PopoverState::Manual,
    };
    let mut restore_focus = false;
    if original != PopoverState::Manual {
        // Hiding the others ran their events: the popover may have
        // changed under them.
        if popover_state(dom, id) != Some(original) {
            return if throw {
                Err(DomError::InvalidState(
                    "showPopover: the popover attribute changed while showing",
                ))
            } else {
                Ok(())
            };
        }
        if !check(dom, id, false, throw)? {
            return Ok(());
        }
        restore_focus = topmost_auto_or_hint(dom).is_none();
    }
    let originally_focused = dom.focused();
    dom.add_to_top_layer(id, TopLayerKind::Popover)?;
    let d = data(dom);
    d.opened.insert(id, stack);
    d.previously_focused.remove(&id);
    match source {
        Some(s) => d.invoker.insert(id, s),
        None => d.invoker.remove(&id),
    };
    focusing_steps(dom, id);
    if restore_focus
        && popover_state(dom, id).is_some()
        && let Some(previous) = originally_focused
    {
        data(dom).previously_focused.insert(id, previous);
    }
    fire(dom, id, "toggle", true, false, source);
    Ok(())
}

/// The popover focusing steps (HTML §6.12.2): a `<dialog>`'s dialog
/// focusing steps; otherwise the element itself when it has
/// `autofocus`, else its autofocus delegate.
fn focusing_steps(dom: &mut TuiDom, id: NodeId) {
    if dom.node(id).tag_name() == Some("dialog") {
        crate::runtime::builtins::dialog::focusing_steps(dom, id);
    } else {
        crate::runtime::autofocus::focus_within_opened(dom, id);
    }
}

/// The flags of one "hide popover" run.
#[derive(Debug, Clone, Copy)]
pub(super) struct Hide {
    pub(super) focus_previous: bool,
    pub(super) fire_events: bool,
    pub(super) throw: bool,
    /// Skip the attribute's validity (the attribute change steps hide a
    /// popover whose attribute already changed).
    pub(super) ignore_dom_state: bool,
}

impl Hide {
    /// `hidePopover()`.
    pub(super) const THROWING: Self = Self {
        focus_previous: true,
        fire_events: true,
        throw: true,
        ignore_dom_state: false,
    };

    /// A popover hidden by another's algorithm.
    const fn quiet(focus_previous: bool, fire_events: bool) -> Self {
        Self {
            focus_previous,
            fire_events,
            throw: false,
            ignore_dom_state: false,
        }
    }
}

/// Whether the hide algorithm may go on: the popover is showing and,
/// unless `ignore_dom_state`, still a valid popover.
fn still_valid(dom: &TuiDom, id: NodeId, how: Hide) -> rdom_core::Result<bool> {
    if how.ignore_dom_state {
        Ok(is_showing(dom, id))
    } else {
        check(dom, id, true, how.throw)
    }
}

/// HTML §6.12.2 "hide popover algorithm".
pub(super) fn hide(dom: &mut TuiDom, id: NodeId, how: Hide) -> rdom_core::Result<()> {
    if !still_valid(dom, id, how)? {
        return Ok(());
    }
    let flag = ShowingOrHiding::enter(dom, id);
    let how = Hide {
        fire_events: how.fire_events && !flag.nested,
        ..how
    };
    let result = hide_flagged(dom, id, how);
    flag.leave(dom);
    result
}

fn hide_flagged(dom: &mut TuiDom, id: NodeId, how: Hide) -> rdom_core::Result<()> {
    if matches!(
        opened(dom, id),
        Some(PopoverState::Auto | PopoverState::Hint)
    ) {
        hide_all_until(dom, Some(id), how.focus_previous, how.fire_events);
        if !still_valid(dom, id, how)? {
            return Ok(());
        }
    }
    let was_top_auto = showing_list(dom, PopoverState::Auto).last() == Some(&id);
    data(dom).invoker.remove(&id);
    if how.fire_events {
        fire(dom, id, "beforetoggle", false, false, None);
        if was_top_auto && showing_list(dom, PopoverState::Auto).last() != Some(&id) {
            hide_all_until(dom, Some(id), how.focus_previous, false);
        }
        if !still_valid(dom, id, how)? {
            return Ok(());
        }
    }
    dom.remove_from_top_layer(id);
    data(dom).opened.remove(&id);
    if how.fire_events {
        fire(dom, id, "toggle", false, false, None);
    }
    let previous = data(dom).previously_focused.remove(&id);
    if let Some(previous) = previous
        && how.focus_previous
        && dom
            .focused()
            .is_some_and(|f| is_inclusive_ancestor(dom, id, f))
        && dom.contains(previous)
        && dom.node(previous).is_connected()
    {
        crate::runtime::focus::focus_node_with_options(
            dom,
            Some(previous),
            crate::runtime::focus::FocusOptions::new().prevent_scroll(true),
        );
    }
    Ok(())
}

/// HTML §6.12.2 "hide all popovers until" `endpoint` (`None`: the
/// document — every popover).
pub(super) fn hide_all_until(
    dom: &mut TuiDom,
    endpoint: Option<NodeId>,
    focus_previous: bool,
    fire_events: bool,
) {
    if endpoint.is_some_and(|e| !is_showing(dom, e)) {
        return;
    }
    if let Some(e) = endpoint
        && opened(dom, e) == Some(PopoverState::Hint)
    {
        hide_stack_until(dom, e, PopoverState::Hint, focus_previous, fire_events);
        return;
    }
    close_entire_list(dom, PopoverState::Hint, focus_previous, fire_events);
    match endpoint {
        None => close_entire_list(dom, PopoverState::Auto, focus_previous, fire_events),
        Some(e) => hide_stack_until(dom, e, PopoverState::Auto, focus_previous, fire_events),
    }
}

/// HTML "hide popover stack until": hide the popovers of the `mode`
/// list above `endpoint`, top down; if their events showed more above
/// it, again without events.
fn hide_stack_until(
    dom: &mut TuiDom,
    endpoint: NodeId,
    mode: PopoverState,
    focus_previous: bool,
    fire_events: bool,
) {
    let mut fire_events = fire_events;
    loop {
        let list = showing_list(dom, mode);
        let Some(at) = list.iter().position(|&p| p == endpoint) else {
            return;
        };
        let Some(&last_to_hide) = list.get(at + 1) else {
            return;
        };
        while is_showing(dom, last_to_hide) {
            let Some(&top) = showing_list(dom, mode).last() else {
                break;
            };
            hide_or_drop(dom, top, Hide::quiet(focus_previous, fire_events));
        }
        let list = showing_list(dom, mode);
        if list.contains(&endpoint) && list.last() != Some(&endpoint) {
            fire_events = false;
        } else {
            return;
        }
    }
}

/// HTML "close entire popover list" for the `mode` list.
fn close_entire_list(
    dom: &mut TuiDom,
    mode: PopoverState,
    focus_previous: bool,
    fire_events: bool,
) {
    while let Some(&top) = showing_list(dom, mode).last() {
        hide_or_drop(dom, top, Hide::quiet(focus_previous, fire_events));
    }
}

/// Hide `id`; if the algorithm leaves it showing (a listener took its
/// attribute away mid-run), take it out of the top layer anyway, so the
/// stack walks above always make progress.
fn hide_or_drop(dom: &mut TuiDom, id: NodeId, how: Hide) {
    let _ = hide(dom, id, how);
    if is_showing(dom, id) {
        let _ = hide(
            dom,
            id,
            Hide {
                ignore_dom_state: true,
                fire_events: false,
                ..how
            },
        );
    }
    if is_showing(dom, id) {
        dom.remove_from_top_layer(id);
        data(dom).opened.remove(&id);
    }
}

/// The last showing hint popover, else the last showing auto one.
fn topmost_auto_or_hint(dom: &TuiDom) -> Option<NodeId> {
    showing_list(dom, PopoverState::Hint)
        .last()
        .or(showing_list(dom, PopoverState::Auto).last())
        .copied()
}

/// HTML §6.12.2 "topmost popover ancestor" of `new` in `list`, through
/// its parent and through `source`. With `is_popover`, `new` is the
/// popover about to show (placed above `list`), and a hint is no
/// ancestor of an auto popover.
pub(super) fn topmost_popover_ancestor(
    dom: &TuiDom,
    new: NodeId,
    list: &[NodeId],
    source: Option<NodeId>,
    is_popover: bool,
) -> Option<NodeId> {
    let mut positions: HashMap<NodeId, usize> =
        list.iter().enumerate().map(|(i, &p)| (p, i)).collect();
    if is_popover {
        positions.insert(new, list.len());
    }
    let mut topmost: Option<NodeId> = None;
    let mut check_ancestor = |candidate: Option<NodeId>| {
        let mut candidate = candidate;
        while let Some(c) = candidate {
            let Some(ancestor) = nearest_inclusive_open_popover(dom, c) else {
                return;
            };
            let Some(&position) = positions.get(&ancestor) else {
                return;
            };
            let ok_nesting = !is_popover
                || popover_state(dom, new) == popover_state(dom, ancestor)
                || popover_state(dom, ancestor) == Some(PopoverState::Auto);
            if ok_nesting {
                if topmost.is_none_or(|t| positions[&t] < position) {
                    topmost = Some(ancestor);
                }
                return;
            }
            candidate = parent(dom, ancestor);
        }
    };
    check_ancestor(parent(dom, new));
    check_ancestor(source);
    topmost
}

/// HTML "nearest inclusive open popover": `node` or its nearest
/// ancestor that is a showing `auto` or `hint` popover.
pub(super) fn nearest_inclusive_open_popover(dom: &TuiDom, node: NodeId) -> Option<NodeId> {
    let mut cur = Some(node);
    while let Some(id) = cur {
        if matches!(
            popover_state(dom, id),
            Some(PopoverState::Auto | PopoverState::Hint)
        ) && is_showing(dom, id)
        {
            return Some(id);
        }
        cur = parent(dom, id);
    }
    None
}

fn parent(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
    dom.node(id).parent_node().map(|p| p.id())
}

/// `ancestor == id` or `ancestor` contains `id`.
pub(super) fn is_inclusive_ancestor(dom: &TuiDom, ancestor: NodeId, id: NodeId) -> bool {
    let mut cur = Some(id);
    while let Some(n) = cur {
        if n == ancestor {
            return true;
        }
        cur = parent(dom, n);
    }
    false
}
