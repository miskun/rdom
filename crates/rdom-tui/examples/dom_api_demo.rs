//! `dom_api_demo` — exercises every M4 accessor family in three
//! themed walk-throughs (form-edit, tree-walk, cssom).
//!
//! **Non-interactive.** Unlike every other example in this
//! directory, this one prints the accessor report to stdout and
//! exits — it does NOT enter TUI mode. A flash of text and a
//! return to the shell is the expected output. The showcase
//! version (browsable under "Built-ins → DOM API walkthrough")
//! renders the same report into a `<pre>` block.
//!
//! Run: `cargo run -p rdom-tui --example dom_api_demo`
//!
//! Implementation lives in `rdom_showcase::demos::dom_api`.

fn main() -> std::io::Result<()> {
    rdom_showcase::demos::dom_api::run_standalone()
}
