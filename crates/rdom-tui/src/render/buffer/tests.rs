//! Unit + property tests for [`Buffer`](super::Buffer): construction,
//! Unicode-width-aware writes, clipping, bulk operations, and diffing.

use super::*;
use crate::render::{CellDiff, Color, Modifier, Style};
use proptest::prelude::*;

fn ascii_style() -> Style {
    Style::new()
        .fg(Color::Rgb(255, 0, 0))
        .add_modifier(Modifier::BOLD)
}

// ── Construction + indexing ──────────────────────────────────────

#[test]
fn empty_has_correct_cell_count() {
    let b = Buffer::empty(Rect::new(0, 0, 10, 5));
    assert_eq!(b.content.len(), 50);
    assert!(b.content.iter().all(|c| *c == Cell::EMPTY));
}

#[test]
fn filled_has_expected_cell() {
    let mut c = Cell::new("X");
    c.fg = Color::Rgb(0, 0, 255);
    let b = Buffer::filled(Rect::new(0, 0, 3, 2), c.clone());
    for cell in &b.content {
        assert_eq!(*cell, c);
    }
}

#[test]
fn index_of_inside_and_outside() {
    let b = Buffer::empty(Rect::new(10, 20, 5, 4));
    assert_eq!(b.index_of(10, 20), Some(0));
    assert_eq!(b.index_of(14, 23), Some(19)); // last cell
    assert_eq!(b.index_of(9, 20), None); // left of
    assert_eq!(b.index_of(15, 20), None); // right edge (exclusive)
    assert_eq!(b.index_of(10, 24), None); // below
}

#[test]
fn cell_returns_copy_of_initial_empty() {
    let b = Buffer::empty(Rect::new(0, 0, 3, 3));
    assert_eq!(b.cell(0, 0), Some(&Cell::EMPTY));
    assert_eq!(b.cell(99, 99), None);
}

// ── set_string (Unicode-width-aware) ─────────────────────────────

#[test]
fn ascii_set_string() {
    let mut b = Buffer::empty(Rect::new(0, 0, 10, 1));
    let end = b.set_string(0, 0, "hello", ascii_style());
    assert_eq!(end, (5, 0));
    assert_eq!(b.cell(0, 0).unwrap().symbol(), "h");
    assert_eq!(b.cell(4, 0).unwrap().symbol(), "o");
    assert_eq!(b.cell(0, 0).unwrap().fg, Color::Rgb(255, 0, 0));
}

#[test]
fn control_chars_are_skipped_not_written_as_cells() {
    // The `unicode-width` crate reports width 1 for LF/TAB/CR, but
    // writing a control char into a cell would emit raw \n / \t into
    // the ANSI stream and move the terminal cursor. set_stringn must
    // treat control chars as width-0 and skip them entirely.
    let mut b = Buffer::empty(Rect::new(0, 0, 10, 1));
    let end = b.set_string(0, 0, "\n  \n\t\r  AB", ascii_style());
    // Cursor advanced by the 4 space/ASCII graphemes that survived
    // plus AB = 2 + 2 + 2 = 6. Control chars did not advance it.
    assert_eq!(end, (6, 0));
    assert_eq!(b.cell(0, 0).unwrap().symbol(), " ");
    assert_eq!(b.cell(1, 0).unwrap().symbol(), " ");
    assert_eq!(b.cell(2, 0).unwrap().symbol(), " ");
    assert_eq!(b.cell(3, 0).unwrap().symbol(), " ");
    assert_eq!(b.cell(4, 0).unwrap().symbol(), "A");
    assert_eq!(b.cell(5, 0).unwrap().symbol(), "B");
}

#[test]
fn cjk_advances_by_two_cells_not_three_bytes() {
    // THE Unicode-width invariant. "中" is 3 bytes UTF-8, 1 codepoint,
    // 1 grapheme, 2 cells wide. Cursor must end at x=2, not x=3
    // (bytes) and not x=1 (codepoints).
    let mut b = Buffer::empty(Rect::new(0, 0, 10, 1));
    let end = b.set_string(0, 0, "中", ascii_style());
    assert_eq!(end, (2, 0), "CJK must advance by visible cell width");
    assert_eq!(b.cell(0, 0).unwrap().symbol(), "中");
    assert!(b.cell(1, 0).unwrap().is_spacer());
}

#[test]
fn emoji_cursor_advance() {
    let mut b = Buffer::empty(Rect::new(0, 0, 10, 1));
    let end = b.set_string(0, 0, "🦀A", ascii_style());
    // 🦀 (width 2) + "A" (width 1) = 3 cells.
    assert_eq!(end, (3, 0));
    assert_eq!(b.cell(0, 0).unwrap().symbol(), "🦀");
    assert!(b.cell(1, 0).unwrap().is_spacer());
    assert_eq!(b.cell(2, 0).unwrap().symbol(), "A");
}

