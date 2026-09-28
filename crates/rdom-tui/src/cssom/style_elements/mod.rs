//! Live `<style>` sheets (`P7-LIVE-STYLE-1`).
//!
//! HTML §4.2.6 "update a style block": a `<style>` element's sheet is
//! (re)created when the element is inserted into or removed from the
//! document, and when its child text changes. CSSOM §6.1 orders the
//! document's sheets in tree order.
//!
//! [`StyleElements`] keeps one parsed sheet per connected `<style>`
//! element, in tree order. A mutation observer marks the set dirty when
//! a connected `<style>` element's text or children change, or a
//! `<style>` element (or a subtree holding one) is inserted into or
//! removed from the document — observers may not mutate or re-enter the
//! tree, and the cascade reads the sheets only at a frame, so the
//! [`App`](crate::runtime::App) flushes it at the frame boundary (before
//! each frame's cascade, `draw_if_dirty` and the off-frame
//! `cascade_and_layout`), and invalidates the cascade when the sheets
//! changed.
//!
//! The observer's check is cheap (`P7G-STYLE-HOLDS-1`): a mutation of a
//! detached subtree is skipped (a `<style>` that is not connected has no
//! sheet — building a subtree bottom-up before inserting it costs
//! nothing here), and an inserted or removed subtree is searched once,
//! iteratively, stopping at the first `<style>`. A flush re-parses
//! only the elements whose text changed; each sheet's parse warnings
//! are kept with it ([`App::style_element_warnings`](crate::runtime::App::style_element_warnings)).
//!
//! **Cascade position:** the `<style>` sheets come first, in tree order,
//! then every sheet registered with the App (`App::new`'s,
//! `push_stylesheet`'s), in push order — the App's sheets play the part
//! of a document's `adoptedStyleSheets`, which CSSOM orders after the
//! document's own sheets. Origin and specificity still decide first;
//! the order breaks ties between author rules.
//!
//! The `media` attribute is not evaluated (rdom evaluates no media
//! queries), and `<style>` has no `disabled` content attribute.

use std::cell::Cell;
use std::rc::Rc;

use rdom_core::{Dom, Mutation, MutationObserver, NodeId, NodeType};
use rdom_css::Warning;
use rdom_style::Stylesheet;

use crate::{TuiDom, TuiExt};

/// One connected `<style>` element's parsed sheet.
#[derive(Debug)]
struct StyleSheetEntry {
    element: NodeId,
    /// The concatenated child text the sheet was parsed from.
    source: String,
    sheet: Stylesheet,
    warnings: Vec<Warning>,
}

/// The document's `<style>` sheets, kept current by a mutation
/// observer and flushed by the App.
#[derive(Debug)]
pub(crate) struct StyleElements {
    dirty: Rc<Cell<bool>>,
    entries: Vec<StyleSheetEntry>,
}

impl StyleElements {
    /// Install the observer on `dom`. The first [`flush`](Self::flush)
    /// parses the `<style>` elements already in the tree.
    pub(crate) fn install(dom: &mut TuiDom) -> Self {
        let dirty = Rc::new(Cell::new(true));
        dom.add_mutation_observer(Box::new(StyleObserver {
            dirty: dirty.clone(),
        }));
        Self {
            dirty,
            entries: Vec::new(),
        }
    }

    /// Bring the sheets up to date with the tree. Returns `true` when
    /// they changed and the cascade must be invalidated.
    pub(crate) fn flush(&mut self, dom: &TuiDom) -> bool {
        if !self.dirty.replace(false) {
            return false;
        }
        let mut previous = std::mem::take(&mut self.entries);
        let before: Vec<NodeId> = previous.iter().map(|e| e.element).collect();
        let mut reparsed = false;
        let styles = dom.elements_by_tag("style");
        for &element in styles.ids() {
            let source = child_text(dom, element);
            let reused = previous
                .iter()
                .position(|e| e.element == element && e.source == source)
                .map(|at| previous.swap_remove(at));
            let entry = reused.unwrap_or_else(|| {
                reparsed = true;
                let parsed = rdom_css::parse(&source);
                StyleSheetEntry {
                    element,
                    source,
                    sheet: parsed.stylesheet,
                    warnings: parsed.warnings,
                }
            });
            self.entries.push(entry);
        }
        // A new or re-parsed sheet, a removed one, or a move changes
        // the cascade.
        reparsed || !self.entries.iter().map(|e| e.element).eq(before)
    }

    /// The sheets, in tree order.
    pub(crate) fn sheets(&self) -> impl Iterator<Item = &Stylesheet> {
        self.entries.iter().map(|e| &e.sheet)
    }

    /// Every sheet's parse warnings, in tree order.
    pub(crate) fn warnings(&self) -> impl Iterator<Item = &Warning> {
        self.entries.iter().flat_map(|e| &e.warnings)
    }
}

/// HTML "child text content": the concatenated data of the element's
/// Text children, in tree order.
fn child_text(dom: &TuiDom, element: NodeId) -> String {
    let mut out = String::new();
    for child in dom.node(element).child_nodes() {
        if child.node_type() == NodeType::Text
            && let Some(s) = child.node_value()
        {
            out.push_str(s);
        }
    }
    out
}

/// Marks [`StyleElements`] dirty on the mutations that can change a
/// `<style>` sheet.
struct StyleObserver {
    dirty: Rc<Cell<bool>>,
}

impl MutationObserver<TuiExt> for StyleObserver {
    fn observe(&mut self, dom: &mut Dom<TuiExt>, record: &Mutation) {
        if self.dirty.get() {
            return;
        }
        let relevant = match record {
            Mutation::CharacterDataChanged { id, .. } => dom
                .node(*id)
                .parent_node()
                .is_some_and(|p| p.tag_name() == Some("style") && p.is_connected()),
            Mutation::ChildListChanged {
                parent,
                added,
                removed,
            } => {
                // Only the document's `<style>` elements have sheets.
                dom.node(*parent).is_connected()
                    && (dom.node(*parent).tag_name() == Some("style")
                        || added
                            .iter()
                            .chain(removed)
                            // A dropped node's subtree is gone: it may have
                            // held one.
                            .any(|&id| !dom.contains(id) || holds_style(dom, id)))
            }
            _ => false,
        };
        if relevant {
            self.dirty.set(true);
        }
    }
}

/// `id` is a `<style>` element or has one among its descendants. A
/// pre-order walk over the parent / sibling links — no recursion, no
/// allocation — that stops at the first `<style>`.
fn holds_style(dom: &TuiDom, id: NodeId) -> bool {
    let mut cur = id;
    loop {
        let node = dom.node(cur);
        if node.tag_name() == Some("style") {
            return true;
        }
        if let Some(child) = node.first_child() {
            cur = child.id();
            continue;
        }
        // No children: the next sibling of the nearest node on the way
        // back up to `id` that has one.
        loop {
            if cur == id {
                return false;
            }
            let node = dom.node(cur);
            if let Some(next) = node.next_sibling() {
                cur = next.id();
                break;
            }
            match node.parent_node() {
                Some(parent) => cur = parent.id(),
                None => return false,
            }
        }
    }
}

#[cfg(test)]
mod tests;
