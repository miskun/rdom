//! `TuiStyle` tests.
/// `FLOW` and `POINTER_EVENTS` shared bit 39
/// (`STYLE-MASK-COLLISION-1`), so `pointer-events: none !important`
/// also made `display`'s derived flow important. Bits are numbered by
/// the property table now; `property_dispatch`'s
/// `every_dispatched_field_has_a_distinct_important_bit` pins them all.
#[test]
fn pointer_events_importance_does_not_mark_flow() {
    use super::ImportantMask as M;
    let style = super::TuiStyle::new().pointer_events_important(crate::layout::PointerEvents::None);
    assert!(style.important.contains(M::POINTER_EVENTS));
    assert!(!style.important.contains(M::FLOW));
}

use super::*;
use crate::layout::{MaxSize, Padding};

#[test]
fn default_is_empty() {
    assert!(TuiStyle::default().is_empty());
    assert_eq!(TuiStyle::default().declared_count(), 0);
}

#[test]
fn flex_builders_set_display_flow_direction() {
    use crate::layout::{Direction, Flow};
    let row = TuiStyle::new().flex_row();
    assert_eq!(row.display, Some(Value::Specified(Display::Block)));
    assert_eq!(row.flow, Some(Value::Specified(Flow::Flex)));
    assert_eq!(row.direction, Some(Value::Specified(Direction::Row)));

    let col = TuiStyle::new().flex_column();
    assert_eq!(col.flow, Some(Value::Specified(Flow::Flex)));
    assert_eq!(col.direction, Some(Value::Specified(Direction::Column)));

    let inl = TuiStyle::new().inline_flex();
    assert_eq!(inl.display, Some(Value::Specified(Display::Inline)));
    assert_eq!(inl.flow, Some(Value::Specified(Flow::Flex)));
}

#[test]
fn flex_avoids_the_display_resets_flow_trap() {
    use crate::layout::Flow;
    // `.flex()` must keep Flow::Flex even though `.display(Block)`
    // would reset it — pins the ordering the convenience guarantees.
    let s = TuiStyle::new().flex();
    assert_eq!(s.flow, Some(Value::Specified(Flow::Flex)));
}

#[test]
fn builder_sets_specified() {
    let s = TuiStyle::new()
        .fg(Color::Rgb(255, 0, 0))
        .bg(Color::Rgb(0, 0, 0))
        .bold(true);
    assert_eq!(
        s.fg,
        Some(Value::Specified(TuiColor::Literal(Color::Rgb(255, 0, 0))))
    );
    assert_eq!(
        s.bg,
        Some(Value::Specified(TuiColor::Literal(Color::Rgb(0, 0, 0))))
    );
    assert_eq!(s.bold, Some(Value::Specified(true)));
    assert_eq!(s.declared_count(), 3);
}

#[test]
fn builder_important_variant_sets_flag() {
    let s = TuiStyle::new().fg_important(Color::Rgb(255, 0, 0));
    assert_eq!(
        s.fg,
        Some(Value::Specified(TuiColor::Literal(Color::Rgb(255, 0, 0))))
    );
    assert!(s.important.contains(ImportantMask::FG));
    assert!(!s.important.contains(ImportantMask::BG));
}

#[test]
fn fg_inherit_sets_inherit_variant() {
    let s = TuiStyle::new().fg_inherit();
    assert_eq!(s.fg, Some(Value::Inherit));
}

#[test]
fn fg_initial_sets_initial_variant() {
    let s = TuiStyle::new().fg_initial();
    assert_eq!(s.fg, Some(Value::Initial));
}

