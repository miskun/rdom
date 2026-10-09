//! Inheritance + layout-change diff helpers used by the cascade walk.
//!
//! `inherit_inheritable_from` seeds the working `ComputedStyle` with
//! its parent's inheritable bits before any rule application.
//!
//! `layout_differs` answers "did any layout-affecting property change
//! from the previous cascade?" — used by the walk to set
//! `layout_dirty` on the element.

use crate::style::{ComputedStyle, Modifier};

/// Copy the inherited subset of properties from `parent` into
/// `working`. Called at the start of every element's cascade to seed
/// from the parent's computed style. The set is
/// `rdom_style::property_dispatch::inherits` — the cascade test
/// `cascade_inherits_exactly_the_style_crates_inherited_set` probes every
/// property against that table (`STYLE-INHERITS-TWO-SOURCES-1`).
pub(super) fn inherit_inheritable_from(working: &mut ComputedStyle, parent: &ComputedStyle) {
    working.fg = parent.fg;
    // Modifiers: copy only the font's bits (bold / italic), which
    // `finalize_font` derives again from the inherited font. The
    // decorations are not modifiers: `text-decoration` does not inherit,
    // it propagates (CSS Text Decoration 4 §2.1) — an underlined
    // element's descendants' text is underlined through
    // `applied_decorations` (`cascade/text_decoration.rs`), not here.
    let inherit_mods = Modifier::BOLD | Modifier::ITALIC;
    working.modifiers = parent.modifiers & inherit_mods;
    // The CSS Text properties all inherit (CSS Text 3 / 4); display
    // does not. Neither does `user-select` (CSS UI 4 §6.1): its *used*
    // value of `auto` depends on the parent's used value, resolved in
    // `style::user_select`.
    working.text = parent.text.clone();
    // CSS Fonts 4: the font properties all inherit.
    working.font = parent.font.clone();
    working.pointer_events = parent.pointer_events;
    // CSS Generated Content 3 §2.1: `quotes` inherits.
    working.quotes = parent.quotes.clone();
    // CSS Lists 3 §3: the list properties inherit.
    working.list_style_type = parent.list_style_type.clone();
    working.list_style_position = parent.list_style_position;
    working.list_style_image = parent.list_style_image.clone();
    working.marker_side = parent.marker_side;
    // CSS Values 5 §11: `interpolate-size` inherits.
    working.interpolate_size = parent.interpolate_size;
    working.visibility = parent.visibility;
    // CSS UI 4 §7.1: `caret-color` inherits; rdom's `caret-text-color`
    // mirrors it.
    working.caret_color = parent.caret_color.clone();
    working.caret_text_color = parent.caret_text_color.clone();
    // CSS UI 4 §4.1: `cursor` inherits.
    working.ui.cursor = parent.ui.cursor.clone();
    // §6.2: the caret's shape and animation inherit.
    working.ui.caret_shape = parent.ui.caret_shape;
    working.ui.caret_animation = parent.ui.caret_animation;
    // §6.3: `accent-color` inherits.
    working.ui.accent_color = parent.ui.accent_color.clone();
    // CSS Color Adjust 1 §2: `color-scheme` inherits.
    working.color_scheme = parent.color_scheme.clone();
    // CSS 2.1 §17.6.1: `border-spacing` inherits; §17.4.1:
    // `caption-side` too.
    working.border_spacing = parent.border_spacing.clone();
    working.table.caption_side = parent.table.caption_side;
    working.table.empty_cells = parent.table.empty_cells;
    // CSS Writing Modes 4 §2.1 / §3.1: `direction` and `writing-mode`
    // inherit.
    working.text_direction = parent.text_direction;
    working.writing_mode = parent.writing_mode;
    // CSS Overflow 4 §4.3: `block-ellipsis` inherits.
    working.block_ellipsis = parent.block_ellipsis.clone();
    // CSS Scrollbars 1 §2: `scrollbar-color` inherits.
    working.scrollbar_color = parent.scrollbar_color.clone();
    // `border-collapse` does NOT inherit in rdom — documented
    // divergence (BORDER-MODEL-1). Containers that want their direct
    // children to participate in collapse declare it themselves;
    // demos and downstream consumer subtrees never inherit the
    // chrome's choice implicitly. `working.border_collapse` keeps
    // its initialized default (`Separate`); explicit author
    // declarations are applied later in `apply_border_collapse`.
    // Inherit custom-property map by Rc::clone (cheap).
    working.vars = parent.vars.clone();
    working.animated_vars = parent.animated_vars.clone();
}

