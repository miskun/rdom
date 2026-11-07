//! What the cascade applied to one element, in the order it applied it
//! (CAPTURE phase 3, `crate::capture::StyleTrace`): the same matched
//! rules, ladder and declaration blocks — `var()` substituted — as
//! `compute_element_style` walks, read through the one iteration both
//! use (`Declarations::sources_of`), so the explanation cannot drift from
//! the cascade it explains.
//!
//! It reads the document as the last cascade left it — the parent's
//! computed style, the element's own direction — and changes nothing.

use std::rc::Rc;

use rdom_core::{Dom, NodeId};

use super::ladder::{Block, Declarations, prepare};
use super::matching::Rules;
use super::registered::PropertyRegistry;
use super::walk::{Scratch, Sheets, merge_root_vars};
use crate::ext::TuiExt;
use crate::node::TuiNodeExt;
use crate::style::{ComputedStyle, PseudoElementTarget, Rule, Stylesheet, TuiStyle};

/// Where an applied block came from.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Origin<'a> {
    /// A rule: its sheet's index in the sheet set, its index in the
    /// sheet, and the rule.
    Rule {
        sheet: usize,
        index: usize,
        rule: &'a Rule,
    },
    /// The element's presentational hints.
    Hint,
    /// The element's `style` attribute.
    Inline,
}

/// One declaration block the cascade applied, at one importance.
#[derive(Debug, Clone)]
pub(crate) struct Applied<'a> {
    pub origin: Origin<'a>,
    /// The ladder step applied the block's `!important` declarations.
    pub important: bool,
    /// The block as applied — a rule's own, or its `var()`-substituted /
    /// direction-mapped declarations.
    pub style: TuiStyle,
}

/// The blocks the cascade applies to element `id` under `sheets` (in
/// cascade order) and `registry`, in application order; `None` for a node
/// the cascade did not style.
pub(crate) fn explain_element<'a>(
    dom: &Dom<TuiExt>,
    stylesheets: &'a [&'a Stylesheet],
    registry: Rc<PropertyRegistry>,
    id: NodeId,
) -> Option<Vec<Applied<'a>>> {
    let own = dom.node(id).computed()?;
    let direction = own.text_direction;
    let sheets = Sheets::new(stylesheets, registry, super::media::document_media(dom));
    let mut scratch = Scratch::default();
    scratch.gather(dom, &sheets, id, &[PseudoElementTarget::None], Rules::Match);
    let hints = super::hints::presentational_hints(dom, id);
    if hints.is_some() {
        scratch.plan.add_hints();
    }
    let inline = dom.node(id).ext().and_then(|e| e.inline_style.as_deref());
    let decls =
        Declarations::new(&scratch.sorted, &scratch.ranks, inline).with_hints(hints.as_ref());
    // The `var()` substitution reads the element's custom properties as
    // its parent's and its own declarations make them.
    let mut working = ComputedStyle::initial();
    let parent = dom
        .node(id)
        .parent_node()
        .and_then(|p| p.computed().cloned());
    match parent {
        Some(parent) => super::inherit::inherit_inheritable_from(&mut working, &parent),
        None => working.vars = merge_root_vars(dom, &sheets),
    }
    working.text_direction = direction;
    let transitions = dom
        .node(id)
        .ext()
        .and_then(|e| e.presentation.as_deref())
        .and_then(|p| p.custom_properties.as_ref());
    let attrs = |name: &str| dom.node(id).get_attribute(name);
    let substituted = prepare(
        &mut working,
        &scratch.plan,
        decls,
        sheets.active_registry(),
        transitions,
        &attrs,
        sheets.viewport_use(),
    );
    let decls = decls.with(substituted.as_ref(), direction);
    let mut out = Vec::new();
    for step in scratch.plan.steps() {
        for (block, style) in decls.sources_of(step) {
            let origin = match block {
                Block::Rule(i) => rule_origin(stylesheets, scratch.sorted[i]),
                Block::Hint => Origin::Hint,
                Block::Inline => Origin::Inline,
                // Only a keyframe style applies keyframe blocks; an
                // element's own style has none.
                Block::Animation => continue,
            };
            out.push(Applied {
                origin,
                important: step.important,
                style: style.clone(),
            });
        }
    }
    Some(out)
}

/// `rule`'s place in `sheets`: found by address, as the matched rules
/// borrow them from their sheets' rule slices.
fn rule_origin<'a>(sheets: &'a [&'a Stylesheet], rule: &Rule) -> Origin<'a> {
    let at = rule as *const Rule as usize;
    for (s, sheet) in sheets.iter().enumerate() {
        let rules = sheet.rules();
        let start = rules.as_ptr() as usize;
        let end = start + std::mem::size_of_val(rules);
        if (start..end).contains(&at) {
            let index = (at - start) / std::mem::size_of::<Rule>();
            return Origin::Rule {
                sheet: s,
                index,
                rule: &rules[index],
            };
        }
    }
    unreachable!("a matched rule is a rule of its sheet set")
}