#[test]
fn unified_layout_fields_settable() {
    let s = TuiStyle::new()
        .width(Size::Fixed(40))
        .padding(Padding::all(2))
        .gap(1)
        .direction(Direction::Row)
        .border(Border::single())
        .overflow(Overflow::Hidden);

    assert_eq!(s.width, Some(Value::Specified(Size::Fixed(40))));
    assert_eq!(
        s.padding,
        Sides::all(Some(Value::Specified(crate::layout::PaddingValue::Cells(
            2
        ))))
    );
    assert_eq!(
        s.row_gap,
        Some(Value::Specified(crate::layout::GapValue::Cells(1)))
    );
    assert_eq!(s.direction, Some(Value::Specified(Direction::Row)));
    assert_eq!(
        s.border_style,
        crate::layout::Sides::all(Some(Value::Specified(crate::layout::BorderStyle::Solid)))
    );
    // `overflow` shorthand writes both longhands.
    assert_eq!(s.overflow_x, Some(Value::Specified(Overflow::Hidden)));
    assert_eq!(s.overflow_y, Some(Value::Specified(Overflow::Hidden)));
    // 2 properties above + 4 padding sides + 2 gaps + 4 border-style
    // sides + 2 axes of overflow + `flex-direction`'s reverse flag = 15.
    assert_eq!(s.declared_count(), 15);
}

#[test]
fn min_max_layout_setters() {
    use crate::layout::MinSize;
    let s = TuiStyle::new()
        .min_width(10)
        .max_width(100)
        .min_height(5)
        .max_height(50);
    assert_eq!(s.min_width, Some(Value::Specified(MinSize::Cells(10))));
    assert_eq!(s.max_width, Some(Value::Specified(MaxSize::Cells(100))));
    assert_eq!(s.min_height, Some(Value::Specified(MinSize::Cells(5))));
    assert_eq!(s.max_height, Some(Value::Specified(MaxSize::Cells(50))));
}

#[test]
fn min_width_setter_accepts_auto_keyword_via_enum() {
    use crate::layout::MinSize;
    let s = TuiStyle::new().min_width(MinSize::Auto);
    assert_eq!(s.min_width, Some(Value::Specified(MinSize::Auto)));
}

#[test]
fn content_setter_accepts_content_enum() {
    let s = TuiStyle::new().content(Content::Str("→".into()));
    assert_eq!(s.content, Some(Value::Specified(Content::Str("→".into()))));
}

#[test]
fn content_var_and_concat() {
    let s = TuiStyle::new().content(Content::Concat(vec![
        Content::Str("▾ ".into()),
        Content::Var("label".into()),
    ]));
    match s.content.unwrap() {
        Value::Specified(Content::Concat(parts)) => {
            assert_eq!(parts.len(), 2);
        }
        _ => panic!("expected concat"),
    }
}

#[test]
fn important_mask_bits_isolated() {
    let s = TuiStyle::new()
        .fg(Color::Rgb(255, 0, 0))
        .bold_important(true);
    assert!(!s.important.contains(ImportantMask::FG));
    assert!(s.important.contains(ImportantMask::BOLD));
}

#[test]
fn is_empty_false_after_any_set() {
    assert!(!TuiStyle::new().fg(Color::Rgb(255, 0, 0)).is_empty());
    assert!(!TuiStyle::new().padding(Padding::all(1)).is_empty());
    assert!(!TuiStyle::new().content(Content::Str("x".into())).is_empty());
}

#[test]
fn clone_preserves_important_bits() {
    let s = TuiStyle::new().fg_important(Color::Rgb(255, 0, 0));
    let c = s.clone();
    assert!(c.important.contains(ImportantMask::FG));
}

#[test]
fn important_mask_bit_ops() {
    let m = ImportantMask::FG | ImportantMask::BG;
    assert!(m.contains(ImportantMask::FG));
    assert!(m.contains(ImportantMask::BG));
    assert!(!m.contains(ImportantMask::BOLD));
}

#[test]
fn every_property_has_a_setter() {
    // Smoke test: call every setter. If one was missed it will fail
    // to compile or miss a field below.
    let s = TuiStyle::new()
        .fg(Color::Rgb(255, 0, 0))
        .bg(Color::Rgb(0, 0, 0))
        .border_fg(Color::Rgb(255, 255, 255))
        .bold(true)
        .italic(true)
        .width(Size::Fixed(1))
        .height(Size::Fixed(1))
        .min_width(1)
        .max_width(1)
        .min_height(1)
        .max_height(1)
        .padding(Padding::all(1))
        .gap(1)
        .border(Border::single())
        .direction(Direction::Row)
        .overflow(Overflow::Hidden)
        .content(Content::Str("x".into()));
    // The `overflow` shorthand counts as 2 (writes both axes), and
    // `border_fg`, `border` and `padding` as 4 each (one longhand per
    // side), and `direction` and `gap` 2 each (the axis and the reverse
    // flag; `row-gap` and `column-gap`).
    assert_eq!(s.declared_count(), 29);
}

