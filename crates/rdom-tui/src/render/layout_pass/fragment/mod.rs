//! Block fragmentation (CSS Fragmentation 3): a laid-out flow cut into
//! fragmentainers — the column boxes of a multi-column container today,
//! pages or regions should they come — and moved into them.
//!
//! The flow is laid out once, unfragmented, as one tall fragmentainer of
//! the final inline size (whole cells: layout is translation-invariant,
//! and a fragmentainer of one width lays its content out the same in
//! each). Then:
//!
//! 1. [`candidates`] reads the possible breaks off the laid-out boxes
//!    (§4.1): between block-level siblings (class A) and between line
//!    boxes (class B), each with the rules of §4.4 it would break — a
//!    `break-before` / `-after` of `avoid` (rule 1), a `break-inside:
//!    avoid` around it (rule 2), `orphans` / `widows` (rule 3) — and
//!    whether `break-before` / `-after` force it (§3.1). A monolithic box
//!    (§4.1: an atomic inline, a flex, grid or table box, a scroll
//!    container, a size-contained box, a nested multi-column container) has
//!    no break inside.
//! 2. [`breaker`] picks the breaks for a fragmentainer height (§4.4: the
//!    last one that fits, relaxing the rules in reverse order when none
//!    does; a monolithic box taller than the fragmentainer overflows it,
//!    §4.2) — and balances, finding the least height whose fragments fit
//!    a count, by a bounded search (`balance`).
//! 3. [`apply`] moves each box into the fragmentainer its rows fall in. A
//!    box all in one fragmentainer moves whole, its subtree with it. A box
//!    split across several gets a fragment list ([`BoxFragments`],
//!    `TuiExt`'s kept layout): each fragment the rows of the box that
//!    fragmentainer holds, drawn as the unfragmented box sliced (§5.4
//!    `slice`); its `layout` rect is the fragments' bounding box (what
//!    `getBoundingClientRect` reports for a fragmented box) and its
//!    `content_layout` its content fragments' — the origin its line boxes
//!    are rebased on, each line moved into its own fragmentainer
//!    (`LineBox::column`).
//!
//! Truncation (§5.2): an unforced break drops the margins around it — the
//! next fragmentainer starts at the next box's border edge; after a forced
//! one the content resumes where it was.

mod apply;
mod breaker;
mod candidates;
#[cfg(test)]
mod tests;

pub(crate) use apply::BoxFragments;
pub(super) use apply::apply;
#[cfg(test)]
pub(super) use breaker::BREAKER_RUNS;
pub(super) use breaker::{Fill, fragmentainers};
pub(super) use candidates::collect;

/// One possible break in a flow (CSS Fragmentation 3 §4.1), in the
/// unfragmented flow's rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Break {
    /// The row the content before the break ends at (exclusive).
    pub(crate) end: i32,
    /// The row the next fragmentainer's content starts at.
    pub(crate) resume: i32,
    /// `break-before` / `break-after` force it (§3.1).
    pub(crate) forced: bool,
    /// The §4.4 rules a break here breaks, as [`RULE_1`] | … bits.
    pub(crate) violates: u8,
}

/// §4.4 rule 1: a class A break whose `break-after` / `break-before` is
/// `avoid`.
pub(crate) const RULE_1: u8 = 1;
/// Rule 2: a break inside a box with `break-inside: avoid`.
pub(crate) const RULE_2: u8 = 2;
/// Rule 3: a break between lines leaving fewer than `orphans` before it or
/// `widows` after it.
pub(crate) const RULE_3: u8 = 4;

/// One fragmentainer's piece of the flow: the rows `start .. end` of the
/// unfragmented flow, moved by `(dx, dy)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Slice {
    pub(crate) start: i32,
    pub(crate) end: i32,
    pub(crate) dx: i32,
    pub(crate) dy: i32,
}

/// Where each piece of a flow goes: its slices in flow order, at least
/// one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Plan {
    pub(crate) slices: Vec<Slice>,
}

impl Plan {
    /// The slice the row `y` falls in. A row before the first slice is the
    /// first's; a row in the truncated rows between two slices (§5.2) —
    /// only a margin or an empty box can start there — the next one's.
    pub(crate) fn index(&self, y: i32) -> usize {
        let i = self
            .slices
            .partition_point(|s| s.start <= y)
            .saturating_sub(1);
        if y >= self.slices[i].end && i + 1 < self.slices.len() {
            i + 1
        } else {
            i
        }
    }

    /// The slices a box spanning the rows `top .. top + height` falls in,
    /// first and last.
    pub(crate) fn span(&self, top: i32, height: u16) -> (usize, usize) {
        let first = self.index(top);
        let last = if height == 0 {
            first
        } else {
            self.index(top + i32::from(height) - 1).max(first)
        };
        (first, last)
    }
}