#[test]
fn zwj_family_is_one_grapheme() {
    let mut b = Buffer::empty(Rect::new(0, 0, 10, 1));
    let fam = "👨\u{200D}👩\u{200D}👧"; // family emoji
    let end = b.set_string(0, 0, fam, ascii_style());
    assert_eq!(end, (2, 0), "ZWJ family must be one wide glyph");
    // The primary cell carries the full ZWJ sequence.
    assert_eq!(b.cell(0, 0).unwrap().symbol(), fam);
    assert!(b.cell(1, 0).unwrap().is_spacer());
}

#[test]
fn combining_mark_folds_into_grapheme() {
    // "é" as "e\u{0301}" is 2 codepoints but 1 grapheme, width 1.
    let mut b = Buffer::empty(Rect::new(0, 0, 10, 1));
    let end = b.set_string(0, 0, "e\u{0301}f", ascii_style());
    assert_eq!(end, (2, 0));
    assert_eq!(b.cell(0, 0).unwrap().symbol(), "e\u{0301}");
    assert_eq!(b.cell(1, 0).unwrap().symbol(), "f");
}

#[test]
fn regional_indicator_flag_is_one_grapheme_width_2() {
    let mut b = Buffer::empty(Rect::new(0, 0, 10, 1));
    let end = b.set_string(0, 0, "🇺🇸", ascii_style());
    assert_eq!(end, (2, 0));
    assert_eq!(b.cell(0, 0).unwrap().symbol(), "🇺🇸");
    assert!(b.cell(1, 0).unwrap().is_spacer());
}

#[test]
fn mixed_ascii_cjk_emoji() {
    let mut b = Buffer::empty(Rect::new(0, 0, 20, 1));
    // "A" (1) + "中" (2) + "🦀" (2) + "B" (1) = 6 cells
    let end = b.set_string(0, 0, "A中🦀B", ascii_style());
    assert_eq!(end, (6, 0));
    assert_eq!(b.cell(0, 0).unwrap().symbol(), "A");
    assert_eq!(b.cell(1, 0).unwrap().symbol(), "中");
    assert!(b.cell(2, 0).unwrap().is_spacer());
    assert_eq!(b.cell(3, 0).unwrap().symbol(), "🦀");
    assert!(b.cell(4, 0).unwrap().is_spacer());
    assert_eq!(b.cell(5, 0).unwrap().symbol(), "B");
}

// ── Clipping / truncation ────────────────────────────────────────

#[test]
fn clip_past_right_edge_stops() {
    let mut b = Buffer::empty(Rect::new(0, 0, 5, 1));
    let end = b.set_string(0, 0, "0123456789", ascii_style());
    assert_eq!(end, (5, 0));
    assert_eq!(b.cell(4, 0).unwrap().symbol(), "4");
    // No cell past the edge.
    assert_eq!(b.cell(5, 0), None);
}

#[test]
fn wide_glyph_clipped_at_right_edge_emits_ellipsis() {
    // Buffer 5 wide; write "中中中" (6 cells requested). Only 2 full
    // wide glyphs fit (4 cells) + 1 cell left → the last wide glyph
    // gets replaced with `…`.
    let mut b = Buffer::empty(Rect::new(0, 0, 5, 1));
    let end = b.set_string(0, 0, "中中中", ascii_style());
    assert_eq!(end, (5, 0));
    assert_eq!(b.cell(0, 0).unwrap().symbol(), "中");
    assert_eq!(b.cell(2, 0).unwrap().symbol(), "中");
    assert_eq!(b.cell(4, 0).unwrap().symbol(), "…");
}

#[test]
fn set_stringn_enforces_max_width() {
    let mut b = Buffer::empty(Rect::new(0, 0, 20, 1));
    let end = b.set_stringn(0, 0, "hello world", 5, ascii_style());
    assert_eq!(end, (5, 0));
    assert_eq!(b.cell(4, 0).unwrap().symbol(), "o");
    assert_eq!(b.cell(5, 0).unwrap().symbol(), " "); // untouched blank
}

#[test]
fn row_out_of_range_is_noop() {
    let mut b = Buffer::empty(Rect::new(0, 0, 5, 1));
    let end = b.set_string(0, 5, "hello", ascii_style());
    assert_eq!(end, (0, 5));
    assert_eq!(b.cell(0, 0).unwrap().symbol(), " "); // untouched
}

#[test]
fn x_past_right_edge_is_noop() {
    let mut b = Buffer::empty(Rect::new(0, 0, 5, 1));
    let end = b.set_string(99, 0, "hello", ascii_style());
    assert_eq!(end, (99, 0));
}

