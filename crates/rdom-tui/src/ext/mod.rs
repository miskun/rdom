//! `TuiExt` — presentation data attached to every Element via `Dom<TuiExt>`.
//!
//! Everything a TUI element carries beyond its tag / attrs / classes
//! lives here: inline style, pseudo-element content, sizing and box
//! model, overflow + scroll state, laid-out geometry, and the cached
//! post-cascade `ComputedStyle`. `rdom-core` never sees any of this —
//! it just holds the `TuiExt` payload behind its `Ext` generic.

mod layout_cache;
mod presentation;
#[cfg(test)]
mod tests;

pub(crate) use layout_cache::MarginChainMemo;
pub use layout_cache::{AnonymousIfc, PseudoLayout, StaticPosition};
pub use presentation::{PresentationStyle, PseudoSlot, StyleSlot};

use crate::layout::LayoutRect;
use crate::render::inline::InlineLayout;
use crate::runtime::editing::EditorState;
use crate::style::{ComputedStyle, TuiStyle};

/// `<select>` type-ahead state (see `runtime::builtins::select`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TypeaheadState {
    /// Lower-cased characters typed within the timeout window.
    pub buffer: String,
    /// When the last character was typed; `None` until the first.
    pub last: Option<std::time::Instant>,
}

impl TuiExt {
    /// The inline style, or the empty style when none is set.
    pub fn inline_style_or_empty(&self) -> &TuiStyle {
        static EMPTY: std::sync::LazyLock<TuiStyle> = std::sync::LazyLock::new(TuiStyle::default);
        self.inline_style.as_deref().unwrap_or(&EMPTY)
    }

    /// The inline style, boxed on first use. Prefer
    /// [`set_inline_style`](Self::set_inline_style) to replace it.
    pub fn inline_style_mut(&mut self) -> &mut TuiStyle {
        self.inline_style.get_or_insert_with(Default::default)
    }

    /// Replace the inline style; an empty style is stored as `None`.
    pub fn set_inline_style(&mut self, style: TuiStyle) {
        self.inline_style = (!style.is_empty()).then(|| Box::new(style));
    }
}

/// Per-Element presentation state used by rdom-tui renderers.
///
/// Grouped for readability:
/// - **style**: `inline_style` + `before_content` / `after_content`
/// - **sizing**: `width` / `height` / `min_*` / `max_*`
/// - **box model**: `direction`, `padding`, `border`, `gap`
/// - **overflow + scroll**: clipping mode + scroll offsets + content dims
/// - **geometry** (written by layout): outer `layout` + inner `content_layout`
/// - **cascade cache**: `computed` / `computed_before` / `computed_after`
/// - **dirty flags**: `style_dirty`, `layout_dirty` — read by cascade
///   and layout passes to skip unchanged subtrees
///
/// Most of these fields are written via extension traits
/// (`TuiNodeMutExt::set_width`, etc.) rather than touched directly.
///
/// `Eq` is omitted because nested `TuiStyle` / `ComputedStyle`
/// contain `f32` opacity. `PartialEq` suffices for the cascade
/// diff comparisons.
///
/// `Clone` is the element's cloning steps, not a field-for-field copy
/// (see its impl): it is what `Dom::clone_node` runs.
#[derive(Debug, Default, PartialEq)]
pub struct TuiExt {
    // ── Style (inline) ────────────────────────────────────────────────
    /// The element's inline style (the `style` attribute, and the
    /// `TuiNodeMutExt` setters). `None` is the empty style (a box
    /// whose declarations were all removed may stay `Some`): most
    /// elements carry none, so the 600-odd-byte `TuiStyle` is boxed
    /// on first write (`PERF-TUIEXT-SIZE-1`). Read it with
    /// `.as_deref()` or [`inline_style_or_empty`](Self::inline_style_or_empty),
    /// write it through
    /// [`inline_style_mut`](Self::inline_style_mut) or
    /// [`set_inline_style`](Self::set_inline_style).
    pub inline_style: Option<Box<TuiStyle>>,

    /// Fallback text for `::before` when no matching stylesheet rule
    /// supplies a `content:` value. Author-facing setters are
    /// `TuiNodeMutExt::set_before_content(...)` / `clear_before_content()`.
    /// Stylesheet `content: "…";` takes precedence when present.
    pub before_content: Option<String>,
    /// Text injected after the element's own content by `::after`.
    pub after_content: Option<String>,

