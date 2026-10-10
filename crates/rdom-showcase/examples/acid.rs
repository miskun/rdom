//! The acid test, full screen, a page at a time (`specs/ACID.md`).
//!
//! Run: `cargo run -p rdom-showcase --example acid -- <page>` (page 1 by
//! default). Ctrl+N / Ctrl+P page forward and back through every page —
//! pages 1–9 and 13 the static tiles, 10–12 the stage-2 tiles, live: hover,
//! click, type and scroll them as the interactive steps do — and Ctrl+C
//! quits. The references are derived for a 120 × 50 terminal.
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
