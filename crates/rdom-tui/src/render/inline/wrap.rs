//! `text-wrap-style` (CSS Text 4 "Selecting How to Wrap"): how a block
//! container chooses among its soft wrap opportunities.
//!
//! - `auto` and `stable` are the packer's greedy breaking: each line takes
//!   what fits, never looking at later content — which is exactly
//!   `stable`'s "content on subsequent lines should not be considered".
//! - `balance` evens out each group of lines between forced breaks (the
//!   spec: "processed separately") of 2 to [`MAX_BALANCED_LINES`] lines: the
//!   narrowest cap on the group's lines that keeps its greedy line count,
//!   found by bisection. All groups are searched together, one replay of
//!   the content per bisection step, so the cost is the content's length
//!   times about log2 of the line width — never quadratic in the text or
//!   in the number of groups.
//! - `pretty` and `avoid-short-last-line`: when the block's last line holds
//!   one word (no space) and the line before holds more than one, cap that
//!   line one cell short of its content so its last word moves down — kept
//!   only when the line count does not change. One replay.
//!
//! Both replay the packer's intake log (`packer::Op`) into replicas with
//! their lines capped; a paragraph beside floats (a float in it, or a line
//! a float shortened) keeps its greedy breaks, as a replay would place its
//! floats again.

use super::LineBox;
use super::packer::{LinePacker, WidthCaps};
use crate::layout::TextWrapStyle;

/// The most lines a group may have to be balanced: Chromium's limit (the
/// spec lets a UA treat `balance` as `auto` past ten).
pub(crate) const MAX_BALANCED_LINES: usize = 6;

/// Pack the content `fill` feeds into `packer` under `style`, and return the
/// lines.
pub(super) fn pack<'a>(
    mut packer: LinePacker<'a>,
    style: TextWrapStyle,
    fill: impl FnOnce(&mut LinePacker<'a>),
) -> Vec<LineBox> {
    let tries = matches!(
        style,
        TextWrapStyle::Balance | TextWrapStyle::Pretty | TextWrapStyle::AvoidShortLastLine
    ) && !packer.is_measuring();
    if tries {
        packer = packer.recording();
    }
    fill(&mut packer);
    packer.finish();
    let greedy = packer.take_lines();
    if !tries || packer.met_float() || greedy.iter().any(|l| l.band.is_some()) {
        return greedy;
    }
    let ops = packer.take_ops();
    let replay = |caps: WidthCaps| {
        let mut replica = packer.replica(caps);
        replica.replay(&ops);
        replica.finish();
        let groups = replica.line_groups().to_vec();
        (replica.take_lines(), groups)
    };
    match style {
        TextWrapStyle::Balance => {
            balance(greedy, packer.line_groups(), packer.content_width(), replay)
        }
        _ => pretty(greedy, replay),
    }
}

/// `balance`: the narrowest cap per group that keeps its line count.
fn balance(
    greedy: Vec<LineBox>,
    groups: &[usize],
    width: u16,
    replay: impl Fn(WidthCaps) -> (Vec<LineBox>, Vec<usize>),
) -> Vec<LineBox> {
    let n_groups = groups.iter().max().map_or(0, |g| g + 1);
    let target = histogram(groups, n_groups);
    // The groups to balance, each with its bisection range [lo, hi]: `hi`
    // keeps the count (the line box width does, greedily).
    let mut range: Vec<Option<(u16, u16)>> = target
        .iter()
        .map(|&n| (2..=MAX_BALANCED_LINES).contains(&n).then_some((1, width)))
        .collect();
    if range.iter().all(Option::is_none) {
        return greedy;
    }
    loop {
        let mids: Vec<u16> = range
            .iter()
            .map(|r| match r {
                Some((lo, hi)) if lo < hi => lo + (hi - lo) / 2,
                Some((_, hi)) => *hi,
                None => u16::MAX,
            })
            .collect();
        if range.iter().flatten().all(|(lo, hi)| lo >= hi) {
            let (lines, _) = replay(WidthCaps {
                groups: mids,
                line: None,
            });
            return lines;
        }
        let (_, got) = replay(WidthCaps {
            groups: mids.clone(),
            line: None,
        });
        let got = histogram(&got, n_groups);
        for (g, r) in range.iter_mut().enumerate() {
            if let Some((lo, hi)) = r
                && *lo < *hi
            {
                if got[g] <= target[g] {
                    *hi = mids[g];
                } else {
                    *lo = mids[g] + 1;
                }
            }
        }
    }
}

/// The number of lines in each of the `n` groups, from each line's group
/// — one pass (a count per group was a pass per group).
fn histogram(groups: &[usize], n: usize) -> Vec<usize> {
    #[cfg(test)]
    COUNTED.with(|c| c.set(c.get() + groups.len()));
    let mut lines = vec![0; n];
    for &g in groups {
        if let Some(count) = lines.get_mut(g) {
            *count += 1;
        }
    }
    lines
}

/// `pretty` / `avoid-short-last-line`: move the previous line's last word
/// down to a one-word last line, keeping the line count.
fn pretty(
    greedy: Vec<LineBox>,
    replay: impl Fn(WidthCaps) -> (Vec<LineBox>, Vec<usize>),
) -> Vec<LineBox> {
    let n = greedy.len();
    if n < 2 {
        return greedy;
    }
    let (last, before) = (&greedy[n - 1], &greedy[n - 2]);
    if has_space(last) || !has_space(before) {
        return greedy;
    }
    let content = before.width - before.hang;
    let (lines, _) = replay(WidthCaps {
        groups: Vec::new(),
        line: Some((n - 2, content.saturating_sub(1))),
    });
    if lines.len() == n { lines } else { greedy }
}

/// Whether a line's text holds a word separator between its words.
fn has_space(line: &LineBox) -> bool {
    let text: String = line
        .fragments
        .iter()
        .map(|f| f.text.as_str())
        .chain(line.generated.iter().map(|g| g.text.as_str()))
        .collect();
    text.trim().contains(' ')
}

#[cfg(test)]
thread_local! {
    /// Lines visited counting `balance`'s groups (cost tests).
    pub(super) static COUNTED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
#[path = "wrap_cost_tests.rs"]
mod cost_tests;
