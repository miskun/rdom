//! `ComputedStyle::initial` — every property's initial value (each
//! spec's "Initial:" line), the style a cascade starts from.

use std::rc::Rc;

use crate::layout::{
    Border, CaretColor, CaretTextColor, Direction, Display, Overflow, Padding, Size, UserSelect,
};
use crate::{Color, Modifier};

use super::{ComputedStyle, VarMap};

thread_local! {
    /// The empty `var()` map every initial style shares: building one is
    /// the cascade's commonest step (each element and pseudo-element
    /// starts from it), and a fresh map per start was an allocation each.
    static NO_VARS: VarMap = Rc::new(std::collections::HashMap::new());
}

impl ComputedStyle {
    /// Spec initial values: what every property starts as before any
    /// cascade input is applied. `Color::Reset` means "use the terminal
    /// default" — `color`'s initial value (`CanvasText`); `background-color`
    /// starts `transparent` (CSS Backgrounds 3 §3.2), which paints nothing,
    /// while a specified `Canvas` / `reset` paints the terminal's default
    /// background. Size/layout defaults match the legacy Element defaults
    /// for continuity.
    pub fn initial() -> Self {
        Self {
            fg: Color::Reset,
            bg: Color::TRANSPARENT,
            border_color: crate::layout::Sides::all(Color::Reset),
            modifiers: Modifier::empty(),
            opacity: 1.0,
            background_clip: crate::layout::VisualBox::BorderBox,
            width: Size::Auto,
            height: Size::Auto,
            min_width: crate::layout::MinSize::Auto,
            max_width: crate::layout::MaxSize::None,
            min_height: crate::layout::MinSize::Auto,
            max_height: crate::layout::MaxSize::None,
            box_sizing: crate::layout::BoxSizing::ContentBox,
            interpolate_size: crate::layout::InterpolateSize::NumericOnly,
            contain_intrinsic_width: crate::layout::ContainIntrinsicSize::default(),
            contain_intrinsic_height: crate::layout::ContainIntrinsicSize::default(),
            aspect_ratio: None,
            padding: Padding::default(),
            margin: crate::layout::Margin::default(),
            margin_trim: crate::layout::MarginTrim::NONE,
            row_gap: crate::layout::GapValue::Normal,
            column_gap: crate::layout::GapValue::Normal,
            flex_shrink: 1.0,
            order: 0,
            flex_grow: 0.0,
            flex_basis: crate::layout::FlexBasis::Auto,
            grid_template_columns: crate::layout::GridTemplate::None,
            grid_template_rows: crate::layout::GridTemplate::None,
            grid_template_areas: crate::layout::GridTemplateAreas::NONE,
            grid_auto_columns: std::borrow::Cow::Borrowed(crate::layout::TrackSize::AUTO_LIST),
            grid_auto_rows: std::borrow::Cow::Borrowed(crate::layout::TrackSize::AUTO_LIST),
            grid_auto_flow: crate::layout::GridAutoFlow::ROW,
            grid_row_start: crate::layout::GridLine::Auto,
            grid_row_end: crate::layout::GridLine::Auto,
            grid_column_start: crate::layout::GridLine::Auto,
            grid_column_end: crate::layout::GridLine::Auto,
            border: Border::none(),
            border_style: Border::none(),
            border_width: crate::layout::Sides::default(),
            border_radius: crate::layout::Corners::default(),
            box_shadow: Vec::new(),
            border_spacing: crate::layout::BorderSpacing::default(),
            border_collapse: crate::layout::BorderCollapse::Separate,
            border_collapse_declared: false,
            direction: Direction::Row,
            flex_reverse: false,
            flex_wrap: crate::layout::FlexWrap::NoWrap,
            justify_content: crate::layout::Alignment::NORMAL,
            align_items: crate::layout::Alignment::NORMAL,
            align_content: crate::layout::Alignment::NORMAL,
            align_self: crate::layout::Alignment::AUTO,
            justify_items: crate::layout::Alignment::LEGACY,
            justify_self: crate::layout::Alignment::AUTO,
            text_direction: crate::layout::TextDirection::Ltr,
            writing_mode: crate::layout::WritingMode::HorizontalTb,
            overflow_x: Overflow::Visible,
            overflow_y: Overflow::Visible,
            overflow_clip_margin: crate::layout::OverflowClipMargin::default(),
            text_overflow: crate::layout::TextOverflow::default(),
            max_lines: None,
            block_ellipsis: crate::layout::BlockEllipsis::NoEllipsis,
            continue_: crate::layout::Continue::Auto,
            webkit_box_orient: crate::layout::BoxOrient::InlineAxis,
            line_clamp_container: false,
            scrollbar_gutter: crate::layout::ScrollbarGutter::Auto,
            scrollbar_width: crate::layout::ScrollbarWidth::Auto,
            scrollbar_color: crate::layout::ScrollbarColor::Auto,
            overscroll_behavior_x: crate::layout::OverscrollBehavior::Auto,
            overscroll_behavior_y: crate::layout::OverscrollBehavior::Auto,
            scroll_padding: crate::layout::Sides::new(
                crate::layout::ScrollPadding::Auto,
                crate::layout::ScrollPadding::Auto,
                crate::layout::ScrollPadding::Auto,
                crate::layout::ScrollPadding::Auto,
            ),
            scroll_margin: crate::layout::Sides::new(0, 0, 0, 0),
            scroll_snap_type: crate::layout::ScrollSnapType::None,
            scroll_snap_align: crate::layout::ScrollSnapAlign {
                block: crate::layout::SnapAlign::None,
                inline: crate::layout::SnapAlign::None,
            },
            scroll_snap_stop: crate::layout::ScrollSnapStop::Normal,
            scroll_behavior: crate::layout::ScrollBehavior::Auto,
            display: Display::Block,
            flow: crate::layout::Flow::Block,
            list_item: false,
            webkit_box: false,
            establishes_new_bfc: false,
            text: crate::layout::TextStyle::default(),
            ui: crate::layout::UiStyle::default(),
            font: crate::layout::Font {
                weight: crate::layout::FontWeight::Number(400.0),
                ..crate::layout::Font::default()
            },
            vertical_align: crate::layout::VerticalAlign::Baseline,
            text_decoration: crate::layout::TextDecorations {
                line: crate::layout::TextDecorationLine::NONE,
                style: crate::layout::TextDecorationStyle::Solid,
                color: Color::Reset,
                thickness: crate::layout::TextDecorationThickness::Auto,
            },
            applied_decorations: crate::layout::AppliedDecorations::NONE,
            user_select: UserSelect::Auto,
            pointer_events: crate::layout::PointerEvents::Auto,
            visibility: crate::layout::Visibility::Visible,
            caret_color: CaretColor::Auto,
            caret_text_color: CaretTextColor::Auto,
            content: None,
            content_alt: None,
            content_quotes: Vec::new(),
            quotes: crate::Quotes::Auto,
            list_style_type: crate::layout::ListStyleType::default(),
            list_style_position: crate::layout::ListStylePosition::Outside,
            list_style_image: crate::layout::ListStyleImage::None,
            marker_side: crate::layout::MarkerSide::MatchSelf,
            position: crate::layout::Position::Static,
            top: crate::layout::Length::Auto,
            right: crate::layout::Length::Auto,
            bottom: crate::layout::Length::Auto,
            left: crate::layout::Length::Auto,
            z_index: crate::layout::ZIndex::Auto,
            overlay: crate::layout::Overlay::None,
            float: crate::layout::Float::None,
            clear: crate::layout::Clear::None,
            transition_property: Vec::new(),
            transition_duration: Vec::new(),
            transition_timing_function: Vec::new(),
            transition_delay: Vec::new(),
            transition_behavior: Vec::new(),
            animation_name: Vec::new(),
            animation_duration: Vec::new(),
            animation_timing_function: Vec::new(),
            animation_delay: Vec::new(),
            animation_iteration_count: Vec::new(),
            animation_direction: Vec::new(),
            animation_fill_mode: Vec::new(),
            animation_play_state: Vec::new(),
            animation_composition: Vec::new(),
            animation_timeline: Vec::new(),
            scroll_timeline_name: Vec::new(),
            scroll_timeline_axis: Vec::new(),
            view_timeline_name: Vec::new(),
            view_timeline_axis: Vec::new(),
            view_timeline_inset: Vec::new(),
            timeline_scope: Default::default(),
            animation_range_start: Vec::new(),
            animation_range_end: Vec::new(),
            counter_reset: Vec::new(),
            counter_increment: Vec::new(),
            counter_set: Vec::new(),
            color_scheme: crate::color::ColorSchemeList::normal(),
            vars: NO_VARS.with(Rc::clone),
            animated_vars: None,
        }
    }
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self::initial()
    }
}
