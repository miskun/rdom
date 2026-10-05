//! CSS-COMPLETE Phase 7 — grid (CSS Grid Layout 2; section numbers are
//! Grid 1's, which Level 2 keeps through §8 and shifts by one after
//! inserting §9 "Subgrids"): each sheet parsed strictly, cascaded and
//! laid out (and painted where the cells are the claim). One submodule
//! per concern; each test cites the spec text that fixes the expected
//! result. The sheet helpers are Phase 5's.

#[allow(unused_imports)]
pub(crate) use super::css_phase5::{el, lay_out, paint, rect, rows, size};

mod auto;
mod container;
mod tracks;
