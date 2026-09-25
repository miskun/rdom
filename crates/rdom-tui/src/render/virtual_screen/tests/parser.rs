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
    assert_eq!(s.cell(0, 0).unwrap().fg, Color::Rgb(255, 0, 0));
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
    assert_eq!(s.cell(0, 0).unwrap().fg, Color::Rgb(240, 128, 128));
}

#[test]
fn sgr_multiple_in_one_csi() {
    let mut s = VirtualScreen::new(5, 1);
    s.apply(b"\x1b[1;31;40mX");
    let c = s.cell(0, 0).unwrap();
    assert_eq!(c.fg, Color::Rgb(255, 0, 0));
    assert_eq!(c.bg, Color::Rgb(0, 0, 0));
    assert!(c.modifier.contains(Modifier::BOLD));
}

#[test]
fn sgr_reset_clears_state() {
    let mut s = VirtualScreen::new(5, 1);
    s.apply(b"\x1b[31;1mA\x1b[0mB");
    assert_eq!(s.cell(0, 0).unwrap().fg, Color::Rgb(255, 0, 0));
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
