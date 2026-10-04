//! `TuiStyle` tests.
/// Every `ImportantMask` flag owns one bit. `FLOW` and
/// `POINTER_EVENTS` shared bit 39 (`STYLE-MASK-COLLISION-1`), so
/// `pointer-events: none !important` also made `display`'s derived
/// flow important.
#[test]
fn important_mask_bits_are_unique() {
    use super::ImportantMask as M;
    let all = [
        M::FG,
        M::BG,
        M::BORDER_FG,
        M::BOLD,
        M::ITALIC,
        M::WIDTH,
        M::HEIGHT,
        M::MIN_WIDTH,
        M::MAX_WIDTH,
        M::MIN_HEIGHT,
        M::MAX_HEIGHT,
        M::PADDING,
        M::GAP,
        M::BORDER,
        M::DIRECTION,
        M::OVERFLOW_X,
        M::CONTENT,
        M::DISPLAY,
        M::WHITE_SPACE,
        M::USER_SELECT,
        M::OVERFLOW_Y,
        M::POSITION,
        M::TOP,
        M::RIGHT,
        M::BOTTOM,
        M::LEFT,
        M::Z_INDEX,
        M::TRANSITIONS,
        M::TEXT_DECORATION,
        M::OPACITY,
        M::ASPECT_RATIO,
        M::MARGIN,
        M::BORDER_COLLAPSE,
        M::CARET_COLOR,
        M::CARET_TEXT_COLOR,
        M::FLEX_SHRINK,
        M::FLEX_BASIS,
        M::POINTER_EVENTS,
        M::FLOW,
        M::COUNTER_RESET,
        M::COUNTER_INCREMENT,
        M::SCROLLBAR_GUTTER,
        M::SCROLL_BEHAVIOR,
        M::COLOR_SCHEME,
    ];
    for (i, a) in all.iter().enumerate() {
        for b in &all[i + 1..] {
            assert_eq!(a.bits() & b.bits(), 0, "{a:?} and {b:?} share a bit");
        }
    }
    assert_eq!(M::all().bits().count_ones() as usize, all.len());
    let style = super::TuiStyle::new().pointer_events_important(crate::layout::PointerEvents::None);
    assert!(!style.important.contains(M::FLOW));
}

use super::*;
use crate::layout::MaxSize;

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
    assert_eq!(s.padding, Some(Value::Specified(Padding::all(2))));
    assert_eq!(
        s.gap,
        Some(Value::Specified(crate::layout::GapValue::Cells(1)))
    );
    assert_eq!(s.direction, Some(Value::Specified(Direction::Row)));
    assert_eq!(s.border, Some(Value::Specified(Border::single())));
    // `overflow` shorthand writes both longhands.
    assert_eq!(s.overflow_x, Some(Value::Specified(Overflow::Hidden)));
    assert_eq!(s.overflow_y, Some(Value::Specified(Overflow::Hidden)));
    // 5 properties above + 2 axes of overflow = 7.
    assert_eq!(s.declared_count(), 7);
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
    // The `overflow` shorthand counts as 2 (writes both axes).
    assert_eq!(s.declared_count(), 18);
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
        .border_collapse_important(crate::layout::BorderCollapse::Collapse)
        .direction_important(Direction::Row)
        .overflow_important(Overflow::Hidden)
        .display_important(Display::Inline)
        .flow_important(crate::layout::Flow::Block)
        .counter_reset_important(vec![])
        .counter_increment_important(vec![])
        .pointer_events_important(crate::layout::PointerEvents::None)
        .scrollbar_gutter_important(crate::layout::ScrollbarGutter::Stable)
        .scroll_behavior_important(crate::layout::ScrollBehavior::Smooth)
        .color_scheme_important(crate::color::ColorSchemeList::normal())
        .white_space_important(WhiteSpace::Pre)
        .user_select_important(UserSelect::None)
        .caret_color_important(CaretColor::Transparent)
        .caret_text_color_important(CaretTextColor::Auto)
        .text_decoration_important(TextDecoration::Underline)
        .opacity_important(0.5)
        .aspect_ratio_important(16, 9)
        .content_important(Content::Str("x".into()))
        .position_important(crate::layout::Position::Absolute)
        .top_important(crate::layout::Length::Cells(1))
        .right_important(crate::layout::Length::Cells(1))
        .bottom_important(crate::layout::Length::Cells(1))
        .left_important(crate::layout::Length::Cells(1))
        .z_index_important(crate::layout::ZIndex::Value(1))
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
