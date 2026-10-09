//! The hand-derived references, one file per tile, in `ACID.md`'s
//! numbering. Each file carries its spec citations and the derivation of
//! every cell (ground rule 1: from the spec, never from rdom's output).

use super::reference::Reference;

pub mod t01_cascade;
pub mod t02_specificity;
pub mod t03_inheritance;
pub mod t04_selectors;
pub mod t05_box_model;
pub mod t06_margins;
pub mod t07_flex;
pub mod t08_inline;
pub mod t09a_generated;

/// Every reference.
pub const ALL: &[&Reference] = &[
    &t01_cascade::REF,
    &t02_specificity::REF,
    &t03_inheritance::REF,
    &t04_selectors::REF,
    &t05_box_model::REF,
    &t06_margins::REF,
    &t07_flex::REF,
    &t08_inline::REF,
    &t09a_generated::REF,
];
