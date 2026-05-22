//! Paint snapshot for the M5.6 `app_shell` example / showcase demo.
//! Pins the shared-border output (`├ ┤ ┬ ┴ ┼` junction glyphs) at
//! a fixed 80×20 viewport.
//!
//! Implementation lives in `rdom_showcase::demos::app_shell` so the
//! standalone example, the snapshot, and the showcase mount share
//! one source of truth.
//!
//! To regenerate after an intentional shell change:
//!
//! ```sh
//! UPDATE_SNAPSHOTS=1 cargo test -p rdom-tui --test app_shell_snapshot
//! ```

mod common;

use rdom_showcase::demos::app_shell;
use rdom_tui::prelude::*;

use common::{assert_snapshot, buffer_to_snapshot, render};

#[test]
fn app_shell_renders_with_collapsed_borders() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let demo_root = app_shell::build(&mut dom);
    dom.append_child(root, demo_root).unwrap();

    let sheet = app_shell::stylesheet();
    let buf = render(&mut dom, &sheet, Rect::new(0, 0, 80, 20));
    let snapshot = buffer_to_snapshot(&buf);
    assert_snapshot(&snapshot, "app_shell.snap");
}
