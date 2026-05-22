//! Counter button — the minimal App-based demo.
//!
//! A single `<button>` that increments a counter on click. The
//! text-node mutation flows through `MutationObserver` →
//! `DirtyTracker`, and the next frame re-cascades + repaints
//! automatically — no manual redraw call in the listener.
//!
//! Controls: click to increment. Ctrl-C to quit.
//!
//! Run: `cargo run --example counter_button -p rdom-tui`
//!
//! The DOM-building + CSS lives in
//! `rdom-showcase::demos::counter_button` so it can be both run
//! standalone (this binary) and mounted in the showcase under
//! "Events → Counter Button". Single source of truth.

fn main() -> std::io::Result<()> {
    rdom_showcase::demos::counter_button::run_standalone()
}
