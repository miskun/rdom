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
use rdom_style::conditional::MediaEnvironment;
use rdom_style::counters::{CounterStyleDefinition, CounterStyleRegistry};

use super::conditions::ConditionResults;
use super::ladder::Plan;
use super::registered::PropertyRegistry;
use crate::style::{Rule, RuleOrigin, Stylesheet};

/// The stylesheets of one cascade run, in cascade order, and the
/// layer order computed once for all of them.
pub(super) struct Sheets<'a> {
    list: &'a [&'a Stylesheet],
    layers: LayerOrder,
    registry: Rc<PropertyRegistry>,
    /// The registrations in effect under `conditions` (`registry`'s
    /// `active`).
    active: Rc<PropertyRegistry>,
    /// The media environment: the viewport, the preferred color scheme
    /// and the preferences `@media` reads.
    media: MediaEnvironment,
    /// Which conditional group rules hold in `media` (`conditions.rs`).
    conditions: Rc<ConditionResults>,
    /// The counter styles and keyframes names resolve to when a
    /// definition sits under a condition: they depend on `conditions`, so
    /// they are found per run rather than kept on the sheet set.
    conditional_counter_styles: std::cell::OnceCell<CounterStyleRegistry>,
    conditional_keyframes: std::cell::OnceCell<KeyframesMap>,
    /// Whether `@starting-style` rules apply (CSS Transitions 2 §3): only
    /// when computing an element's starting style (`starting.rs`).
    starting: bool,
    /// A keyframe's declaration blocks, applied in the animation origin
    /// when computing a keyframe style (`keyframe_style.rs`); empty
    /// otherwise.
    animation: &'a [&'a crate::style::TuiStyle],
}

/// What a sheet set holds that every element's cascade asks, each found
/// on first use and kept for the set — on its registry, the set's
/// identity (`stamp`): an `App`'s frames and the stateless cascades over
/// unchanged sheets scan the rules once, not once per run.
#[derive(Debug, Default)]
pub(super) struct SheetFacts {
    /// The counter styles the sheets define over the predefined ones.
    counter_styles: std::cell::OnceCell<CounterStyleRegistry>,
    /// Whether any rule styles `::first-line` / `::first-letter`: an
    /// element's are matched only then.
    first_rules: std::cell::OnceCell<(bool, bool)>,
    /// The names `::highlight()` rules style.
    highlight_names: std::cell::OnceCell<Vec<std::sync::Arc<str>>>,
    /// Whether any rule is a `@starting-style` rule.
    starting_rules: std::cell::OnceCell<bool>,
    /// Each `@keyframes` name and the rule it resolves to: `(sheet,
    /// index)` into the sheets' `Stylesheet::keyframes`.
    keyframes: std::cell::OnceCell<KeyframesMap>,
    /// Whether a `@counter-style` or `@keyframes` rule sits under a
    /// conditional group rule: then which one a name resolves to depends
    /// on the media environment.
    conditional_definitions: std::cell::OnceCell<bool>,
    /// The conditions' results in the last media environment
    /// (`conditions.rs`).
    pub(super) conditions: super::conditions::ConditionCache,
    /// The registry in effect under the last results, when a registration
    /// is conditional (`PropertyRegistry::active`).
    pub(super) active_registry: std::cell::RefCell<
        Option<(
            std::rc::Rc<super::conditions::ConditionResults>,
            std::rc::Rc<super::registered::PropertyRegistry>,
        )>,
    >,
}

/// Each `@keyframes` name and the rule it resolves to: `(sheet, index)`.
type KeyframesMap = std::collections::HashMap<std::sync::Arc<str>, (usize, usize)>;

