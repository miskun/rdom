//! The sheets of one cascade run, with their merged cascade-layer
//! order (CSS Cascade 5 §6.4).
//!
//! An `App` cascades the document's `<style>` sheets in tree order,
//! then its own (`App::new` / `push_stylesheet`); the slice given to
//! `cascade_all` is that list. Its sheets share one layer order —
//! CSSOM orders all of a document's sheets together — so a layer name
//! means the same layer in every sheet and the first declaration in
//! any of them fixes its place ([`LayerOrder`]).

use rdom_style::LayerOrder;

use super::ladder::Plan;
use super::registered::Registry;
use crate::style::{Rule, RuleOrigin, Stylesheet};

/// The stylesheets of one cascade run, in cascade order, and the
/// layer order computed once for all of them.
pub(super) struct Sheets<'a> {
    list: &'a [&'a Stylesheet],
    layers: LayerOrder,
    registry: Registry,
}

impl<'a> Sheets<'a> {
    pub(super) fn new(list: &'a [&'a Stylesheet]) -> Self {
        Sheets {
            list,
            layers: LayerOrder::new(list),
            registry: Registry::new(list),
        }
    }

    /// The custom properties the sheets register.
    pub(super) fn registry(&self) -> &Registry {
        &self.registry
    }

    /// Each matched rule's layer rank (parallel to `matched`, which is
    /// `(sheet index, rule)` pairs) and the ladder for those rules.
    pub(super) fn plan_for(&self, matched: &[(usize, &Rule)]) -> (Vec<u32>, Plan) {
        let ranks: Vec<u32> = matched
            .iter()
            .map(|(sheet, rule)| self.layers.rank(*sheet, rule.layer))
            .collect();
        let author = matched
            .iter()
            .zip(&ranks)
            .filter(|((_, rule), _)| rule.origin == RuleOrigin::Author)
            .map(|(_, rank)| *rank);
        let plan = Plan::new(author);
        (ranks, plan)
    }
}

impl<'a> std::ops::Deref for Sheets<'a> {
    type Target = [&'a Stylesheet];

    fn deref(&self) -> &Self::Target {
        self.list
    }
}
