//! The cascade engine.
//!
//! `Dom::cascade(&stylesheet)` walks the tree top-down, computes a
//! `ComputedStyle` for every element (and any matching `::before` /
//! `::after` pseudo-elements), and writes the result back to each
//! `TuiExt`. Dirty flags get cleared; `layout_dirty` gets set whenever
//! a layout-affecting property value changes.
//!
//! ## Algorithm (per element)
//!
//! 1. Start from `ComputedStyle::initial()`.
//! 2. Inherit the inherited properties (`rdom_style::property_dispatch::inherits`) from the parent
//!    (`inherit`).
//! 3. Collect matching rules via `rdom_core::Dom::matches_list`.
//! 4. Sort candidates by (specificity, scope proximity, sheet,
//!    source_idx) — nearer `@scope` roots win after specificity
//!    (Cascade 6 §6.1). Ascending = late-wins.
//! 5. Apply declarations in origin + importance order (`ladder`):
//!    1. UA normal, Author normal (per layer), Inline normal,
//!    2. Author important (layers reversed), Inline important, UA
//!       important — the `style` attribute beats rules of its
//!       origin at both importances (Cascade 4 §6.1).
//!
//!    Within each ladder step, sort by (specificity, source_idx).
//! 6. Resolve the CSS-wide keywords per property (`apply`): `inherit`
//!    / `initial`, and `revert` from the ladder's rollback state.
//! 7. Resolve `content` (`content`) — pseudo-element body.
//! 8. Finalize the `border-*-color`s (an undeclared side is the final `fg`).
//! 9. Write to `TuiExt.computed` and flip `style_dirty=false`; if
//!    any layout-affecting property's new value differs, set
//!    `layout_dirty=true` (`inherit::layout_differs`).
//!
//! Pseudo-elements use the same algorithm but start from the host's
//! computed style (not the parent's). They contribute a concrete
//! `content: Option<String>` resolved from `TuiStyle.content` plus any
//! fallback `before_content` / `after_content` set directly on
//! `TuiExt`.
//!
//! ## Module layout
//!
//! - `walk` — `cascade_subtree`. The tree recursion lives here.
//! - `element` — `compute_element_style`, one element's ladder and its
//!   computed-value fix-ups.
//! - `subtrees` — partial cascades over a set of roots, and the ordered
//!   walk that keeps counters exact (`counters`) across them.
//! - `matching` — the rules matching one element or pseudo-element, in
//!   cascade order; its buffers are reused for a whole pass.
//! - `pseudo` — `compute_pseudo_style` (`::before`, `::after`, …).
//! - `ladder` — the cascade ladder (`Plan` / `Step`) and the memoized
//!   rollback states `revert` / `revert-layer` read.
//! - `sheets` — the sheets of one run and their shared cascade-layer
//!   order.
//! - `scope` — `@scope` matching and scope proximity (Cascade 6).
//! - `custom` — custom properties through the ladder.
//! - `apply` — per-property applicators; `colors` / `decoration` the
//!   color and background / border ones.
//! - `inherit` — `inherit_inheritable_from`, `layout_differs`.
//! - `content` — pseudo-element `content` resolution.
//!
//! ## Inheritance model
//!
//! Which properties inherit is a fact about the property, declared once
//! in `rdom_style::property_dispatch::inherits` (it also decides what
//! `unset` means). `inherit::inherit_inheritable_from` copies exactly
//! that set from parent to child, and a cascade test probes every
//! property against the table so the two cannot drift.

mod apply;
mod blockify;
pub(crate) use blockify::children_are_items;
pub(crate) use early_pseudos::is_block_container;
mod content;
mod counters;
mod custom;
mod hints;
mod inherit;
mod keyframes;
mod ladder;
mod line_clamp;
mod matching;
mod media;
mod paired;
mod pseudo;
mod quotes;
mod registered;
pub(crate) use inherit::anonymous_box_style;
pub(crate) use keyframes::{keyframe_style, keyframes_rule};
pub(crate) use matching::MatchedRules;
#[cfg(test)]
pub(crate) use matching::probe as match_probe;
pub(crate) use media::{
    document_media, document_media_preferences, must_restyle, set_document_media_preferences,
};
pub(crate) use registered::PropertyRegistry;
#[cfg(test)]
pub(crate) use registered::probe as registry_probe;
pub(crate) use scheme::{document_color_scheme, set_document_color_scheme};
pub(crate) use starting::starting_style;
pub(crate) use viewport::{document_viewport, set_document_viewport};
mod colors;
pub(crate) mod conditions;
pub(crate) mod container;
mod decoration;
pub(crate) mod details;
mod early_pseudos;
mod element;
mod field_sizing;
mod finish;
mod font;
mod root_vars;
mod scheme;
mod scope;
mod sheets;
mod starting;
mod subtrees;
mod text;
mod text_decoration;
mod viewport;
mod walk;

