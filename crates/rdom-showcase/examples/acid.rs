//! The acid test, one page full screen (`specs/ACID.md`).
//!
//! Run: `cargo run -p rdom-showcase --example acid -- <page>` (page 1 by
//! default). The references are derived for a 120 × 50 terminal.
//!
//! Implementation lives in `rdom_showcase::demos::acid` — every page is
//! browsable, stacked, in the showcase under "Built-ins → Acid".

fn main() -> std::io::Result<()> {
    let page = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(1);
    rdom_showcase::demos::acid::run_standalone(page)
}
