//! `TuiExt` — presentation data attached to every Element via `Dom<TuiExt>`.
//!
//! Everything a TUI element carries beyond its tag / attrs / classes
//! lives here: inline style, pseudo-element content, sizing and box
//! model, overflow + scroll state, laid-out geometry, and the cached
//! post-cascade `ComputedStyle`. `rdom-core` never sees any of this —
//! it just holds the `TuiExt` payload behind its `Ext` generic.

use crate::layout::{
    Border, Direction, LayoutRect, Length, Overflow, Padding, Position, Size, ZIndex,
};
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
/// document-order indices (within the parent's element-or-text
/// child list) the anon box wraps — paint walks this back to find
/// the source ::before / ::after pseudos and styling context.
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
}

impl PresentationStyle {
    /// True when no animation is currently driving any property.
    /// The hot path uses this to skip the override read.
    pub fn is_empty(&self) -> bool {
        self.fg.is_none()
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
#[derive(Debug, Default, Clone, PartialEq)]
pub struct TuiExt {
    // ── Style (inline) ────────────────────────────────────────────────
    pub inline_style: TuiStyle,

    /// Fallback text for `::before` when no matching stylesheet rule
    /// supplies a `content:` value. Author-facing setters are
    /// `TuiNodeMutExt::set_before_content(...)` / `clear_before_content()`.
    /// Stylesheet `content: "…";` takes precedence when present.
    pub before_content: Option<String>,
    /// Text injected after the element's own content by `::after`.
    pub after_content: Option<String>,

    // ── Sizing ────────────────────────────────────────────────────────
    pub width: Size,
    pub height: Size,
    pub min_width: Option<rdom_style::layout::MinSize>,
    pub max_width: Option<u16>,
    pub min_height: Option<rdom_style::layout::MinSize>,
    pub max_height: Option<u16>,

    // ── Box model ─────────────────────────────────────────────────────
    pub direction: Direction,
    pub padding: Padding,
    pub border: Border,
    pub gap: u16,

    // ── Overflow + scroll ─────────────────────────────────────────────
    pub overflow: Overflow,
    /// Horizontal scroll offset in cells.
    pub scroll_x: usize,
    /// Vertical scroll offset in cells.
    pub scroll_y: usize,
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
    pub computed: Option<ComputedStyle>,
    /// Snapshot of `computed` from the *previous* cascade pass. Used
    /// by the M3 transition engine to diff against the current
    /// `computed` and detect which animatable properties changed.
    /// `None` on the first cascade pass — no diff to perform.
    pub computed_prev: Option<ComputedStyle>,
    /// In-flight transition values. Sparse: only properties an
    /// active animation is currently driving have their slot
    /// populated; everything else falls back to `computed`. Paint,
    /// layout, and hit-test consult this first via the
    /// `effective_*` helpers.
    pub presentation: PresentationStyle,
    /// `::before` pseudo-element computed style. `None` if no content
    /// and no matching `::before` rules.
    pub computed_before: Option<ComputedStyle>,
    /// `::after` pseudo-element computed style.
    pub computed_after: Option<ComputedStyle>,
    /// `::backdrop` pseudo-element computed style — populated for
    /// modal `<dialog>` elements whose stylesheet has a matching
    /// `dialog::backdrop` rule. The paint pass overlays the
    /// backdrop across the viewport after normal paint and before
    /// re-painting the dialog. See Polish #8.
    pub computed_backdrop: Option<ComputedStyle>,
    /// `::selection` pseudo-element computed style — populated
    /// for elements whose stylesheet has a matching `::selection`
    /// rule. The selection-overlay paint walks up from each
    /// selected text fragment to the nearest ancestor with this
    /// style and applies its bg/fg/modifier. The UA stylesheet
    /// ships a default `*::selection { background-color: #394B7E;
    /// color: white }` rule, so every selectable always has a
    /// computed selection style unless an author explicitly
    /// overrides it back to `initial`.
    pub computed_selection: Option<ComputedStyle>,
    /// `::scrollbar` pseudo-element computed style — populated
    /// for elements with non-`Visible`/`Hidden` overflow on at
    /// least one axis. Drives the scrollbar track paint: `bg`
    /// fills every track cell, `fg` colors the `content` glyph
    /// (default `" "` from UA — a colored gutter via `bg` is the
    /// modern look). Authors override via
    /// `selector::scrollbar { bg: …; content: "▒"; }` to retheme.
    pub computed_scrollbar: Option<ComputedStyle>,
    /// `::scrollbar-thumb` pseudo-element computed style — same
    /// shape as `computed_scrollbar` but for the thumb cells
    /// (default content `┃`, fg `Gray`, bg matches the track so
    /// the thumb cell renders as a vertical-bar glyph on the
    /// colored gutter). Authors override via
    /// `selector::scrollbar-thumb { fg: …; content: "█"; }`.
    pub computed_scrollbar_thumb: Option<ComputedStyle>,

    // ── Dirty flags (read by cascade + layout, set by mutation hooks) ─
    /// This element needs re-cascade next frame. Set by the
    /// `DirtyTracker` mutation observer whenever an attribute / class
    /// / tree-shape change could affect which rules match.
    pub style_dirty: bool,
    /// This element needs re-layout. Set by the cascade when a
    /// layout-affecting computed value changes. Separate from
    /// `style_dirty` so a pure color change skips re-layout entirely.
    pub layout_dirty: bool,

    // ── Editing state (Phase B) ──────────────────────────────────────
    /// Per-editable state (undo/redo history, coalescing metadata).
    /// `None` until the element first receives an edit; `Some(Box<_>)`
    /// thereafter. Boxed so `TuiExt` stays small for non-editable
    /// elements (the common case).
    pub editor_state: Option<Box<EditorState>>,

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
}

impl TuiExt {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::Color;

    #[test]
    fn defaults_are_sensible() {
        let ext = TuiExt::new();
        assert_eq!(ext.direction, Direction::Column);
        assert_eq!(ext.width, Size::Auto);
        assert_eq!(ext.height, Size::Auto);
        assert_eq!(ext.overflow, Overflow::Visible);
        assert_eq!(ext.border, Border::none());
        assert_eq!(ext.gap, 0);
        assert_eq!(ext.scroll_x, 0);
        assert_eq!(ext.scroll_y, 0);
        assert!(ext.inline_style.is_empty());
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

    #[test]
    fn clone_preserves_fields() {
        let ext = TuiExt {
            inline_style: TuiStyle::new().fg(Color::Rgb(255, 0, 0)),
            width: Size::Fixed(80),
            padding: Padding::all(2),
            ..Default::default()
        };
        let cloned = ext.clone();
        assert_eq!(
            cloned.inline_style.fg,
            Some(crate::style::Value::Specified(
                crate::style::TuiColor::Literal(Color::Rgb(255, 0, 0))
            ))
        );
        assert_eq!(cloned.width, Size::Fixed(80));
        assert_eq!(cloned.padding, Padding::all(2));
    }

    #[test]
    fn partial_eq_works() {
        let a = TuiExt {
            width: Size::Fixed(10),
            ..Default::default()
        };
        let b = TuiExt {
            width: Size::Fixed(10),
            ..Default::default()
        };
        let c = TuiExt {
            width: Size::Fixed(11),
            ..Default::default()
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
