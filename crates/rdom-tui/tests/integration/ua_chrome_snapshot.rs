//! Paint snapshot for the `ua_chrome` example / showcase demo.
//! Pins the UA stylesheet output (button brackets, summary
//! triangle, ul bullets, dialog border + padding).
//!
//! Implementation lives in `rdom_showcase::demos::ua_chrome` so
//! the standalone example, the snapshot test, and the showcase
//! mount share one source of truth — no chance of inline DOM
//! construction in the test drifting from the example's actual
//! shape.
//!
//! To regenerate after an intentional UA / chrome change:
//!
//! ```sh
//! UPDATE_SNAPSHOTS=1 cargo test -p rdom-tui --test ua_chrome_snapshot
//! ```
use rdom_showcase::demos::ua_chrome;
use rdom_tui::prelude::*;

use crate::common::{assert_snapshot, buffer_to_snapshot, render};

#[test]
fn ua_chrome_paints_naked_native_built_ins() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let demo_root = ua_chrome::build(&mut dom);
    dom.append_child(root, demo_root).unwrap();

    let sheet = ua_chrome::stylesheet();
    // Wide enough for the dialog's chrome to land within a single
    // row's worth of border, tall enough that no section gets
    // clipped.
    let buf = render(&mut dom, &sheet, Rect::new(0, 0, 60, 30));
    let snap = buffer_to_snapshot(&buf);
    assert_snapshot(&snap, "ua_chrome.snap");
}
