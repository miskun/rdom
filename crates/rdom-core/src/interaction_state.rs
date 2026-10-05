//! The document's interaction state on `Dom<Ext>`: the hovered, focused
//! and active nodes and `:focus-visible`'s flag (Selectors 4 §9, §13),
//! pointer capture and drag autoscroll, and the selection with its
//! serial — the getters and the setters that fire
//! `Mutation::InteractionChanged` / `SelectionChanged`. Split out of
//! `dom.rs` (C7G-SIZES); the fields stay on `Dom`.

use crate::dom::Dom;
use crate::node_id::NodeId;
use crate::observer::{InteractionKind, Mutation};

impl<Ext> Dom<Ext> {
    /// The node currently flagged as hovered (see `:hover`). `None` when
    /// nothing is hovered. Matching consults this field directly.
    pub fn hovered(&self) -> Option<NodeId> {
        self.hovered
    }

    /// The node currently flagged as focused (see `:focus`).
    pub fn focused(&self) -> Option<NodeId> {
        self.focused
    }

    /// The element currently being activated (see `:active`): the
    /// backend sets it for the duration of a primary-button press.
    /// `None` when nothing is being activated.
    pub fn active(&self) -> Option<NodeId> {
        self.active
    }

    /// Whether the focused element's focus should be made evident —
    /// the UA's judgement behind `:focus-visible` (Selectors 4 §13.2),
    /// which matches the focused element exactly while this is `true`.
    /// Starts `true` (focus before any pointer interaction is evident,
    /// as browsers treat script focus on a fresh page); a backend flips
    /// it with [`set_focus_visible`](Self::set_focus_visible) from its
    /// input-modality heuristics. It persists across focus changes, so
    /// focus moved by script keeps the previous element's visibility,
    /// as the spec asks.
    pub fn focus_visible(&self) -> bool {
        self.focus_visible
    }

    /// The node that currently owns the pointer via
    /// [`set_pointer_capture`](Self::set_pointer_capture). `None`
    /// (the default) means routing uses hit-testing as usual.
    pub fn pointer_capture(&self) -> Option<NodeId> {
        self.pointer_capture
    }

    /// The document's text selection, if any. `None` means no
    /// selection (not even a caret). Mutated via
    /// [`set_selection`](Self::set_selection) which fires
    /// `Mutation::SelectionChanged` for paint observers.
    pub fn selection(&self) -> Option<&crate::Selection> {
        self.selection.as_ref()
    }

    /// The current selection normalized to a document-ordered
    /// `Range` — `start` precedes `end` per
    /// [`compare_boundary_points`](Self::compare_boundary_points).
    /// Useful for paint + copy walks that need ordered traversal.
    ///
    /// Returns `None` when nothing is selected OR when the
    /// anchor/focus nodes are disconnected (shouldn't happen in
    /// practice, but handled defensively).
    pub fn selection_range(&self) -> Option<crate::Range> {
        let sel = self.selection.as_ref()?;
        let a = sel.anchor;
        let f = sel.focus;
        let (start, end) = match self.compare_boundary_points(a, f)? {
            std::cmp::Ordering::Greater => (f, a),
            _ => (a, f),
        };
        Some(crate::Range::ordered_unchecked(start, end))
    }
}