    // ── Sizing / box model / overflow ────────────────────────────────
    //
    // (Removed in `EXT-LAYOUT-SETTERS-1`: width/height/min_*/max_*/
    // direction/padding/border/gap/overflow used to live here as raw
    // fields, but layout reads only `ComputedStyle`, so they were dead.
    // The `set_*` node setters now write `inline_style` instead — the
    // cascade carries it into `computed`, which is what layout reads.)
    /// Used main-axis width of a table cell, computed by
    /// [`runtime::builtins::table::size_columns`](crate::runtime::builtins::table::size_columns)
    /// from the column's author widths + content. **Layout output, not author
    /// input** (`TABLE-COLSYNC-1`): flex reads this as the cell's main size,
    /// overriding the normal `computed.width` resolution, so every cell in a
    /// column lines up — without the column-sync pass overwriting author
    /// `inline_style.width` (which would conflate intent with result). `None`
    /// for non-table cells / before the pass runs.
    pub table_used_width: Option<u16>,

    // ── Scroll ────────────────────────────────────────────────────────
    /// Horizontal scroll offset in cells. **Runtime-managed**: write it
    /// through [`TuiAccessorsMut`](crate::TuiAccessorsMut)
    /// (`set_scroll_left`, `scroll_to`, …), which clamps it, fires
    /// `scroll` and honors `scroll-behavior`. A direct write — through
    /// `App::dom_mut`, or from an input event's listener — still
    /// repaints on the `App`'s next frame (`P7-SCROLL-REPAINT-1`; the
    /// App checks offsets after such code ran, `P7G-IDLE-WALKS-1`), as
    /// does one from a timer, injected closure or `on_tick` that also
    /// left evidence of a change (a mutation, `request_redraw`;
    /// `P7G-TICK-TOUCHED-1`). It is clamped by the next layout, but
    /// fires no `scroll` event and leaves a smooth scroll in flight
    /// running.
    pub scroll_x: usize,
    /// Vertical scroll offset in cells. Runtime-managed, as
    /// [`scroll_x`](Self::scroll_x).
    pub scroll_y: usize,
    /// Scroll bookkeeping only scroll containers use — the offsets last
    /// painted and last laid out, and the smooth scroll in flight —
    /// boxed on first use (`runtime::scrollbar::state`,
    /// `P7G-FORM-STATE-BOX-1`). **Runtime-managed.**
    pub(crate) scroll_state: Option<Box<crate::runtime::scrollbar::state::ScrollState>>,
    /// Total content size (max of children's extents). Used to compute
    /// scrollbar size and thumb position.
    pub scroll_content_width: usize,
    pub scroll_content_height: usize,

    // ── Geometry (written by layout pass) ─────────────────────────────
    /// The outer rectangle this element occupies in its parent's
    /// coordinate space (after scroll). Signed so off-screen elements
    /// remain tracked for partial clipping.
    pub layout: LayoutRect,
    /// Inner rect after applying padding + border. Where children lay out.
    pub content_layout: LayoutRect,
    /// Static position of this element when it is `position: absolute
    /// | fixed` (see [`StaticPosition`]). Written by the parent's
    /// phase-1 layout; `None` until that has run. Meaningless for
    /// in-flow elements.
    pub static_position: Option<StaticPosition>,

    // ── Positioned pseudo-element layout (M5-now Stage B) ────────────
    /// Layout rect + cascaded `position` for the `::before` pseudo-
    /// element, populated by the positioned-pseudo layout pass when
    /// the cascaded `position` is non-`Static`. `None` otherwise —
    /// the inline-append paint path handles static-position pseudos
    /// (the existing default).
    ///
    /// **Hit-test divergence from CSS.** Positioned pseudo rects are
    /// NOT in the hit-test set. Clicks that land on a `::before` /
    /// `::after` rect fall through to the host element — `target`
    /// in the synthesized `MouseEvent` is always the host, never the
    /// pseudo. Web browsers route clicks to the pseudo when
    /// `pointer-events` allows. rdom 0.1.0 always falls through; the
    /// pseudo carries no `NodeId`, so there's no event target to
    /// resolve. Authors who need clickable bracket chrome should
    /// promote it to a real `<span>` instead of a pseudo.
    pub before_layout: Option<PseudoLayout>,
    /// `::after` companion of [`before_layout`](Self::before_layout).
    /// Same population rules; same hit-test divergence (clicks fall
    /// through to the host).
    pub after_layout: Option<PseudoLayout>,
    /// Aggregated cascade output: true when this element OR any
    /// descendant has a `::before` / `::after` pseudo whose cascaded
    /// `position` is non-`Static`. Written bottom-up by the cascade
    /// pass; read by `place_positioned_pseudos` (layout) and
    /// `paint_positioned_pseudos` (paint) to skip the full-tree walk
    /// in the common case where no positioned pseudos are in play.
    ///
    /// Conservative across incremental cascade: a `cascade_subtrees`
    /// call that DROPS positioned pseudos from a subtree may leave
    /// ancestors stale-`true` (extra walks; never missed paints). A
    /// call that ADDS a positioned pseudo bubbles up to ancestors so
    /// the flag never stale-`false`s.
    pub tree_has_positioned_pseudo: bool,

