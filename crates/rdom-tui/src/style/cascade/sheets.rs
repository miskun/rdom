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
use rdom_style::color::ColorScheme;
use rdom_style::counters::{CounterStyleDefinition, CounterStyleRegistry};

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
    color_scheme: ColorScheme,
    /// The counter styles the sheets define over the predefined ones,
    /// built on first use.
    counter_styles: std::cell::OnceCell<CounterStyleRegistry>,
    /// Whether any rule styles `::first-line` / `::first-letter`, found
    /// on first use: an element's are matched only then.
    first_rules: std::cell::OnceCell<(bool, bool)>,
    /// The names `::highlight()` rules style, found on first use.
    highlight_names: std::cell::OnceCell<Vec<std::sync::Arc<str>>>,
}

impl<'a> Sheets<'a> {
    /// `list`, with the custom properties it registers (`registry`, built
    /// from `list`), cascaded for a terminal of `viewport`'s size whose
    /// preferred color scheme is `color_scheme`.
    pub(super) fn new(
        list: &'a [&'a Stylesheet],
        registry: Rc<PropertyRegistry>,
        viewport: Viewport,
        color_scheme: ColorScheme,
    ) -> Self {
        Sheets {
            list,
            layers: LayerOrder::new(list),
            registry,
            viewport,
            color_scheme,
            counter_styles: std::cell::OnceCell::new(),
            first_rules: std::cell::OnceCell::new(),
            highlight_names: std::cell::OnceCell::new(),
        }
    }

    /// The highlight names the sheets' `::highlight()` rules style (CSS
    /// Custom Highlight API 1 §5.1), each once: an element's highlight
    /// styles are matched for these alone.
    pub(super) fn highlight_names(&self) -> &[std::sync::Arc<str>] {
        self.highlight_names.get_or_init(|| {
            let mut names: Vec<std::sync::Arc<str>> = Vec::new();
            for rule in self.list.iter().flat_map(|s| s.rules()) {
                if let crate::style::PseudoElementTarget::Highlight(name) = &rule.pseudo
                    && !names.contains(name)
                {
                    names.push(name.clone());
                }
            }
            names
        })
    }

    /// Whether any of the sheets has a `::first-line` rule, and a
    /// `::first-letter` one (CSS Pseudo-Elements 4 §2.2, §2.3).
    pub(super) fn styles_first(&self) -> (bool, bool) {
        *self.first_rules.get_or_init(|| {
            let has = |target| {
                self.list
                    .iter()
                    .any(|s| s.rules().iter().any(|r| r.pseudo == target))
            };
            (
                has(crate::style::PseudoElementTarget::FirstLine),
                has(crate::style::PseudoElementTarget::FirstLetter),
            )
        })
    }

    /// The counter styles names resolve to (CSS Counter Styles 3 §3):
    /// the sheets' `@counter-style` definitions over the predefined
    /// styles, a later definition of a name winning — by cascade layer
    /// (unlayered last), then sheet, then source order (CSS Cascade 5
    /// §6.4.3).
    pub(super) fn counter_styles(&self) -> &CounterStyleRegistry {
        self.counter_styles.get_or_init(|| {
            let mut defs: Vec<(u32, usize, usize, &CounterStyleDefinition)> = Vec::new();
            for (sheet, s) in self.list.iter().enumerate() {
                for (i, def) in s.counter_styles().iter().enumerate() {
                    defs.push((self.layers.rank(sheet, def.layer), sheet, i, def));
                }
            }
            defs.sort_by_key(|&(rank, sheet, i, _)| (rank, sheet, i));
            let mut registry = CounterStyleRegistry::new();
            for (_, _, _, def) in defs {
                registry.define(def.name.clone(), def.rule.clone());
            }
            registry
        })
    }

    /// The document's preferred color scheme (CSS Color Adjust 1 §2.1).
    pub(super) fn color_scheme(&self) -> ColorScheme {
        self.color_scheme
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
