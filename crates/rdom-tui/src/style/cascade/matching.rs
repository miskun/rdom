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

use rdom_core::{Dom, NodeId, SelectorCaches};

use super::ladder::Plan;
use super::registered::PropertyRegistry;
use super::scope::{ScopeMemo, SheetRef, match_rule};
use super::sheets::Sheets;
use crate::ext::TuiExt;
use crate::style::{PseudoElementTarget, Rule};

/// The buffers of one element's rule matching, kept for the whole
/// cascade pass and reused by every element and pseudo-element
/// (`C1G-CASCADE-ALLOC`): candidate indices, the matched rules, their
/// sorted order, their layer ranks and the ladder — and the walk state a
/// restyle reads across elements, `items_changed` and
/// `root_line_height_moved`.
#[derive(Default)]
pub(super) struct Scratch<'a> {
    candidates: Vec<u32>,
    /// `@scope` roots learned this pass.
    scopes: ScopeMemo,
    /// The selector matcher's caches for this pass (`:nth-*()`'s
    /// sibling indices, C11-NTH).
    selectors: SelectorCaches,
    matching: Vec<Matched<'a>>,
    pub(super) sorted: Vec<&'a Rule>,
    pub(super) ranks: Vec<u32>,
    pub(super) plan: Plan,
    /// Elements a restyle gave a new answer to "are my children's boxes
    /// flex or grid items?" — their `display: contents`-ness or their
    /// flex / grid flow changed — this pass (`walk::style_element`'s
    /// restyle guard, C7G-MINOR). Empty unless one did.
    pub(super) items_changed: Vec<rdom_core::NodeId>,
    /// A restyle moved the root element's used line height this pass:
    /// every `rlh` below reads it (CSS Values 4 §6.1.1), so no element
    /// keeps its subtree (`walk::style_element`).
    pub(super) root_line_height_moved: bool,
    /// The last gather's rules include one whose selector matched but
    /// whose trailing pseudo-classes do not hold (`::first-letter:hover`
    /// off the letter): the pseudo-element exists, unstyled by it.
    pub(super) gated: bool,
    /// The last `::highlight()` matches recorded per name and the last
    /// list of them: elements matching the same rules share them
    /// ([`intern_highlights`](Self::intern_highlights)).
    highlight_refs: Vec<Rc<[MatchRef]>>,
    highlight_list: Option<HighlightRefs>,
    /// Buffers of one element's `::highlight()` boxes, reused.
    pub(super) highlight_buf: Vec<Rc<[MatchRef]>>,
    pub(super) highlight_styles: Vec<(std::sync::Arc<str>, Rc<crate::style::ComputedStyle>)>,
}

/// One matched rule: which of the requested targets it styles, its
/// scope proximity and its sheet.
struct Matched<'a> {
    target: usize,
    /// Its trailing pseudo-classes hold (always, for a rule without
    /// them): it applies. A rule whose do not is recorded all the same —
    /// its selector matched — so a cached reload re-checks it.
    live: bool,
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
    Marker,
    FirstLine,
    FirstLetter,
    DetailsContent,
    BeforeMarker,
    AfterMarker,
}

const SLOTS: usize = 14;

/// An element's matched rules per box, as of its last cascade, under one
/// sheet set (`Sheets::stamp`). `None` for a box that was not matched
/// (the scrollbar parts of an element that does not scroll).
#[derive(Debug)]
pub(crate) struct MatchedRules {
    stamp: Rc<PropertyRegistry>,
    slots: [Option<Rc<[MatchRef]>>; SLOTS],
    /// The `::highlight(name)` boxes' matches, by the name's place in
    /// `Sheets::highlight_names`; `None` when they were not matched.
    highlights: Option<HighlightRefs>,
}

/// The matches of each `::highlight()` name, in `Sheets::highlight_names`
/// order — shared by the elements whose matches are the same
/// ([`Scratch::intern_highlights`]).
pub(super) type HighlightRefs = Rc<[Rc<[MatchRef]>]>;

impl PartialEq for MatchedRules {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.stamp, &other.stamp)
            && self.slots == other.slots
            && self.highlights == other.highlights
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

    /// The rules of the `k`th `::highlight()` name, as [`rules`](Self::rules).
    pub(super) fn highlight_rules(&self, k: usize) -> Rules<'_> {
        match self.highlights.as_deref().and_then(|h| h.get(k)) {
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
    highlights: Option<HighlightRefs>,
    changed: bool,
}

impl Recorder {
    /// Record over `previous` (the element's last record, if it was made
    /// under the same sheets). With `reuse`, the boxes this walk does not
    /// match keep their previous matches (a restyle reloads them);
    /// without, every box is matched afresh and an unmatched one has
    /// none.
    pub(super) fn new(previous: Option<Rc<MatchedRules>>, reuse: bool) -> Self {
        let (slots, highlights) = match &previous {
            Some(p) if reuse => (p.slots.clone(), p.highlights.clone()),
            _ => Default::default(),
        };
        Recorder {
            changed: previous.is_none(),
            previous,
            slots,
            highlights,
        }
    }

