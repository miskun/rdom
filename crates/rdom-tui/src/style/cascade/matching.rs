//! Rule matching for one element or pseudo-element: the sheets' rule
//! index narrows the candidates, `scope::match_rule` decides each one,
//! and the matches are sorted into cascade order with their layer ranks
//! and ladder. The buffers live for a whole cascade pass.
//!
//! The matches are recorded on the element ([`MatchedRules`]), so a
//! restyle that changes no selector's result — the per-frame restyle of
//! a registered custom property's transition (`C1G-PROPERTY-RESTYLE`) —
//! reloads them ([`Rules::Cached`]) instead of matching again.

use std::cmp::Reverse;
use std::rc::Rc;

use rdom_core::{Dom, NodeId};

use super::ladder::Plan;
use super::registered::PropertyRegistry;
use super::scope::{ScopeMemo, SheetRef, match_rule};
use super::sheets::Sheets;
use crate::ext::TuiExt;
use crate::style::{PseudoElementTarget, Rule};

/// The buffers of one element's rule matching, kept for the whole
/// cascade pass and reused by every element and pseudo-element
/// (`C1G-CASCADE-ALLOC`): candidate indices, the matched rules, their
/// sorted order, their layer ranks and the ladder — and the one piece of
/// walk state a restyle reads across elements, `items_changed`.
#[derive(Default)]
pub(super) struct Scratch<'a> {
    candidates: Vec<u32>,
    /// `@scope` roots learned this pass.
    scopes: ScopeMemo,
    matching: Vec<Matched<'a>>,
    pub(super) sorted: Vec<&'a Rule>,
    pub(super) ranks: Vec<u32>,
    pub(super) plan: Plan,
    /// Elements a restyle gave a new answer to "are my children's boxes
    /// flex or grid items?" — their `display: contents`-ness or their
    /// flex / grid flow changed — this pass (`walk::style_element`'s
    /// restyle guard, C7G-MINOR). Empty unless one did.
    pub(super) items_changed: Vec<rdom_core::NodeId>,
}

/// One matched rule: which of the requested targets it styles, its
/// scope proximity and its sheet.
struct Matched<'a> {
    target: usize,
    proximity: u32,
    sheet: usize,
    index: u32,
    rule: &'a Rule,
}

impl Matched<'_> {
    fn as_ref(&self) -> MatchRef {
        MatchRef {
            sheet: self.sheet as u32,
            rule: self.index,
        }
    }
}

/// A matched rule by position: its sheet's index in the sheet set and
/// its index in the sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MatchRef {
    sheet: u32,
    rule: u32,
}

/// Where one element's or pseudo-element's rules come from.
#[derive(Clone, Copy)]
pub(super) enum Rules<'r> {
    /// Match the selectors.
    Match,
    /// The recorded matches, in cascade order.
    Cached(&'r [MatchRef]),
}

/// The boxes of an element whose matches are recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Slot {
    Element,
    Before,
    Backdrop,
    Selection,
    Scrollbar,
    ThumbVertical,
    ThumbHorizontal,
    After,
}

const SLOTS: usize = 8;

/// An element's matched rules per box, as of its last cascade, under one
/// sheet set (`Sheets::stamp`). `None` for a box that was not matched
/// (the scrollbar parts of an element that does not scroll).
#[derive(Debug)]
pub(crate) struct MatchedRules {
    stamp: Rc<PropertyRegistry>,
    slots: [Option<Rc<[MatchRef]>>; SLOTS],
}

impl PartialEq for MatchedRules {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.stamp, &other.stamp) && self.slots == other.slots
    }
}

impl MatchedRules {
    /// Recorded under `sheets`?
    pub(super) fn is_for(&self, sheets: &Sheets<'_>) -> bool {
        Rc::ptr_eq(&self.stamp, sheets.stamp())
    }