    /// Bottom-up flag: `true` iff *any* element in this element's
    /// subtree (including itself) has `border-collapse: collapse`.
    /// Mirrors the `tree_has_positioned_pseudo` pattern. Set during
    /// cascade so the paint joiner can short-circuit a full-tree
    /// walk when no element collapses anywhere.
    ///
    /// Conservative across incremental cascade: a subtree update
    /// that DROPS the only `collapse` element may leave ancestors
    /// stale-`true` (the joiner runs but the per-cell scan still
    /// short-circuits when no glyph is box-drawing — at worst one
    /// extra buffer walk). A subtree update that ADDS a `collapse`
    /// bubbles up so the flag never stale-`false`s.
    pub tree_has_collapse: bool,

    // ── Inline layout (populated when this is an IFC block) ───────────
    /// Line-packed layout of inline content. `Some` for elements that
    /// establish an inline formatting context; `None` otherwise. Used
    /// by paint to render each fragment at its computed position and
    /// by hit testing (Phase F) to find the inline element under a
    /// cursor click.
    pub inline_layout: Option<InlineLayout>,
    /// **Anonymous block boxes** synthesized by the block layout pass
    /// for runs of inline-level children inside a `Flow::Block`
    /// container that also has block-level children. Each entry
    /// holds its own IFC layout + rect — paint, hit-test, and
    /// selection walk this Vec alongside the singular `inline_layout`
    /// field. Empty when the container is pure flex, pure block, or
    /// a single-IFC container (which uses `inline_layout` instead).
    /// Populated by `layout_pass::block::layout_block_children` per
    /// CSS 2.1 §9.2.1.1.
    pub anonymous_blocks: Vec<AnonymousIfc>,

