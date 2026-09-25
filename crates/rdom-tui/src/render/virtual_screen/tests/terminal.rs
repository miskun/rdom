//! Real `Terminal` output replayed through the emulator, including
//! the vertical and horizontal resize regression tests.

use crate::render::{Color, VirtualScreen};

// ── Integration with real Terminal + CrosstermBackend ────────────

#[test]
fn terminal_output_renders_correctly() {
    use crate::render::{Buffer, Style, Terminal, TestBackend};

    let tb = TestBackend::new(10, 2);
    let mut term = Terminal::new(tb).unwrap();
    term.draw(|buf: &mut Buffer| {
        buf.set_string(0, 0, "Hello", Style::new().fg(Color::Rgb(255, 0, 0)));
        buf.set_string(0, 1, "World", Style::new().fg(Color::Rgb(0, 0, 255)));
        Ok(())
    })
    .unwrap();

    let mut screen = VirtualScreen::new(10, 2);
    screen.apply(term.backend().bytes());
    assert_eq!(screen.row(0).trim_end(), "Hello");
    assert_eq!(screen.row(1).trim_end(), "World");
    assert_eq!(screen.cell(0, 0).unwrap().fg, Color::Rgb(255, 0, 0));
    assert_eq!(screen.cell(0, 1).unwrap().fg, Color::Rgb(0, 0, 255));
}

// ── Resize regression tests ─────────────────────────────────────

#[test]
fn resize_smaller_no_stale_cells_on_screen() {
    use crate::render::{Buffer, Style, Terminal, TestBackend};

    // Paint a layout with content on multiple rows.
    let tb = TestBackend::new(20, 6);
    let mut term = Terminal::new(tb).unwrap();
    term.draw(|buf: &mut Buffer| {
        buf.set_string(0, 0, "LONG HEADER TEXT", Style::new());
        buf.set_string(0, 2, "body line", Style::new());
        buf.set_string(0, 5, "footer text", Style::new());
        Ok(())
    })
    .unwrap();

    let mut screen = VirtualScreen::new(20, 6);
    screen.apply(term.backend().bytes());

    // Shrink terminal. Virtual screen keeps intersection rows 0..4
    // — so "LONG HEADER TEXT" at row 0 and "body line" at row 2
    // survive in our model of the terminal.
    term.backend_mut().resize(20, 4);
    screen.resize(20, 4);
    let _ = term.backend_mut().take_bytes();

    // Next paint has SHORTER text on row 0 and NOTHING on row 2.
    // Without backend.clear() on resize, force_full_redraw would
    // skip blank cells in the new buffer → the tail of the old
    // "LONG HEADER TEXT" and the entire "body line" would persist.
    term.draw(|buf: &mut Buffer| {
        buf.set_string(0, 0, "HI", Style::new());
        Ok(())
    })
    .unwrap();
    screen.apply(term.backend().bytes());

    assert_eq!(
        screen.row(0).trim_end(),
        "HI",
        "row 0 has stale tail from previous longer text"
    );
    assert_eq!(
        screen.row(2).trim_end(),
        "",
        "row 2 has stale 'body line' content"
    );
}

#[test]
fn resize_larger_new_rows_are_blank() {
    use crate::render::{Buffer, Style, Terminal, TestBackend};

    // Paint a compact layout with content on every row.
    let tb = TestBackend::new(10, 3);
    let mut term = Terminal::new(tb).unwrap();
    term.draw(|buf: &mut Buffer| {
        buf.set_string(0, 0, "hello", Style::new());
        buf.set_string(0, 1, "world", Style::new());
        buf.set_string(0, 2, "fooba", Style::new());
        Ok(())
    })
    .unwrap();

    let mut screen = VirtualScreen::new(10, 3);
    screen.apply(term.backend().bytes());

    // Grow. Intersection rows 0..3 retain their old content in our
    // model of the terminal.
    term.backend_mut().resize(10, 6);
    screen.resize(10, 6);
    let _ = term.backend_mut().take_bytes();

    // Next paint has SHORTER text on each row — blanks at positions
    // where the old paint had non-blanks. Without clear-on-resize
    // those stale cells would leak through.
    term.draw(|buf: &mut Buffer| {
        buf.set_string(0, 0, "hi", Style::new());
        buf.set_string(0, 1, "w", Style::new());
        // Row 2 empty.
        Ok(())
    })
    .unwrap();
    screen.apply(term.backend().bytes());

    assert_eq!(screen.row(0).trim_end(), "hi", "row 0 has stale tail");
    assert_eq!(screen.row(1).trim_end(), "w", "row 1 has stale tail");
    assert_eq!(screen.row(2).trim_end(), "", "row 2 has stale content");
    for y in 3..6 {
        assert_eq!(screen.row(y).trim_end(), "", "new row {y} not blank");
    }
}

