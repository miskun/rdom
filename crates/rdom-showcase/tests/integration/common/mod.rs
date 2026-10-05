//! Test helpers shared by the showcase integration suite: the
//! paint-snapshot harness that pins what every demo / example paints.
//!
//! ## Snapshots
//!
//! A snapshot is a row-per-line plain-text dump of the painted
//! `Buffer`. Each row is the concatenation of cell `symbol()`s
//! left-to-right, with `is_spacer()` cells skipped so wide glyphs
//! (CJK, emoji) appear once rather than once + a blank. Trailing
//! whitespace is preserved up to the line's `Cell::EMPTY` tail and
//! trimmed at write time — the harness handles the trim so authors
//! don't have to manage trailing-space noise.
//!
//! Cell backgrounds are part of the snapshot, as a second layer after
//! the glyph rows (a key letter per distinct color, `.` for the
//! terminal default; omitted when no cell has one) — a fill is what a
//! user sees. Foreground colors and modifiers are not: style drift
//! there is caught by paint-pass unit tests.
//!
//! ### Updating snapshots
//!
//! On mismatch, the test fails and prints a unified diff. To
//! regenerate, set `UPDATE_SNAPSHOTS=1` and re-run:
//!
//! ```sh
//! UPDATE_SNAPSHOTS=1 cargo test -p rdom-showcase --test integration ua_chrome
//! ```
//!
//! Snapshots live under `crates/rdom-showcase/tests/snapshots/`. Review
//! the diff in `git diff` before committing — the diff IS the
//! visual change.

#![allow(dead_code)] // shared helpers; not every test uses every fn

use std::env;
use std::fs;
use std::path::Path;

use rdom_tui::prelude::*;
use rdom_tui::render::Buffer;

/// Run the full pipeline against `viewport` and return the painted
/// `Buffer`. Equivalent to what `App::draw_if_dirty` does for one
/// frame, minus the backend write — with what the app does before it:
/// `style` attributes parsed into inline styles (`App::build` seeds
/// them, `seed_inline_styles`), and the showcase's sheet stack, the
/// shell's base sheet, then the demo's `sheet`, as the app mounts them
/// (`demo_sheet`) — so a snapshot pins what the app shows.
pub fn render(dom: &mut TuiDom, sheet: &Stylesheet, viewport: Rect) -> Buffer {
    let warnings = rdom_tui::seed_inline_styles(dom);
    assert!(warnings.is_empty(), "style attributes parse: {warnings:?}");
    let base = rdom_showcase::shell::base_stylesheet();
    dom.cascade_all(&[&base, sheet]);
    dom.layout_dom(viewport);
    let mut buf = Buffer::empty(viewport);
    dom.paint_dom(&mut buf, viewport);
    buf
}

/// Convert a painted `Buffer` to its snapshot string — one line per
/// row, cell symbols concatenated, spacer cells skipped, trailing
/// whitespace per row trimmed — then, when any cell has a background
/// color, the background layer: the same grid with a key letter per
/// distinct color (`.` for the terminal default, `Color::Reset`) and the
/// key below it, so a snapshot pins the fills users see too.
pub fn buffer_to_snapshot(buf: &Buffer) -> String {
    let mut out = String::new();
    for y in buf.area.y..buf.area.bottom() {
        let mut row = String::new();
        for x in buf.area.x..buf.area.right() {
            if let Some(c) = buf.cell(x, y) {
                if c.is_spacer() {
                    continue;
                }
                row.push_str(c.symbol());
            }
        }
        out.push_str(row.trim_end());
        out.push('\n');
    }
    out.push_str(&background_layer(buf));
    out
}

/// The background layer of [`buffer_to_snapshot`]; empty when every cell
/// keeps the terminal's default background.
fn background_layer(buf: &Buffer) -> String {
    const KEYS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut colors: Vec<Color> = Vec::new();
    let mut grid = String::new();
    for y in buf.area.y..buf.area.bottom() {
        let mut row = String::new();
        for x in buf.area.x..buf.area.right() {
            let Some(c) = buf.cell(x, y) else { continue };
            if c.is_spacer() {
                continue;
            }
            if c.bg == Color::Reset {
                row.push('.');
                continue;
            }
            let i = colors.iter().position(|&k| k == c.bg).unwrap_or_else(|| {
                colors.push(c.bg);
                colors.len() - 1
            });
            row.push(char::from(KEYS[i.min(KEYS.len() - 1)]));
        }
        grid.push_str(row.trim_end_matches('.'));
        grid.push('\n');
    }
    if colors.is_empty() {
        return String::new();
    }
    let mut out = String::from("── background ──\n");
    out.push_str(&grid);
    for (i, c) in colors.iter().enumerate() {
        out.push_str(&format!(
            "{} {c:?}\n",
            char::from(KEYS[i.min(KEYS.len() - 1)])
        ));
    }
    out
}

/// Compare `actual` against the golden file at `golden_relpath`
/// (relative to `crates/rdom-showcase/tests/snapshots/`). On mismatch,
/// print a unified diff and fail. With `UPDATE_SNAPSHOTS=1` in the
/// environment, write `actual` to disk instead and pass.
pub fn assert_snapshot(actual: &str, golden_relpath: &str) {
    let snapshots_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots");
    let path = snapshots_dir.join(golden_relpath);

    let update = env::var_os("UPDATE_SNAPSHOTS").is_some();
    if update {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create snapshots dir");
        }
        fs::write(&path, actual).expect("write snapshot");
        eprintln!("snapshot updated: {}", path.display());
        return;
    }

    let expected = fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!(
            "snapshot {} not found ({err}). Run with UPDATE_SNAPSHOTS=1 to create it.",
            path.display()
        );
    });

    if expected != actual {
        let diff = unified_diff(&expected, actual);
        panic!(
            "snapshot mismatch: {}\n\n{}\nRun with UPDATE_SNAPSHOTS=1 to accept the new output.",
            path.display(),
            diff
        );
    }
}

/// Tiny unified-diff helper — line-by-line, no context windowing.
/// Good enough for snapshot mismatches; the snapshot is always
/// short. Avoids pulling in a `similar` / `difference` dep.
fn unified_diff(expected: &str, actual: &str) -> String {
    let mut out = String::new();
    out.push_str("--- expected\n+++ actual\n");
    let exp_lines: Vec<&str> = expected.lines().collect();
    let act_lines: Vec<&str> = actual.lines().collect();
    let len = exp_lines.len().max(act_lines.len());
    for i in 0..len {
        match (exp_lines.get(i), act_lines.get(i)) {
            (Some(e), Some(a)) if e == a => {
                out.push_str(&format!("  {e}\n"));
            }
            (Some(e), Some(a)) => {
                out.push_str(&format!("- {e}\n"));
                out.push_str(&format!("+ {a}\n"));
            }
            (Some(e), None) => out.push_str(&format!("- {e}\n")),
            (None, Some(a)) => out.push_str(&format!("+ {a}\n")),
            (None, None) => {}
        }
    }
    out
}
