//! Layout data model for flexbox-style TUI layout — the value types
//! the cascade computes and rdom-tui's layout pass reads.
//!
//! Cell-based dimensions throughout. [`LayoutRect`] uses signed `i32`
//! for position so elements can sit above / left of the viewport
//! (needed for scroll clipping). [`Size`] + [`Direction`] +
//! [`Overflow`] + [`Border`] + [`Padding`] cover the CSS-like sizing
//! model.
//!
//! The geometry that applies these values to rectangles (content
//! area, padding box, min / max clamping) belongs to the layout pass
//! and lives in rdom-tui (`rdom_tui::layout::compute_content_area`
//! and friends).
//!
//! - `rect` — `LayoutRect`
//! - `keywords` — keyword-valued properties
//! - `sizing` — `Size`, `MinSize`, `AspectRatio`, `GapValue`, `Length`
//! - `box_model` — borders, `border-collapse`, padding, margin

mod box_model;
mod keywords;
mod rect;
mod sizing;

pub use box_model::{
    Border, BorderCollapse, BorderStyle, CornerStyle, Margin, MarginValue, Padding, PaddingValue,
};
pub use keywords::{
    Align, CaretColor, CaretTextColor, Direction, Display, Flow, Overflow, PointerEvents, Position,
    ScrollbarGutter, TextDecoration, UserSelect, WhiteSpace, ZIndex,
};
pub use rect::LayoutRect;
pub use sizing::{AspectRatio, GapValue, Length, MinSize, Size};
