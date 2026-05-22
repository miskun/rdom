//! Tab-navigable form built from native `<form>` + `<input>` +
//! `<button>` builtins.
//!
//! Run: `cargo run -p rdom-tui --example tab_form`
//!
//! Implementation lives in `rdom_showcase::demos::tab_form` — same
//! DOM is browsable in the showcase under "Forms → Tab form".

fn main() -> std::io::Result<()> {
    rdom_showcase::demos::tab_form::run_standalone()
}