#[cfg(test)]
mod apply_tests;
#[cfg(test)]
mod color_tests;
#[cfg(test)]
mod cost_tests;
#[cfg(test)]
mod counter_tests;
#[cfg(test)]
mod css_wide_tests;
#[cfg(test)]
mod layer_tests;
#[cfg(test)]
mod nesting_tests;
#[cfg(test)]
mod property_tests;
#[cfg(test)]
mod scope_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod var_tests;

use std::rc::Rc;

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::style::{ComputedStyle, Stylesheet};
use rdom_style::calc::Viewport;

// ─── Public entry point ─────────────────────────────────────────────

/// Extension trait adding the cascade methods to `Dom<TuiExt>`.
/// Lives in rdom-tui so `Dom` in rdom-core stays style-agnostic. Users
/// pull it in with `use rdom_tui::CascadeExt;` (or via
/// `use rdom_tui::*;`).
///
/// Each method comes in two forms: the single-`Stylesheet` form for
/// ergonomic use in tests and the rare app with one sheet, and the
/// `&[Stylesheet]` form that the runtime uses when an `App` has
/// multiple sheets registered (`push_stylesheet` / `set_stylesheet` /
/// construction). Within the slice, later sheets win same-specificity
/// contests — push order is the tiebreaker, matching `Document.styleSheets`
/// ordering on the web — and the sheets share one cascade-layer order
/// (`@layer`, CSS Cascade 5 §6.4): a layer name is one layer across
/// them, placed by its first declaration in slice order. The single-sheet form is a thin wrapper around
/// the slice form with a one-element slice.
///
/// The trait is sealed: only `Dom<TuiExt>` implements it, so a new
/// document-level setting (like the viewport or the color scheme) can
/// join it without breaking anyone.
///
/// ```compile_fail
/// use rdom_tui::{CascadeExt, ColorScheme, NodeId, Stylesheet, Viewport};
/// struct Mine;
/// impl CascadeExt for Mine {
///     fn cascade(&mut self, _: &Stylesheet) {}
///     fn cascade_all(&mut self, _: &[&Stylesheet]) {}
///     fn cascade_subtrees(&mut self, _: &Stylesheet, _: &[NodeId]) {}
///     fn cascade_subtrees_all(&mut self, _: &[&Stylesheet], _: &[NodeId]) {}
///     fn set_viewport(&mut self, _: Viewport) {}
///     fn viewport(&self) -> Viewport { Viewport::new(0, 0) }
///     fn set_color_scheme(&mut self, _: ColorScheme) {}
///     fn color_scheme(&self) -> ColorScheme { ColorScheme::Dark }
///     fn set_media_preferences(&mut self, _: rdom_tui::MediaPreferences) {}
///     fn media_preferences(&self) -> rdom_tui::MediaPreferences { Default::default() }
/// }
/// ```
pub trait CascadeExt: crate::sealed::Sealed {
    /// Cascade the whole document against `stylesheet`. Writes
    /// `ComputedStyle` entries to every element's `TuiExt`, clears
    /// `style_dirty`, sets `layout_dirty` on elements whose
    /// layout-affecting property values changed. Use for initial
    /// paint or after a stylesheet swap.
    fn cascade(&mut self, stylesheet: &Stylesheet);

    /// Multi-sheet variant of [`Self::cascade`]. Rules are merged
    /// across all sheets; later sheets win same-specificity contests.
    /// Custom-property (`var()`) definitions are merged with
    /// later-wins semantics per var name.
    fn cascade_all(&mut self, stylesheets: &[&Stylesheet]);

