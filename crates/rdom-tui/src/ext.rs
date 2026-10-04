//! `TuiExt` — presentation data attached to every Element via `Dom<TuiExt>`.
//!
//! Everything a TUI element carries beyond its tag / attrs / classes
//! lives here: inline style, pseudo-element content, sizing and box
//! model, overflow + scroll state, laid-out geometry, and the cached
//! post-cascade `ComputedStyle`. `rdom-core` never sees any of this —
//! it just holds the `TuiExt` payload behind its `Ext` generic.

use crate::layout::{LayoutRect, Length, Padding, Position, Size, ZIndex};
use crate::render::inline::InlineLayout;
use crate::runtime::editing::EditorState;
use crate::style::{Color, ComputedStyle, TuiStyle};

/// Layout state for a positioned `::before` / `::after` pseudo-
/// element. Carries the rect (where the pseudo paints) plus the
/// cascaded `position` (so paint can route static pseudos through
/// the inline-append path and non-static pseudos through the
/// positioned-pseudo paint pass). Populated by the layout pass's
/// `place_positioned_pseudos` phase.
///
/// Static-position pseudos (the default) do NOT populate this —
/// they paint inline via the inline-content path. Only
/// `Position::Relative | Absolute | Fixed` produces a slot here.
///
/// Consumers reading this for debug snapshots or hit-test work
/// should note the divergence on `TuiExt::before_layout` /
/// `after_layout` — positioned pseudo rects do not participate
/// in hit-testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PseudoLayout {
    pub rect: LayoutRect,
    pub position: Position,
}

/// The **static position** of an out-of-flow positioned element
/// (CSS 2.1 §10.3.7 / §10.6.4): where its top-left corner would be
/// if it were `position: static`, in the same coordinate space as
/// [`TuiExt::layout`]. Phase-1 layout records it at the point in the
/// parent's flow where the element's hypothetical box would have
/// gone; phase-2 placement reads it for every axis whose two insets
/// are both `auto`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticPosition {
    pub x: i32,
    pub y: i32,
}

/// Memoized CSS 2.1 §8.3.1 outer-margin chains of one block, valid for
/// one containing-block width and one layout pass
/// (`BFC1-PERF-MARGIN-CHAIN-1`). Placing a block walks its first- /
/// last-child collapse chain; without the memo every level of a deep
/// chain re-walked the levels below it when its own turn came. Each
/// accumulator is `(largest positive margin, most negative margin)`.
///
/// Invariant: a chain walk only visits in-flow block-level children of
/// a block container that does not establish a new formatting context
/// (it stops at inline content, at a BFC and at padding / borders), and
/// every such child is laid out by `layout_node` later in the same
/// pass, which clears its entry — so no entry outlives the pass
/// (checked in debug builds at the end of `layout_dom`). An entry is
/// used only for the containing-block width it was computed against;
/// a width that differs (a scrollbar gutter, a border-collapse inset)
/// makes the walk recompute, never misread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MarginChainMemo {
    /// The width the chain's percentages were resolved against.
    pub containing_block_width: u16,
    /// The chain surfacing at the block's outer top edge.
    pub outer_top: Option<(i16, i16)>,
    /// The chain surfacing at the block's outer bottom edge.
    pub outer_bottom: Option<(i16, i16)>,
}

/// `<select>` type-ahead state (see `runtime::builtins::select`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TypeaheadState {
    /// Lower-cased characters typed within the timeout window.
    pub buffer: String,
    /// When the last character was typed; `None` until the first.
    pub last: Option<std::time::Instant>,
}

/// One synthesized **anonymous block box** wrapping a run of
/// inline-level children inside a block container. Per CSS 2.1
/// §9.2.1.1, when a block-flow container has mixed block + inline
/// children, the inline runs are wrapped in anonymous boxes that
/// each establish their own IFC.
///
/// Anonymous boxes have no `NodeId` (they're layout-pass ephemera
/// allocated per cascade). Their `inline_layout` carries text
/// fragments owned by real source nodes; hit-test and selection
/// resolve through those owners. `child_range` records the
/// document-order indices (within the parent's full list of child
/// nodes) the anon box wraps. The host's static `::before` /
/// `::after` are packed into the first / last box's `inline_layout`
/// (as `LineBox::generated`); a pseudo whose host starts / ends with a
/// block-level child gets a box of its own, with an empty
/// `child_range`.
#[derive(Debug, Clone, PartialEq)]
pub struct AnonymousIfc {
    /// Where this anonymous box sits in its parent's content area.
    /// Width = parent content width; height = inline_layout.height().
    pub rect: LayoutRect,
    /// IFC packing of the wrapped inline run.
    pub inline_layout: InlineLayout,
    /// Indices into the parent's `child_nodes()` iteration covered
    /// by this anonymous box, as `[start, end)`. Hit-test and
    /// selection use this to map a fragment to its surrounding DOM
    /// neighbors.
    pub child_range: (usize, usize),
}