    // ── Cascade cache (populated by Dom::cascade) ─────────────────────
    /// Post-cascade style for this element. `None` means "no cascade run
    /// yet, or this element's ext was just created"; layout and paint
    /// must treat `None` as `ComputedStyle::initial()` by convention.
    pub computed: Option<std::rc::Rc<ComputedStyle>>,
    /// Snapshot of `computed` from the *previous* cascade pass. Used
    /// by the M3 transition engine to diff against the current
    /// `computed` and detect which animatable properties changed.
    /// `None` on the first cascade pass — no diff to perform.
    pub computed_prev: Option<std::rc::Rc<ComputedStyle>>,
    /// In-flight transition values. Sparse: only properties an
    /// active animation is currently driving have their slot
    /// populated; everything else falls back to `computed`. Paint,
    /// layout, and hit-test consult this first via the
    /// `effective_*` helpers.
    ///
    /// `None` while no transition drives any property: the box is
    /// created by the first animated write and dropped when the last
    /// override clears (`PERF-TUIEXT-SIZE-1`), so an element that never
    /// animates pays one pointer for it. Read it through
    /// [`presentation_for`](Self::presentation_for); the transition
    /// engine writes it.
    pub presentation: Option<Box<PresentationStyle>>,
    /// `::before` pseudo-element computed style. `None` if no content
    /// and no matching `::before` rules.
    pub computed_before: Option<std::rc::Rc<ComputedStyle>>,
    /// `::after` pseudo-element computed style.
    pub computed_after: Option<std::rc::Rc<ComputedStyle>>,
    /// Previous-cascade snapshots of the two pseudo-element styles, so
    /// the transition engine can diff them like `computed_prev`
    /// (`D-M3-3`).
    pub computed_before_prev: Option<std::rc::Rc<ComputedStyle>>,
    pub computed_after_prev: Option<std::rc::Rc<ComputedStyle>>,
    /// Animation overrides for `::before` / `::after` paint
    /// properties (`color`, `background-color`, `border-color`).
    /// Geometry of positioned pseudo-elements does not transition.
    /// Boxed and `None` while no transition drives the slot, like
    /// [`presentation`](Self::presentation).
    pub presentation_before: Option<Box<PresentationStyle>>,
    pub presentation_after: Option<Box<PresentationStyle>>,
    /// `::backdrop` pseudo-element computed style — populated for
    /// modal `<dialog>` elements whose stylesheet has a matching
    /// `dialog::backdrop` rule. The paint pass overlays the
    /// backdrop across the viewport after normal paint and before
    /// re-painting the dialog. See Polish #8.
    pub computed_backdrop: Option<std::rc::Rc<ComputedStyle>>,
    /// `::selection` pseudo-element computed style — populated
    /// for elements whose stylesheet has a matching `::selection`
    /// rule. The selection-overlay paint walks up from each
    /// selected text fragment to the nearest ancestor with this
    /// style and applies its bg/fg/modifier. The UA stylesheet
    /// ships a default `*::selection { background-color: #394B7E;
    /// color: white }` rule, so every selectable always has a
    /// computed selection style unless an author explicitly
    /// overrides it back to `initial`.
    pub computed_selection: Option<std::rc::Rc<ComputedStyle>>,
    /// `::scrollbar` pseudo-element computed style — populated
    /// for elements with non-`Visible`/`Hidden` overflow on at
    /// least one axis. Drives the scrollbar track paint: `bg`
    /// fills every track cell, `fg` colors the `content` glyph
    /// (default `" "` from UA — a colored gutter via `bg` is the
    /// modern look). Authors override via
    /// `selector::scrollbar { bg: …; content: "▒"; }` to retheme.
    pub computed_scrollbar: Option<std::rc::Rc<ComputedStyle>>,
    /// `::scrollbar-thumb` computed style for the **vertical** bar:
    /// axis-neutral `::scrollbar-thumb` rules with
    /// `::scrollbar-thumb:vertical` layered on top (default content
    /// `┃`). Authors override via `selector::scrollbar-thumb { … }`
    /// or the axis form.
    pub computed_scrollbar_thumb_vertical: Option<std::rc::Rc<ComputedStyle>>,
    /// Same for the **horizontal** bar (`::scrollbar-thumb` +
    /// `::scrollbar-thumb:horizontal`, default content `━`).
    pub computed_scrollbar_thumb_horizontal: Option<std::rc::Rc<ComputedStyle>>,
    /// The rules each box of this element matched in its last cascade,
    /// under that cascade's sheets — reused by the restyle a registered
    /// custom property's transition runs each frame
    /// (`style::cascade::restyle_vars`), which changes no selector's
    /// result.
    pub(crate) matched: Option<std::rc::Rc<crate::style::cascade::MatchedRules>>,

    // ── Dirty flags (read by cascade + layout, set by mutation hooks) ─
    /// This element needs re-cascade next frame. Set by the
    /// `DirtyTracker` mutation observer whenever an attribute / class
    /// / tree-shape change could affect which rules match.
    pub style_dirty: bool,
    /// This element needs re-layout. Set by the cascade when a
    /// layout-affecting computed value changes. Separate from
    /// `style_dirty` so a pure color change skips re-layout entirely.
    pub layout_dirty: bool,
    /// Margin-collapse chain results an ancestor's placement computed
    /// for this block during the current layout pass (see
    /// [`MarginChainMemo`]). Consumed when this block is placed and
    /// cleared when it is laid out, so nothing outlives the pass —
    /// which is why it is not part of the public layout output.
    pub(crate) margin_chain: Option<MarginChainMemo>,
    /// `defaultValue` of a text control or `<input type=range>`: the
    /// value it was authored / seeded with, captured before its first
    /// change. `None` until a control has been seeded or changed
    /// (`FORM-DEFAULTS-1`); `<form>` reset restores it.
    ///
    /// **Runtime-managed**: the builtins capture it and `<form>` reset
    /// reads it. Read it through `TuiAccessors::default_value` and set
    /// it through `TuiAccessorsMut::set_default_value`, which also
    /// cover the controls whose default is the `value` attribute.
    pub default_value: Option<String>,
    /// `defaultChecked` of a checkbox / radio, captured before its first
    /// flip; `<form>` reset restores it. **Runtime-managed** — use
    /// `TuiAccessors::default_checked` /
    /// `TuiAccessorsMut::set_default_checked`.
    pub default_checked: Option<bool>,
    /// `defaultSelected` of an `<option>`: its `selected` attribute as
    /// authored, captured (for every option of the select) before the
    /// select's first change, and for one option before the
    /// selectedness setting algorithm flips it; `<form>` reset restores
    /// it. `None` until then — the attribute is still the authored
    /// state. **Runtime-managed** — use `TuiAccessors::default_selected`
    /// / `TuiAccessorsMut::set_default_selected`.
    pub default_selected: Option<bool>,
    /// A caret move or edit asked this inline-flow container to reveal
    /// the caret; the runtime re-runs the reveal after the next layout,
    /// when the container's extent is current
    /// (`CARET-REVEAL-STALE-LAYOUT-1`).
    pub(crate) caret_reveal_pending: bool,
    /// The caret blink is in its off phase for this editing host, so the
    /// caret painter skips it. **Runtime-managed** by the App's caret
    /// blink (`runtime::caret_blink`); `false` (steady caret) outside
    /// an `App`.
    pub(crate) caret_blink_off: bool,

