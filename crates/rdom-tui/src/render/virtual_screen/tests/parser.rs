//! Hand-written ANSI streams: basic paint, SGR, cursor, wide glyphs,
//! synchronized output, and resize of the model itself.

use crate::render::{Color, Modifier, VirtualScreen};

// ── Basic paint ──────────────────────────────────────────────────

#[test]
fn blank_screen() {
    let s = VirtualScreen::new(10, 3);
    assert_eq!(s.row(0), "          ");
    assert_eq!(s.cursor(), (0, 0));
}

#[test]
fn cup_then_text() {
    let mut s = VirtualScreen::new(10, 3);
    s.apply(b"\x1b[2;3Hhello"); // CUP (3, 2) = (x=2, y=1), then "hello"
    assert_eq!(s.row(1).trim_end(), "  hello");
    assert_eq!(s.cursor(), (7, 1));
}

#[test]
fn clear_screen() {
    let mut s = VirtualScreen::new(5, 2);
    s.apply(b"\x1b[1;1Habc\x1b[2;1Hdef\x1b[2J");
    for y in 0..2 {
        assert_eq!(s.row(y).trim_end(), "");
    }
}

// ── SGR ──────────────────────────────────────────────────────────

#[test]
fn sgr_fg_ansi16() {
    let mut s = VirtualScreen::new(5, 1);
    s.apply(b"\x1b[31mA");
    // SGR 31 selects palette entry 1, whose color is the theme's.
    assert_eq!(s.cell(0, 0).unwrap().fg, Color::Indexed(1));
}

#[test]
fn sgr_fg_rgb_truecolor() {
    let mut s = VirtualScreen::new(5, 1);
    s.apply(b"\x1b[38;2;10;20;30mX");
    assert_eq!(s.cell(0, 0).unwrap().fg, Color::Rgb(10, 20, 30));
}

#[test]
fn sgr_bg_indexed() {
    let mut s = VirtualScreen::new(5, 1);
    s.apply(b"\x1b[48;5;204mX");
    assert_eq!(s.cell(0, 0).unwrap().bg, Color::Indexed(204));
}

#[test]
fn sgr_bright_fg() {
    let mut s = VirtualScreen::new(5, 1);
    s.apply(b"\x1b[91mX");
    assert_eq!(s.cell(0, 0).unwrap().fg, Color::Indexed(9));
}

#[test]
fn sgr_multiple_in_one_csi() {
    let mut s = VirtualScreen::new(5, 1);
    s.apply(b"\x1b[1;31;40mX");
    let c = s.cell(0, 0).unwrap();
    assert_eq!(c.fg, Color::Indexed(1));
    assert_eq!(c.bg, Color::Indexed(0));
    assert!(c.modifier.contains(Modifier::BOLD));
}

#[test]
fn sgr_reset_clears_state() {
    let mut s = VirtualScreen::new(5, 1);
    s.apply(b"\x1b[31;1mA\x1b[0mB");
    assert_eq!(s.cell(0, 0).unwrap().fg, Color::Indexed(1));
    assert_eq!(s.cell(1, 0).unwrap().fg, Color::Reset);
    assert!(!s.cell(1, 0).unwrap().modifier.contains(Modifier::BOLD));
}

#[test]
fn sgr_22_clears_bold() {
    let mut s = VirtualScreen::new(5, 1);
    // Apply bold + dim; SGR-2 (dim) is ignored post-T8 — the
    // first cell takes only BOLD. SGR-22 clears BOLD on the
    // second cell.
    s.apply(b"\x1b[1;2mA\x1b[22mB");
    assert!(s.cell(0, 0).unwrap().modifier.contains(Modifier::BOLD));
    assert!(!s.cell(1, 0).unwrap().modifier.contains(Modifier::BOLD));
}

/// C9G-MISC-CORRECTNESS — ECMA-48's plain `4` is a single underline: after
/// a styled one (`4:3`, curly) it sets the solid style again, as terminals
/// do, rather than keeping the curly bit.
#[test]
fn a_plain_4_resets_the_underline_style() {
    let mut s = VirtualScreen::new(5, 1);
    s.apply(b"\x1b[4:3mA\x1b[4mB");
    let a = s.cell(0, 0).unwrap().modifier;
    let b = s.cell(1, 0).unwrap().modifier;
    assert!(a.contains(Modifier::UNDERLINED | Modifier::UNDERLINE_CURLY));
    assert!(b.contains(Modifier::UNDERLINED));
    assert!(!b.contains(Modifier::UNDERLINE_CURLY), "{b:?}");
}

