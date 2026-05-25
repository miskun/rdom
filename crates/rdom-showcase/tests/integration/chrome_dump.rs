//! Cell-by-cell dump of the rendered chrome — diagnostic test
//! for the visual analysis after the substrate fixes landed.
//!
//! Runs the full cascade + layout + paint pipeline against a
//! TestBackend at a known viewport, then prints the rendered grid
//! row by row so we can verify each ASCII glyph against expectations.
//!
//! Run with: `cargo test -p rdom-showcase --test chrome_dump -- --nocapture`

use rdom_showcase::{DEMOS, build_shell, shell::base_stylesheet};
use rdom_tui::render::{Buffer, Rect};
use rdom_tui::{CascadeExt, LayoutExt, PaintExt, TuiDom};

#[test]
fn dump_full_chrome() {
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);
    let demo = DEMOS[0];
    let demo_root = demo.build(&mut dom);
    dom.append_child(handles.main, demo_root).unwrap();

    let base = base_stylesheet();
    let demo_sheet = demo.stylesheet();
    let sheets: Vec<&_> = vec![&base, &demo_sheet];

    let viewport = Rect::new(0, 0, 80, 24);
    dom.cascade_all(&sheets);
    dom.layout_dom(viewport);
    let mut buf = Buffer::empty(viewport);
    dom.paint_dom(&mut buf, viewport);

    eprintln!("=== Chrome dump @ 80×24 ===");
    eprintln!("    {}", ruler(viewport.width));
    for y in 0..viewport.height {
        let mut line = String::new();
        for x in 0..viewport.width {
            if let Some(cell) = buf.cell(x, y) {
                if cell.is_spacer() {
                    continue;
                }
                line.push_str(cell.symbol());
            } else {
                line.push('?');
            }
        }
        eprintln!("{y:>2}: {line}");
    }
    eprintln!("=== end dump ===");

    // Force the test to "fail" so eprintln output prints under
    // --nocapture. Comment out when not diagnosing.
    // panic!("diagnostic dump");
}

fn ruler(width: u16) -> String {
    let mut s = String::new();
    for x in 0..width {
        s.push(char::from_digit((x % 10) as u32, 10).unwrap_or('?'));
    }
    s
}
