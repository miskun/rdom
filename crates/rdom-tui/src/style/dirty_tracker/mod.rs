//! `DirtyTracker` — a `MutationObserver` that flips `style_dirty` on
//! affected elements and maintains a worklist of subtree roots for
//! incremental re-cascade.
//!
//! ## Usage
//!
//! ```
//! # use rdom_tui::{TuiDom, Stylesheet, TuiStyle, Color, CascadeExt};
//! # use rdom_tui::style::dirty_tracker::DirtyTracker;
//! let mut dom: TuiDom = TuiDom::new();
//! let tracker = DirtyTracker::install(&mut dom);
//!
//! let sheet = Stylesheet::bare()
//!     .rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
//!
//! // Initial cascade — everything, writes computed styles.
//! dom.cascade(&sheet);
//!
//! // Later, some mutations happen; the tracker collects dirty roots.
//! let div = dom.create_element("div");
//! dom.append_child(dom.root(), div).unwrap();
//!
//! // Re-cascade only the changed subtrees.
//! let roots = tracker.take_roots();
//! dom.cascade_subtrees(&sheet, &roots);
//! ```
//!
//! ## What counts as dirty
//!
//! - Attribute changes, class changes, inline style edits → **node +
//!   subtree** (conservative — selectors like `a b` mean a parent's
//!   attribute change can affect descendants)
//! - Tree mutations (insert / remove / clear) → **inserted subtree +
//!   all element children of the affected parent** (for
//!   sibling-dependent selectors like `:first-child`, `+`, `~`)
//! - Interaction changes (hover / active / focus) → the elements whose
//!   match flipped. `:hover`, `:active` and `:focus-within` match every
//!   ancestor of the element holding the state (Selectors 4 §9.2 /
//!   §9.4 / §13.3), so that is **the old and new ancestor chains minus
//!   their common part**, marked at each side's topmost element
//!   (`:focus-visible` flips on the focused element alone)
//! - Sibling combinators: `a:hover + b`, `[x] ~ p`, `.e:empty + p` let
//!   an element's match read a *previous sibling's* state. When a
//!   compound left of a `+` / `~` in the sheets can read the kind of
//!   change made — a pseudo-class for hover / focus / text / `:empty`,
//!   the attribute's own name (`class`, `id` included) for an attribute
//!   change (`style::sibling_triggers`, `P7G-SIBLING-MARK-NARROW-1`;
//!   everything until the App tells the tracker its sheets) — an
//!   element whose own state changed also dirties its parent's element
//!   children, once per parent per drain
//! - The parent of a tree mutation is dirtied too when its own match
//!   can change: `:empty` (its first element / text child arrived or
//!   its last one left) and, when text nodes come or go,
//!   `:placeholder-shown` / `::placeholder`, which read the text
//!   content (`Dom::is_placeholder_shown`)
//! - Character-data changes dirty only the elements whose selector
//!   state reads text, when the text went from empty to non-empty or
//!   back: the parent (its `:empty`) and an ancestor with a
//!   `placeholder` attribute
//!   (`P7G-PAINT-ONLY-FRAME-1` — the runtime no longer re-cascades the
//!   whole tree on a text edit, so the tracker must name them). Every
//!   text change also changes layout and painted output, so the tracker
//!   keeps a separate `paint_dirty` flag (consumed via
//!   `take_paint_dirty()`) for the runtime to lay out and repaint even
//!   though no cascade work is queued.
//! - Content changes under a `dir=auto` element or a `<bdi>` (text
//!   edits, children coming or going) dirty that element: its
//!   directionality — `:dir()`, and the UA's `direction` rules written
//!   with it — reads its first strong character (HTML §3.2.6.4)
//! - Selection changes (`SelectionChanged`) dirty no element — paint
//!   reads `Dom::selection` — but set a `selection_dirty` flag
//!   (`take_selection_dirty()`) the runtime turns into a repaint.
//!
//! The App's frame prelude consumes both flags on every frame, not only
//! after an input event (`P7G-OFF-EVENT-PAINT-1`).
//!
//! ## Dedupe policy
//!
//! When marking `X` dirty, we check whether any ancestor of `X` is
//! already in the roots list. If yes, the ancestor will
//! re-cascade the whole subtree including `X`, so we skip pushing.
//! This keeps the roots list small even under bursts of mutations.

use std::cell::RefCell;
use std::rc::Rc;

use rdom_core::{Dom, NodeId, ObserverId};

use crate::ext::TuiExt;
use crate::style::sibling_triggers::SiblingTriggers;

