//! Evidence of writes to runtime-managed `TuiExt` state that no DOM
//! `Mutation` reports (`P7G-TICK-TOUCHED-1`): a scroll offset moved
//! through the scroll API, a smooth scroll started or replaced, a custom
//! validity message set, a control's user-edited flag cleared.
//!
//! The `App` runs its whole-tree frame checks (validity marks, scroll
//! offsets moved since paint, smooth scrolls) only after code that may
//! have changed what they read. For a callback — `on_tick`, a timer, an
//! injected closure — "may have" is decided from evidence: the dirty
//! tracker saw a mutation, or one of these writers ran. Each writer
//! calls [`note`]; the App compares [`generation`] before and after the
//! callback. Only the difference matters, so the counter's absolute
//! value (per thread, shared by every `App` on it) never does.

use std::cell::Cell;

thread_local! {
    static WRITES: Cell<u64> = const { Cell::new(0) };
}

/// Record one write of runtime-managed state.
pub(crate) fn note() {
    WRITES.with(|w| w.set(w.get().wrapping_add(1)));
}

/// The number of writes recorded on this thread so far.
pub(crate) fn generation() -> u64 {
    WRITES.with(Cell::get)
}
