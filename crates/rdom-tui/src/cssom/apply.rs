//! `<style>` block extraction + inline-style seeding — the
//! parse-and-apply glue between [`rdom_css`] (the parser) and
//! [`TuiDom`] (the tree being styled).
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
//! Both are one-shot snapshots. An [`App`](crate::runtime::App) needs
//! neither: it keeps `<style>` sheets live itself
//! (`cssom::style_elements`), seeds the `style="…"` attributes present
//! at mount, and the CSSOM observer re-parses `style="…"` on every
//! later write (`P7G-INLINE-STYLE-SEED-1`). Calling
//! `seed_inline_styles` before `App::new` is optional — it is
//! idempotent, and it is how to get the mount-time parse warnings.
//!
//! ## Layering
//!
//! `cssom/` is the canonical home for the rdom-css ↔ rdom-tui
//! glue. Step 26's `StyleDeclaration` lands as a sibling module
//! here. New code that needs both crates belongs in this module,
//! not scattered through the tree.

use rdom_core::{NodeId, NodeType};
use rdom_css::{Warning, parse};
use rdom_style::Stylesheet;

use crate::TuiDom;

/// Walk `dom` for `<style>` elements and merge their parsed CSS
/// into `sheet`. Returns the warnings collected from every
/// inner parse, concatenated in document order.
///
/// Behavior is fully additive: existing rules and vars on `sheet`
/// are preserved; the parsed rules are appended at the end (giving
/// them later source order, so they win cascade ties at equal
/// specificity).
///
/// A snapshot, for a cascade run without an `App`. An
/// [`App`](crate::runtime::App) applies the document's `<style>`
/// elements itself and keeps them live; merging them into a sheet
/// handed to an `App` as well applies their rules twice, and the
/// merged copy goes stale when the element's text changes.
///
/// An `@import` warns and imports nothing here; use
/// [`extend_from_style_tags_with_loader`].
pub fn extend_from_style_tags(dom: &TuiDom, sheet: &mut Stylesheet) -> Vec<Warning> {
    extend(dom, sheet, None)
}

/// [`extend_from_style_tags`], resolving each sheet's `@import`s through
/// `loader` (CSS Cascade 5 §3; a `<style>` sheet has no URL of its own,
/// so its imports get no base — `rdom_css::ImportLoader`), as an `App`
/// does with `App::set_import_loader`.
pub fn extend_from_style_tags_with_loader(
    dom: &TuiDom,
    sheet: &mut Stylesheet,
    loader: &dyn rdom_css::ImportLoader,
) -> Vec<Warning> {
    extend(dom, sheet, Some(loader))
}

fn extend(
    dom: &TuiDom,
    sheet: &mut Stylesheet,
    loader: Option<&dyn rdom_css::ImportLoader>,
) -> Vec<Warning> {
    let mut warnings = Vec::new();
    let style_ids = collect_style_elements(dom);
    for id in style_ids {
        let css = collect_text_content(dom, id);
        let mut result = match loader {
            Some(loader) => rdom_css::parse_with_loader(&css, loader),
            None => parse(&css),
        };
        // The `<style>` element owns its sheet (CSSOM `ownerNode`): a
        // prelude-less `@scope` in it roots at the element's parent
        // (CSS Cascade 6 §2.5.1), and `append` keeps that on the scope.
        result.stylesheet.set_owner_node(Some(id));
        // Merge rules, cascade layers and vars: `append` keeps each
        // rule's layer (merging layer names with `sheet`'s, as the
        // document's sheets share one layer order).
        sheet.append(&result.stylesheet);
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
/// the value via [`rdom_css::parse_inline`], and write the resulting
/// `TuiStyle` into the element's `TuiExt::inline_style` slot. The
/// cascade then reads it through its existing inline rung — beating
/// every author rule short of `!important` per the CSS spec.
///
/// Returns the warnings collected from every inline parse,
/// concatenated in document order.
///
/// Idempotent: an element whose slot disagrees with its attribute has
/// the slot *replaced* by the attribute's parse; one whose attribute
/// already reads as the slot's serialization — a direct style setter or
/// CSSOM write since, which reflect into the attribute — keeps its slot
/// exactly (`cssom::inline`, `P7G-SEED-PRESERVE-1`). So a second call
/// changes nothing, and `set_width` before `App::build` survives it
/// alongside the markup's own `style` declarations. `App::build`
/// runs it (discarding the warnings — call it yourself first to see
/// them), and the CSSOM observer keeps the slots in step with later
/// `style` writes. Call it directly only for a `TuiDom` cascaded
/// without an `App`, whose `style` attributes were set before
/// `cssom::install_default_observers`.
pub fn seed_inline_styles(dom: &mut TuiDom) -> Vec<Warning> {
    let mut warnings = Vec::new();
    let candidates = collect_styled_elements(dom);
    for id in candidates {
        warnings.extend(crate::cssom::inline::sync_from_attribute(dom, id));
    }
    warnings
}

/// Every node carrying a `style` attribute, in tree order: a pre-order
/// walk over the parent / sibling links — no recursion, so a very deep
/// tree cannot overflow the stack at mount.
fn collect_styled_elements(dom: &TuiDom) -> Vec<NodeId> {
    let mut out = Vec::new();
    let root = dom.root();
    let mut cur = Some(root);
    while let Some(id) = cur {
        let node = dom.node(id);
        if node.has_attribute("style") {
            out.push(id);
        }
        cur = node.first_child().map(|c| c.id()).or_else(|| {
            let mut up = Some(node);
            while let Some(n) = up {
                if n.id() == root {
                    return None;
                }
                if let Some(next) = n.next_sibling() {
                    return Some(next.id());
                }
                up = n.parent_node();
            }
            None
        });
    }
    out
}
