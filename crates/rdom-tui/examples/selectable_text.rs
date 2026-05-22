//! Selectable text — prose + code + CJK, all selectable with the
//! usual mouse gestures. Demonstrates `user-select: none` as an
//! opt-out for UI chrome.
//!
//! Run: `cargo run -p rdom-tui --example selectable_text`
//!
//! Implementation lives in `rdom_showcase::demos::selectable_text`
//! so the showcase ("Selection → Selectable text") and this binary
//! share one source of truth.

fn main() -> std::io::Result<()> {
    rdom_showcase::demos::selectable_text::run_standalone()
}