    // ── Editing state (Phase B) ──────────────────────────────────────
    /// Per-editable state (undo/redo history, coalescing metadata).
    /// `None` until the element first receives an edit; `Some(Box<_>)`
    /// thereafter. Boxed so `TuiExt` stays small for non-editable
    /// elements (the common case).
    pub editor_state: Option<Box<EditorState>>,
    /// `<dialog>` only: the element that had focus when `showModal()`
    /// ran, so `close()` can return focus to it (HTML §4.11.4 "dialog
    /// focusing steps" / "previously focused element").
    pub dialog_return_focus: Option<rdom_core::NodeId>,
    /// `<select>` only: the type-ahead buffer and the time of its last
    /// keystroke. Per node — two selects never share a buffer — and on
    /// the scheduler clock under an `App`.
    pub typeahead: Option<Box<TypeaheadState>>,

    // ── Canvas paint callback (Phase C.9) ────────────────────────────
    /// Raw-buffer paint hook for `<canvas>` elements. When `Some`,
    /// the paint pass invokes the callback with a bounded
    /// `RenderContext` instead of running the normal inline paint
    /// path. `None` for every other element (and for `<canvas>`
    /// without a registered callback — they render their fallback
    /// DOM children per HTML's unsupported-canvas behavior).
    ///
    /// See `runtime::builtins::canvas` for the registration helpers.
    pub canvas_paint: Option<crate::runtime::builtins::canvas::CanvasPaint>,

    // ── Form state (P7-VALIDATION-1, P7G-SUBMIT-REENTRY-1) ───────────
    /// Custom validity, the user-edited flag, a `<form>`'s "firing
    /// submission events" flag and the compiled `pattern`, boxed on
    /// first use (`P7G-FORM-STATE-BOX-1`). **Runtime-managed** — use
    /// `TuiAccessorsMut::set_custom_validity` and the form builtins.
    pub(crate) form_state: crate::runtime::builtins::form_state::FormControlSlot,
}

impl TuiExt {
    pub fn new() -> Self {
        Self::default()
    }
}

/// The element's **cloning steps** (DOM §4.5 "clone a node", step
/// "run any cloning steps"; `P7G-CLONE-RESET-1`) — `Dom::clone_node` is
/// the only caller that needs `TuiExt: Clone`. A clone is a new
/// element: it keeps what the author gave the original and starts every
/// piece of per-activation runtime state fresh.
///
/// **Copied:** the inline style (the `style` attribute's cache — the
/// attribute itself is copied by rdom-core), `before_content` /
/// `after_content`, and the captured `default_value` /
/// `default_checked` / `default_selected` (rdom keeps HTML's
/// `defaultValue` / `defaultChecked` / `defaultSelected` here while the
/// live `value` / `checked` / `selected` attributes, which HTML §4.10.5's
/// input cloning steps propagate, are copied with the other attributes;
/// see DIVERGENCES).
///
/// **Reset** (HTML copies none of it): the custom validity message, the
/// "last changed by a user edit" flag and a `<form>`'s "firing
/// submission events" flag; scroll offsets and the smooth scroll in
/// flight (a new element is unscrolled); caret blink and reveal state;
/// the undo history; a `<dialog>`'s return focus and a `<select>`'s
/// type-ahead buffer; the `<canvas>` paint callback (a cloned canvas
/// starts with a blank bitmap; listeners are not cloned either); the
/// cascade, transition and layout caches and the dirty flags, which a
/// new element gets from its first cascade and layout once inserted.
///
/// A field added to `TuiExt` is therefore reset by default; copying it
/// is a decision made here.
impl Clone for TuiExt {
    fn clone(&self) -> Self {
        Self {
            inline_style: self.inline_style.clone(),
            before_content: self.before_content.clone(),
            after_content: self.after_content.clone(),
            default_value: self.default_value.clone(),
            default_checked: self.default_checked,
            default_selected: self.default_selected,
            ..Self::default()
        }
    }
}
