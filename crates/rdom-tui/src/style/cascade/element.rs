//! One element's style: the cascade ladder run over its matched rules,
//! from its initial and inherited values, then the computed-value
//! fix-ups (`content`, blockification, the BFC predicate, viewport
//! units, the used border). The tree walk that calls it is `walk`.

use rdom_core::NodeId;

use crate::layout::TextDirection;
use crate::style::{ComputedStyle, PseudoElementTarget};

use super::apply::finalize_bfc_formation;
use super::content::{declared_content, resolve_onto};
use super::decoration::finalize_used_border;
use super::inherit::inherit_inheritable_from;
use super::ladder::{Declarations, apply_cascade_ladder, prepare};
use super::matching::{Rules, Scratch};
use super::walk::ElementCx;

/// Per-element cascade: start from initial + inheritance, collect
/// matching rules, apply the ladder, resolve `content`, finalize
/// the `border-*-color`s.
pub(super) fn compute_element_style(
    cx: &mut ElementCx<'_, '_>,
    parent: &ComputedStyle,
    parent_id: Option<NodeId>,
    rules: Rules<'_>,
) -> ComputedStyle {
    let (dom, sheets, id) = (cx.dom, cx.sheets, cx.id);
    // Collect matching non-pseudo-element rules across all sheets.
    // Cascade order is (specificity, scope proximity, sheet_idx,
    // source_idx) — later sheets win same-specificity contests just
    // like later rules in a single sheet do.
    cx.scratch
        .gather(dom, sheets, id, &[PseudoElementTarget::None], rules);
    // HTML's presentational hints (`<ol start>`, `<li value>`): author
    // origin, before every author rule (CSS Cascade 4 §6.4.4).
    let hints = super::hints::presentational_hints(dom, id);
    if hints.is_some() {
        cx.scratch.plan.add_hints();
    }
    let Scratch {
        sorted,
        ranks,
        plan,
        ..
    } = &*cx.scratch;
    let counters = &mut *cx.counters;

    // Inline style on this element (may be empty).
    let inline = dom.node(id).ext().and_then(|e| e.inline_style.as_deref());

    let decls = Declarations::new(sorted, ranks, inline).with_hints(hints.as_ref());
    // Running transitions of registered custom properties
    // (`runtime::animation`).
    let transitions = dom
        .node(id)
        .ext()
        .and_then(|e| e.presentation.as_deref())
        .and_then(|p| p.custom_properties.as_ref());
    // `attr()` reads this element's attributes (CSS Values 5 §8.7).
    let attrs = |name: &str| dom.node(id).get_attribute(name);
    let preferred = sheets.color_scheme();
    // An inline-axis flow-relative property maps by the element's own
    // `direction` (CSS Logical 1 §4), which this very ladder decides: the
    // first run assumes the inherited one, and a block holding such a
    // property re-runs with the element's own when they differ.
    let directional = decls.has_directional();
    let ((mut working, substituted, colors), settled) =
        settle_direction(parent.text_direction, |direction| {
            // Start from initial + inherit subset from parent. That
            // includes the custom-property map (an `Rc` clone;
            // `apply_cascade_ladder` copies on write only when this
            // element declares `--*`).
            let mut working = ComputedStyle::initial();
            inherit_inheritable_from(&mut working, parent);
            working.text_direction = direction;
            let substituted = prepare(
                &mut working,
                plan,
                decls,
                sheets.registry(),
                transitions,
                &attrs,
                sheets.viewport(),
            );
            let colors = apply_cascade_ladder(
                &mut working,
                plan,
                decls.with(substituted.as_ref(), direction),
                parent,
                preferred,
            );
            // Only a flow-relative property reads the direction it ran
            // with; without one the first run stands.
            let own = if directional {
                working.text_direction
            } else {
                direction
            };
            ((working, substituted, colors), own)
        });
    // A flow-relative property cannot change `direction`, so the run with
    // the element's own direction settles it.
    debug_assert!(settled, "the direction re-run settles `direction`");
    let decls = decls.with(substituted.as_ref(), working.text_direction);
    // `currentcolor` takes the element's final `color`, `light-dark()`
    // its final `color-scheme`.
    colors.finalize(&mut working, parent.fg, preferred);

    // This element's counter ops take effect before its own generated
    // content and its children are seen — a reversed counter's computed
    // initial value first (CSS Lists 3 §4.2).
    let owner = super::counters::Owner::element(id);
    super::counters::reversed::resolve(dom, owner, &mut working);
    counters.enter(parent_id, owner, &working);

    // The element's own `content` (CSS Generated Content 3 §2): computed
    // and kept, but it generates nothing — no engine replaces an
    // element's children with a `<content-list>` (DIVERGENCES §2) — so
    // its `<quote>` items move no quote depth.
    if let Some(declared) = declared_content(plan, decls) {
        resolve_onto(
            &mut working,
            &declared,
            counters,
            false,
            None,
            sheets.counter_styles(),
        );
    }

    // BFC formation predicate (CSS 2.1 §9.4.1). Computed AFTER the
    // cascade ladder so it reads the final values of `flow`,
    // `display`, `overflow_*`, `position`. Used by the block-layout
    // margin-collapse pass — landing here in phase 1 so phase 5 has
    // it ready to consume.
    super::apply::finalize_unusual_contents(&mut working, dom.node(id).tag_name());
    if super::blockify::children_are_items(dom, parent_id, parent) {
        super::blockify::blockify(&mut working);
    }
    super::blockify::finalize_float(&mut working);
    super::font::finalize_font(&mut working, parent);
    super::quotes::finalize_quotes(&mut working, parent, dom, parent_id);
    super::text_decoration::finalize_applied_decorations(&mut working, parent.applied_decorations);
    super::apply::finalize_justify_items(&mut working, parent);
    let root = parent_id.is_none_or(|p| dom.node(p).node_type() != rdom_core::NodeType::Element);
    super::text::finalize_text_align(&mut working, parent, root);
    // CSS Overflow 3 §3.1's computed value, which the BFC rule reads.
    working.normalize_overflow();
    super::line_clamp::finalize_line_clamp(&mut working);
    finalize_bfc_formation(&mut working);
    // Viewport-percentage and line-height lengths are absolute at
    // computed-value time (CSS Values 4 §6.1), `line-height` first: `lh`
    // reads it.
    let root_rows = (!root).then(|| super::text::root_line_height(dom));
    let units =
        super::text::finalize_line_height(&mut working, parent, root_rows, sheets.viewport());
    working.resolve_context_units(&units);
    finalize_used_border(&mut working);

    working
}

/// Run an element's cascade ladder (`run`: the direction it assumes →
/// its result and the element's own `direction`) first with the
/// `inherited` direction and, when the element's own differs, once more
/// with that (CSS Logical 1 §4: flow-relative inline properties map by
/// the element's own `direction`, which the same ladder decides). At
/// most two runs, bounded here rather than by the ladder's behaviour;
/// returns the last run's result and whether its direction held.
pub(super) fn settle_direction<T>(
    inherited: TextDirection,
    mut run: impl FnMut(TextDirection) -> (T, TextDirection),
) -> (T, bool) {
    let (first, own) = run(inherited);
    if own == inherited {
        return (first, true);
    }
    let (second, settled) = run(own);
    (second, settled == own)
}
