//! Test helpers shared by the `rdom-tui` integration suite: run the
//! cascade → layout → paint pipeline headlessly and dump the painted
//! buffer as text.
//!
//! The golden-file snapshot harness (and every demo snapshot) lives
//! in `rdom-showcase`'s suite; `rdom-tui`'s own tests assert on the
//! dump directly.

#![allow(dead_code)] // shared helpers; not every test uses every fn

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