#[test]
fn multiple_resizes_preserve_correctness() {
    use crate::render::{Buffer, Style, Terminal, TestBackend};

    let tb = TestBackend::new(20, 10);
    let mut term = Terminal::new(tb).unwrap();
    let mut screen = VirtualScreen::new(20, 10);

    // Sequence of resizes, each followed by a paint.
    let sizes = [(15, 8), (30, 12), (5, 3), (25, 7), (20, 10)];
    for (w, h) in sizes {
        term.backend_mut().resize(w, h);
        screen.resize(w, h);
        term.draw(|buf: &mut Buffer| {
            let text = format!("{w}x{h}");
            buf.set_string(0, 0, &text, Style::new());
            Ok(())
        })
        .unwrap();
        screen.apply(term.backend_mut().take_bytes().as_slice());

        // Current screen should show the new size's label.
        let expected = format!("{w}x{h}");
        assert_eq!(
            screen.row(0).trim_end(),
            expected,
            "after resize to {w}x{h}, row 0 wrong: {:?}",
            screen.row(0)
        );
        // All other rows blank.
        for y in 1..h {
            assert_eq!(
                screen.row(y).trim_end(),
                "",
                "after resize to {w}x{h}, row {y} has stale content"
            );
        }
    }
}

// ── Horizontal resize regression tests ──────────────────────────

#[test]
fn resize_narrower_no_stale_cells_to_the_right() {
    use crate::render::{Buffer, Style, Terminal, TestBackend};

    // Paint content spanning the full width on several rows.
    let tb = TestBackend::new(20, 3);
    let mut term = Terminal::new(tb).unwrap();
    term.draw(|buf: &mut Buffer| {
        buf.set_string(0, 0, "AAAAAAAAAAAAAAAAAAAA", Style::new()); // 20 A's
        buf.set_string(0, 1, "BBBBBBBBBBBBBBBBBBBB", Style::new());
        buf.set_string(0, 2, "CCCCCCCCCCCCCCCCCCCC", Style::new());
        Ok(())
    })
    .unwrap();

    let mut screen = VirtualScreen::new(20, 3);
    screen.apply(term.backend().bytes());

    // Narrow to width 8. Intersection preserves first 8 columns.
    term.backend_mut().resize(8, 3);
    screen.resize(8, 3);
    let _ = term.backend_mut().take_bytes();

    // Next paint has much SHORTER text — without clear-on-resize
    // the force_full_redraw would skip blank cells 1..8 on rows 1,2
    // leaving the stale "BBBBBBBB" and "CCCCCCCC".
    term.draw(|buf: &mut Buffer| {
        buf.set_string(0, 0, "x", Style::new());
        // Rows 1 and 2 blank.
        Ok(())
    })
    .unwrap();
    screen.apply(term.backend().bytes());

    assert_eq!(screen.row(0).trim_end(), "x");
    assert_eq!(screen.row(1).trim_end(), "", "row 1 has stale B's");
    assert_eq!(screen.row(2).trim_end(), "", "row 2 has stale C's");
}