    /// Cascade only the subtrees rooted at `roots`. Each root's
    /// parent is consulted for inheritance (so a root's computed fg
    /// still inherits correctly from its ancestor chain). Empty list
    /// = no-op.
    ///
    /// Use after incremental mutations: pair with `DirtyTracker` to
    /// get the list of roots that actually need re-cascade. The
    /// resulting performance scales with the size of changed
    /// subtrees, not the whole tree.
    fn cascade_subtrees(&mut self, stylesheet: &Stylesheet, roots: &[NodeId]);

    /// Multi-sheet variant of [`Self::cascade_subtrees`].
    fn cascade_subtrees_all(&mut self, stylesheets: &[&Stylesheet], roots: &[NodeId]);

    /// Set the viewport the document is presented in: the size the
    /// viewport-percentage units (`vw`, `vh`, `vmin`, …) resolve
    /// against (CSS Values 4 §6.1.2), for every cascade form. The `App`
    /// sets its terminal's size each frame, and
    /// [`LayoutExt::layout_dom`](crate::LayoutExt::layout_dom) records
    /// its area; a headless document is 0 × 0 until one of them runs.
    /// A new size does not re-cascade: cascade the whole tree again
    /// (the `App` does).
    fn set_viewport(&mut self, viewport: Viewport);

    /// The viewport the document's style resolves against
    /// ([`Self::set_viewport`]).
    fn viewport(&self) -> Viewport;

    /// Set the document's preferred color scheme (CSS Color Adjust 1
    /// §2.1): what an element with `color-scheme: normal` uses, and so
    /// what `light-dark()` picks by. The `App` sets it from the
    /// terminal's background (or `App::with_color_scheme`); dark until
    /// set. A new scheme does not re-cascade: cascade the whole tree
    /// again (`App::set_color_scheme` does).
    fn set_color_scheme(&mut self, scheme: rdom_style::color::ColorScheme);

    /// The document's preferred color scheme
    /// ([`Self::set_color_scheme`]).
    fn color_scheme(&self) -> rdom_style::color::ColorScheme;

    /// Set the preferences the document's media queries read beyond the
    /// viewport and the color scheme (Media Queries 5 §12):
    /// `prefers-reduced-motion`, `prefers-contrast`, the pointer, … — a
    /// terminal's defaults until set. The `App` sets them
    /// (`App::with_media_preferences`). A change does not re-cascade:
    /// cascade the whole tree again (`App::set_media_preferences` does
    /// when a query flips).
    fn set_media_preferences(&mut self, preferences: rdom_style::conditional::MediaPreferences);

    /// The preferences the document's media queries read
    /// ([`Self::set_media_preferences`]).
    fn media_preferences(&self) -> rdom_style::conditional::MediaPreferences;
}

impl CascadeExt for Dom<TuiExt> {
    fn cascade(&mut self, stylesheet: &Stylesheet) {
        self.cascade_all(&[stylesheet]);
    }

    fn cascade_all(&mut self, stylesheets: &[&Stylesheet]) {
        cascade_all_with(self, stylesheets, None);
        // Layout re-cascades the query containers' subtrees with these.
        container::remember_inputs(self, stylesheets);
        // No transition runs outside an `App`: an element waiting to leave
        // the top layer leaves at this style update (CSS Position 4 §3.3).
        crate::runtime::top_layer::finish_removals(self);
    }

    fn cascade_subtrees(&mut self, stylesheet: &Stylesheet, roots: &[NodeId]) {
        self.cascade_subtrees_all(&[stylesheet], roots);
    }

    fn cascade_subtrees_all(&mut self, stylesheets: &[&Stylesheet], roots: &[NodeId]) {
        cascade_subtrees_all_with(self, stylesheets, None, roots);
        container::remember_inputs(self, stylesheets);
        crate::runtime::top_layer::finish_removals(self);
    }

    fn set_viewport(&mut self, viewport: Viewport) {
        set_document_viewport(self, viewport);
    }

    fn viewport(&self) -> Viewport {
        document_viewport(self)
    }

    fn set_color_scheme(&mut self, scheme: rdom_style::color::ColorScheme) {
        set_document_color_scheme(self, scheme);
    }

    fn color_scheme(&self) -> rdom_style::color::ColorScheme {
        document_color_scheme(self)
    }

    fn set_media_preferences(&mut self, preferences: rdom_style::conditional::MediaPreferences) {
        set_document_media_preferences(self, preferences);
    }

    fn media_preferences(&self) -> rdom_style::conditional::MediaPreferences {
        document_media_preferences(self)
    }
}

