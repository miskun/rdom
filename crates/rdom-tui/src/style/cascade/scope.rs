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

use std::collections::HashMap;
use std::rc::Rc;

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::style::{Rule, Stylesheet};
use rdom_style::ScopeId;

/// The proximity of an unscoped rule: farther than any scoped one.
pub(super) const UNSCOPED: u32 = u32::MAX;

/// A node's scoping roots for one scope, nearest first, with the
/// generations up to each: the roots that have the node in scope.
type Roots = Rc<[(NodeId, u32)]>;

/// What scoping learned in one cascade pass, so each answer is computed
/// once (`C1G-SCOPE-COST`): per (sheet, scope, node), whether the node
/// is a scoping root and which roots have it in scope. A node's roots
/// extend its parent's — the parent's roots it is not a limit for, one
/// generation farther, and itself when it is a root — so the work is
/// one root test per node and one limit test per (node, root above it).
#[derive(Default)]
pub(super) struct ScopeMemo {
    roots: HashMap<(usize, ScopeId, NodeId), Roots>,
    is_root: HashMap<(usize, ScopeId, NodeId), bool>,
}

/// One sheet of the pass, by index (the memo key) and reference.
#[derive(Clone, Copy)]
pub(super) struct SheetRef<'s> {
    pub index: usize,
    pub sheet: &'s Stylesheet,
}

/// Whether `rule` (of `sheet`) matches `id`, and with which scope
/// proximity: the generations to the nearest root that has `id` in
/// scope and with which, as `:scope`, the rule's selector matches.
pub(super) fn match_rule(
    dom: &Dom<TuiExt>,
    id: NodeId,
    sheet: SheetRef<'_>,
    rule: &Rule,
    memo: &mut ScopeMemo,
) -> Option<u32> {
    match rule.scope {
        None => dom.matches_list(id, &rule.selector).then_some(UNSCOPED),
        Some(scope) => roots_of(dom, id, sheet, scope, memo)
            .iter()
            .find(|(root, _)| counted(dom.matches_list_in_scope(id, &rule.selector, Some(*root))))
            .map(|(_, hops)| *hops),
    }
}

/// The roots of `scope` that have `node` in scope, nearest first: an
/// inclusive ancestor that is a root, with no scoping limit on the path
/// from `node` up to (not including) it.
fn roots_of(
    dom: &Dom<TuiExt>,
    node: NodeId,
    sheet: SheetRef<'_>,
    scope: ScopeId,
    memo: &mut ScopeMemo,
) -> Roots {
    let key = (sheet.index, scope, node);
    if let Some(roots) = memo.roots.get(&key) {
        return roots.clone();
    }
    let above = match dom.node(node).parent_node().map(|p| p.id()) {
        Some(parent) => roots_of(dom, parent, sheet, scope, memo),
        None => Rc::from([]),
    };
    let mut roots = Vec::with_capacity(above.len() + 1);
    if is_root(dom, node, sheet, scope, memo) {
        roots.push((node, 0));
    }
    let end = sheet.sheet.scopes()[scope.index()].end.as_ref();
    for &(root, hops) in above.iter() {
        // `node` is a scoping limit for `root`: out of its scope.
        let limit =
            end.is_some_and(|end| counted(dom.matches_list_in_scope(node, end, Some(root))));
        if !limit {
            roots.push((root, hops + 1));
        }
    }
    let roots: Roots = roots.into();
    memo.roots.insert(key, roots.clone());
    roots
}

/// Is `node` a scoping root of `scope`?
fn is_root(
    dom: &Dom<TuiExt>,
    node: NodeId,
    sheet: SheetRef<'_>,
    scope: ScopeId,
    memo: &mut ScopeMemo,
) -> bool {
    let key = (sheet.index, scope, node);
    if let Some(&known) = memo.is_root.get(&key) {
        return known;
    }
    let s = &sheet.sheet.scopes()[scope.index()];
    let root = match &s.start {
        // Prelude-less: the parent element of the sheet's owner node,
        // or the document root for a sheet with none (§2.5.1); a nested
        // one's root must be in a scope of the enclosing one.
        None => {
            node == implicit_root(dom, s.owner_in(sheet.sheet))
                && s.parent
                    .is_none_or(|outer| !roots_of(dom, node, sheet, outer, memo).is_empty())
        }
        Some(start) => {
            dom.node(node).node_type() == NodeType::Element
                && match s.parent {
                    None => counted(dom.matches_list(node, start)),
                    // Matched with each enclosing root as `:scope`.
                    Some(outer) => roots_of(dom, node, sheet, outer, memo)
                        .iter()
                        .any(|(r, _)| counted(dom.matches_list_in_scope(node, start, Some(*r)))),
                }
        }
    };
    memo.is_root.insert(key, root);
    root
}

fn implicit_root(dom: &Dom<TuiExt>, owner: Option<NodeId>) -> NodeId {
    owner
        .filter(|&owner| dom.contains(owner))
        .and_then(|owner| dom.node(owner).parent_node().map(|p| p.id()))
        .unwrap_or_else(|| dom.root())
}

/// Count one selector match made for scoping (test-only work counter).
fn counted(matched: bool) -> bool {
    #[cfg(test)]
    probe::MATCHES.with(|c| c.set(c.get() + 1));
    matched
}

/// Test-only: how many selector matches scoping made on this thread.
#[cfg(test)]
pub(super) mod probe {
    thread_local! {
        pub static MATCHES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    pub fn take() -> usize {
        MATCHES.with(|c| c.replace(0))
    }
}
