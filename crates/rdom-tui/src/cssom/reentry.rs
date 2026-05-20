//! `CSSOM_REENTRY` thread-local — flips on while
//! [`StyleDeclarationMut`](super::StyleDeclarationMut) is writing
//! back to the `style="…"` attribute, so the inline-style
//! observer (`crate::cssom::observer`) doesn't re-parse what we
//! just serialized.
//!
//! Pattern: scope-guarded RAII via [`ReentryGuard`]. The guard's
//! `Drop` impl restores the flag even on panic — required by
//! the M4b architect-pass risk #2 lock so a panic during
//! `set_property` can't leave the flag stuck `true`.
//!
//! Single source, no recursion. Same shape the retired
//! `runtime::current_event` thread-local used.

use std::cell::Cell;

thread_local! {
    static CSSOM_REENTRY: Cell<bool> = const { Cell::new(false) };
}

/// RAII guard. Sets `CSSOM_REENTRY` true on construction, restores
/// the prior value on `Drop` (panic-safe).
///
/// Restoring the *prior* value (rather than always setting false)
/// lets the guard nest safely — though in current usage nesting
/// shouldn't happen, the cost is one extra `bool` and it's safer.
pub(crate) struct ReentryGuard {
    prev: bool,
}

impl ReentryGuard {
    /// Enter a CSSOM-originated write scope.
    #[inline]
    pub(crate) fn enter() -> Self {
        let prev = CSSOM_REENTRY.with(|f| f.replace(true));
        Self { prev }
    }
}

impl Drop for ReentryGuard {
    #[inline]
    fn drop(&mut self) {
        CSSOM_REENTRY.with(|f| f.set(self.prev));
    }
}

/// Read the current re-entry state. The inline-style observer
/// checks this and bails when the write came from a CSSOM
/// operation that already updated `TuiExt::inline_style` directly.
#[inline]
pub(crate) fn is_in_cssom_write() -> bool {
    CSSOM_REENTRY.with(|f| f.get())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_false() {
        assert!(!is_in_cssom_write());
    }

    #[test]
    fn guard_sets_flag_inside_scope() {
        assert!(!is_in_cssom_write());
        {
            let _g = ReentryGuard::enter();
            assert!(is_in_cssom_write());
        }
        assert!(!is_in_cssom_write(), "flag must be restored after Drop");
    }

    #[test]
    fn guard_nests_correctly() {
        {
            let _outer = ReentryGuard::enter();
            assert!(is_in_cssom_write());
            {
                let _inner = ReentryGuard::enter();
                assert!(is_in_cssom_write());
            }
            // Inner dropped — outer still active.
            assert!(is_in_cssom_write());
        }
        assert!(!is_in_cssom_write());
    }

    #[test]
    fn guard_restores_flag_on_panic() {
        // Architect-pass risk #2 acceptance criterion: a panic
        // during a CSSOM write must not leave the flag stuck
        // true (which would silently suppress legitimate
        // external `style="…"` writes that follow).
        let outcome = std::panic::catch_unwind(|| {
            let _g = ReentryGuard::enter();
            assert!(is_in_cssom_write());
            panic!("simulated panic inside CSSOM write");
        });
        assert!(outcome.is_err(), "expected panic to propagate");
        assert!(
            !is_in_cssom_write(),
            "guard's Drop must clear the flag even on panic"
        );
    }
}
