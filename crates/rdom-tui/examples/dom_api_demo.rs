//! `dom_api_demo` — exercises every M4 accessor family in three
//! themed walk-throughs (form-edit, tree-walk, cssom). Prints
//! accessor output to stdout — non-interactive.
//!
//! Run: `cargo run -p rdom-tui --example dom_api_demo`
//!
//! Implementation lives in `rdom_showcase::demos::dom_api` — same
//! report is browsable in the showcase under
//! "Built-ins → DOM API walkthrough" (rendered into a `<pre>` block).

fn main() -> std::io::Result<()> {
    rdom_showcase::demos::dom_api::run_standalone()
}
