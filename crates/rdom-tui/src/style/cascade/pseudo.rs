//! Pseudo-element styles (`::before`, `::after`, `::backdrop`,
//! `::selection`, `::placeholder`, the scrollbar parts): the same
//! ladder as an element, inheriting from the host's computed style.

use rdom_core::{Dom, NodeId};

use super::apply::{finalize_bfc_formation, finalize_border_fg};
use super::content::resolve_content_on;
use super::counters::CounterState;
use super::inherit::inherit_inheritable_from;
use super::ladder::{Declarations, apply_cascade_ladder, prepare};
use super::matching::Scratch;
use super::sheets::Sheets;
use crate::ext::TuiExt;
use crate::style::{ComputedStyle, PseudoElementTarget};

/// The rule targets that style `id`'s `::before` box. An `<input>` /
/// `<textarea>` showing its placeholder paints the placeholder text as
/// its `::before` (UA `:placeholder-shown::before { content:
/// attr(placeholder) }`), so that box *is* the `::placeholder`
/// pseudo-element (CSS Pseudo-Elements 4 §4.3) and its rules layer on
/// top — a `::placeholder` rule wins a specificity tie with a
/// `::before` one. The rules were cut to the `::first-line` property
/// subset when they were built, so they can restyle the text but not
/// replace or move it.
pub(super) fn before_targets(dom: &Dom<TuiExt>, id: NodeId) -> &'static [PseudoElementTarget] {
    const BEFORE: &[PseudoElementTarget] = &[PseudoElementTarget::Before];
    const PLACEHOLDER: &[PseudoElementTarget] = &[
        PseudoElementTarget::Before,
        PseudoElementTarget::Placeholder,
    ];
    let node = dom.node(id);
    let control = matches!(node.tag_name(), Some("input" | "textarea"));
    if control && dom.is_placeholder_shown(id) {
        PLACEHOLDER
    } else {
        BEFORE
    }
}

/// Pseudo-element computation. Returns `None` if the pseudo-element
/// should not render (no matching rules AND no legacy
/// `before_content` / `after_content` text set AND no `content`
/// resolved).
pub(super) fn compute_pseudo_style<'a>(
    dom: &Dom<TuiExt>,
    sheets: &Sheets<'a>,
    id: NodeId,
    host_computed: &ComputedStyle,
    target: PseudoElementTarget,
    counters: &mut CounterState,
    scratch: &mut Scratch<'a>,
) -> Option<ComputedStyle> {
    compute_pseudo_style_layered(dom, sheets, id, host_computed, &[target], counters, scratch)
}

/// [`compute_pseudo_style`] over several targets: rules for any of
/// `targets` match, and at equal specificity a rule for a later target
/// wins (the axis-specific `::scrollbar-thumb:vertical` layers over the
/// axis-neutral `::scrollbar-thumb`). Content fallback and the
/// `Some`-ness rule are those of the first target.
pub(super) fn compute_pseudo_style_layered<'a>(
    dom: &Dom<TuiExt>,
    sheets: &Sheets<'a>,
    id: NodeId,
    host_computed: &ComputedStyle,
    targets: &[PseudoElementTarget],
    counters: &mut CounterState,
    scratch: &mut Scratch<'a>,
) -> Option<ComputedStyle> {
    let target = targets[0];
    if target == PseudoElementTarget::None {
        return None;
    }

    // Collect matching rules for this pseudo across all sheets, with
    // sheet_idx as the secondary tiebreaker.
    scratch.collect(dom, sheets, id, targets);
    let fallback = legacy_content(dom, id, target);
    // No rule styles it and it has no legacy content: there is no box,
    // and nothing to cascade.
    if scratch.sorted.is_empty() && fallback.is_none() {
        return None;
    }
    let Scratch {
        sorted,
        ranks,
        plan,
        ..
    } = &*scratch;

    // Pseudo-elements inherit from the host's computed style (per spec),
    // not from the host's parent.
    let mut working = ComputedStyle::initial();
    inherit_inheritable_from(&mut working, host_computed);
    // Pseudo-elements share the host's vars (which came from the
    // merged stylesheet roots).
    working.vars = host_computed.vars.clone();

    // Pseudo-elements don't have their own inline_style on `TuiExt`.
    let decls = Declarations::new(sorted, ranks, None);
    let substituted = prepare(&mut working, plan, decls, sheets.registry(), None);
    let decls = decls.with(substituted.as_ref());
    apply_cascade_ladder(&mut working, plan, decls, host_computed);

    // Border_fg fallback (same rule as for host elements).
    finalize_border_fg(&mut working, decls);
    finalize_bfc_formation(&mut working);

    // Resolve content:
    //   - None  = no `content:` declaration at all → use legacy fallback
    //   - Some(None) = `content: none;` declared → suppress (NO fallback)
    //   - Some(Some(s)) = content resolved to string
    // Pseudo-elements read attributes from the HOST element — `attr(label)`
    // on `optgroup::before` looks up the `<optgroup>`'s `label` attribute.
    // The pseudo-element's own `counter-reset` / `counter-increment`
    // (the `h2::before { counter-increment: sec }` idiom). It is a child
    // of the host, so its instances are scoped to the host's subtree.
    counters.enter(Some(id), &working.counter_reset, &working.counter_increment);
    let attr_lookup = |name: &str| dom.node(id).get_attribute(name).map(|s| s.to_string());
    let counter_lookup = |name: &str| counters.value(name);
    let declared = resolve_content_on(&working, plan, decls, &attr_lookup, &counter_lookup);
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

/// The legacy `before_content` / `after_content` text of `id`'s
/// `target` box, used when no rule declares `content`.
fn legacy_content(dom: &Dom<TuiExt>, id: NodeId, target: PseudoElementTarget) -> Option<String> {
    dom.node(id).ext().and_then(|e| match target {
        PseudoElementTarget::Before => e.before_content.clone(),
        PseudoElementTarget::After => e.after_content.clone(),
        // `::backdrop`, `::selection`, `::scrollbar`, and
        // `::scrollbar-thumb` have no legacy `before_content`-style
        // field — they're purely style-driven. No fallback content.
        PseudoElementTarget::Backdrop
        | PseudoElementTarget::Selection
        | PseudoElementTarget::Scrollbar
        | PseudoElementTarget::ScrollbarThumb
        | PseudoElementTarget::ScrollbarThumbVertical
        | PseudoElementTarget::ScrollbarThumbHorizontal
        | PseudoElementTarget::Placeholder
        | PseudoElementTarget::None => None,
        // `PseudoElementTarget` is `#[non_exhaustive]`: a later
        // pseudo-element has no legacy content field either.
        _ => None,
    })
}
