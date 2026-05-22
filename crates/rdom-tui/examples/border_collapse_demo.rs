//! Focused single-purpose demo of `border-collapse: collapse`.
//!
//! Run: `cargo run -p rdom-tui --example border_collapse_demo`.
//!
//! Implementation lives in `rdom_showcase::demos::border_collapse`
//! so the showcase ("Layout → Border collapse") and this binary
//! share one source of truth.

fn main() -> std::io::Result<()> {
    rdom_showcase::demos::border_collapse::run_standalone()
}