#[test]
fn fg_var_sets_var_reference() {
    let s = TuiStyle::new().fg_var("accent");
    match &s.fg {
        Some(Value::Specified(TuiColor::Var { name, fallback })) => {
            assert_eq!(name, "accent");
            assert!(fallback.is_none());
        }
        _ => panic!("expected var(--accent)"),
    }
}

#[test]
fn fg_accepts_literal_and_var_via_into() {
    // Both forms compile and produce the right variant.
    let a = TuiStyle::new().fg(Color::Rgb(255, 0, 0));
    let b = TuiStyle::new().fg(TuiColor::var("accent"));
    assert!(matches!(a.fg, Some(Value::Specified(TuiColor::Literal(_)))));
    assert!(matches!(b.fg, Some(Value::Specified(TuiColor::Var { .. }))));
}

#[test]
fn every_property_has_important_setter() {
    let s = TuiStyle::new()
        .fg_important(Color::Rgb(255, 0, 0))
        .bg_important(Color::Rgb(0, 0, 0))
        .border_fg_important(Color::Rgb(255, 255, 255))
        .border_width_important(crate::layout::BorderWidth::Thick)
        .bold_important(true)
        .italic_important(true)
        .width_important(Size::Fixed(1))
        .height_important(Size::Fixed(1))
        .min_width_important(1)
        .max_width_important(1)
        .min_height_important(1)
        .max_height_important(1)
        .padding_important(Padding::all(1))
        .margin_important(crate::layout::Margin::all_cells(1))
        .gap_important(1)
        .flex_shrink_important(1.0)
        .flex_basis_important(crate::layout::FlexBasis::Auto)
        .border_important(Border::single())
        .border_radius_important(crate::layout::BorderRadius::cells(1.0))
        .box_shadow_important(vec![])
        .border_spacing_important(crate::layout::BorderSpacing::default())
        .background_image_important(vec![])
        .background_position_important(vec![])
        .background_size_important(vec![])
        .background_repeat_important(vec![])
        .background_attachment_important(vec![])
        .background_origin_important(vec![])
        .background_clip_important(vec![])
        .border_collapse_important(crate::layout::BorderCollapse::Collapse)
        .direction_important(Direction::Row)
        .overflow_important(Overflow::Hidden)
        .display_important(Display::Inline)
        .flow_important(crate::layout::Flow::Block)
        .counter_reset_important(vec![])
        .counter_increment_important(vec![])
        .pointer_events_important(crate::layout::PointerEvents::None)
        .visibility_important(crate::layout::Visibility::Hidden)
        .order_important(1)
        .grid_template_columns_important(crate::layout::GridTemplate::None)
        .grid_template_rows_important(crate::layout::GridTemplate::None)
        .grid_template_areas_important(crate::layout::GridTemplateAreas::NONE)
        .grid_auto_columns_important([crate::layout::TrackSize::AUTO])
        .grid_auto_rows_important([crate::layout::TrackSize::AUTO])
        .grid_auto_flow_important(crate::layout::GridAutoFlow::ROW)
        .grid_row_start_important(crate::layout::GridLine::Auto)
        .grid_row_end_important(crate::layout::GridLine::Auto)
        .grid_column_start_important(crate::layout::GridLine::Auto)
        .grid_column_end_important(crate::layout::GridLine::Auto)
        .flex_wrap_important(crate::layout::FlexWrap::Wrap)
        .justify_content_important(crate::layout::Align::Center)
        .align_items_important(crate::layout::Align::Center)
        .align_content_important(crate::layout::Align::Center)
        .justify_items_important(crate::layout::Align::Center)
        .justify_self_important(crate::layout::Align::Center)
        .align_self_important(crate::layout::Align::Center)
        .flex_grow_important(1.0)
        .scrollbar_gutter_important(crate::layout::ScrollbarGutter::Stable)
        .scrollbar_width_important(crate::layout::ScrollbarWidth::Thin)
        .scrollbar_color_important(crate::layout::ScrollbarColor::Auto)
        .overscroll_behavior_x_important(crate::layout::OverscrollBehavior::Contain)
        .overscroll_behavior_y_important(crate::layout::OverscrollBehavior::None)
        .scroll_padding_top_important(crate::layout::ScrollPadding::Auto)
        .scroll_padding_right_important(crate::layout::ScrollPadding::Auto)
        .scroll_padding_bottom_important(crate::layout::ScrollPadding::Auto)
        .scroll_padding_left_important(crate::layout::ScrollPadding::Auto)
        .scroll_margin_top_important(1)
        .scroll_margin_right_important(1)
        .scroll_margin_bottom_important(1)
        .scroll_margin_left_important(1)
        .scroll_snap_type_important(crate::layout::ScrollSnapType::None)
        .scroll_snap_align_important(crate::layout::ScrollSnapAlign::default())
        .scroll_snap_stop_important(crate::layout::ScrollSnapStop::Always)
        .overflow_clip_margin_important(Default::default())
        .text_overflow_important(Default::default())
        .max_lines_important(Some(1))
        .block_ellipsis_important(Default::default())
        .continue_important(Default::default())
        .webkit_box_orient_important(Default::default())
        .scroll_behavior_important(crate::layout::ScrollBehavior::Smooth)
        .color_scheme_important(crate::color::ColorSchemeList::normal())
        .white_space_important(crate::layout::WhiteSpace::Pre)
        .word_break_important(crate::layout::WordBreak::KeepAll)
        .overflow_wrap_important(crate::layout::OverflowWrap::Anywhere)
        .line_break_important(crate::layout::LineBreak::Strict)
        .hyphens_important(crate::layout::Hyphens::None)
        .tab_size_important(crate::layout::TabSize::Number(4.0))
        .line_height_important(crate::layout::LineHeight::Number(2.0))
        .vertical_align_important(crate::layout::VerticalAlign::Super)
        .text_transform_important(crate::layout::TextTransform::NONE)
        .text_indent_important(crate::layout::TextIndent::cells(1))
        .text_align_important(crate::layout::TextAlign::Center)
        .text_justify_important(crate::layout::TextJustify::None)
        .text_wrap_style_important(crate::layout::TextWrapStyle::Balance)
        .user_select_important(UserSelect::None)
        .caret_color_important(CaretColor::Transparent)
        .caret_text_color_important(CaretTextColor::Auto)
        .text_decoration_important(TextDecoration::Underline)
        .opacity_important(0.5)
        .aspect_ratio_important(16, 9)
        .box_sizing_important(crate::layout::BoxSizing::BorderBox)
        .margin_trim_important(crate::layout::MarginTrim::BLOCK)
        .text_direction_important(crate::layout::TextDirection::Rtl)
        .writing_mode_important(crate::layout::WritingMode::VerticalRl)
        .contain_intrinsic_width_important(Default::default())
        .contain_intrinsic_height_important(Default::default())
        .content_important(Content::Str("x".into()))
        .position_important(crate::layout::Position::Absolute)
        .top_important(crate::layout::Length::Cells(1))
        .right_important(crate::layout::Length::Cells(1))
        .bottom_important(crate::layout::Length::Cells(1))
        .left_important(crate::layout::Length::Cells(1))
        .z_index_important(crate::layout::ZIndex::Value(1))
        .float_important(crate::layout::Float::Left)
        .clear_important(crate::layout::Clear::Both)
        .flow_important(crate::layout::Flow::Block)
        .transitions_important();
    assert_eq!(s.important, ImportantMask::all());
}

