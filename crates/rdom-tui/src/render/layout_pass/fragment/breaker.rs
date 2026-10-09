//! Choosing the breaks (CSS Fragmentation 3 §4.4) for a fragmentainer
//! height, and the height that balances a flow over a number of
//! fragmentainers (CSS Multi-column 1 §7.1).
//!
//! A fragmentainer takes the content up to the last break that fits it,
//! a forced break before that ending it early (§3.1). When no break fits
//! the rules are relaxed in §4.4's order — "rule 3 is dropped … then
//! rules 1, 2 and 4 are dropped in order": `orphans` / `widows` first,
//! then `break-before` / `-after: avoid`, then `break-inside: avoid` at a
//! class A point, then at a class B or C one — then a break inside a
//! float (rdom keeps floats whole); and when none fits at all the content
//! overflows to the first break after it (§4.2: a monolithic box taller
//! than the fragmentainer).
//!
//! Balancing is a binary search for the least height whose fragments are
//! no more than the count and none overflows: `⌈log₂ h⌉ + 1` runs of the
//! breaker over the flow's `b` breaks for a flow `h` rows tall — `O(b log
//! h)`, pinned by `fragment::tests`.

use super::{Break, FLOAT, RULE_1, RULE_2, RULE_3, RULE_4};

#[cfg(test)]
thread_local! {
    /// Runs of [`slices`] (cost tests).
    pub(in crate::render::layout_pass) static BREAKER_RUNS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// Breaks the forced-break scan looked at (cost tests).
    pub(in crate::render::layout_pass) static FORCED_SCANS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// How a flow fills its fragmentainers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::render::layout_pass) enum Fill {
    /// One after another, each this many rows (`column-fill: auto` under
    /// a constrained height): what does not fit goes on to more.
    Sequential(u16),
    /// Balanced over `count` fragmentainers (`column-fill: balance`): the
    /// least height that fits the flow in them, no taller than `cap`
    /// (a constrained height) — past it, sequentially at the cap.
    Balance { count: u16, cap: Option<u16> },
}

/// One fragmentainer's piece of a flow: its rows `start .. end`, and the
/// rows a `box-decoration-break: clone` box split at its edges adds — `lead`
/// above them (the cloned top edges of the boxes the break before it is
/// inside), `tail` below (their bottom edges at the break after it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::render::layout_pass) struct Frag {
    pub(in crate::render::layout_pass) start: i32,
    pub(in crate::render::layout_pass) end: i32,
    pub(in crate::render::layout_pass) lead: u16,
    pub(in crate::render::layout_pass) tail: u16,
}

impl Frag {
    /// The rows it takes in its fragmentainer.
    pub(in crate::render::layout_pass) fn height(&self) -> i32 {
        i32::from(self.lead) + (self.end - self.start) + i32::from(self.tail)
    }
}

/// The fragmentainers of the flow whose rows run `start .. end`, with
/// `breaks` its possible breaks by end row, filled by `fill`: each one's
/// piece, and the fragmentainer height used — the balanced height, or the
/// sequential one.
pub(in crate::render::layout_pass) fn fragmentainers(
    breaks: &[Break],
    start: i32,
    end: i32,
    fill: Fill,
) -> (Vec<Frag>, u16) {
    match fill {
        Fill::Sequential(h) => (slices(breaks, start, end, h), h),
        Fill::Balance { count, cap } => {
            let h = balanced(breaks, start, end, count);
            match cap {
                Some(cap) if h > cap => (slices(breaks, start, end, cap), cap),
                _ => {
                    let s = slices(breaks, start, end, h);
                    let used = s.iter().map(Frag::height).max().unwrap_or(0);
                    (s, used.clamp(0, i32::from(u16::MAX)) as u16)
                }
            }
        }
    }
}

