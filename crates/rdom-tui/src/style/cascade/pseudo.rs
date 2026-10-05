//! Pseudo-element styles (`::before`, `::after`, `::backdrop`,
//! `::selection`, `::placeholder`, the scrollbar parts): the same
//! ladder as an element, inheriting from the host's computed style.

use rdom_core::{Dom, NodeId};

use super::apply::finalize_bfc_formation;
use super::content::resolve_content_on;
use super::decoration::finalize_used_border;
use super::inherit::inherit_inheritable_from;
use super::ladder::{Declarations, apply_cascade_ladder, prepare};
use super::matching::{Rules, Scratch};
use super::walk::ElementCx;
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

/// A pseudo-element's computed style over `targets`: rules for any of
/// them apply, and at equal specificity a rule for a later target wins
/// (the axis-specific `::scrollbar-thumb:vertical` layers over the
/// axis-neutral `::scrollbar-thumb`). Content fallback and the
/// `Some`-ness rule are those of the first target. `None` if the
/// pseudo-element should not render (no matching rules AND no legacy
/// `before_content` / `after_content` text set AND no `content`
/// resolved).
pub(super) fn compute_pseudo_style(
    cx: &mut ElementCx<'_, '_>,
    host_computed: &ComputedStyle,
    targets: &[PseudoElementTarget],
    rules: Rules<'_>,
) -> Option<ComputedStyle> {
    let target = targets[0];
    if target == PseudoElementTarget::None {
        return None;
    }
    let (dom, id) = (cx.dom, cx.id);

    // Collect matching rules for this pseudo across all sheets, with
    // sheet_idx as the secondary tiebreaker.
    cx.scratch.gather(dom, cx.sheets, id, targets, rules);
    let fallback = legacy_content(dom, id, target);
    // No rule styles it and it has no legacy content: there is no box,
    // and nothing to cascade.
    if cx.scratch.sorted.is_empty() && fallback.is_none() {
        return None;
    }
    let Scratch {
        sorted,
        ranks,
        plan,
        ..
    } = &*cx.scratch;
    let counters = &mut *cx.counters;

    // Pseudo-elements inherit from the host's computed style (per spec),
    // not from the host's parent.
    // Pseudo-elements don't have their own inline_style on `TuiExt`.
    let decls = Declarations::new(sorted, ranks, None);
    // `attr()` on a pseudo-element reads its originating element's
    // attributes (CSS Values 5 §8.7).
    let attrs = |name: &str| dom.node(id).get_attribute(name);
    let preferred = cx.sheets.color_scheme();
    // The inline-axis flow-relative properties map by the pseudo-element's
    // own `direction`, as for an element (`walk::compute_element_style`).
    let directional = decls.has_directional();
    let mut direction = host_computed.text_direction;
    let mut runs = 0;
    let (mut working, substituted, colors) = loop {
        // Pseudo-elements inherit from the host's computed style (per
        // spec), not from the host's parent, and share the host's vars
        // (which came from the merged stylesheet roots).
        let mut working = ComputedStyle::initial();
        inherit_inheritable_from(&mut working, host_computed);
        working.vars = host_computed.vars.clone();
        working.text_direction = direction;
        let substituted = prepare(
            &mut working,
            plan,
            decls,
            cx.sheets.registry(),
            None,
            &attrs,
            cx.sheets.viewport(),
        );
        let colors = apply_cascade_ladder(
            &mut working,
            plan,
            decls.with(substituted.as_ref(), direction),
            host_computed,
            preferred,
        );
        if !directional || working.text_direction == direction {
            break (working, substituted, colors);
        }
        // A flow-relative property cannot change `direction`, so the run
        // with the element's own direction settles it: two runs at most.
        runs += 1;
        debug_assert!(runs < 2, "the direction re-run settles `direction`");
        direction = working.text_direction;
    };
    let decls = decls.with(substituted.as_ref(), working.text_direction);
    colors.finalize(&mut working, host_computed.fg, preferred);

    // `::before` / `::after` are child boxes of the host (CSS
    // Pseudo-Elements 4 §4): flex items, blockified, when it is a flex
    // container (CSS Flexbox §4).
    if matches!(
        target,
        PseudoElementTarget::Before | PseudoElementTarget::After
    ) && super::blockify::children_are_flex_items(dom, Some(id), host_computed)
    {
        super::blockify::blockify(&mut working);
    }
    super::apply::finalize_justify_items(&mut working, host_computed);
    finalize_bfc_formation(&mut working);
    working.resolve_viewport_units(cx.sheets.viewport());
    finalize_used_border(&mut working);

    // Resolve content:
    //   - None  = no `content:` declaration at all → use legacy fallback
    //   - Some(None) = `content: none;` declared → suppress (NO fallback)
    //   - Some(Some(s)) = content resolved to string
    // (An `attr()` read the HOST element's attribute when `prepare`
    // substituted it: `optgroup::before { content: attr(label) }`.)
    // The pseudo-element's own `counter-reset` / `counter-increment`
    // (the `h2::before { counter-increment: sec }` idiom). It is a child
    // of the host, so its instances are scoped to the host's subtree.
    counters.enter(Some(id), &working.counter_reset, &working.counter_increment);
    let counter_lookup = |name: &str| counters.value(name);
    let declared = resolve_content_on(&working, plan, decls, &counter_lookup);
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