/// C2G-LAYOUT-SAFETY — CSS Flexbox §7.1: flex factors are `<number
/// [0,∞]>`. A factor built in Rust goes through the setter, which keeps
/// it in range: a negative or NaN grow is no grow (`Size::Auto`, as the
/// parser maps `flex: 0`), an infinite one the largest finite factor;
/// a negative or NaN shrink is 0.
#[test]
fn rust_built_flex_factors_are_validated_at_the_setter() {
    use crate::layout::Size;
    let w = |s: Size| TuiStyle::new().width(s).width;
    assert_eq!(w(Size::Flex(-1.0)), Some(Value::Specified(Size::Auto)));
    assert_eq!(w(Size::Flex(f32::NAN)), Some(Value::Specified(Size::Auto)));
    assert_eq!(w(Size::Flex(0.0)), Some(Value::Specified(Size::Auto)));
    assert_eq!(
        w(Size::Flex(f32::INFINITY)),
        Some(Value::Specified(Size::Flex(f32::MAX)))
    );
    assert_eq!(w(Size::Flex(2.5)), Some(Value::Specified(Size::Flex(2.5))));
    assert_eq!(w(Size::Fixed(3)), Some(Value::Specified(Size::Fixed(3))));
    assert_eq!(
        TuiStyle::new().height_important(Size::Flex(-2.0)).height,
        Some(Value::Specified(Size::Auto))
    );
    let shrink = |v: f32| TuiStyle::new().flex_shrink(v).flex_shrink;
    assert_eq!(shrink(-1.0), Some(Value::Specified(0.0)));
    assert_eq!(shrink(f32::NAN), Some(Value::Specified(0.0)));
    assert_eq!(shrink(f32::INFINITY), Some(Value::Specified(f32::MAX)));
    assert_eq!(shrink(1.5), Some(Value::Specified(1.5)));
    assert_eq!(
        TuiStyle::new().flex_shrink_important(-1.0).flex_shrink,
        Some(Value::Specified(0.0))
    );
}