use marks::mark_style_dirty;
use observe::Shim;

mod marks;
mod observe;
#[cfg(test)]
mod tests;

/// Shared handle to the dirty-roots list. Created by
/// `DirtyTracker::install`; the tracker uses it internally, and
/// callers retrieve accumulated roots via `take_roots()`.
#[derive(Debug, Clone, Default)]
pub struct DirtyTracker {
    inner: Rc<RefCell<DirtyState>>,
    observer_id: Option<ObserverId>,
}

#[derive(Debug)]
pub(super) struct DirtyState {
    pub(super) roots: Vec<NodeId>,
    /// Mirror of `roots` for O(1) membership (the Vec keeps insertion
    /// order for deterministic cascade).
    pub(super) roots_set: std::collections::HashSet<NodeId>,
    /// Parents whose element children were all marked dirty for
    /// sibling-dependent selectors since the last drain. A second
    /// `ChildListChanged` on the same parent before the cascade runs
    /// finds them dirty already, so the O(children) loop is skipped —
    /// appending n rows one by one is O(n) marks, not O(n²).
    pub(super) sibling_marked: std::collections::HashSet<NodeId>,
    /// Text-only mutations don't affect the cascade (selectors don't
    /// match against text content) but they DO change painted output.
    /// Set by `CharacterDataChanged`; consumed by the runtime's redraw
    /// decision via `take_paint_dirty()`. Without this flag, a
    /// `set_node_value` call from inside an event handler is invisible
    /// until something else dirties the cascade.
    pub(super) paint_dirty: bool,
    /// The selection or caret moved (`SelectionChanged`): paint draws
    /// the `::selection` overlay and the caret from `Dom::selection`,
    /// so only a repaint is due. Consumed via `take_selection_dirty()`.
    pub(super) selection_dirty: bool,
    /// Records observed since install — evidence that code changed the
    /// tree (`records_seen`, `P7G-TICK-TOUCHED-1`).
    pub(super) records: u64,
    /// Which changes can reach a sibling's match through a `+` / `~`
    /// combinator (`style::sibling_triggers`,
    /// `P7G-SIBLING-MARK-NARROW-1`). Every change until the App says
    /// otherwise.
    pub(super) siblings: SiblingTriggers,
}

impl Default for DirtyState {
    fn default() -> Self {
        Self {
            roots: Vec::new(),
            roots_set: std::collections::HashSet::new(),
            sibling_marked: std::collections::HashSet::new(),
            paint_dirty: false,
            selection_dirty: false,
            records: 0,
            siblings: SiblingTriggers::all(),
        }
    }
}

impl DirtyTracker {
    /// Register a `MutationObserver` that writes into a fresh tracker.
    /// The returned `DirtyTracker` holds a `Rc`-cloned handle to the
    /// same state — both the observer (inside `Dom.observers`) and
    /// external callers read/write a shared `RefCell`.
    pub fn install(dom: &mut Dom<TuiExt>) -> Self {
        let inner = Rc::new(RefCell::new(DirtyState::default()));
        let shim = Shim {
            inner: inner.clone(),
        };
        let observer_id = dom.add_mutation_observer(Box::new(shim));
        Self {
            inner,
            observer_id: Some(observer_id),
        }
    }

    /// Remove the observer from `dom` and return the final dirty-roots
    /// list. After calling this, `take_roots()` returns an empty Vec.
    pub fn uninstall(self, dom: &mut Dom<TuiExt>) -> Vec<NodeId> {
        if let Some(id) = self.observer_id {
            dom.remove_mutation_observer(id);
        }
        let mut state = self.inner.borrow_mut();
        state.roots_set.clear();
        state.sibling_marked.clear();
        std::mem::take(&mut state.roots)
    }

    /// Return the accumulated dirty roots, clearing the internal list.
    /// Call this right before `cascade_subtrees`.
    pub fn take_roots(&self) -> Vec<NodeId> {
        let mut state = self.inner.borrow_mut();
        state.roots_set.clear();
        state.sibling_marked.clear();
        std::mem::take(&mut state.roots)
    }

    /// Peek at the current dirty roots without clearing. Useful in
    /// tests.
    pub fn roots_snapshot(&self) -> Vec<NodeId> {
        self.inner.borrow().roots.clone()
    }