/// [`CascadeExt::cascade_all`] with the sheets' registrations already
/// built — the `App` keeps one per stylesheet set; `None`: the one the
/// document keeps for the last sheet set it was cascaded with
/// (`registered::document_registry`).
pub(crate) fn cascade_all_with(
    dom: &mut Dom<TuiExt>,
    stylesheets: &[&Stylesheet],
    registry: Option<Rc<PropertyRegistry>>,
) {
    let registry = registry.unwrap_or_else(|| registered::document_registry(dom, stylesheets));
    let reads = media::begin(dom);
    media::begin_tree(dom);
    container::begin(dom);
    container::begin_tree(dom);
    crate::style::content_visibility::begin(dom);
    let sheets = walk::Sheets::new(stylesheets, registry.clone(), media::document_media(dom));
    note_first_rules(dom, &sheets);
    details::reclaim_content_boxes(dom);
    let merged_vars = walk::merge_root_vars(dom, &sheets);
    let root = dom.root();
    // The root's parent carries the sheet-level (`define_var` /
    // `:root`) variables; every element then inherits its parent's
    // map and layers its own declarations on top.
    let mut parent = ComputedStyle::initial();
    parent.vars = merged_vars.clone();
    // Full-tree cascade: `tree_has_positioned_pseudo` flags get
    // written authoritatively, top-to-bottom. No bubble-up needed
    // because the walk visits every ancestor.
    let mut counters = walk::CounterState::default();
    let mut scratch = walk::Scratch::default();
    let _ = walk::cascade_subtree(
        dom,
        &sheets,
        root,
        &parent,
        &mut counters,
        &mut scratch,
        walk::Mode::Cascade,
    );
    scratch.flag_has_anchors(dom);
    media::finish(dom, reads, &sheets);
    // A reversed counter's initial value read the boxes after it as last
    // cascaded (CSS Lists 3 §4.2): re-cascade from those it moved.
    let stale = counters.stale_reversed(dom);
    if !stale.is_empty() {
        subtrees::subtrees(
            dom,
            stylesheets,
            Some(registry),
            &stale,
            walk::Mode::Cascade,
        );
    }
}

/// Record on the document whether `sheets` style `::first-line` or
/// `::first-letter`, which layout asks before looking for a first
/// formatted line (`render::inline::first_line::hosts`).
fn note_first_rules(dom: &mut Dom<TuiExt>, sheets: &walk::Sheets<'_>) {
    let (line, letter) = sheets.styles_first();
    crate::style::doc_flags::set_first_rules(dom, line || letter);
}

/// [`CascadeExt::cascade_subtrees_all`] with the sheets' registrations
/// already built (`None`: the document's, as for [`cascade_all_with`]).
pub(crate) fn cascade_subtrees_all_with(
    dom: &mut Dom<TuiExt>,
    stylesheets: &[&Stylesheet],
    registry: Option<Rc<PropertyRegistry>>,
    roots: &[NodeId],
) -> Vec<NodeId> {
    subtrees::subtrees(dom, stylesheets, registry, roots, walk::Mode::Cascade)
}

/// Restyle the subtrees at `roots` after a change no selector can see —
/// a registered custom property's animated value moving
/// (`runtime::animation`, CSS Properties and Values 1 §6.2): each
/// element's recorded matches are reused (`walk::Mode::Restyle`), and
/// an element whose style comes out unchanged keeps its subtree. A root
/// whose counter ops move (`counter-increment: c var(--step)`) moves the
/// counters after it, and the elements after it that read one are
/// restyled too (`subtrees`). Returns the roots of every subtree it
/// restyled.
pub(crate) fn restyle_vars(
    dom: &mut Dom<TuiExt>,
    stylesheets: &[&Stylesheet],
    registry: Rc<PropertyRegistry>,
    roots: &[NodeId],
) -> Vec<NodeId> {
    subtrees::subtrees(dom, stylesheets, Some(registry), roots, walk::Mode::Restyle)
}

// ─── Small helper re-exported for test support ──────────────────────

/// Quick probe: the computed style of `id`, or `initial()` if none
/// (pre-cascade, or non-element). Useful for tests.
pub fn computed_of(dom: &Dom<TuiExt>, id: NodeId) -> ComputedStyle {
    dom.node(id)
        .ext()
        .and_then(|e| e.computed.as_deref().cloned())
        .unwrap_or_else(ComputedStyle::initial)
}
