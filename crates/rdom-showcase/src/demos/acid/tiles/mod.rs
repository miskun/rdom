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
mod t20_media;
mod t21_supports;
mod t22_container;
mod t23_transforms;
mod t24_effects;
mod t25_multicol;
mod t26_anchor;
mod t27_logical;
mod t28_boxes;
mod t29_text;
mod t30_layout;
mod t31_motion;
mod t32_scroll;
mod t33_states;

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
    &t20_media::TILE,
    &t21_supports::TILE,
    &t22_container::TILE,
    &t23_transforms::TILE,
    &t24_effects::TILE,
    &t25_multicol::TILE,
    &t26_anchor::TILE,
    &t27_logical::TILE,
    &t28_boxes::TILE,
    &t29_text::TILE,
    &t30_layout::TILE,
    &t31_motion::TILE,
    &t32_scroll::TILE,
    &t33_states::TILE,
];

/// The sheets the tiles' `<style>` elements `@import`, by URL — what the
/// page's import loader ([`super::import_loader`]) serves.
pub const IMPORTS: &[&[(&str, &str)]] = &[t20_media::IMPORT, t21_supports::IMPORT];
