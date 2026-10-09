//! Pixel and font-relative lengths where they select rather than measure
//! (DESIGN "Pixel lengths select, cells measure"): a terminal cell is
//! about one `ch` wide — half the 16px initial font size — and one line
//! tall, so a length in `px`, `em` / `rem` (the initial font size, 16px —
//! Media Queries 4 §1.3) or an absolute unit (CSS Values 4 §6.2: 96px an
//! inch) is taken at 8px a column and 16px a row. Media and container
//! query breakpoints (C14G-PX-BREAKPOINTS) and `column-width`, which only
//! selects a column count (C15G-COLUMN-WIDTH-PX), read lengths so.

/// Pixels to a column.
pub(crate) const PX_PER_COLUMN: f64 = 8.0;
/// Pixels to a row.
pub(crate) const PX_PER_ROW: f64 = 16.0;

/// The pixels in one `unit` (ASCII case-insensitive), for the units a
/// selecting length takes; `None` for any other.
pub(crate) fn px_per(unit: &str) -> Option<f64> {
    Some(match unit.to_ascii_lowercase().as_str() {
        "px" => 1.0,
        "em" | "rem" | "pc" => 16.0,
        "in" => 96.0,
        "cm" => 96.0 / 2.54,
        "mm" => 96.0 / 25.4,
        "q" => 96.0 / 101.6,
        "pt" => 96.0 / 72.0,
        _ => return None,
    })
}