/// The computed style of an anonymous box whose parent box is styled
/// `parent` (CSS 2.1 §9.2.1.1, CSS Display 3 §2.2): it inherits the
/// inheritable properties and takes every other one's initial value —
/// a `display: block` box with no margins, padding or border.
pub(crate) fn anonymous_box_style(parent: &ComputedStyle) -> ComputedStyle {
    let mut style = ComputedStyle::initial();
    inherit_inheritable_from(&mut style, parent);
    style
}

/// True iff any layout-affecting computed property differs between
/// `a` and `b`. When a new layout-affecting property lands, add its
/// field comparison here (the cascade test `layout_differs_covers_…`
/// probes the geometry fields).
pub(super) fn layout_differs(a: &ComputedStyle, b: &ComputedStyle) -> bool {
    a.width != b.width
        || a.height != b.height
        || a.min_width != b.min_width
        || a.max_width != b.max_width
        || a.min_height != b.min_height
        || a.max_height != b.max_height
        || a.box_sizing != b.box_sizing
        || a.aspect_ratio != b.aspect_ratio
        || a.padding != b.padding
        || a.margin != b.margin
        || a.margin_trim != b.margin_trim
        || a.row_gap != b.row_gap
        || a.column_gap != b.column_gap
        || a.flex_grow != b.flex_grow
        || a.flex_shrink != b.flex_shrink
        || a.flex_basis != b.flex_basis
        || a.order != b.order
        || a.grid_template_columns != b.grid_template_columns
        || a.grid_template_rows != b.grid_template_rows
        || a.grid_template_areas != b.grid_template_areas
        || a.grid_auto_columns != b.grid_auto_columns
        || a.grid_auto_rows != b.grid_auto_rows
        || a.grid_auto_flow != b.grid_auto_flow
        || a.grid_row_start != b.grid_row_start
        || a.grid_row_end != b.grid_row_end
        || a.grid_column_start != b.grid_column_start
        || a.grid_column_end != b.grid_column_end
        || a.border != b.border
        || a.border_style != b.border_style
        || a.border_width != b.border_width
        || a.border_collapse != b.border_collapse
        || a.border_collapse_declared != b.border_collapse_declared
        || a.direction != b.direction
        || a.flex_reverse != b.flex_reverse
        || a.flex_wrap != b.flex_wrap
        || a.justify_content != b.justify_content
        || a.align_items != b.align_items
        || a.align_content != b.align_content
        || a.justify_items != b.justify_items
        || a.justify_self != b.justify_self
        || a.align_self != b.align_self
        || a.text_direction != b.text_direction
        || a.overflow_x != b.overflow_x
        || a.overflow_y != b.overflow_y
        || a.overflow_clip_margin != b.overflow_clip_margin
        || a.line_clamp_container != b.line_clamp_container
        || a.max_lines != b.max_lines
        || a.display != b.display
        // The table model (CSS 2.1 §17).
        || a.table != b.table
        || a.border_spacing != b.border_spacing
        // `collapse` removes a flex item or table row from layout.
        || a.visibility != b.visibility
        || a.text != b.text
        || a.vertical_align != b.vertical_align
        // Positioning: the box's placement, its containing-block role,
        // and stacking all feed layout / paint order.
        || a.position != b.position
        || a.top != b.top
        || a.right != b.right
        || a.bottom != b.bottom
        || a.left != b.left
        || a.z_index != b.z_index
        || a.float != b.float
        || a.clear != b.clear
        || a.flow != b.flow
        || a.scrollbar_gutter != b.scrollbar_gutter
        || a.scrollbar_width != b.scrollbar_width
        // Size containment and the size it gives a contained box (CSS
        // Containment 2 §3.1, CSS Sizing 4 §6.1).
        || a.container_type != b.container_type
        // Containment's formatting context, containing blocks, clip and
        // stacking (CSS Containment 2 §3, CSS Will Change 1 §3).
        || a.contain != b.contain
        || a.content_visibility != b.content_visibility
        || a.will_change != b.will_change
        || a.contain_intrinsic_width != b.contain_intrinsic_width
        || a.contain_intrinsic_height != b.contain_intrinsic_height
        // A transform's translation moves the box; any transform or filter
        // makes it a containing block (CSS Transforms 1 §2, Filter Effects
        // 1 §5).
        || a.effects.layout_differs(&b.effects)
}
