//! `rdom-showcase` binary entry point.
//!
//! Builds the shell, mounts the demo at `DEMOS[0]` into the main
//! view, pushes the demo's stylesheet onto the App's sheet stack,
//! runs the event loop. M2 ships this static layout; M3 makes the
//! sidebar interactive.

use rdom_showcase::{DEMOS, build_shell, shell::base_stylesheet};
use rdom_tui::{App, TuiDom};

fn main() -> std::io::Result<()> {
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);

    let demo = DEMOS[0];
    let demo_root = demo.build(&mut dom);
    dom.append_child(handles.main, demo_root).unwrap();

    let mut app = App::new(dom, base_stylesheet())?;
    let _demo_sheet_id = app.push_stylesheet(demo.stylesheet());
    app.run()
}
