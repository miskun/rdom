//! UA inline chrome — the paint-time text substitutions the built-in
//! elements supply, gathered into one table for the paint pass.
//!
//! Each entry is a built-in's `inline_chrome` adapter: given an element
//! and the single-row width available to it, return the
//! [`ChromeText`] to paint in place of the element's own text, or
//! `None` when the element is not one of that built-in's. The entries
//! are disjoint by tag (`<progress>` / `<meter>`, `<select>`,
//! `<input type="password">`), so table order never changes a result.
//!
//! The paint pass reaches this table through exactly one call
//! (`render::paint_pass::inline_paint::chrome`), and that file says
//! why the table is compiled in rather than registered per `Dom`: a
//! bare `Dom<TuiExt>` painted without `App::build` must still show its
//! chrome.

use rdom_core::NodeId;

use super::{gauge, input, select};
use crate::TuiDom;
use crate::render::paint_pass::{ChromeText, InlineChromeFn};

/// The built-ins that substitute chrome for an element's own text.
const SUPPLIERS: &[InlineChromeFn] = &[
    gauge::inline_chrome,
    select::inline_chrome,
    input::password_inline_chrome,
];

/// The chrome to paint for `id` in a single row of `width` cells, or
/// `None` when no built-in substitutes for it.
pub(crate) fn lookup(dom: &TuiDom, id: NodeId, width: u16) -> Option<ChromeText> {
    SUPPLIERS
        .iter()
        .find_map(|supplier| supplier(dom, id, width))
}
