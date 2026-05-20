//! `<style>` block extraction + inline-style seeding — the
//! parse-and-apply glue between [`rdom_css`] (the parser) and
//! [`TuiDom`](crate::TuiDom) (the tree being styled).
//!
//! Two helpers:
//!
//! - [`extend_from_style_tags`] walks a populated `TuiDom`, finds
//!   every `<style>` element, concatenates its text-node children
//!   into a single CSS source string, and parses + merges the
//!   result into the target `Stylesheet`. Custom properties
//!   declared in `:root` blocks are picked up too.
//! - [`seed_inline_styles`] walks a populated `TuiDom`, finds every
//!   element with a `style="…"` attribute, parses the value, and
//!   writes the resulting `TuiStyle` into the element's
//!   `TuiExt::inline_style` slot.
//!
//! Both are pre-`App::build` setup hooks: the cascade reads what
//! they wrote on its first pass. M1 limitation: re-running after
//! later DOM mutation requires a fresh call.
//!
//! ## Layering
//!
//! `cssom/` is the canonical home for the rdom-css ↔ rdom-tui
//! glue. Step 26's `StyleDeclaration` lands as a sibling module
//! here. New code that needs both crates belongs in this module,
//! not scattered through the tree.

use rdom_core::{NodeId, NodeType};
use rdom_css::{Warning, parse, parse_inline};
use rdom_style::Stylesheet;

use crate::{TuiDom, TuiNodeMutExt};

/// Walk `dom` for `<style>` elements and merge their parsed CSS
/// into `sheet`. Returns the warnings collected from every
/// inner parse, concatenated in document order.
///
/// Behavior is fully additive: existing rules and vars on `sheet`
/// are preserved; the parsed rules are appended at the end (giving
/// them later source order, so they win cascade ties at equal
/// specificity).
pub fn extend_from_style_tags(dom: &TuiDom, sheet: &mut Stylesheet) -> Vec<Warning> {
    let mut warnings = Vec::new();
    let style_ids = collect_style_elements(dom);
    for id in style_ids {
        let css = collect_text_content(dom, id);
        let result = parse(&css);
        // Merge rules — re-parse each selector via add_rule. The
        // alternative would be a Stylesheet::append_rules method
        // that copies pre-parsed Rule values; deferred until the
        // perf becomes a concern. add_rule's internal split-on-
        // commas + selector::parse path matches what the inner
        // parser already produced, so this round-trips cleanly.
        for rule in result.stylesheet.rules() {
            let _ = sheet.add_rule(&rule.source_text, rule.style.clone());
        }
        // Merge vars. Stylesheet::define_var is fluent (consumes
        // self) so we mem-swap.
        for (k, v) in result.stylesheet.vars() {
            let owned = std::mem::take(sheet);
            *sheet = owned.define_var(k, v);
        }
        warnings.extend(result.warnings);
    }
    warnings
}

fn collect_style_elements(dom: &TuiDom) -> Vec<NodeId> {
    let mut out = Vec::new();
    walk(dom, dom.root(), &mut out);
    out
}

fn walk(dom: &TuiDom, id: NodeId, out: &mut Vec<NodeId>) {
    let node = dom.node(id);
    if node.tag_name() == Some("style") {
        out.push(id);
        // No need to recurse into a `<style>` — its children are
        // text content, not nested elements.
        return;
    }
    for child in node.child_nodes() {
        walk(dom, child.id(), out);
    }
}

fn collect_text_content(dom: &TuiDom, id: NodeId) -> String {
    let mut out = String::new();
    for child in dom.node(id).child_nodes() {
        if child.node_type() == NodeType::Text
            && let Some(s) = child.node_value()
        {
            out.push_str(s);
        }
    }
    out
}

/// Walk `dom` for every element with a `style="…"` attribute, parse
/// the value via [`parse_inline`], and write the resulting
/// `TuiStyle` into the element's `TuiExt::inline_style` slot. The
/// cascade then reads it through its existing inline rung — beating
/// every author rule short of `!important` per the CSS spec.
///
/// Returns the warnings collected from every inline parse,
/// concatenated in document order.
///
/// M1 limitation: this is a one-shot pass run before `App::new`.
/// Mutating the `style` attribute later does not re-trigger the
/// parse — apps that need that should call `seed_inline_styles`
/// again or use the typed `set_inline_style` API.
pub fn seed_inline_styles(dom: &mut TuiDom) -> Vec<Warning> {
    let mut warnings = Vec::new();
    let candidates = collect_styled_elements(dom);
    for id in candidates {
        let Some(text) = dom.node(id).get_attribute("style").map(|s| s.to_string()) else {
            continue;
        };
        let result = parse_inline(&text);
        warnings.extend(result.warnings);
        dom.node_mut(id).set_inline_style(result.style);
    }
    warnings
}

fn collect_styled_elements(dom: &TuiDom) -> Vec<NodeId> {
    let mut out = Vec::new();
    walk_styled(dom, dom.root(), &mut out);
    out
}

fn walk_styled(dom: &TuiDom, id: NodeId, out: &mut Vec<NodeId>) {
    if dom.node(id).has_attribute("style") {
        out.push(id);
    }
    for child in dom.node(id).child_nodes() {
        walk_styled(dom, child.id(), out);
    }
}
