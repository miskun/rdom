//! The longhand table: every longhand the dispatch table knows, with
//! its animation type as its specification's property definition states
//! it, and — where rdom computes a value for it — how its computed value
//! animates (`entry.rs`). Companion fields a value carries (`display`'s
//! inner type, `flex-direction`'s reversal) move with it.
//!
//! The table is two halves, in the dispatch table's order: `paint` (the
//! colors, fonts, decorations, display, UI, overflow, sizing, effects,
//! alignment, multi-column, fragmentation and anchor longhands) then
//! `flow` (grid, box, borders, tables, generated content, positioning,
//! transitions, animations, timelines, counters and text), joined at
//! compile time.

use super::entry::Entry;

mod flow;
mod paint;

/// Every longhand, by the order of the dispatch table's names.
pub(super) static LONGHANDS: &[Entry] = &ALL;

static ALL: [Entry; paint::PAINT.len() + flow::FLOW.len()] = join(paint::PAINT, flow::FLOW);

/// `a` then `b`, as one array of `N` entries (`N` their lengths' sum).
const fn join<const N: usize>(a: &[Entry], b: &[Entry]) -> [Entry; N] {
    let mut out = [a[0]; N];
    let mut i = 0;
    while i < a.len() {
        out[i] = a[i];
        i += 1;
    }
    let mut j = 0;
    while j < b.len() {
        out[a.len() + j] = b[j];
        j += 1;
    }
    out
}
