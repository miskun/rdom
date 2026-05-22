//! Paint snapshot for the `counter_button` example / showcase
//! demo. Both the standalone example
//! (`crates/rdom-tui/examples/counter_button.rs`) and the
//! showcase ("Events → Counter Button") share the same
//! `rdom_showcase::demos::counter_button::{build, stylesheet}`,
//! so the snapshot pins exactly what consumers see.
//!
//! Catches visible regressions to the button's UA chrome
//! (`::before` / `::after` brackets), the demo's structural
//! layout (flex column, padding, gap), and the initial label
//! text without requiring a TTY.
//!
//! To regenerate the golden after an intentional change:
//!
//! ```sh
//! UPDATE_SNAPSHOTS=1 cargo test -p rdom-tui --test counter_button_snapshot
//! ```
//!
//! Then `git diff` the snapshot to review the visual change
//! before committing.

mod common;

use rdom_showcase::demos::counter_button;
use rdom_tui::prelude::*;

use common::{assert_snapshot, buffer_to_snapshot, render};

#[test]
fn counter_button_initial_paint_matches_golden() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let demo_root = counter_button::build(&mut dom);
    dom.append_child(root, demo_root).unwrap();

    let sheet = counter_button::stylesheet();
    // 50×8 — enough for the title, hint, and button row with some
    // breathing room on either side.
    let buf = render(&mut dom, &sheet, Rect::new(0, 0, 50, 8));
    let snap = buffer_to_snapshot(&buf);
    assert_snapshot(&snap, "counter_button.snap");
}
