//! All three crates composing: parse an HTML-ish template with
//! `rdom-parser`, cascade + layout + paint via `rdom-tui`.
//!
//! Run: `cargo run -p rdom-tui --example parse_and_render`
//!
//! Implementation lives in `rdom_showcase::demos::parse_and_render`
//! — same DOM is browsable in the showcase under "Built-ins → Parse + render".

fn main() -> std::io::Result<()> {
    rdom_showcase::demos::parse_and_render::run_standalone()
}