/// Sparse override on top of `ComputedStyle`. Only populated for
/// properties that an active transition is currently driving.
/// Paint, layout, and hit-test read these slots before falling
/// back to `ComputedStyle` — see `effective_*` helpers below.
///
/// M3 covers the animatable subset. Discrete properties (display,
/// position, content, etc.) toggle in `ComputedStyle` directly
/// at midpoint and are not covered here.
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct PresentationStyle {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub border_fg: Option<Color>,
    pub width: Option<Size>,
    pub height: Option<Size>,
    pub padding: Option<Padding>,
    pub gap: Option<u16>,
    pub top: Option<Length>,
    pub right: Option<Length>,
    pub bottom: Option<Length>,
    pub left: Option<Length>,
    pub z_index: Option<ZIndex>,
    /// The running transitions of registered custom properties (name
    /// without dashes → animated value). Not read by paint: the cascade
    /// applies them on top of the cascaded values
    /// (`ComputedStyle::animated_vars`) so `var()` consumers follow.
    pub custom_properties: Option<std::collections::HashMap<String, rdom_style::CustomValue>>,
}

/// Which style a transition animates: the element itself or one of
/// its generated pseudo-elements (CSS Transitions 1 §5:
/// `TransitionEvent.pseudoElement`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum StyleSlot {
    #[default]
    Host,
    Before,
    After,
}

impl StyleSlot {
    /// The `TransitionEvent.pseudoElement` value.
    pub fn pseudo_element(self) -> Option<&'static str> {
        match self {
            StyleSlot::Host => None,
            StyleSlot::Before => Some("::before"),
            StyleSlot::After => Some("::after"),
        }
    }
}

/// Which generated pseudo-element a piece of generated content belongs
/// to: a [`StyleSlot`] that can never be [`StyleSlot::Host`]. Laid-out
/// generated content (`GeneratedFragment::slot`) carries one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PseudoSlot {
    Before,
    After,
}

impl From<PseudoSlot> for StyleSlot {
    fn from(slot: PseudoSlot) -> Self {
        match slot {
            PseudoSlot::Before => StyleSlot::Before,
            PseudoSlot::After => StyleSlot::After,
        }
    }
}

impl TuiExt {
    /// The animation overrides for `slot`; `None` while no transition
    /// drives it.
    pub fn presentation_for(&self, slot: StyleSlot) -> Option<&PresentationStyle> {
        match slot {
            StyleSlot::Host => self.presentation.as_deref(),
            StyleSlot::Before => self.presentation_before.as_deref(),
            StyleSlot::After => self.presentation_after.as_deref(),
        }
    }

    /// The animation overrides for `slot`, boxed on first use.
    /// Transition-engine plumbing (`runtime::animation`).
    pub(crate) fn presentation_for_mut(&mut self, slot: StyleSlot) -> &mut PresentationStyle {
        self.presentation_slot(slot)
            .get_or_insert_with(Default::default)
    }

    /// Drop `slot`'s override box once no property is overridden, so an
    /// element whose transitions finished is back to one `None`.
    /// Transition-engine plumbing (`runtime::animation`).
    pub(crate) fn release_empty_presentation(&mut self, slot: StyleSlot) {
        let boxed = self.presentation_slot(slot);
        if boxed.as_deref().is_some_and(PresentationStyle::is_empty) {
            *boxed = None;
        }
    }

    fn presentation_slot(&mut self, slot: StyleSlot) -> &mut Option<Box<PresentationStyle>> {
        match slot {
            StyleSlot::Host => &mut self.presentation,
            StyleSlot::Before => &mut self.presentation_before,
            StyleSlot::After => &mut self.presentation_after,
        }
    }

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

impl PresentationStyle {
    /// True when no animation is currently driving any property.
    /// The hot path uses this to skip the override read.
    pub fn is_empty(&self) -> bool {
        self.custom_properties.is_none()
            && self.fg.is_none()
            && self.bg.is_none()
            && self.border_fg.is_none()
            && self.width.is_none()
            && self.height.is_none()
            && self.padding.is_none()
            && self.gap.is_none()
            && self.top.is_none()
            && self.right.is_none()
            && self.bottom.is_none()
            && self.left.is_none()
            && self.z_index.is_none()
    }

