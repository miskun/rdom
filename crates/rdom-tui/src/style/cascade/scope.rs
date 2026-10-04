//! `@scope` in the cascade (CSS Cascade 6 §2.5, §6.1): whether a scoped
//! rule matches an element, and its *scope proximity* — the number of
//! generations between the element and the nearest scoping root it
//! matches through.
//!
//! An element is in a scope when it is an inclusive descendant of the
//! scoping root and not an inclusive descendant of a scoping limit
//! below the root. A rule's selector is matched with the root as
//! `:scope`; a nested `@scope`'s roots must themselves be in a scope of
//! the enclosing `@scope`. The cascade sorts proximity after
//! specificity and before order of appearance, nearer winning; an
//! unscoped rule is infinitely far ([`UNSCOPED`]).

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::style::{Rule, Stylesheet};
use rdom_style::ScopeId;

/// The proximity of an unscoped rule: farther than any scoped one.
pub(super) const UNSCOPED: u32 = u32::MAX;

/// Whether `rule` (of `sheet`) matches `id`, and with which scope
/// proximity.
pub(super) fn match_rule(
    dom: &Dom<TuiExt>,
    id: NodeId,
    sheet: &Stylesheet,
    rule: &Rule,
) -> Option<u32> {
    match rule.scope {
        None => dom.matches_list(id, &rule.selector).then_some(UNSCOPED),
        Some(scope) => nearest_root(dom, id, sheet, scope, &|root| {
            dom.matches_list_in_scope(id, &rule.selector, Some(root))
        }),
    }
}

/// Generations from `id` up to the nearest root of `scope` that has
/// `id` in scope and satisfies `matches`.
fn nearest_root(
    dom: &Dom<TuiExt>,
    id: NodeId,
    sheet: &Stylesheet,
    scope: ScopeId,
    matches: &dyn Fn(NodeId) -> bool,
) -> Option<u32> {
    let mut hops = 0;
    let mut cur = Some(id);
    while let Some(root) = cur {
        if is_root(dom, root, sheet, scope)
            && in_scope(dom, id, root, sheet, scope)
            && matches(root)
        {
            return Some(hops);
        }
        cur = dom.node(root).parent_node().map(|p| p.id());
        hops += 1;
    }
    None
}

/// Is `node` a scoping root of `scope`?
fn is_root(dom: &Dom<TuiExt>, node: NodeId, sheet: &Stylesheet, scope: ScopeId) -> bool {
    let s = &sheet.scopes()[scope.index()];
    // A nested `@scope`'s root must be in a scope of the enclosing one.
    let in_outer = |matches: &dyn Fn(NodeId) -> bool| match s.parent {
        None => true,
        Some(outer) => nearest_root(dom, node, sheet, outer, matches).is_some(),
    };
    match &s.start {
        // Prelude-less: the parent element of the sheet's owner node,
        // or the document root for a sheet with none (§2.5.1).
        None => node == implicit_root(dom, sheet) && in_outer(&|_| true),
        Some(start) => {
            dom.node(node).node_type() == NodeType::Element
                && match s.parent {
                    None => dom.matches_list(node, start),
                    Some(_) => {
                        in_outer(&|outer| dom.matches_list_in_scope(node, start, Some(outer)))
                    }
                }
        }
    }
}

fn implicit_root(dom: &Dom<TuiExt>, sheet: &Stylesheet) -> NodeId {
    sheet
        .owner_node()
        .filter(|&owner| dom.contains(owner))
        .and_then(|owner| dom.node(owner).parent_node().map(|p| p.id()))
        .unwrap_or_else(|| dom.root())
}

/// Is `id` in scope of `root` — no scoping limit on the path from `id`
/// up to (not including) `root`?
fn in_scope(
    dom: &Dom<TuiExt>,
    id: NodeId,
    root: NodeId,
    sheet: &Stylesheet,
    scope: ScopeId,
) -> bool {
    let Some(end) = &sheet.scopes()[scope.index()].end else {
        return true;
    };
    let mut cur = id;
    while cur != root {
        if dom.matches_list_in_scope(cur, end, Some(root)) {
            return false;
        }
        match dom.node(cur).parent_node() {
            Some(p) => cur = p.id(),
            None => return false,
        }
    }
    true
}
