//! Build-script-generated camelCase aliases on
//! [`StyleDeclaration`](super::StyleDeclaration) — one method per
//! shipped CSS property in
//! [`rdom_style::property_dispatch::property_names`].
//!
//! The actual method definitions are written to
//! `$OUT_DIR/cssom_aliases.rs` by `crates/rdom-tui/build.rs` and
//! `include!`d into the `impl` block below. Authors then write
//! `el.style().color()` instead of
//! `el.style().get_property_value("color")` for the common case.
//!
//! Single source of truth: `build.rs` calls
//! `property_dispatch::property_names()` at build time, so the
//! alias list cannot drift from the runtime dispatch table.

// The generated file is a full `impl crate::cssom::StyleDeclaration`
// block — included here at module scope so Rust can parse it.
include!(concat!(env!("OUT_DIR"), "/cssom_aliases.rs"));
