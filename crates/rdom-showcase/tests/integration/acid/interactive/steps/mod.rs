//! The stage-2 steps, one file per step, in `ACID.md`'s numbering. Each
//! file carries its spec citations, the derivation of every checkpoint
//! (ground rule 1: from the spec, never from rdom's output), its
//! per-checkpoint references and its script.

use super::session::Step;

pub mod i01_hover;
pub mod i02_active;
pub mod i03_focus;
pub mod i04_validity;
pub mod i05_toggles;
pub mod i06_transitions;
pub mod i07_cssom;
pub mod i08_smooth_scroll;
pub mod i09_caret;
pub mod i10_pointer_events;
pub mod i11_snap;
pub mod i12_pseudo;
pub mod i13_invalidation;
pub mod i14_user_validity;

/// Every step, in `ACID.md` order.
pub const ALL: &[&Step] = &[
    &i01_hover::STEP,
    &i02_active::STEP,
    &i03_focus::STEP,
    &i04_validity::STEP,
    &i05_toggles::STEP,
    &i06_transitions::STEP,
    &i07_cssom::STEP,
    &i08_smooth_scroll::STEP,
    &i09_caret::STEP,
    &i10_pointer_events::STEP,
    &i11_snap::STEP,
    &i12_pseudo::STEP,
    &i13_invalidation::STEP,
    &i14_user_validity::STEP,
];