// ── Style writes ─────────────────────────────────────────────────

#[test]
fn set_style_preserves_symbol() {
    let mut b = Buffer::empty(Rect::new(0, 0, 5, 1));
    b.set_string(0, 0, "hi", Style::new());
    b.set_style(0, 0, Style::new().fg(Color::Rgb(255, 0, 0)));
    assert_eq!(b.cell(0, 0).unwrap().symbol(), "h");
    assert_eq!(b.cell(0, 0).unwrap().fg, Color::Rgb(255, 0, 0));
}

#[test]
fn set_char_writes_single_codepoint() {
    let mut b = Buffer::empty(Rect::new(0, 0, 3, 1));
    b.set_char(1, 0, 'X', Style::new().fg(Color::Rgb(0, 128, 0)));
    assert_eq!(b.cell(1, 0).unwrap().symbol(), "X");
    assert_eq!(b.cell(1, 0).unwrap().fg, Color::Rgb(0, 128, 0));
}

// ── fill ─────────────────────────────────────────────────────────

#[test]
fn fill_overwrites_region() {
    let mut b = Buffer::empty(Rect::new(0, 0, 5, 3));
    b.fill(Rect::new(1, 1, 3, 1), Cell::new("#"));
    assert_eq!(b.cell(0, 0).unwrap().symbol(), " ");
    assert_eq!(b.cell(1, 1).unwrap().symbol(), "#");
    assert_eq!(b.cell(2, 1).unwrap().symbol(), "#");
    assert_eq!(b.cell(3, 1).unwrap().symbol(), "#");
    assert_eq!(b.cell(4, 1).unwrap().symbol(), " ");
    assert_eq!(b.cell(1, 0).unwrap().symbol(), " ");
}

#[test]
fn fill_clips_to_buffer() {
    let mut b = Buffer::empty(Rect::new(0, 0, 3, 3));
    // Overflow past the right — should not panic, only cells 0..2 in x get filled.
    b.fill(Rect::new(0, 0, 100, 100), Cell::new("#"));
    for x in 0..3 {
        for y in 0..3 {
            assert_eq!(b.cell(x, y).unwrap().symbol(), "#");
        }
    }
}

// ── clear ────────────────────────────────────────────────────────

#[test]
fn clear_restores_all_empty() {
    let mut b = Buffer::empty(Rect::new(0, 0, 3, 3));
    b.set_string(0, 0, "hello", Style::new().fg(Color::Rgb(255, 0, 0)));
    b.clear();
    for c in &b.content {
        assert_eq!(*c, Cell::EMPTY);
    }
}

// ── merge ────────────────────────────────────────────────────────

#[test]
fn merge_copies_overlapping_cells() {
    let mut dst = Buffer::empty(Rect::new(0, 0, 5, 3));
    let mut src = Buffer::empty(Rect::new(1, 1, 3, 1));
    src.set_string(1, 1, "foo", Style::new());
    dst.merge(&src);
    assert_eq!(dst.cell(1, 1).unwrap().symbol(), "f");
    assert_eq!(dst.cell(2, 1).unwrap().symbol(), "o");
    assert_eq!(dst.cell(3, 1).unwrap().symbol(), "o");
    assert_eq!(dst.cell(0, 0).unwrap().symbol(), " "); // untouched
}

#[test]
fn merge_non_overlapping_is_noop() {
    let before = Buffer::empty(Rect::new(0, 0, 3, 3));
    let mut dst = before.clone();
    let src = Buffer::filled(Rect::new(100, 100, 3, 3), Cell::new("X"));
    dst.merge(&src);
    assert_eq!(dst, before);
}

// ── resize ───────────────────────────────────────────────────────

#[test]
fn resize_preserves_intersection() {
    let mut b = Buffer::empty(Rect::new(0, 0, 5, 3));
    b.set_string(0, 0, "hello", Style::new().fg(Color::Rgb(255, 0, 0)));
    let changed = b.resize(Rect::new(0, 0, 3, 5));
    assert!(changed);
    // "hel" preserved; "lo" dropped.
    assert_eq!(b.cell(0, 0).unwrap().symbol(), "h");
    assert_eq!(b.cell(2, 0).unwrap().symbol(), "l");
    assert_eq!(b.cell(2, 4).unwrap().symbol(), " "); // new cell, empty
}

#[test]
fn resize_identity_returns_false() {
    let mut b = Buffer::empty(Rect::new(0, 0, 5, 3));
    assert!(!b.resize(Rect::new(0, 0, 5, 3)));
}

// ── diff ─────────────────────────────────────────────────────────

