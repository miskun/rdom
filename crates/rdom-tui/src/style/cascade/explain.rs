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

use super::element::{gather_element, start_style};
use super::ladder::{Block, Declarations};
use super::matching::Rules;
use super::registered::PropertyRegistry;
use super::walk::{Scratch, Sheets, merge_root_vars};
use crate::ext::TuiExt;
use crate::node::TuiNodeExt;
use crate::style::{ComputedStyle, Rule, Stylesheet, TuiStyle};

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
    let sheets = Sheets::new(stylesheets, registry, super::media::document_media(dom));
    let mut scratch = Scratch::default();
    let hints = gather_element(dom, &sheets, id, &mut scratch, Rules::Match);
    let inline = dom.node(id).ext().and_then(|e| e.inline_style.as_deref());
    let decls = Declarations::new(&scratch.sorted, &scratch.ranks, inline)
        .with_hints(hints.as_ref())
        .with_animation(sheets.animation());
    // The style it inherits from, as the cascade chose it: its parent's,
    // or a `<details>` element's `::details-content` slot's; the root's
    // parent carries the sheets' root variables.
    let parent = style_parent(dom, id).unwrap_or_else(|| {
        let mut p = ComputedStyle::initial();
        p.vars = merge_root_vars(dom, &sheets);
        Rc::new(p)
    });
    // The run the cascade settled on: the element's own direction when a
    // flow-relative declaration reads it, else the inherited one
    // (`compute_element_style`'s `settle_direction`).
    let direction = if decls.has_directional() {
        own.text_direction
    } else {
        parent.text_direction
    };
    let (_, substituted) = start_style(dom, &sheets, id, &parent, direction, &scratch.plan, decls);
    let decls = decls.with(substituted.as_ref(), own.text_direction);
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

/// The style element `id` inherits from as the last cascade had it: its
/// parent's computed style, or — for a `<details>` element's slotted
/// content — its `::details-content` slot's (`details::inherited_style`,
/// the walk's own rule). `None` at the root.
pub(crate) fn style_parent(dom: &Dom<TuiExt>, id: NodeId) -> Option<Rc<ComputedStyle>> {
    let parent = dom.node(id).parent_node()?.id();
    super::details::inherited_style(dom, parent, id)
        .or_else(|| dom.node(parent).ext().and_then(|e| e.computed.clone()))
}