#[test]
fn resize_wider_new_columns_are_blank() {
    use crate::render::{Buffer, Style, Terminal, TestBackend};

    let tb = TestBackend::new(5, 2);
    let mut term = Terminal::new(tb).unwrap();
    term.draw(|buf: &mut Buffer| {
        buf.set_string(0, 0, "hello", Style::new());
        buf.set_string(0, 1, "world", Style::new());
        Ok(())
    })
    .unwrap();

    let mut screen = VirtualScreen::new(5, 2);
    screen.apply(term.backend().bytes());

    // Grow to width 15. Intersection preserves first 5 columns.
    term.backend_mut().resize(15, 2);
    screen.resize(15, 2);
    let _ = term.backend_mut().take_bytes();

    // Paint shorter content — new columns 5..15 must be blank, not
    // filled with residual terminal state.
    term.draw(|buf: &mut Buffer| {
        buf.set_string(0, 0, "hi", Style::new());
        buf.set_string(0, 1, "w", Style::new());
        Ok(())
    })
    .unwrap();
    screen.apply(term.backend().bytes());

    assert_eq!(screen.row(0).trim_end(), "hi");
    assert_eq!(screen.row(1).trim_end(), "w");
    // Columns 2..15 on row 0 must be blank (no stale "llo").
    for x in 2..15 {
        assert_eq!(
            screen.cell(x, 0).unwrap().symbol(),
            " ",
            "cell ({x},0) not blank after wider resize"
        );
    }
    // Columns 1..15 on row 1 must be blank (no stale "orld").
    for x in 1..15 {
        assert_eq!(
            screen.cell(x, 1).unwrap().symbol(),
            " ",
            "cell ({x},1) not blank after wider resize"
        );
    }
}

#[test]
fn resize_narrower_then_wider_stays_clean() {
    use crate::render::{Buffer, Style, Terminal, TestBackend};

    // 20 wide.
    let tb = TestBackend::new(20, 2);
    let mut term = Terminal::new(tb).unwrap();
    let mut screen = VirtualScreen::new(20, 2);

    term.draw(|buf: &mut Buffer| {
        buf.set_string(0, 0, "FIRST-WIDTH-20-LINE!", Style::new()); // 20 chars
        Ok(())
    })
    .unwrap();
    screen.apply(term.backend_mut().take_bytes().as_slice());

    // Narrow to 10.
    term.backend_mut().resize(10, 2);
    screen.resize(10, 2);
    term.draw(|buf: &mut Buffer| {
        buf.set_string(0, 0, "narrow", Style::new());
        Ok(())
    })
    .unwrap();
    screen.apply(term.backend_mut().take_bytes().as_slice());
    assert_eq!(screen.row(0).trim_end(), "narrow");
    assert_eq!(screen.row(1).trim_end(), "");

    // Grow back to 25 (wider than the original 20 — new columns
    // should be blank, not whatever was in the real terminal's
    // off-screen buffer).
    term.backend_mut().resize(25, 2);
    screen.resize(25, 2);
    term.draw(|buf: &mut Buffer| {
        buf.set_string(0, 0, "hi", Style::new());
        Ok(())
    })
    .unwrap();
    screen.apply(term.backend_mut().take_bytes().as_slice());

    assert_eq!(screen.row(0).trim_end(), "hi");
    for x in 2..25 {
        assert_eq!(
            screen.cell(x, 0).unwrap().symbol(),
            " ",
            "cell ({x},0) leaked after narrow→wider"
        );
    }
    assert_eq!(screen.row(1).trim_end(), "");
}

#[test]
fn horizontal_resize_preserves_styled_cells() {
    use crate::render::{Buffer, Style, Terminal, TestBackend};

    // Paint styled content, resize narrower, paint narrower content,
    // verify the remaining cells carry the new style (not stale old
    // style bleeding through).
    let tb = TestBackend::new(20, 1);
    let mut term = Terminal::new(tb).unwrap();
    let mut screen = VirtualScreen::new(20, 1);

    term.draw(|buf: &mut Buffer| {
        buf.set_string(
            0,
            0,
            "RED-RED-RED-RED-RED!",
            Style::new().fg(Color::Rgb(255, 0, 0)),
        );
        Ok(())
    })
    .unwrap();
    screen.apply(term.backend_mut().take_bytes().as_slice());
    assert_eq!(screen.cell(0, 0).unwrap().fg, Color::Rgb(255, 0, 0));

    // Narrow + repaint in Blue.
    term.backend_mut().resize(10, 1);
    screen.resize(10, 1);
    term.draw(|buf: &mut Buffer| {
        buf.set_string(0, 0, "blue", Style::new().fg(Color::Rgb(0, 0, 255)));
        Ok(())
    })
    .unwrap();
    screen.apply(term.backend_mut().take_bytes().as_slice());

    assert_eq!(screen.row(0).trim_end(), "blue");
    assert_eq!(screen.cell(0, 0).unwrap().fg, Color::Rgb(0, 0, 255));
    assert_eq!(screen.cell(3, 0).unwrap().fg, Color::Rgb(0, 0, 255));
    // Blank cells past the new content should not carry Red fg.
    for x in 4..10 {
        // Blank cells: symbol is " ". Style is "reset" i.e. default
        // — whatever the terminal chose after \x1b[2J. The important
        // thing is the previous Red doesn't bleed.
        let c = screen.cell(x, 0).unwrap();
        assert_eq!(c.symbol(), " ", "cell ({x},0) not blank");
    }
}

