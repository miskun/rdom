//! CSS-COMPLETE Phase 6 — display, visibility, flexbox and box
//! alignment: each sheet parsed strictly, cascaded and laid out (and
//! painted where the cells are the claim). One submodule per item; each
//! test cites the spec text that fixes the expected result. The sheet
//! helpers are Phase 5's.

#[allow(unused_imports)]
pub(crate) use super::css_phase5::{el, lay_out, paint, rect, rows, size};

mod align;
mod align_content;
mod anon_items;
mod collapse;
mod contents;
mod direction_reverse;
mod display;
mod flex_basis_zero;
mod flex_direction_initial;
mod flex_longhands;
mod flex_spec;
mod gap;
mod justify;
mod margin_sides;
mod order;
mod place;
mod visibility;
mod wrap;
