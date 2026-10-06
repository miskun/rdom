//! C9-TEXT-WRAP — `text-wrap` (the shorthand of `text-wrap-mode` and
//! `text-wrap-style`) and `text-wrap-style: auto | balance | stable |
//! pretty | avoid-short-last-line` (CSS Text 4 §5.3–§5.4 "Joint Wrapping
//! Control", "Selecting How to Wrap").

use super::{paint_text, rows, text_block};

fn lines(decl: &str, text: &str, w: u16, h: u16) -> Vec<String> {
    paint_text(&format!("width: {w}; {decl}"), text, w, h)
}

/// `balance`: "line breaks are chosen to balance the remaining (empty)
/// space in each line box" without changing the number of line boxes —
/// the narrowest width that keeps the line count.
#[test]
fn balance_evens_out_the_lines() {
    assert_eq!(
        lines("", "aa bb cc dd ee ff", 14, 2),
        ["aa bb cc dd ee", "ff            "]
    );
    assert_eq!(
        lines("text-wrap: balance", "aa bb cc dd ee ff", 14, 2),
        ["aa bb cc      ", "dd ee ff      "]
    );
    assert_eq!(
        lines("text-wrap-style: balance", "aa bb cc dd ee ff", 14, 2),
        ["aa bb cc      ", "dd ee ff      "]
    );
}

/// "Groups of lines separated by a forced line break are processed
/// separately"; "UAs may treat this value as auto if there are more than
/// ten lines" — rdom balances groups of up to six lines, as Chromium does.
#[test]
fn balance_works_per_group_and_up_to_six_lines() {
    assert_eq!(
        lines(
            "text-wrap: balance; white-space: pre-line",
            "aa bb cc dd ee ff\nx",
            14,
            3
        ),
        ["aa bb cc      ", "dd ee ff      ", "x             "]
    );
    // Six lines balance (the narrowest width that keeps six is 7); seven
    // keep the greedy breaks, though a narrower width would keep seven.
    let six = "xxx x xx xx xxx xxx xx xxx xx xxx xx xxx";
    assert_eq!(
        lines("text-wrap: balance", six, 9, 6),
        [
            "xxx x    ",
            "xx xx    ",
            "xxx xxx  ",
            "xx xxx   ",
            "xx xxx   ",
            "xx xxx   "
        ]
    );
    let seven = "xxx xx xx xx xxx xx x x xxx xx xx xxx x xxx";
    assert_eq!(
        lines("text-wrap: balance", seven, 7, 7),
        lines("", seven, 7, 7)
    );
}

/// `pretty` / `avoid-short-last-line`: rdom's rule — a last line holding
/// one word takes the previous line's last word when that line has more
/// than one and the line count does not change.
#[test]
fn pretty_avoids_a_one_word_last_line() {
    assert_eq!(
        lines("", "aaa bbb ccc ddd", 11, 2),
        ["aaa bbb ccc", "ddd        "]
    );
    for decl in [
        "text-wrap: pretty",
        "text-wrap-style: avoid-short-last-line",
    ] {
        assert_eq!(
            lines(decl, "aaa bbb ccc ddd", 11, 2),
            ["aaa bbb    ", "ccc ddd    "],
            "{decl}"
        );
    }
    assert_eq!(
        lines("text-wrap: pretty", "aaaaaaaaaa bbb", 10, 2),
        ["aaaaaaaaaa", "bbb       "]
    );
    // Moving `bbbbbb` down would push `cccccc` to a third line: kept.
    assert_eq!(
        lines("text-wrap: pretty", "aaaa bbbbbb cccccc", 11, 3),
        lines("", "aaaa bbbbbb cccccc", 11, 3)
    );
}

/// `stable`: "content on subsequent lines should not be considered when
/// making break decisions so that when editing text any content before
/// the cursor remains stable" — adding text at the end leaves the first
/// line as it was; `balance` reflows it.
#[test]
fn stable_keeps_earlier_lines_when_text_is_added() {
    let first_row = |decl: &str, text: &str| {
        let (mut dom, _, _) = text_block(text);
        let css = format!(".b {{ width: 14; {decl} }}");
        let buf = super::paint(&mut dom, &css, 14, 3);
        rows(&buf, 14, 3).remove(0)
    };
    for decl in ["text-wrap: stable", "text-wrap: auto"] {
        assert_eq!(
            first_row(decl, "aa bb cc dd ee ff"),
            first_row(decl, "aa bb cc dd ee ff gg"),
            "{decl}"
        );
    }
    assert_ne!(
        first_row("text-wrap: balance", "aa bb cc dd ee ff"),
        first_row("text-wrap: balance", "aa bb cc dd ee ff gg")
    );
}

/// `text-wrap: nowrap` sets `text-wrap-mode` (Text 4: the shorthand of
/// both): nothing wraps, nothing to balance.
#[test]
fn text_wrap_nowrap_does_not_wrap() {
    assert_eq!(
        lines("text-wrap: nowrap balance", "aa bb cc", 5, 1),
        ["aa bb"]
    );
}
