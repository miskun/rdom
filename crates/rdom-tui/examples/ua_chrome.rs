//! UA chrome showcase — what naked HTML built-ins look like with
//! rdom's UA stylesheet only.
//!
//! Run: `cargo run -p rdom-tui --example ua_chrome`
//!
//! Implementation lives in `rdom_showcase::demos::ua_chrome` — same
//! DOM is browsable in the showcase under "Built-ins → UA chrome".

fn main() -> std::io::Result<()> {
    rdom_showcase::demos::ua_chrome::run_standalone()
}
