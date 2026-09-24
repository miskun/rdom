//! `position: sticky` — pinned header inside a scrollable list.
//!
//! Run: `cargo run -p rdom-showcase --example sticky_demo`
//!
//! Implementation lives in `rdom_showcase::demos::sticky` so the
//! same DOM + CSS is browsable in the showcase under
//! "Positioning → Sticky header" without code duplication.

fn main() -> std::io::Result<()> {
    rdom_showcase::demos::sticky::run_standalone()
}