/// C2G-LAYOUT-SAFETY — CSS Values 4 §5.7: a `<ratio>`'s terms are
/// `<number [0,∞]>`; the terms are private, so `AspectRatio::new` is
/// the only way to build one and a non-finite or negative term is no
/// ratio.
#[test]
fn aspect_ratio_terms_are_validated() {
    use crate::layout::AspectRatio;
    assert_eq!(AspectRatio::new(f32::NAN, 1.0), None);
    assert_eq!(AspectRatio::new(1.0, f32::INFINITY), None);
    assert_eq!(AspectRatio::new(-1.0, 1.0), None);
    let r = AspectRatio::new(16.0, 9.0).unwrap().with_auto(true);
    assert_eq!(
        (r.numerator(), r.denominator(), r.auto()),
        (16.0, 9.0, true)
    );
}

/// C6G-DECLARED-COUNT: `declared_count` counts every storage field the
/// property table has — the ones the hand-written count had drifted
/// from (`z-index`, `opacity`, `position`, the insets, `box-shadow`,
/// the transition longhands, the counters) included — plus each
/// declaration kept for the cascade that writes no field (a
/// flow-relative one, one waiting for substitution) and each custom
/// property.
#[test]
fn declared_count_counts_every_field_kind() {
    use crate::property_dispatch::{property_mask, set};
    let physical = [
        ("z-index", "3"),
        ("opacity", "0.5"),
        ("position", "relative"),
        ("top", "1"),
        ("box-shadow", "1 1 red"),
        ("transition", "color 1s"),
        ("counter-reset", "a"),
        ("color", "red"),
    ];
    let mut s = TuiStyle::new();
    let mut fields = 0;
    for (name, value) in physical {
        set(name, value, &mut s).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        fields += property_mask(name).expect("a property").count();
    }
    set("margin-inline-start", "1", &mut s).unwrap();
    set("width", "var(--w)", &mut s).unwrap();
    set("--x", "1", &mut s).unwrap();
    assert_eq!(s.declared_count(), fields + 3);
}

// ─── Alignment and flex-direction (C6G-ALIGN-API) ───────────────────

/// The alignment setters take `impl Into<Alignment>`: a bare keyword.
#[test]
fn alignment_setters_take_a_keyword() {
    use crate::layout::{Align, Alignment};
    let s = TuiStyle::new()
        .justify_content(Align::Center)
        .align_items(Alignment::safe(Align::End));
    assert_eq!(
        s.justify_content,
        Some(Value::Specified(Alignment::new(Align::Center)))
    );
    assert_eq!(
        s.align_items,
        Some(Value::Specified(Alignment::safe(Align::End)))
    );
}

