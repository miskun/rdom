//! Consolidated `rdom-css` integration tests. See
//! `crates/rdom-tui/tests/integration/main.rs` for the pattern.

#![allow(dead_code)]

mod at_rules;
mod calc_parsing;
mod colors;
mod counter_style;
mod custom_properties;
mod display_flow;
mod import;
mod important;
mod inline_style;
mod layers;
mod lengths;
mod malformed_declarations;
mod nesting;
mod padding_shorthand;
mod positioning;
mod properties;
mod property;
mod round_trip;
mod scope;
mod selectors;
mod strict;
mod tokenizer;
mod transitions;