    /// The rules of `slot`: the recorded ones, or [`Rules::Match`] when
    /// that box was not matched.
    pub(super) fn rules(&self, slot: Slot) -> Rules<'_> {
        match &self.slots[slot as usize] {
            Some(refs) => Rules::Cached(refs),
            None => Rules::Match,
        }
    }
}

/// Builds an element's [`MatchedRules`] as its boxes are matched,
/// keeping the previous record — no allocation — when nothing changed.
pub(super) struct Recorder {
    previous: Option<Rc<MatchedRules>>,
    slots: [Option<Rc<[MatchRef]>>; SLOTS],
    changed: bool,
}

impl Recorder {
    /// Record over `previous` (the element's last record, if it was made
    /// under the same sheets). With `reuse`, the boxes this walk does not
    /// match keep their previous matches (a restyle reloads them);
    /// without, every box is matched afresh and an unmatched one has
    /// none.
    pub(super) fn new(previous: Option<Rc<MatchedRules>>, reuse: bool) -> Self {
        let slots = match &previous {
            Some(p) if reuse => p.slots.clone(),
            _ => Default::default(),
        };
        Recorder {
            changed: previous.is_none(),
            previous,
            slots,
        }
    }

    /// `slot`'s matches: the last [`Scratch::collect`].
    pub(super) fn record(&mut self, slot: Slot, scratch: &Scratch<'_>) {
        let current = scratch.matching.iter().map(Matched::as_ref);
        let kept = self
            .previous
            .as_ref()
            .and_then(|p| p.slots[slot as usize].as_ref())
            .filter(|old| old.iter().copied().eq(current.clone()));
        self.slots[slot as usize] = match kept {
            Some(old) => Some(old.clone()),
            None => {
                self.changed = true;
                Some(current.collect())
            }
        };
    }

    /// The record: the previous one when every box matched the same
    /// rules, else a new one.
    pub(super) fn finish(self, sheets: &Sheets<'_>) -> Rc<MatchedRules> {
        match self.previous {
            Some(previous) if !self.changed && previous.slots == self.slots => previous,
            _ => Rc::new(MatchedRules {
                stamp: sheets.stamp().clone(),
                slots: self.slots,
            }),
        }
    }
}

impl<'a> Scratch<'a> {
    /// [`collect`](Self::collect) or [`load`](Self::load), per `rules`.
    pub(super) fn gather(
        &mut self,
        dom: &Dom<TuiExt>,
        sheets: &Sheets<'a>,
        id: NodeId,
        targets: &[PseudoElementTarget],
        rules: Rules<'_>,
    ) {
        match rules {
            Rules::Match => self.collect(dom, sheets, id, targets),
            Rules::Cached(refs) => self.load(sheets, refs),
        }
    }

    /// Recorded matches into `sorted`, with their layer ranks and the
    /// ladder — no selector is matched.
    fn load(&mut self, sheets: &Sheets<'a>, refs: &[MatchRef]) {
        self.matching.clear();
        self.sorted.clear();
        self.sorted.extend(
            refs.iter()
                .map(|r| &sheets[r.sheet as usize].rules()[r.rule as usize]),
        );
        sheets.plan_into(
            refs.iter()
                .zip(&self.sorted)
                .map(|(r, rule)| (r.sheet as usize, *rule)),
            &mut self.ranks,
            &mut self.plan,
        );
    }

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
        #[cfg(test)]
        probe::COLLECTS.with(|c| c.set(c.get() + 1));

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
                    && let Some(proximity) = match_rule(
                        dom,
                        id,
                        SheetRef {
                            index: sheet_idx,
                            sheet,
                        },
                        rule,
                        &mut self.scopes,
                    )
                {
                    self.matching.push(Matched {
                        target,
                        proximity,
                        sheet: sheet_idx,
                        index: ri,
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

/// Test-only: how many rule-matching passes ran on this thread.
#[cfg(test)]
pub(crate) mod probe {
    thread_local! {
        pub static COLLECTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    pub fn take() -> usize {
        COLLECTS.with(|c| c.replace(0))
    }
}