    /// Drop every override. Called by the engine when an
    /// animation reaches its end value (so paint sees the
    /// committed `ComputedStyle` from the next cascade onward).
    pub fn clear(&mut self) {
        *self = PresentationStyle::default();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::Color;

    #[test]
    fn defaults_are_sensible() {
        let ext = TuiExt::new();
        // Geometry now lives in `inline_style` (empty by default) — the
        // raw `ext` geometry fields were removed in EXT-LAYOUT-SETTERS-1.
        assert!(ext.inline_style.is_none());
        assert_eq!(ext.scroll_x, 0);
        assert_eq!(ext.scroll_y, 0);
        assert!(ext.inline_style.is_none());
        assert!(ext.before_content.is_none());
        assert!(ext.after_content.is_none());
        assert_eq!(ext.layout, LayoutRect::default());
        // Cascade cache starts empty; cascade populates it on first pass.
        assert!(ext.computed.is_none());
        assert!(ext.computed_before.is_none());
        assert!(ext.computed_after.is_none());
        // Dirty flags default false — a brand-new `TuiExt` has no cascade
        // work yet; the subtree root gets marked dirty when first attached.
        assert!(!ext.style_dirty);
        assert!(!ext.layout_dirty);
    }

    /// `P7G-CLONE-RESET-1`: cloning is the element's cloning steps —
    /// the author inputs and the captured defaults are copied, the
    /// per-activation runtime state starts fresh.
    #[test]
    fn clone_copies_author_inputs_and_resets_runtime_state() {
        let mut dom: crate::TuiDom = crate::TuiDom::new();
        let opener = dom.create_element("button");
        let mut ext = TuiExt {
            inline_style: Some(Box::new(
                TuiStyle::new()
                    .fg(Color::Rgb(255, 0, 0))
                    .width(Size::Fixed(80))
                    .padding(Padding::all(2)),
            )),
            before_content: Some("▾ ".into()),
            after_content: Some(" ←".into()),
            default_value: Some("hi".into()),
            default_checked: Some(true),
            default_selected: Some(false),
            scroll_x: 3,
            scroll_y: 9,
            scroll_state: Some(Box::default()),
            caret_reveal_pending: true,
            caret_blink_off: true,
            style_dirty: true,
            dialog_return_focus: Some(opener),
            ..Default::default()
        };
        {
            let form = ext.form_state.get_mut();
            form.custom_validity = "taken".into();
            form.value_user_edited = true;
            form.firing_submission_events = true;
        }
        let cloned = ext.clone();
        assert_eq!(cloned.inline_style, ext.inline_style);
        assert_eq!(cloned.before_content, ext.before_content);
        assert_eq!(cloned.after_content, ext.after_content);
        assert_eq!(cloned.default_value, ext.default_value);
        assert_eq!(cloned.default_checked, ext.default_checked);
        assert_eq!(cloned.default_selected, ext.default_selected);
        let fresh = TuiExt {
            inline_style: ext.inline_style.clone(),
            before_content: ext.before_content.clone(),
            after_content: ext.after_content.clone(),
            default_value: ext.default_value.clone(),
            default_checked: ext.default_checked,
            default_selected: ext.default_selected,
            ..Default::default()
        };
        assert_eq!(cloned, fresh, "everything else is at its default");
        assert!(cloned.form_state.get().is_none(), "no form state carried");
        assert!(cloned.scroll_state.is_none(), "no scroll in flight");
    }

    /// Every element pays for `TuiExt`, so growth should be a decision:
    /// state only some elements use (form controls, scroll containers,
    /// editing hosts, selects) lives behind a lazily created box
    /// (`P7G-FORM-STATE-BOX-1`: 4496 → 4344 bytes on 64-bit targets),
    /// and so do the pseudo-element styles (`Rc`), the transition
    /// overrides and the inline style (`PERF-TUIEXT-SIZE-1`: 4344 →
    /// 432). Raise the bound deliberately, with the reason in the commit:
    /// 440 for the recorded matches a vars-only restyle reuses
    /// (`C1G-PROPERTY-RESTYLE`, one `Rc`).
    #[test]
    fn tui_ext_size_tripwire() {
        const MAX: usize = 440;
        let size = std::mem::size_of::<TuiExt>();
        let computed = std::mem::size_of::<ComputedStyle>();
        let inline = std::mem::size_of::<TuiStyle>();
        let presentation = std::mem::size_of::<PresentationStyle>();
        eprintln!(
            "TuiExt {size} B; ComputedStyle {computed} B, TuiStyle {inline} B, \
             PresentationStyle {presentation} B"
        );
        assert!(size <= MAX, "size_of::<TuiExt>() = {size}, bound {MAX}");
    }

    #[test]
    fn partial_eq_works() {
        let a = TuiExt {
            inline_style: Some(Box::new(TuiStyle::new().width(Size::Fixed(10)))),
            ..Default::default()
        };
        let b = TuiExt {
            inline_style: Some(Box::new(TuiStyle::new().width(Size::Fixed(10)))),
            ..Default::default()
        };
        let c = TuiExt {
            inline_style: Some(Box::new(TuiStyle::new().width(Size::Fixed(11)))),
            ..Default::default()
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