    /// Consume and return the paint-dirty flag: a mutation that queues
    /// no (or not only) cascade work but changes layout and painted
    /// output — text edits like `set_node_value`, text nodes coming or
    /// going. The App's frame prelude reads it on every frame, whoever
    /// made the change (a listener, a timer, an injected closure,
    /// `dom_mut()`), and lays out and repaints
    /// (`P7G-OFF-EVENT-PAINT-1`).
    pub fn take_paint_dirty(&self) -> bool {
        std::mem::take(&mut self.inner.borrow_mut().paint_dirty)
    }

    /// Peek at the paint-dirty flag without clearing. Useful in tests.
    pub fn paint_dirty_snapshot(&self) -> bool {
        self.inner.borrow().paint_dirty
    }

    /// Consume and return the selection-dirty flag: the selection or
    /// caret moved since the last call. Paint reads the selection
    /// directly, so the App's frame prelude repaints without laying out
    /// (`P7G-OFF-EVENT-PAINT-1`).
    pub fn take_selection_dirty(&self) -> bool {
        std::mem::take(&mut self.inner.borrow_mut().selection_dirty)
    }

    /// Whether the next frame has anything the tracker recorded to
    /// draw: dirty roots, the paint-dirty flag or the selection-dirty
    /// flag. Does not clear anything.
    #[cfg(test)]
    pub(crate) fn has_pending(&self) -> bool {
        let state = self.inner.borrow();
        !state.roots.is_empty() || state.paint_dirty || state.selection_dirty
    }

    /// How many mutation records the tracker has observed since it was
    /// installed. The App compares it around a callback to tell whether
    /// the callback changed the tree (`P7G-TICK-TOUCHED-1`).
    pub(crate) fn records_seen(&self) -> u64 {
        self.inner.borrow().records
    }

    /// Registered observer's handle. `None` after `uninstall`.
    pub fn observer_id(&self) -> Option<ObserverId> {
        self.observer_id
    }

    /// Say whether the stylesheets cascaded against this tree use a
    /// sibling combinator (`+` / `~`) anywhere
    /// ([`uses_sibling_combinators`]). While they may (the default), an
    /// element whose own state changes also dirties its siblings, whose
    /// match can read that state (`a:hover + b`); with `false` it does
    /// not, which keeps a hover move in a long list from re-cascading
    /// the list. The App sets it whenever its sheets change.
    pub fn set_sibling_combinators(&self, used: bool) {
        self.inner.borrow_mut().siblings = if used {
            SiblingTriggers::all()
        } else {
            SiblingTriggers::none()
        };
    }

    /// Say which changes can reach a sibling's match under the sheets
    /// now cascaded — the precise form of
    /// [`set_sibling_combinators`](Self::set_sibling_combinators) the
    /// App uses (`P7G-SIBLING-MARK-NARROW-1`): `h1 + p` reads nothing a
    /// change can flip, `a:hover + b` reads state, `[x] + b` reads `x`.
    pub(crate) fn set_sibling_triggers(&self, triggers: SiblingTriggers) {
        self.inner.borrow_mut().siblings = triggers;
    }

    /// Manually mark a subtree dirty. Escape hatch for cases the
    /// `MutationObserver` doesn't cover — a direct write to a `TuiExt`
    /// field the cascade reads, such as `TuiExt::set_inline_style`
    /// (which mutates the ext data, not DOM state, so no `Mutation`
    /// fires). The `TuiNodeMutExt` setters on a `NodeMut` reflect into
    /// the `style` attribute and need no manual mark.
    ///
    /// Behavior matches the automatic path: flips `style_dirty` on the
    /// node, dedupes against dirty ancestors, pushes to the roots list
    /// only when necessary.
    pub fn mark_dirty(&self, dom: &mut Dom<TuiExt>, id: NodeId) {
        let mut state = self.inner.borrow_mut();
        mark_style_dirty(dom, &mut state, id);
    }
}

/// Whether any selector `sheet` matches with has a sibling combinator
/// (`+` / `~`) anywhere — in a rule's selector or an `@scope`'s start /
/// end, inside `:not()` / `:is()` / `:where()` too
/// (`style::selector_walk`) — the input of
/// [`DirtyTracker::set_sibling_combinators`].
pub fn uses_sibling_combinators(sheet: &crate::style::Stylesheet) -> bool {
    use crate::style::selector_walk::{any_complex, sheet_selectors};
    use rdom_core::selectors::Combinator;
    sheet_selectors(sheet).any(|c| {
        any_complex(c, &mut |c| {
            c.ancestors.iter().any(|(comb, _)| {
                matches!(
                    comb,
                    Combinator::AdjacentSibling | Combinator::GeneralSibling
                )
            })
        })
    })
}
