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
pub mod i10_pointer_events;

/// Every step, in `ACID.md` order.
pub const ALL: &[&Step] = &[
    &i01_hover::STEP,
    &i02_active::STEP,
    &i03_focus::STEP,
    &i04_validity::STEP,
    &i05_toggles::STEP,
    &i10_pointer_events::STEP,
];
