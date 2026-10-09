//! The breaker (CSS Fragmentation 3 §4.4, CSS Multi-column 1 §7.1) over
//! hand-made breaks: which break ends a fragmentainer, the rules relaxed
//! in order, forced breaks, overflow, and balancing's bounded cost.

use super::breaker::{BREAKER_RUNS, slices};
use super::{Break, Fill, RULE_1, RULE_2, RULE_3, fragmentainers};

/// `n` one-row lines from row 0, a break between each two.
fn lines(n: i32) -> Vec<Break> {
    (1..n)
        .map(|k| Break {
            end: k,
            resume: k,
            forced: false,
            violates: 0,
        })
        .collect()
}

/// §4.4: a fragmentainer takes the content up to the last break that fits.
#[test]
fn a_fragmentainer_ends_at_the_last_break_that_fits() {
    assert_eq!(slices(&lines(7), 0, 7, 3), [(0, 3), (3, 6), (6, 7)]);
    // The rest fits: no break.
    assert_eq!(slices(&lines(3), 0, 3, 5), [(0, 3)]);
}

/// §3.1: a forced break ends the fragmentainer early, wherever it is.
#[test]
fn a_forced_break_ends_the_fragmentainer() {
    let mut b = lines(6);
    b[0].forced = true;
    assert_eq!(slices(&b, 0, 6, 4), [(0, 1), (1, 5), (5, 6)]);
}

/// §4.4: the rules are dropped in reverse order — orphans / widows first,
/// then `break-inside: avoid`, then `break-before` / `-after: avoid`.
#[test]
fn the_rules_relax_in_reverse_order() {
    let mut b = lines(6);
    b[2].violates = RULE_3; // a break after row 3
    b[1].violates = RULE_2; // after row 2
    b[0].violates = RULE_1; // after row 1
    // Height 3: the break after row 3 breaks only rule 3 — taken before the
    // rule-2 and rule-1 ones.
    assert_eq!(slices(&b, 0, 6, 3)[0], (0, 3));
    b[2].violates = RULE_1;
    // Now rule 2's (after row 2) is the least important broken.
    assert_eq!(slices(&b, 0, 6, 3)[0], (0, 2));
    b[1].violates = RULE_1;
    b[0].violates = RULE_2;
    assert_eq!(slices(&b, 0, 6, 3)[0], (0, 1));
}

/// §4.2: content taller than the fragmentainer with no break inside it
/// overflows to the first break after it; an unforced break drops the
/// margin before the next box (§5.2: `resume` past it).
#[test]
fn monolithic_content_overflows_and_margins_truncate() {
    let b = [
        Break {
            end: 5,
            resume: 7,
            forced: false,
            violates: 0,
        },
        Break {
            end: 8,
            resume: 8,
            forced: false,
            violates: 0,
        },
    ];
    assert_eq!(slices(&b, 0, 9, 3), [(0, 5), (7, 9)]);
}

/// Multi-column 1 §7.1: balancing finds the least height that fits the
/// count — six lines over three columns are two each — in
/// `⌈log₂ h⌉ + 1` breaker runs (here 6 rows: at most 4, plus the result
/// run), never one per height.
#[test]
fn balancing_is_a_bounded_search() {
    BREAKER_RUNS.with(|c| c.set(0));
    let (rows, h) = fragmentainers(
        &lines(6),
        0,
        6,
        Fill::Balance {
            count: 3,
            cap: None,
        },
    );
    assert_eq!((rows, h), (vec![(0, 2), (2, 4), (4, 6)], 2));
    let runs = BREAKER_RUNS.with(|c| c.get());
    assert!(runs <= 5, "{runs} breaker runs");
    // A thousand rows balance in at most ⌈log₂ 1000⌉ + 2 runs.
    BREAKER_RUNS.with(|c| c.set(0));
    let (_, h) = fragmentainers(
        &lines(1000),
        0,
        1000,
        Fill::Balance {
            count: 3,
            cap: None,
        },
    );
    assert_eq!(h, 334);
    let runs = BREAKER_RUNS.with(|c| c.get());
    assert!(runs <= 12, "{runs} breaker runs");
}

/// §7.1: a balanced height past the cap fills sequentially at the cap; a
/// monolithic piece taller than the balanced share makes the columns as
/// tall as it (no column overflows when balancing).
#[test]
fn balancing_respects_the_cap_and_monolithic_pieces() {
    let (rows, h) = fragmentainers(
        &lines(9),
        0,
        9,
        Fill::Balance {
            count: 2,
            cap: Some(3),
        },
    );
    assert_eq!((rows, h), (vec![(0, 3), (3, 6), (6, 9)], 3));
    // A 4-row piece, then two lines: two columns, four rows each.
    let b = [
        Break {
            end: 4,
            resume: 4,
            forced: false,
            violates: 0,
        },
        Break {
            end: 5,
            resume: 5,
            forced: false,
            violates: 0,
        },
    ];
    let (rows, h) = fragmentainers(
        &b,
        0,
        6,
        Fill::Balance {
            count: 2,
            cap: None,
        },
    );
    assert_eq!((rows, h), (vec![(0, 4), (4, 6)], 4));
}
