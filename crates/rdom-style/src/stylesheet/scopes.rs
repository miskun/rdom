//! `@scope` in the data model (CSS Cascade 6 §2.5): the scopes one
//! stylesheet declares, and the owner node a prelude-less `@scope`
//! roots at.
//!
//! A [`Scope`] holds its `<scope-start>` / `<scope-end>` selectors,
//! already resolved by the parser — the start against its nesting
//! context (a parent style rule, or the enclosing scope's root), the
//! end and the scoped rules relative to the scoping root
//! (`rdom_core::selectors::parse_scoped`) — and its enclosing scope.
//! A rule records its innermost scope ([`Rule::scope`](super::Rule::scope));
//! the backend's cascade finds each element's scoping roots and the
//! scope proximity the cascade sorts by.

use rdom_core::NodeId;
use rdom_core::selectors::SelectorList;

use super::Stylesheet;

/// A scope declared in one [`Stylesheet`]: an index into
/// [`Stylesheet::scopes`]. Meaningful only for the sheet that issued it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ScopeId(u32);

impl ScopeId {
    /// The index into [`Stylesheet::scopes`].
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// One `@scope` rule's boundaries.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Scope {
    /// `<scope-start>`: every element it matches is a scoping root. When
    /// the `@scope` sits in another, it is matched with that scope's
    /// root as `:scope`, and a root must be in that scope. `None` for a
    /// prelude-less `@scope`, whose root is the parent element of the
    /// sheet's owner node ([`Stylesheet::owner_node`]).
    pub start: Option<SelectorList>,
    /// `<scope-end>`: the scoping limits, matched with the root as
    /// `:scope`; an element inside a limit (or a limit) is out of
    /// scope.
    pub end: Option<SelectorList>,
    /// The enclosing `@scope`, if nested in one.
    pub parent: Option<ScopeId>,
}

impl Scope {
    pub fn new(
        start: Option<SelectorList>,
        end: Option<SelectorList>,
        parent: Option<ScopeId>,
    ) -> Self {
        Scope { start, end, parent }
    }
}

impl Stylesheet {
    /// The scopes this sheet declares, in source order.
    pub fn scopes(&self) -> &[Scope] {
        &self.scopes
    }

    /// Declare an `@scope` and return its id, for
    /// [`RuleContext::in_scope`](super::RuleContext::in_scope).
    pub fn declare_scope(&mut self, scope: Scope) -> ScopeId {
        self.scopes.push(scope);
        ScopeId(self.scopes.len() as u32 - 1)
    }

    /// The node that owns this sheet — CSSOM `ownerNode`: the `<style>`
    /// element it was parsed from, `None` for a sheet built in Rust.
    pub fn owner_node(&self) -> Option<NodeId> {
        self.owner_node
    }

    /// Set the owning node (the backend does this for `<style>` sheets).
    pub fn set_owner_node(&mut self, node: Option<NodeId>) {
        self.owner_node = node;
    }

    /// Append `other`'s scopes, returning the id map for its rules.
    pub(super) fn append_scopes(&mut self, other: &Stylesheet) -> Vec<ScopeId> {
        let mut map = Vec::with_capacity(other.scopes.len());
        for scope in &other.scopes {
            let mut scope = scope.clone();
            scope.parent = scope.parent.map(|p: ScopeId| map[p.index()]);
            map.push(self.declare_scope(scope));
        }
        map
    }
}
