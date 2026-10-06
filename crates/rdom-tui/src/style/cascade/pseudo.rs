//! Pseudo-element styles (`::before`, `::after`, `::backdrop`,
//! `::selection`, `::placeholder`, the scrollbar parts): the same
//! ladder as an element, inheriting from the host's computed style.

use rdom_core::{Dom, NodeId};

use super::apply::finalize_bfc_formation;
use super::content::{declared_content, resolve_onto};
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
    // own `direction`, as for an element (`element::compute_element_style`).
    let directional = decls.has_directional();
    let mut direction = host_computed.text_direction;
    let mut runs = 0;
    let (mut working, substituted, colors) = loop {
        // Pseudo-elements inherit from the host's computed style (per
        // spec), not from the host's parent, and share the host's vars
        // (which came from the merged stylesheet roots).
        let mut working = ComputedStyle::initial();
        // `display` is `inline` until declared (CSS Display 3 §2: its
        // initial value); rdom's elements start from `block` and take
        // their `display` from the UA sheet, which styles no
        // pseudo-element's (C8G-PSEUDO-BOXES).
        working.display = crate::layout::Display::Inline;
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
    ) && super::blockify::children_are_items(dom, Some(id), host_computed)
    {
        super::blockify::blockify(&mut working);
    }
    super::font::finalize_font(&mut working, host_computed);
    super::quotes::finalize_quotes(&mut working, host_computed, dom, Some(id));
    super::text_decoration::finalize_applied_decorations(
        &mut working,
        host_computed.applied_decorations,
    );
    super::apply::finalize_justify_items(&mut working, host_computed);
    super::text::finalize_text_align(&mut working, host_computed, false);
    // CSS Overflow 3 §3.1's computed value, which the BFC rule reads.
    working.normalize_overflow();
    super::line_clamp::finalize_line_clamp(&mut working);
    finalize_bfc_formation(&mut working);
    let root_rows = Some(super::text::root_line_height(dom));
    let units = super::text::finalize_line_height(
        &mut working,
        host_computed,
        root_rows,
        cx.sheets.viewport(),
    );
    working.resolve_context_units(&units);
    finalize_used_border(&mut working);

    // Resolve content:
    //   - no `content:` declaration at all → the legacy fallback text
    //   - `content: none;` declared → suppress (NO fallback)
    //   - a `<content-list>` → its text, alt text and quote items
    // (An `attr()` read the HOST element's attribute when `prepare`
    // substituted it: `optgroup::before { content: attr(label) }`.)
    // The pseudo-element's own counter ops (the `h2::before {
    // counter-increment: sec }` idiom) — `::before` and `::after` only,
    // the pseudo-elements that are boxes of the tree (CSS Lists 3 §4).
    // It is a child of the host, so its instances are scoped to the
    // host's subtree.
    let slot = match target {
        PseudoElementTarget::Before => Some(super::counters::OpBox::Before),
        PseudoElementTarget::After => Some(super::counters::OpBox::After),
        _ => None,
    };
    if let Some(slot) = slot {
        let owner = super::counters::Owner { element: id, slot };
        super::counters::reversed::resolve(dom, owner, &mut working);
        counters.enter(Some(id), owner, &working);
    }
    match declared_content(plan, decls) {
        Some(declared) => {
            // `quotes: auto` takes the host's content language (§2.1).
            let lang = declared
                .uses_quotes()
                .then(|| super::quotes::content_language(dom, id))
                .flatten();
            let styles = cx.sheets.counter_styles();
            resolve_onto(&mut working, &declared, counters, true, lang, styles)
        }
        None => working.content = fallback,
    }

    // Skip entirely if the pseudo-element has nothing to contribute.
    if sorted.is_empty() && working.content.is_none() {
        return None;
    }
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
