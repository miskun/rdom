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
pub mod t09b_lists;
pub mod t09c_first;
pub mod t10_positioning;
pub mod t11_stacking;
pub mod t12_opacity;
pub mod t13_overflow;
pub mod t14_tables;
pub mod t15a_forms;
pub mod t15b_top_layer;
pub mod t15c_modal;
pub mod t16_display;

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
    &t09b_lists::REF,
    &t09c_first::REF,
    &t10_positioning::REF,
    &t11_stacking::REF,
    &t12_opacity::REF,
    &t13_overflow::REF,
    &t14_tables::REF,
    &t15a_forms::REF,
    &t15b_top_layer::REF,
    &t15c_modal::REF,
    &t16_display::REF,
];