#[test]
fn diff_empty_on_identical() {
    let a = Buffer::empty(Rect::new(0, 0, 10, 5));
    let b = Buffer::empty(Rect::new(0, 0, 10, 5));
    assert_eq!(a.diff_iter(&b).count(), 0);
}

#[test]
fn diff_yields_changed_cells() {
    let a = Buffer::empty(Rect::new(0, 0, 5, 1));
    let mut b = a.clone();
    b.set_string(2, 0, "ab", Style::new());
    let diffs: Vec<_> = b
        .diff_iter(&a)
        .map(|(x, y, c)| (x, y, c.symbol().to_string()))
        .collect();
    assert_eq!(diffs, vec![(2, 0, "a".into()), (3, 0, "b".into())]);
}

#[test]
fn diff_skips_spacer_cells() {
    let a = Buffer::empty(Rect::new(0, 0, 5, 1));
    let mut b = a.clone();
    // "中" writes primary at 0 + spacer at 1.
    b.set_string(0, 0, "中", Style::new());
    let diffs: Vec<_> = b.diff_iter(&a).collect();
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].0, 0);
    assert_eq!(diffs[0].2.symbol(), "中");
}

#[test]
fn diff_honors_skip_flag() {
    let a = Buffer::empty(Rect::new(0, 0, 3, 1));
    let mut b = a.clone();
    b.set_string(0, 0, "abc", Style::new());
    b.cell_mut(1, 0).unwrap().diff = CellDiff::Skip;
    let positions: Vec<_> = b.diff_iter(&a).map(|(x, _, _)| x).collect();
    assert_eq!(positions, vec![0, 2]);
}

#[test]
fn diff_honors_always_update() {
    let a = Buffer::empty(Rect::new(0, 0, 3, 1));
    let mut b = a.clone();
    // Cells are identical to `a` but we force `AlwaysUpdate` on one.
    b.cell_mut(1, 0).unwrap().diff = CellDiff::AlwaysUpdate;
    let positions: Vec<_> = b.diff_iter(&a).map(|(x, _, _)| x).collect();
    assert_eq!(positions, vec![1]);
}

// ── Property tests ───────────────────────────────────────────────

proptest! {
    #[test]
    fn set_string_cursor_matches_unicode_width(s in r"[A-Za-z0-9\u{4e00}-\u{9fff}\u{1f300}-\u{1f5ff} ]{0,20}") {
        let mut b = Buffer::empty(Rect::new(0, 0, 100, 1));
        let expected = unicode_width::UnicodeWidthStr::width(s.as_str()) as u16;
        let (end_x, _) = b.set_string(0, 0, &s, Style::new());
        prop_assert_eq!(
            end_x,
            expected.min(100),
            "cursor advance must equal UnicodeWidthStr::width, not s.len() or s.chars().count()"
        );
    }

    #[test]
    fn spacer_after_every_wide_glyph(s in r"[\u{4e00}-\u{9fff}A-Za-z]{0,15}") {
        // After any write, every cell with cell_width == 2 is followed
        // by a spacer (or the buffer right edge).
        let mut b = Buffer::empty(Rect::new(0, 0, 50, 1));
        b.set_string(0, 0, &s, Style::new());
        for x in 0..49 {
            let c = b.cell(x, 0).unwrap();
            if c.cell_width() == 2 {
                let next = b.cell(x + 1, 0).unwrap();
                prop_assert!(next.is_spacer(), "cell at x={} is wide but x+1 is not a spacer", x);
            }
        }
    }

    #[test]
    fn resize_preserves_overlapping_symbols(
        w1 in 1u16..30, h1 in 1u16..10,
        w2 in 1u16..30, h2 in 1u16..10
    ) {
        let mut b = Buffer::empty(Rect::new(0, 0, w1, h1));
        // Paint each cell with its (x, y) as a char (mod 94 for printable ASCII).
        for y in 0..h1 {
            for x in 0..w1 {
                let ch = char::from_u32(33 + ((x as u32 + y as u32 * 31) % 94)).unwrap();
                b.set_char(x, y, ch, Style::new());
            }
        }
        let snapshot = b.clone();
        b.resize(Rect::new(0, 0, w2, h2));
        // Intersection cells match.
        for y in 0..h1.min(h2) {
            for x in 0..w1.min(w2) {
                prop_assert_eq!(
                    b.cell(x, y).unwrap().symbol(),
                    snapshot.cell(x, y).unwrap().symbol()
                );
            }
        }
    }

    #[test]
    fn diff_self_vs_self_is_empty(w in 1u16..20, h in 1u16..5) {
        let a = Buffer::empty(Rect::new(0, 0, w, h));
        prop_assert_eq!(a.diff_iter(&a).count(), 0);
    }
}