    /// The `::highlight()` boxes' matches, every name matched afresh.
    pub(super) fn record_highlights(&mut self, refs: HighlightRefs) {
        let kept = self
            .previous
            .as_ref()
            .and_then(|p| p.highlights.as_ref())
            .filter(|old| **old == refs);
        self.highlights = Some(match kept {
            Some(old) => old.clone(),
            None => {
                self.changed = true;
                refs
            }
        });
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
            Some(previous)
                if !self.changed
                    && previous.slots == self.slots
                    && previous.highlights == self.highlights =>
            {
                previous
            }
            _ => Rc::new(MatchedRules {
                stamp: sheets.stamp().clone(),
                slots: self.slots,
                highlights: self.highlights,
            }),
        }
    }
}

impl<'a> Scratch<'a> {
    /// Flag every element a `:has()` was evaluated for in this pass
    /// (`TuiExt::has_anchor`, `style::has_triggers`).
    pub(super) fn flag_has_anchors(&self, dom: &mut Dom<TuiExt>) {
        let mut any = false;
        for id in self.selectors.has_anchors() {
            if let Some(ext) = dom.node_mut(id).ext_mut() {
                ext.has_anchor = true;
                any = true;
            }
        }
        if any {
            crate::style::doc_flags::note_has_anchor(dom);
        }
    }

    /// The last [`collect`](Self::collect)'s matches, as the `k`th
    /// highlight name's record: the one recorded last for that name when
    /// they are the same (no allocation), else a new one.
    pub(super) fn intern_highlight(&mut self, k: usize) -> Rc<[MatchRef]> {
        let current = self.matching.iter().map(Matched::as_ref);
        if let Some(last) = self.highlight_refs.get(k)
            && last.iter().copied().eq(current.clone())
        {
            return last.clone();
        }
        let refs: Rc<[MatchRef]> = current.collect();
        if k < self.highlight_refs.len() {
            self.highlight_refs[k] = refs.clone();
        } else {
            self.highlight_refs.resize(k + 1, refs.clone());
        }
        refs
    }

    /// `refs` (one per highlight name) as a shared list: the last one
    /// built when its records are the same ones, else a new one.
    pub(super) fn intern_highlights(&mut self, refs: &[Rc<[MatchRef]>]) -> HighlightRefs {
        if let Some(list) = &self.highlight_list
            && list.len() == refs.len()
            && list.iter().zip(refs).all(|(a, b)| Rc::ptr_eq(a, b))
        {
            return list.clone();
        }
        let list: HighlightRefs = refs.iter().cloned().collect();
        self.highlight_list = Some(list.clone());
        list
    }

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
            Rules::Cached(refs) => self.load(dom, sheets, id, refs),
        }
    }

    /// Recorded matches into `sorted`, with their layer ranks and the
    /// ladder — no selector is matched; only the pseudo-element pointer
    /// state of a rule with trailing pseudo-classes is read again.
    fn load(&mut self, dom: &Dom<TuiExt>, sheets: &Sheets<'a>, id: NodeId, refs: &[MatchRef]) {
        self.matching.clear();
        self.sorted.clear();
        let rules = refs.iter().map(|r| {
            let rule = &sheets[r.sheet as usize].rules()[r.rule as usize];
            (r.sheet as usize, rule)
        });
        let live = rules
            .clone()
            .filter(move |(_, rule)| applies(dom, id, rule));
        self.gated = rules.len() != live.clone().count();
        self.sorted.extend(live.clone().map(|(_, rule)| rule));
        sheets.plan_into(live, &mut self.ranks, &mut self.plan);
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
                dom.class_list(id),
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
                        &mut self.selectors,
                    )
                {
                    self.matching.push(Matched {
                        target,
                        live: applies(dom, id, rule),
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
        let live = self.matching.iter().filter(|m| m.live);
        self.gated = self.matching.len() != live.clone().count();
        self.sorted.extend(live.clone().map(|m| m.rule));
        sheets.plan_into(
            live.map(|m| (m.sheet, m.rule)),
            &mut self.ranks,
            &mut self.plan,
        );
    }
}

/// Whether `rule`, whose selector matched `id`, applies: its trailing
/// pseudo-classes (`::before:hover`, Selectors 4 §3.6.3), if any,
/// describe the pseudo-element's own pointer state, which the runtime
/// keeps beside the DOM (`style::pseudo_pointer`).
fn applies(dom: &Dom<TuiExt>, id: NodeId, rule: &Rule) -> bool {
    crate::style::pseudo_pointer::matches(dom, id, &rule.pseudo, rule.pseudo_state)
}

/// Test-only: a pass's selector cache work, counted when it ends.
#[cfg(test)]
impl Drop for Scratch<'_> {
    fn drop(&mut self) {
        let work = self.selectors.work();
        probe::NTH.with(|c| c.set(c.get() + work.nth_siblings));
        probe::HAS.with(|c| c.set(c.get() + work.has_nodes));
    }
}

/// Test-only: how many rule-matching passes ran on this thread, and the
/// sibling steps the passes' nth indices took.
#[cfg(test)]
pub(crate) mod probe {
    thread_local! {
        pub static COLLECTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
        pub static NTH: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
        pub static HAS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    }

    pub fn take_has() -> u64 {
        HAS.with(|c| c.replace(0))
    }

    pub fn take() -> usize {
        COLLECTS.with(|c| c.replace(0))
    }

    pub fn take_nth() -> u64 {
        NTH.with(|c| c.replace(0))
    }
}