/// CSS Box Alignment 3 §6.1: `space-between` is not in `align-self`'s
/// grammar — a typed setter refuses it, loudly in a debug build.
#[test]
#[should_panic(expected = "align-self")]
fn an_out_of_grammar_keyword_is_refused() {
    let _ = TuiStyle::new().align_self(crate::layout::Align::SpaceBetween);
}

/// `flex-direction` in one value: the builder sets the axis and the
/// reverse flag together.
#[test]
fn flex_direction_sets_axis_and_reverse() {
    use crate::layout::{Direction, FlexDirection};
    let s = TuiStyle::new().flex_direction(FlexDirection::ColumnReverse);
    assert_eq!(s.direction, Some(Value::Specified(Direction::Column)));
    assert_eq!(s.flex_reverse, Some(Value::Specified(true)));
    let s = TuiStyle::new().direction_reverse_important(Direction::Row);
    assert!(s.important.contains(ImportantMask::FLEX_DIRECTION));
}

// ─── Per-side spacing setters (C6G-SIDE-SETTERS) ────────────────────

/// CSS Box 3 §3.2 / §4.2: `margin-left` and `padding-top` are longhands
/// of their own — a per-side setter declares that side alone, and its
/// `!important` twin marks that side's bit alone.
#[test]
fn per_side_setters_write_one_longhand() {
    use crate::layout::{MarginValue, PaddingValue};
    let s = TuiStyle::new()
        .margin_left(MarginValue::Auto)
        .padding_top(2u16)
        .margin_bottom_important(-1i16);
    assert_eq!(s.margin.left, Some(Value::Specified(MarginValue::Auto)));
    assert_eq!(
        s.margin.bottom,
        Some(Value::Specified(MarginValue::Cells(-1)))
    );
    assert_eq!((s.margin.top.clone(), s.margin.right.clone()), (None, None));
    assert_eq!(
        s.padding.top,
        Some(Value::Specified(PaddingValue::Cells(2)))
    );
    assert!(s.padding.right.is_none() && s.padding.left.is_none());
    assert!(s.important.contains(ImportantMask::MARGIN_BOTTOM));
    assert!(!s.important.contains(ImportantMask::MARGIN_LEFT));
}

/// `padding` takes `impl Into<Padding>` as `margin` takes `impl
/// Into<Margin>`: a plain count is every side.
#[test]
fn padding_takes_a_count_as_margin_does() {
    assert_eq!(
        TuiStyle::new().padding(2u16),
        TuiStyle::new().padding(Padding::all(2))
    );
    assert_eq!(
        TuiStyle::new().margin(1i16),
        TuiStyle::new().margin(crate::layout::Margin::all_cells(1))
    );
}

/// C7G-GRID-SETTERS: `place-items` / `place-content` / `place-self` (CSS
/// Box Alignment 3 §6.4, §5.5, §6.5) set the `align-*` longhand from their
/// first value and the `justify-*` one from their second, each checked as
/// its longhand's builder checks it.
#[test]
fn place_builders_set_both_longhands() {
    use crate::layout::Align;
    assert_eq!(
        TuiStyle::new().place_items(Align::Center, Align::Start),
        TuiStyle::new()
            .align_items(Align::Center)
            .justify_items(Align::Start)
    );
    assert_eq!(
        TuiStyle::new().place_content(Align::End, Align::SpaceBetween),
        TuiStyle::new()
            .align_content(Align::End)
            .justify_content(Align::SpaceBetween)
    );
    assert_eq!(
        TuiStyle::new().place_self(Align::Stretch, Align::Center),
        TuiStyle::new()
            .align_self(Align::Stretch)
            .justify_self(Align::Center)
    );
}

/// C7G-GRID-SETTERS: `grid-area: head` (CSS Grid 2 §8.4) — a lone
/// `<custom-ident>` — sets all four placement longhands to that name, as
/// the shorthand's omission rule copies it.
#[test]
fn grid_area_named_sets_all_four_lines() {
    use crate::layout::GridLine;
    let head = || GridLine::named("head");
    assert_eq!(
        TuiStyle::new().grid_area_named("head"),
        TuiStyle::new().grid_area(head(), head(), head(), head())
    );
}