#[test]
fn multiple_horizontal_resizes_preserve_correctness() {
    use crate::render::{Buffer, Style, Terminal, TestBackend};

    let tb = TestBackend::new(20, 2);
    let mut term = Terminal::new(tb).unwrap();
    let mut screen = VirtualScreen::new(20, 2);

    // Sequence alternating narrow/wide, each paints a label of its
    // own length. Regression for stale-cells on every transition.
    let widths = [10u16, 30, 5, 25, 15, 8];
    for &w in &widths {
        term.backend_mut().resize(w, 2);
        screen.resize(w, 2);
        let label = format!("W={w}");
        let label_for_paint = label.clone();
        term.draw(|buf: &mut Buffer| {
            buf.set_string(0, 0, &label_for_paint, Style::new());
            Ok(())
        })
        .unwrap();
        screen.apply(term.backend_mut().take_bytes().as_slice());

        assert_eq!(
            screen.row(0).trim_end(),
            label,
            "after resize to w={w}, row 0 wrong: {:?}",
            screen.row(0)
        );
        assert_eq!(
            screen.row(1).trim_end(),
            "",
            "after resize to w={w}, row 1 has stale content"
        );
        // Every cell past the label must be blank.
        let label_len = label.len() as u16;
        for x in label_len..w {
            assert_eq!(
                screen.cell(x, 0).unwrap().symbol(),
                " ",
                "after w={w}, cell ({x},0) has stale content"
            );
        }
    }
}

#[test]
fn simultaneous_width_and_height_resize() {
    use crate::render::{Buffer, Style, Terminal, TestBackend};

    // Paint a layout, then resize both axes at once (mimics a
    // terminal-window corner-drag).
    let tb = TestBackend::new(20, 5);
    let mut term = Terminal::new(tb).unwrap();
    term.draw(|buf: &mut Buffer| {
        for y in 0..5 {
            buf.set_string(0, y, "XXXXXXXXXXXXXXXXXXXX", Style::new());
        }
        Ok(())
    })
    .unwrap();

    let mut screen = VirtualScreen::new(20, 5);
    screen.apply(term.backend().bytes());

    // Corner-drag smaller: both narrower and shorter.
    term.backend_mut().resize(8, 2);
    screen.resize(8, 2);
    let _ = term.backend_mut().take_bytes();

    term.draw(|buf: &mut Buffer| {
        buf.set_string(0, 0, "ok", Style::new());
        Ok(())
    })
    .unwrap();
    screen.apply(term.backend().bytes());

    assert_eq!(screen.row(0).trim_end(), "ok");
    assert_eq!(screen.row(1).trim_end(), "");
    // Row 0 columns 2..8 must be blank (no stale X's from the
    // original paint's first 8 X's).
    for x in 2..8 {
        assert_eq!(
            screen.cell(x, 0).unwrap().symbol(),
            " ",
            "cell ({x},0) stale after corner resize"
        );
    }

    // Corner-drag larger: both wider and taller.
    term.backend_mut().resize(25, 6);
    screen.resize(25, 6);
    let _ = term.backend_mut().take_bytes();

    term.draw(|buf: &mut Buffer| {
        buf.set_string(0, 0, "ok", Style::new());
        Ok(())
    })
    .unwrap();
    screen.apply(term.backend().bytes());

    assert_eq!(screen.row(0).trim_end(), "ok");
    // All new cells (rows 2..6, columns 2..25 on row 0) must be blank.
    for y in 1..6 {
        assert_eq!(
            screen.row(y).trim_end(),
            "",
            "row {y} has stale content after wider+taller resize"
        );
    }
    for x in 2..25 {
        assert_eq!(
            screen.cell(x, 0).unwrap().symbol(),
            " ",
            "cell ({x},0) stale after wider+taller resize"
        );
    }
}