/// The least height at which the flow fits `count` fragmentainers (or as
/// few as its forced breaks allow) with none overflowing.
fn balanced(breaks: &[Break], start: i32, end: i32, count: u16) -> u16 {
    let total = (end - start).clamp(0, i32::from(u16::MAX)) as u16;
    if total == 0 {
        return 0;
    }
    // One fragmentainer per forced break at the full height — always
    // feasible but where cloned box edges make a fragment taller than the
    // flow, the most the count can be held to.
    let whole = slices(breaks, start, end, total);
    let target = whole.len().max(usize::from(count.max(1)));
    let fits = |h: u16| {
        let s = slices(breaks, start, end, h);
        s.len() <= target && s.iter().all(|f| f.height() <= i32::from(h))
    };
    let mut lo = total
        .div_ceil(target.min(usize::from(u16::MAX)) as u16)
        .max(1);
    let mut hi = whole
        .iter()
        .map(Frag::height)
        .max()
        .unwrap_or(0)
        .clamp(i32::from(total), i32::from(u16::MAX)) as u16;
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if fits(mid) {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    hi
}

/// The pieces of each fragmentainer `h` rows tall the flow `start .. end`
/// fills, one after another.
pub(super) fn slices(breaks: &[Break], start: i32, end: i32, h: u16) -> Vec<Frag> {
    #[cfg(test)]
    BREAKER_RUNS.with(|c| c.set(c.get() + 1));
    let mut out = Vec::new();
    let mut s = start;
    let mut lead = 0;
    let mut from = 0;
    while s < end {
        // The cloned edges above the content come out of the height.
        let limit = s
            .saturating_add(i32::from(h))
            .saturating_sub(i32::from(lead));
        // The breaks after the fragmentainer's start.
        from += breaks[from..].partition_point(|b| b.end <= s);
        match pick(&breaks[from..], s, limit, end) {
            Some(b) if b.resume < end => {
                out.push(Frag {
                    start: s,
                    end: b.end,
                    lead,
                    tail: b.tail,
                });
                s = b.resume.max(b.end);
                lead = b.lead;
            }
            Some(b) => {
                out.push(Frag {
                    start: s,
                    end: b.end.max(s),
                    lead,
                    tail: b.tail,
                });
                return out;
            }
            None => break,
        }
    }
    out.push(Frag {
        start: s,
        end: end.max(s),
        lead,
        tail: 0,
    });
    out
}

/// The break ending the fragmentainer that starts at `s` and fits up to
/// `limit` (§4.4) — its cloned edges below it included — among `breaks`
/// (all ending after `s`); `None` when the rest of the flow, ending at
/// `end`, fits — or has no break left.
fn pick(breaks: &[Break], s: i32, limit: i32, end: i32) -> Option<Break> {
    let fits = |b: &Break| b.end + i32::from(b.tail) <= limit;
    // The breaks this fragmentainer can end at: the forced-break scan and
    // the relaxation look at these only, so a run is linear in the breaks
    // (C15G-MULTICOL-COST).
    let fitting = &breaks[..breaks.partition_point(|b| b.end <= limit)];
    // A forced break that fits ends it (§3.1).
    if let Some(b) = fitting
        .iter()
        .inspect(|_| {
            #[cfg(test)]
            FORCED_SCANS.with(|c| c.set(c.get() + 1));
        })
        .find(|b| b.forced)
        && b.end <= end
        && fits(b)
    {
        return Some(*b);
    }
    if end <= limit {
        return None;
    }
    // Rules dropped in §4.4's order: 3, then 1, 2 and 4; a float last.
    const RELAXED: [u8; 6] = [
        0,
        RULE_3,
        RULE_3 | RULE_1,
        RULE_3 | RULE_1 | RULE_2,
        RULE_3 | RULE_1 | RULE_2 | RULE_4,
        RULE_3 | RULE_1 | RULE_2 | RULE_4 | FLOAT,
    ];
    for allowed in RELAXED {
        if let Some(b) = fitting
            .iter()
            .rev()
            .find(|b| b.end > s && fits(b) && b.violates & !allowed == 0)
        {
            return Some(*b);
        }
    }
    // Nothing fits: overflow to the first break after it (§4.2).
    breaks.iter().find(|b| b.end > s).copied()
}
