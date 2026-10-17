//! What the last cascade did, for tests (`test-util`): the style rules it
//! matched for an element — the acid page's coverage test counts a
//! declaration as used only when its rule styled something
//! (C16G-ACID-RIGOUR).

use rdom_core::NodeId;

use super::App;
use crate::render::Backend;
use crate::style::{Rule, Stylesheet};

impl<B: Backend> App<B> {
    /// The style rules the last cascade matched for `id` — for its own
    /// box, its pseudo-elements and its `::highlight()` boxes, each under
    /// a condition that held (`@media`, `@supports`, `@container`) — with
    /// the sheet each sits in: the `<style>` elements' sheets, the `App`'s
    /// sheets, the registrations sheet, as the cascade orders them (the UA
    /// sheet's rules sit in the sheets they were parsed into). A rule
    /// whose selector matched and whose trailing pseudo-classes
    /// (`::before:hover`) did not hold is listed too. Empty for an element
    /// no cascade styled, and stale until the next frame after a sheet
    /// changes.
    pub fn matched_rules(&self, id: NodeId) -> Vec<(&Stylesheet, &Rule)> {
        let sheets = self.prelude.cascade_order(&self.stylesheets);
        let Some(matched) = self.dom.node(id).ext().and_then(|e| e.matched.as_deref()) else {
            return Vec::new();
        };
        matched
            .positions()
            .filter_map(|(s, r)| {
                let sheet: &Stylesheet = sheets.get(s)?;
                Some((sheet, sheet.rules().get(r)?))
            })
            .collect()
    }
}
