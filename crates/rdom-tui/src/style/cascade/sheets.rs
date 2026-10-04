//! The sheets of one cascade run, with their merged cascade-layer
//! order (CSS Cascade 5 §6.4).
//!
//! An `App` cascades the document's `<style>` sheets in tree order,
//! then its own (`App::new` / `push_stylesheet`); the slice given to
//! `cascade_all` is that list. Its sheets share one layer order —
//! CSSOM orders all of a document's sheets together — so a layer name
//! means the same layer in every sheet and the first declaration in
//! any of them fixes its place ([`LayerOrder`]).

use std::rc::Rc;

use rdom_style::LayerOrder;
use rdom_style::calc::Viewport;

use super::ladder::Plan;
use super::registered::PropertyRegistry;
use crate::style::{Rule, RuleOrigin, Stylesheet};

/// The stylesheets of one cascade run, in cascade order, and the
/// layer order computed once for all of them.
pub(super) struct Sheets<'a> {
    list: &'a [&'a Stylesheet],
    layers: LayerOrder,
    registry: Rc<PropertyRegistry>,
    viewport: Viewport,
}

impl<'a> Sheets<'a> {
    /// `list`, with the custom properties it registers (`registry`, built
    /// from `list`), cascaded for a terminal of `viewport`'s size.
    pub(super) fn new(
        list: &'a [&'a Stylesheet],
        registry: Rc<PropertyRegistry>,
        viewport: Viewport,
    ) -> Self {
        Sheets {
            list,
            layers: LayerOrder::new(list),
            registry,
            viewport,
        }
    }

    /// The terminal size the viewport-percentage units resolve against.
    pub(super) fn viewport(&self) -> Viewport {
        self.viewport
    }

    /// The custom properties the sheets register.
    pub(super) fn registry(&self) -> &PropertyRegistry {
        &self.registry
    }

    /// The identity of this sheet set: the registry `Rc`, rebuilt by an
    /// `App` exactly when its sheets change. Recorded matches
    /// (`matching::MatchedRules`) are valid only under the same one.
    pub(super) fn stamp(&self) -> &Rc<PropertyRegistry> {
        &self.registry
    }

    /// Each matched rule's layer rank into `ranks` (parallel to
    /// `matched`, `(sheet index, rule)` pairs) and the ladder for those
    /// rules into `plan` — both buffers reused across elements.
    pub(super) fn plan_into<'r>(
        &self,
        matched: impl Iterator<Item = (usize, &'r Rule)> + Clone,
        ranks: &mut Vec<u32>,
        plan: &mut Plan,
    ) {
        ranks.clear();
        ranks.extend(
            matched
                .clone()
                .map(|(sheet, rule)| self.layers.rank(sheet, rule.layer)),
        );
        let author = matched
            .zip(ranks.iter())
            .filter(|((_, rule), _)| rule.origin == RuleOrigin::Author)
            .map(|(_, rank)| *rank);
        plan.rebuild(author);
    }
}

impl<'a> std::ops::Deref for Sheets<'a> {
    type Target = [&'a Stylesheet];

    fn deref(&self) -> &Self::Target {
        self.list
    }
}
