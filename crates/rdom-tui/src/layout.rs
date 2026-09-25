//! `rdom_tui::layout` — rdom-style's layout data model
//! ([`rdom_style::layout`], re-exported whole) plus the layout pass's
//! box geometry ([`compute_content_area`], [`compute_padding_box`],
//! [`clamp_size`], …), which lives in `render::layout_pass::geometry`.

pub use crate::render::layout_pass::geometry::{
    clamp_size, compute_content_area, compute_content_area_collapsed, compute_padding_box,
};
pub use rdom_style::layout::*;