impl<Ext: 'static> Dom<Ext> {
    /// Set or clear the hovered node. Fires an `InteractionChanged`
    /// mutation record when the value actually changes; no-op when
    /// setting to the current value.
    pub fn set_hovered(&mut self, id: Option<NodeId>) {
        if self.hovered == id {
            return;
        }
        let prev = self.hovered;
        self.hovered = id;
        self.fire_mutation(Mutation::InteractionChanged {
            prev,
            next: id,
            kind: InteractionKind::Hover,
        });
    }

    /// Set or clear the element being activated (`:active`). Fires an
    /// `InteractionChanged { kind: Active }` record when the value
    /// changes; no-op otherwise. Detaching the element clears it, like
    /// hover and focus.
    pub fn set_active(&mut self, id: Option<NodeId>) {
        if self.active == id {
            return;
        }
        let prev = self.active;
        self.active = id;
        self.fire_mutation(Mutation::InteractionChanged {
            prev,
            next: id,
            kind: InteractionKind::Active,
        });
    }

    /// Set or clear the focused node. Fires an `InteractionChanged`
    /// record on change.
    pub fn set_focused(&mut self, id: Option<NodeId>) {
        if self.focused == id {
            return;
        }
        let prev = self.focused;
        self.focused = id;
        self.fire_mutation(Mutation::InteractionChanged {
            prev,
            next: id,
            kind: InteractionKind::Focus,
        });
    }

    /// Set whether the focused element's focus should be made evident
    /// (see [`focus_visible`](Self::focus_visible)). Fires an
    /// `InteractionChanged { kind: FocusVisible }` record naming the
    /// focused element as both `prev` and `next` when the value
    /// changes; no-op otherwise.
    pub fn set_focus_visible(&mut self, visible: bool) {
        if self.focus_visible == visible {
            return;
        }
        self.focus_visible = visible;
        self.fire_mutation(Mutation::InteractionChanged {
            prev: self.focused,
            next: self.focused,
            kind: InteractionKind::FocusVisible,
        });
    }

    /// Claim the pointer for `id`. While set, the runtime routes
    /// `mousemove` / drag / `mouseup` to `id` regardless of where
    /// the cursor lands — critical for drag-select, resize
    /// handles, scrubbing.
    ///
    /// Typical usage from a `mousedown` listener:
    ///
    /// ```ignore
    /// dom.add_event_listener(handle, "mousedown",
    ///     ListenerOptions::default(), |ctx| {
    ///         let target = ctx.event.target.unwrap();
    ///         ctx.dom.set_pointer_capture(target).unwrap();
    ///     })?;
    /// ```
    ///
    /// The capture releases automatically on `mouseup`, or can be
    /// released explicitly via [`release_pointer_capture`].
    ///
    /// Returns `Err(DomError::InvalidNode)` if `id` doesn't exist.
    /// Does **not** fire a mutation record — pointer capture
    /// doesn't affect cascade / selectors (no `:pointer-captured`
    /// pseudo in v1).
    ///
    /// [`release_pointer_capture`]: Self::release_pointer_capture
    pub fn set_pointer_capture(&mut self, id: NodeId) -> crate::Result<()> {
        self.node_or_err(id)?;
        self.pointer_capture = Some(id);
        // A fresh capture starts without autoscroll; the owner opts in
        // explicitly via `set_drag_autoscroll(true)`. Resetting here means a
        // scrollbar / slider capture never inherits a stale autoscroll flag.
        self.drag_autoscroll = false;
        Ok(())
    }

    /// Release any active pointer capture. Idempotent — no-op when
    /// nothing was captured. Also clears the drag-autoscroll opt-in.
    pub fn release_pointer_capture(&mut self) {
        self.pointer_capture = None;
        self.drag_autoscroll = false;
    }

    /// Opt the active captured drag into edge autoscroll (DRAG-AUTOSCROLL):
    /// while set, the backend scrolls the nearest scroll container as the
    /// pointer dwells at its edge. Pair with
    /// [`set_pointer_capture`](Self::set_pointer_capture) from a drag's
    /// `mousedown` handler (the same handler should `prevent_default` so the
    /// runtime's own text-selection/scrollbar defaults don't claim the drag).
    /// Auto-cleared when the capture releases. No-op without an active capture.
    pub fn set_drag_autoscroll(&mut self, on: bool) {
        if self.pointer_capture.is_some() {
            self.drag_autoscroll = on;
        }
    }

    /// Whether the active captured drag opted into edge autoscroll. Read by the
    /// backend each frame to decide whether to run the autoscroll phase.
    pub fn drag_autoscroll(&self) -> bool {
        self.drag_autoscroll
    }

    /// Set the document selection. `None` clears it.
    ///
    /// Fires `Mutation::SelectionChanged { prev, next }` on change
    /// so paint observers can refresh the `::selection` overlay.
    /// No-op when `next == current selection`.
    pub fn set_selection(&mut self, next: Option<crate::Selection>) {
        if self.selection == next {
            return;
        }
        let prev = self.selection.take();
        self.selection = next;
        self.selection_serial = self.selection_serial.next();
        self.fire_mutation(Mutation::SelectionChanged { prev, next });
    }

    /// A counter that advances on every actual selection change —
    /// every [`set_selection`](Self::set_selection) that fires
    /// `Mutation::SelectionChanged`, including the clear when the
    /// selected nodes leave the tree. Equal readings mean the selection
    /// was not touched in between, even if it moved away and back.
    ///
    /// Backends use it to tell their own caret updates from foreign
    /// ones without observing mutations — rdom-tui's undo grouping
    /// keeps a typing run open only while the selection is the one its
    /// last edit left (Blink closes the typing command on any other
    /// selection change). No web API exposes this; it is bookkeeping.
    pub fn selection_serial(&self) -> crate::SelectionSerial {
        self.selection_serial
    }
}
