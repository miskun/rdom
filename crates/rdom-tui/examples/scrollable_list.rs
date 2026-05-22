//! Scrollable list — scroll a long list with the mouse wheel, watch
//! `:hover` follow the cursor.
//!
//! Run: `cargo run -p rdom-tui --example scrollable_list`
//!
//! Implementation lives in `rdom_showcase::demos::scrollable_list`
//! so the showcase ("Layout → Scrollable list") and this binary
//! share one source of truth.

fn main() -> std::io::Result<()> {
    rdom_showcase::demos::scrollable_list::run_standalone()
}
