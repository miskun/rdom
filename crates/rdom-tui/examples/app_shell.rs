//! M5.6 — TUI app-shell demo. Showcases `border-collapse: collapse`
//! in the headline grid layout: outer shell + header + 3-column
//! body + footer, every internal border shared with the outer
//! shell's frame.
//!
//! Run: `cargo run -p rdom-tui --example app_shell`
//!
//! Implementation lives in `rdom_showcase::demos::app_shell` — same
//! DOM is browsable in the showcase under "Layout → App shell".

fn main() -> std::io::Result<()> {
    rdom_showcase::demos::app_shell::run_standalone()
}