impl<'a> Sheets<'a> {
    /// `list`, with the custom properties it registers (`registry`, built
    /// from `list`), cascaded in the media environment `media`: a
    /// terminal of its viewport's size, with its preferred color scheme
    /// and preferences.
    pub(super) fn new(
        list: &'a [&'a Stylesheet],
        registry: Rc<PropertyRegistry>,
        media: MediaEnvironment,
    ) -> Self {
        let conditions = registry.facts.conditions.get(list, &media);
        // CSS Cascade 5 §6.4.3: a layer declaration under a condition that
        // does not hold takes no place in the order (an `@container`'s
        // holds here); an `@property` registers only while its holds.
        let layers = LayerOrder::new_where(list, |sheet, c| conditions.holds(sheet, c));
        let active = registry.active(list, &conditions);
        Sheets {
            list,
            layers,
            active,
            registry,
            media,
            conditions,
            conditional_counter_styles: std::cell::OnceCell::new(),
            conditional_keyframes: std::cell::OnceCell::new(),
            starting: false,
            animation: &[],
        }
    }

    /// The results of the sheets' conditional group rules in this run's
    /// media environment: the same `Rc` while no condition flips.
    pub(super) fn conditions(&self) -> &Rc<ConditionResults> {
        &self.conditions
    }

    /// Whether a counter style or keyframes definition of sheet `sheet`
    /// under `condition` takes part in this run.
    fn defines(&self, sheet: usize, condition: Option<rdom_style::ConditionId>) -> bool {
        self.conditions.holds(sheet, condition)
    }

    /// Whether any `@counter-style` or `@keyframes` rule is conditional.
    fn has_conditional_definitions(&self) -> bool {
        *self.registry.facts.conditional_definitions.get_or_init(|| {
            self.list.iter().any(|s| {
                s.media().is_some()
                    || s.counter_styles().iter().any(|d| d.condition.is_some())
                    || s.keyframes().iter().any(|k| k.condition.is_some())
            })
        })
    }

    /// These sheets with a keyframe's `blocks` applying in the animation
    /// origin (CSS Cascade 5 §6.1) — what a keyframe style is computed
    /// under.
    pub(super) fn with_animation(self, blocks: &'a [&'a crate::style::TuiStyle]) -> Self {
        Sheets {
            animation: blocks,
            ..self
        }
    }

    /// The keyframe blocks of [`with_animation`](Self::with_animation).
    pub(super) fn animation(&self) -> &'a [&'a crate::style::TuiStyle] {
        self.animation
    }

    /// These sheets with their `@starting-style` rules applying — what an
    /// element's starting style is computed under.
    pub(super) fn with_starting_style(self) -> Self {
        Sheets {
            starting: true,
            ..self
        }
    }

    /// Whether `rule`, of sheet `sheet`, applies under this set: a
    /// `@starting-style` rule only when computing a starting style, a rule
    /// in a conditional group rule only while its conditions hold (CSS
    /// Conditional 3 §2).
    pub(super) fn applies(&self, sheet: usize, rule: &Rule) -> bool {
        (self.starting || !rule.starting_style) && self.conditions.holds(sheet, rule.condition)
    }

    /// Whether `rule`, of sheet `sheet`, also needs its `@container`
    /// conditions tested for each element (CSS Conditional 5 §6.4).
    pub(super) fn deferred(&self, sheet: usize, rule: &Rule) -> bool {
        self.conditions.deferred(sheet, rule.condition)
    }

    /// Whether any of the sheets has a `@starting-style` rule.
    pub(super) fn has_starting_rules(&self) -> bool {
        *self.registry.facts.starting_rules.get_or_init(|| {
            self.list
                .iter()
                .any(|s| s.rules().iter().any(|r| r.starting_style))
        })
    }

    /// The highlight names the sheets' `::highlight()` rules style (CSS
    /// Custom Highlight API 1 §5.1), each once: an element's highlight
    /// styles are matched for these alone.
    pub(super) fn highlight_names(&self) -> &[std::sync::Arc<str>] {
        self.registry.facts.highlight_names.get_or_init(|| {
            #[cfg(test)]
            cost::FACT_BUILDS.with(|c| c.set(c.get() + 1));
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
        *self.registry.facts.first_rules.get_or_init(|| {
            #[cfg(test)]
            cost::FACT_BUILDS.with(|c| c.set(c.get() + 1));
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
        let build = || {
            #[cfg(test)]
            cost::FACT_BUILDS.with(|c| c.set(c.get() + 1));
            let mut defs: Vec<(u32, usize, usize, &CounterStyleDefinition)> = Vec::new();
            for (sheet, s) in self.list.iter().enumerate() {
                for (i, def) in s.counter_styles().iter().enumerate() {
                    if self.defines(sheet, def.condition) {
                        defs.push((self.layers.rank(sheet, def.layer), sheet, i, def));
                    }
                }
            }
            defs.sort_by_key(|&(rank, sheet, i, _)| (rank, sheet, i));
            let mut registry = CounterStyleRegistry::new();
            for (_, _, _, def) in defs {
                registry.define(def.name.clone(), def.rule.clone());
            }
            registry
        };
        if self.has_conditional_definitions() {
            self.conditional_counter_styles.get_or_init(build)
        } else {
            self.registry.facts.counter_styles.get_or_init(build)
        }
    }

    /// The `@keyframes` rule `name` resolves to (CSS Animations 1 §3):
    /// the last of that name — by cascade layer (unlayered last), then
    /// sheet, then source order (CSS Cascade 5 §6.4.3).
    pub(super) fn keyframes_rule(&self, name: &str) -> Option<&'a rdom_style::KeyframesRule> {
        let build = || {
            #[cfg(test)]
            cost::FACT_BUILDS.with(|c| c.set(c.get() + 1));
            let mut defs: Vec<(u32, usize, usize)> = Vec::new();
            for (sheet, s) in self.list.iter().enumerate() {
                for (i, rule) in s.keyframes().iter().enumerate() {
                    if self.defines(sheet, rule.condition) {
                        defs.push((self.layers.rank(sheet, rule.layer), sheet, i));
                    }
                }
            }
            defs.sort_unstable();
            let mut map = std::collections::HashMap::new();
            for (_, sheet, i) in defs {
                map.insert(self.list[sheet].keyframes()[i].name.clone(), (sheet, i));
            }
            map
        };
        let map = if self.has_conditional_definitions() {
            self.conditional_keyframes.get_or_init(build)
        } else {
            self.registry.facts.keyframes.get_or_init(build)
        };
        let &(sheet, i) = map.get(name)?;
        self.list.get(sheet)?.keyframes().get(i)
    }

    /// The document's preferred color scheme (CSS Color Adjust 1 §2.1).
    pub(super) fn color_scheme(&self) -> ColorScheme {
        self.media.color_scheme
    }

    /// The terminal size the viewport-percentage units resolve against.
    pub(super) fn viewport(&self) -> Viewport {
        self.media.viewport
    }

    /// The custom properties the sheets register in this run's
    /// environment (`PropertyRegistry::active`: an `@property` under a
    /// condition only while it holds).
    pub(super) fn active_registry(&self) -> &PropertyRegistry {
        &self.active
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

/// Test-only counters of the sheet scans.
#[cfg(test)]
pub(crate) mod cost {
    thread_local! {
        /// Scans of a sheet set's rules for what it holds (the first-line
        /// and highlight rules, the counter styles).
        pub(crate) static FACT_BUILDS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }
}
