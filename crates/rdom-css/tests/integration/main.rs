//! Consolidated `rdom-css` integration tests. See
//! `crates/rdom-tui/tests/integration/main.rs` for the pattern.

#![allow(dead_code)]

mod at_rules;
mod calc_parsing;
mod colors;
mod custom_properties;
mod display_flow;
mod important;
mod inline_style;
mod lengths;
mod malformed_declarations;
mod padding_shorthand;
mod positioning;
mod properties;
mod round_trip;
mod selectors;
mod strict;
mod tokenizer;
mod transitions;
