//! The conditional group rules a sheet declares (CSS Conditional 3 §2):
//! `@media`, and — as they land — `@supports` and `@container`. Kept as
//! rule context, as `@layer` and `@scope` are: each [`ConditionRule`] is
//! declared once with its enclosing one, and every rule inside records
//! the innermost ([`Rule::condition`](super::Rule::condition),
//! [`RuleContext::in_condition`](super::RuleContext::in_condition)). A
//! rule applies while its condition and every enclosing one hold; the
//! backend's cascade evaluates them.

use super::Stylesheet;
use crate::conditional::MediaList;

/// A conditional group rule declared in one [`Stylesheet`]: an index
/// into [`Stylesheet::conditions`]. Meaningful only for the sheet that
/// issued it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConditionId(u32);

impl ConditionId {
    /// The index into [`Stylesheet::conditions`].
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// One conditional group rule: its condition and the rule it sits in.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ConditionRule {
    /// What it tests.
    pub kind: ConditionKind,
    /// The enclosing conditional group rule, if nested in one.
    pub parent: Option<ConditionId>,
}

impl ConditionRule {
    pub fn new(kind: ConditionKind, parent: Option<ConditionId>) -> Self {
        ConditionRule { kind, parent }
    }
}

/// What a conditional group rule tests.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ConditionKind {
    /// `@media <media-query-list>` (Media Queries 4 §2, CSS Conditional
    /// 3 §3), and an `@import`'s media list.
    Media(MediaList),
}

impl Stylesheet {
    /// The conditional group rules this sheet declares, in source order.
    pub fn conditions(&self) -> &[ConditionRule] {
        &self.conditions
    }

    /// Declare a conditional group rule and return its id, for
    /// [`RuleContext::in_condition`](super::RuleContext::in_condition).
    pub fn declare_condition(&mut self, rule: ConditionRule) -> ConditionId {
        self.touch();
        self.conditions.push(rule);
        ConditionId(self.conditions.len() as u32 - 1)
    }

    /// `condition` and every rule enclosing it, innermost first.
    pub fn condition_chain(
        &self,
        condition: Option<ConditionId>,
    ) -> impl Iterator<Item = &ConditionRule> {
        let mut next = condition;
        std::iter::from_fn(move || {
            let rule = self.conditions.get(next?.index())?;
            next = rule.parent;
            Some(rule)
        })
    }

    /// Append `other`'s conditions, returning the id map for its rules.
    pub(super) fn append_conditions(&mut self, other: &Stylesheet) -> Vec<ConditionId> {
        let mut map = Vec::with_capacity(other.conditions.len());
        for rule in &other.conditions {
            let mut rule = rule.clone();
            rule.parent = rule.parent.map(|p: ConditionId| map[p.index()]);
            map.push(self.declare_condition(rule));
        }
        map
    }
}
