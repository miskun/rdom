//! The acid tiles, one module each, in `ACID.md`'s numbering.

use super::Tile;

mod t01_cascade;
mod t02_specificity;
mod t03_inheritance;
mod t04_selectors;
mod t05_box_model;
mod t06_margins;
mod t07_flex;
mod t08_inline;
mod t09a_generated;
mod t09b_lists;
mod t09c_first;
mod t10_positioning;
mod t11_stacking;
mod t12_opacity;
mod t13_overflow;
mod t14_tables;
mod t15a_forms;
mod t15b_top_layer;
mod t15c_modal;
mod t16_display;
mod t17_selection;
mod t18_grid;
mod t19_floats;

/// Every tile, in `ACID.md` order.
pub const TILES: &[&Tile] = &[
    &t01_cascade::TILE,
    &t02_specificity::TILE,
    &t03_inheritance::TILE,
    &t04_selectors::TILE,
    &t05_box_model::TILE,
    &t06_margins::TILE,
    &t07_flex::TILE,
    &t08_inline::TILE,
    &t09a_generated::TILE,
    &t09b_lists::TILE,
    &t09c_first::TILE,
    &t10_positioning::TILE,
    &t11_stacking::TILE,
    &t12_opacity::TILE,
    &t13_overflow::TILE,
    &t14_tables::TILE,
    &t15a_forms::TILE,
    &t15b_top_layer::TILE,
    &t15c_modal::TILE,
    &t16_display::TILE,
    &t17_selection::TILE,
    &t18_grid::TILE,
    &t19_floats::TILE,
];
