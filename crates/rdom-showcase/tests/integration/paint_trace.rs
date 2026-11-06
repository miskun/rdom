//! CAPTURE phase 2 (`specs/CAPTURE.md` §Paint trace): on every acid page —
//! the test that makes rdom's features meet — a traced paint is the
//! frame's paint, every cell's last operation is exactly what the cell
//! shows, and every operation is attributed to an element's part (or the
//! border joiner).

use rdom_showcase::demos::acid;
use rdom_tui::Cell;
use rdom_tui::capture::{PaintPart, trace_paint};

use crate::acid::compare::open_page;

#[test]
fn every_acid_cell_is_explained_by_its_trace() {
    let mut failures = Vec::new();
    for page in 1..=acid::page_count() {
        let mut app = open_page(page, None);
        app.advance(0).unwrap();
        let frame = app.terminal().buffer();
        let area = frame.area;
        let (traced, trace) = trace_paint(app.dom(), area, area);
        if &traced != frame {
            failures.push(format!(
                "page {page}: the traced paint differs from the frame"
            ));
            continue;
        }
        for op in trace.ops() {
            if op.part == PaintPart::Unattributed {
                failures.push(format!("page {page}: unattributed {op:?}"));
                break;
            }
        }
        'cells: for y in area.y..area.bottom() {
            for x in area.x..area.right() {
                let shown = frame.cell(x, y).unwrap();
                let ok = match trace.top_at(x, y) {
                    Some(op) => &op.cell == shown,
                    None => shown == &Cell::EMPTY,
                };
                if !ok {
                    failures.push(format!(
                        "page {page}: cell ({x}, {y}) shows {shown:?}, its trace ends {:?}",
                        trace.top_at(x, y)
                    ));
                    break 'cells;
                }
            }
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

/// What a traced paint costs beside a plain one, on every acid page —
/// printed, not asserted (timings belong to a quiet machine and a release
/// build: `cargo test --release -p rdom-showcase --test integration
/// paint_trace::cost -- --ignored --nocapture`).
#[test]
#[ignore = "a measurement, run by hand"]
fn cost() {
    use rdom_tui::render::{Buffer, PaintExt};
    use std::time::Instant;
    for page in 1..=acid::page_count() {
        let mut app = open_page(page, None);
        app.advance(0).unwrap();
        let area = app.terminal().buffer().area;
        let runs = 20;
        let t = Instant::now();
        for _ in 0..runs {
            let mut buf = Buffer::empty(area);
            app.dom().paint_dom(&mut buf, area);
        }
        let plain = t.elapsed() / runs;
        let t = Instant::now();
        let mut ops = 0;
        for _ in 0..runs {
            ops = trace_paint(app.dom(), area, area).1.ops().len();
        }
        let traced = t.elapsed() / runs;
        println!("page {page:2}: paint {plain:?}, traced {traced:?} ({ops} ops)");
    }
}
