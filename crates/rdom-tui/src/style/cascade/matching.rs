//! Rule matching for one element or pseudo-element: the sheets' rule
//! index narrows the candidates, `scope::match_rule` decides each one,
//! and the matches are sorted into cascade order with their layer ranks
//! and ladder. The buffers live for a whole cascade pass.

use std::cmp::Reverse;

use rdom_core::{Dom, NodeId};

use super::ladder::Plan;
use super::scope::match_rule;
use super::sheets::Sheets;
use crate::ext::TuiExt;
use crate::style::{PseudoElementTarget, Rule};

/// The buffers of one element's rule matching, kept for the whole
/// cascade pass and reused by every element and pseudo-element
/// (`C1G-CASCADE-ALLOC`): candidate indices, the matched rules, their
/// sorted order, their layer ranks and the ladder.
#[derive(Default)]
pub(super) struct Scratch<'a> {
    candidates: Vec<u32>,
    matching: Vec<Matched<'a>>,
    pub(super) sorted: Vec<&'a Rule>,
    pub(super) ranks: Vec<u32>,
    pub(super) plan: Plan,
}

/// One matched rule: which of the requested targets it styles, its
/// scope proximity and its sheet.
struct Matched<'a> {
    target: usize,
    proximity: u32,
    sheet: usize,
    rule: &'a Rule,
}

impl<'a> Scratch<'a> {
    /// The rules of `sheets` styling `targets` of `id`, sorted by
    /// specificity, scope proximity (nearer wins, CSS Cascade 6 §6.1),
    /// target (a later target wins a tie), sheet and source order — into
    /// `sorted`, with their layer ranks and the element's ladder.
    pub(super) fn collect(
        &mut self,
        dom: &Dom<TuiExt>,
        sheets: &Sheets<'a>,
        id: NodeId,
        targets: &[PseudoElementTarget],
    ) {
        self.matching.clear();
        let node = dom.node(id);
        for (sheet_idx, &sheet) in sheets.iter().enumerate() {
            // The sheet's rightmost-selector index
            // (`CASCADE-INITIAL-ALLOC-1`): a superset of the matches.
            sheet.rule_index().candidates(
                node.tag_name(),
                node.id_attr(),
                node.class_list().iter(),
                &mut self.candidates,
            );
            for &ri in &self.candidates {
                let rule = &sheet.rules()[ri as usize];
                if let Some(target) = targets.iter().position(|t| *t == rule.pseudo)
                    && let Some(proximity) = match_rule(dom, id, sheet, rule)
                {
                    self.matching.push(Matched {
                        target,
                        proximity,
                        sheet: sheet_idx,
                        rule,
                    });
                }
            }
        }
        self.matching.sort_by_key(|m| {
            (
                m.rule.specificity,
                Reverse(m.proximity),
                m.target,
                m.sheet,
                m.rule.source_idx,
            )
        });
        self.sorted.clear();
        self.sorted.extend(self.matching.iter().map(|m| m.rule));
        sheets.plan_into(
            self.matching.iter().map(|m| (m.sheet, m.rule)),
            &mut self.ranks,
            &mut self.plan,
        );
    }
}
