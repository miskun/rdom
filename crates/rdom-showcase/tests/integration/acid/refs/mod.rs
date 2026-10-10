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
pub mod t17_selection;
pub mod t18_grid;
pub mod t19_floats;
pub mod t20_media;
pub mod t21_supports;
pub mod t22_container;
pub mod t23_transforms;
pub mod t24_effects;
pub mod t25_multicol;
pub mod t26_anchor;
pub mod t27_logical;
pub mod t28_boxes;
pub mod t29_text;
pub mod t30_layout;
pub mod t31_motion;
pub mod t32_scroll;
pub mod t33_states;
pub mod t34_pointer;
pub mod t35_focus;
pub mod t36_form_state;
pub mod t37_transitions;
pub mod t38_cssom;
pub mod t39_smooth_scroll;
pub mod t40_caret;
pub mod t41_snap;

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
    &t17_selection::REF,
    &t18_grid::REF,
    &t19_floats::REF,
    &t20_media::REF,
    &t21_supports::REF,
    &t22_container::REF,
    &t23_transforms::REF,
    &t24_effects::REF,
    &t25_multicol::REF,
    &t26_anchor::REF,
    &t27_logical::REF,
    &t28_boxes::REF,
    &t29_text::REF,
    &t30_layout::REF,
    &t31_motion::REF,
    &t32_scroll::REF,
    &t33_states::REF,
    &t34_pointer::REF,
    &t35_focus::REF,
    &t36_form_state::REF,
    &t37_transitions::REF,
    &t38_cssom::REF,
    &t39_smooth_scroll::REF,
    &t40_caret::REF,
    &t41_snap::REF,
];
