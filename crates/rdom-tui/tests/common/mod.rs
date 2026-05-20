//! Test helpers shared by the integration suite — currently the
//! paint-snapshot harness used by `ua_chrome_snapshot.rs` and any
//! future example-coverage tests.
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
//! Cell foreground / background / modifiers are NOT part of the
//! snapshot. We snapshot what's *visible*, not what's *styled* —
//! style drift is a separate concern caught by paint-pass unit
//! tests.
//!
//! ### Updating snapshots
//!
//! On mismatch, the test fails and prints a unified diff. To
//! regenerate, set `UPDATE_SNAPSHOTS=1` and re-run:
//!
//! ```sh
//! UPDATE_SNAPSHOTS=1 cargo test -p rdom-tui --test ua_chrome_snapshot
//! ```
//!
//! Snapshots live under `crates/rdom-tui/tests/snapshots/`. Review
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
/// frame, minus the backend write.
pub fn render(dom: &mut TuiDom, sheet: &Stylesheet, viewport: Rect) -> Buffer {
    dom.cascade(sheet);
    dom.layout_dom(viewport);
    let mut buf = Buffer::empty(viewport);
    dom.paint_dom(&mut buf, viewport);
    buf
}

/// Convert a painted `Buffer` to its snapshot string — one line per
/// row, cell symbols concatenated, spacer cells skipped, trailing
/// whitespace per row trimmed.
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
    out
}

/// Compare `actual` against the golden file at `golden_relpath`
/// (relative to `crates/rdom-tui/tests/snapshots/`). On mismatch,
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