// ── Cursor ───────────────────────────────────────────────────────

#[test]
fn cursor_hide_show() {
    let mut s = VirtualScreen::new(5, 1);
    assert!(s.cursor_visible());
    s.apply(b"\x1b[?25l");
    assert!(!s.cursor_visible());
    s.apply(b"\x1b[?25h");
    assert!(s.cursor_visible());
}

#[test]
fn cursor_advances_after_writes() {
    let mut s = VirtualScreen::new(10, 1);
    s.apply(b"abc");
    assert_eq!(s.cursor(), (3, 0));
}

// ── Wide glyphs ──────────────────────────────────────────────────

#[test]
fn cjk_takes_two_cells() {
    let mut s = VirtualScreen::new(5, 1);
    s.apply("\x1b[1;1H中X".as_bytes());
    assert_eq!(s.cell(0, 0).unwrap().symbol(), "中");
    assert!(s.cell(1, 0).unwrap().is_spacer());
    assert_eq!(s.cell(2, 0).unwrap().symbol(), "X");
    assert_eq!(s.cursor(), (3, 0));
}

#[test]
fn emoji_zwj_stays_one_grapheme() {
    let mut s = VirtualScreen::new(5, 1);
    let input = "\x1b[1;1H👨\u{200D}👩\u{200D}👧".as_bytes();
    s.apply(input);
    // Family emoji is one grapheme of width 2.
    assert_eq!(s.cell(0, 0).unwrap().symbol(), "👨\u{200D}👩\u{200D}👧");
    assert!(s.cell(1, 0).unwrap().is_spacer());
    assert_eq!(s.cursor(), (2, 0));
}

// ── Synchronized output (ignored) ────────────────────────────────

#[test]
fn bsu_esu_are_silent() {
    let mut s = VirtualScreen::new(5, 1);
    s.apply(b"\x1b[?2026h\x1b[1;1HX\x1b[?2026l");
    assert_eq!(s.row(0).trim_end(), "X");
}

// ── Operating system commands ─────────────────────────────────────

/// ACID-FIX-1. An OSC string (ECMA-48 §8.3.89: `ESC ]` … ST, or BEL as
/// xterm also accepts) is a control string, not text: the OSC 8
/// hyperlink a backend wraps a link's cells in (`ESC ] 8 ; params ; URI
/// ST`, an empty URI closing it) must not print. Its URI becomes the link
/// of the cells written while it is open, as `Cell::link` records it.
#[test]
fn osc8_hyperlinks_are_not_text_and_mark_their_cells() {
    let mut s = VirtualScreen::new(10, 1);
    s.apply(b"a\x1b]8;;https://x.test\x1b\\bc\x1b]8;;\x1b\\d\x1b]2;title\x07e");
    assert_eq!(s.row(0).trim_end(), "abcde");
    assert_eq!(s.cell(0, 0).unwrap().link(), None);
    assert_eq!(s.cell(1, 0).unwrap().link(), Some("https://x.test"));
    assert_eq!(s.cell(2, 0).unwrap().link(), Some("https://x.test"));
    assert_eq!(s.cell(3, 0).unwrap().link(), None);
}

// ── Resize ───────────────────────────────────────────────────────

#[test]
fn resize_preserves_intersection() {
    let mut s = VirtualScreen::new(5, 3);
    s.apply(b"\x1b[1;1HABCDE");
    s.resize(10, 3);
    assert_eq!(s.row(0).trim_end(), "ABCDE");
}

#[test]
fn resize_smaller_drops_excess() {
    let mut s = VirtualScreen::new(10, 3);
    s.apply(b"\x1b[1;1HHello World!");
    s.resize(5, 3);
    assert_eq!(s.row(0).trim_end(), "Hello");
}

#[test]
fn resize_larger_new_rows_blank() {
    let mut s = VirtualScreen::new(5, 2);
    s.apply(b"\x1b[1;1HAB");
    s.resize(5, 5);
    for y in 2..5 {
        assert_eq!(s.row(y).trim_end(), "");
    }
}
