//! C4G-REEXPORTS, C5G-REEXPORTS-AND-ROOT — every type a CHANGELOG
//! `[Unreleased]` migration hint names is reachable by a consumer that
//! depends on `rdom-tui` alone and writes `use rdom_tui::*;` — the
//! root, the full public surface (the prelude is a typical app's set,
//! not a migration surface). Module paths a hint names are reached from
//! the root as `calc::…`, and rdom-style's declaration-level modules as
//! `style::parse::…`, `style::property_dispatch::…`, `style::backend::…`.
//! One test per hint group, each citing the hint's item id; nothing else
//! is imported.

use rdom_tui::*;

/// C3G-MIN-AUTO, C2-PERCENT, C2G-MAX-NONE, C3G-API: `MinSize::Auto` (and
/// its `Default`), `MaxSize::Cells`, the percentage constructors — the
/// same variant shape for `Size`, `MinSize` and `MaxSize` — and the node
/// setters' `impl Into<…>` forms.
#[test]
fn sizing_hints() {
    let computed = ComputedStyle::initial();
    assert_eq!(computed.min_width, MinSize::Auto);
    assert_eq!(MinSize::default(), MinSize::Auto);
    assert_eq!(computed.min_width.cells(Some(10)), None);
    assert_eq!(computed.max_width.cells(Some(10)), None);
    let _: Option<Value<MaxSize>> = Some(Value::Specified(MaxSize::Cells(4)));
    assert_eq!(Size::percent(50.0), Size::Percent(50.0));
    assert_eq!(MinSize::percent(50.0), MinSize::Percent(50.0));
    assert_eq!(MaxSize::percent(50.0), MaxSize::Percent(50.0));
    assert_eq!(MinSize::percent(50.0).cells(Some(10)), Some(5));
    assert_eq!(MaxSize::percent(50.0).cells(Some(10)), Some(5));
    let _ = TuiStyle::new()
        .max_width(20)
        .flex_shrink(0.0)
        .width(Size::Flex(1.0));
    let _ = FlexBasis::Auto;
    let r = AspectRatio::new(16.0, 9.0).unwrap();
    assert_eq!(r.numerator(), 16.0);
    assert!(r.value().is_some());

    let mut dom: TuiDom = TuiDom::new();
    let div = dom.create_element("div");
    dom.node_mut(div)
        .set_max_width(40u16)
        .set_max_width(MaxSize::None)
        .set_min_width(MinSize::percent(10.0));
    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .remove_property("min-width")
        .unwrap();
}

/// C1-REVERT, C1-LAYER, C1-VAR-ANY, C1-PROPERTY, C1G-VAR-COST: the
/// cascade keywords and custom values.
#[test]
fn cascade_hints() {
    let v: Value<u16> = Value::Revert;
    assert!(matches!(v, Value::Revert | Value::RevertLayer));
    let style = TuiStyle {
        ..Default::default()
    };
    assert!(style.pending.is_empty());
    let value = CustomValue::new("1px");
    assert_eq!(value.as_str(), "1px");
    let _: CustomValue = "text".into();
    let ComputedStyle {
        animated_vars: _, ..
    } = ComputedStyle::initial();
}

/// C1G-VAR-TOKENS, C2-MINMAX, C2G-CALC-SEMANTICS, C2G-CELLS-CONVERSIONS:
/// tokens and math, reached through the root's `parse` and `calc`.
#[test]
fn token_and_math_hints() {
    let tokens = style::parse::tokenize("2px").unwrap();
    assert!(matches!(
        tokens.as_slice(),
        [style::parse::Token::Dimension { .. }]
    ));
    let three = style::parse::tokenize("calc(1 + 2)").unwrap();
    let expr: calc::CalcExpr = style::parse::values::parse_calc(&three).unwrap();
    assert_eq!(expr.resolve(&calc::ResolveCtx::new(0)), 3);
    assert_eq!(calc::to_cells(2.5), 2);
}

/// C2G-CONTENT-ATTR, C2-ATTR: `content` set through the dispatch table,
/// and resolved outside a cascade.
#[test]
fn content_hints() {
    let mut style = TuiStyle::new();
    style::property_dispatch::set("content", "attr(x)", &mut style).unwrap();
    assert_eq!(style.pending.len(), 1);
    fn lookup(name: &str) -> Option<&'static str> {
        (name == "x").then_some("hi")
    }
    let vars = std::collections::HashMap::new();
    let cx = style::backend::SubstitutionContext::new().with_attrs(&lookup);
    let resolved = style.substituted_pending(&vars, &cx);
    assert_eq!(
        resolved.content,
        Some(Value::Specified(Content::Str("hi".into())))
    );
    fn _implements(_: &dyn ContentContext) {}
}

/// C3-RGB, C3-CURRENTCOLOR, C3-MIX, C3-SYSTEM: color variants and the
/// color context.
#[test]
fn color_hints() {
    let c = Color::rgba(1, 2, 3, 128);
    assert!(matches!(c, Color::Rgba(1, 2, 3, 128)));
    assert_eq!(c.opaque(), Color::Rgb(1, 2, 3));
    let scheme = ColorScheme::Dark;
    let cx = ColorContext::new(Color::Rgb(9, 9, 9)).with_scheme(scheme);
    let vars = std::collections::HashMap::new();
    assert_eq!(
        resolve_tui_color(&TuiColor::CurrentColor, &vars, Color::Reset, &cx),
        Color::Rgb(9, 9, 9)
    );
    let s = SystemColor::Canvas;
    let _ = (s.color(), s.definite(scheme));
    let f: Option<&ColorFunction> = None;
    let _ = f.map(|f| (f.compute(&cx), f.css_text()));
}

/// C4-BORDER-SHORTHAND, C4-BORDER-WIDTH, C4-RADIUS, C4-SHADOW,
/// C4-SPACING, C4G-IMPORTANT-BITSET: the Phase 4 value types.
#[test]
fn border_hints() {
    let mut style = TuiStyle::new()
        .border(Border::single())
        .border_radius(BorderRadius::cells(1.0));
    style.border_color = Sides::all(Some(Value::Specified(TuiColor::from(Color::Rgb(1, 1, 1)))));
    let top = style.border_color.top.clone();
    assert!(top.is_some());
    let styles = style
        .border_style
        .map(|s| s.and_then(|v| v.as_specified().copied()).unwrap());
    assert_eq!(Border::from_sides(styles), Border::single());
    let mask = ImportantMask::BORDER_TOP_COLOR | ImportantMask::BORDER_TOP_STYLE;
    assert!(mask.contains(ImportantMask::BORDER_TOP_COLOR) && !mask.is_empty());
    let _ = mask.without(ImportantMask::TRANSITIONS) & ImportantMask::all();
    let _: Corners<Option<BorderRadius>> = Corners::all(None);
    let _: (BorderWidth, BorderWeight, BorderStyle, CornerStyle) = (
        BorderWidth::default(),
        BorderWeight::Light,
        BorderStyle::Solid,
        CornerStyle::Square,
    );
    let _: Option<(BoxShadow, PaintLength, BorderSpacing)> = None;
    let _: Option<(
        VisualBox,
        RepeatStyle,
        BackgroundRepeat,
        BackgroundAttachment,
    )> = None;
    let computed = ComputedStyle::initial();
    let _ = (
        &computed.border_color.left,
        &computed.border_style,
        &computed.box_shadow,
        &computed.border_spacing,
        &computed.background_clip,
    );
}

/// C4G-REEXPORTS: `set_border_radius` beside `set_border`, read back by
/// `border_radius`.
#[test]
fn node_border_radius_accessor() {
    let mut dom: TuiDom = TuiDom::new();
    let div = dom.create_element("div");
    dom.node_mut(div)
        .set_border(Border::single())
        .set_border_radius(BorderRadius::cells(1.0));
    assert_eq!(
        dom.node(div).border_radius(),
        Some(Corners::all(BorderRadius::cells(1.0)))
    );
}

/// C3-ALPHA, C4-BORDER-WIDTH: the paint buffer and a hand-built border
/// contribution.
#[test]
fn render_hints() {
    let area = Rect::new(0, 0, 2, 1);
    let _ = Buffer::empty(area);
    let _ = Buffer::filled(area, Cell::default());
    let _ = Buffer::with_cells(area, vec![Cell::default(); 2]);
    let _ = render::buffer::BorderContribution {
        style: BorderStyle::Solid,
        fg: Color::Reset,
        weight: BorderWeight::Light,
        priority: 0,
        corner_style: CornerStyle::Square,
        side: render::buffer::BorderSide::Top,
    };
}

/// C5-BOX-SIZING, C5-INTRINSIC, C5-MARGIN-TRIM, C5-CONTAIN-SIZE,
/// C5-WRITING, C5G-API-EDGES: the Phase 5 style fields, their builders
/// and values, the node setter, the border-box reset and the renamed
/// `flex-direction` bit.
#[test]
fn box_model_hints() {
    let flex_direction = ImportantMask::FLEX_DIRECTION;
    assert!(flex_direction.intersects(ImportantMask::all()));
    let s = TuiStyle::new()
        .box_sizing(BoxSizing::BorderBox)
        .width(Size::Intrinsic(IntrinsicSize::MinContent))
        .min_width(MinSize::Intrinsic(IntrinsicSize::MaxContent))
        .margin_trim(MarginTrim::NONE)
        .contain_intrinsic_width(ContainIntrinsicSize::default())
        .contain_intrinsic_height(ContainIntrinsicSize::default())
        .text_direction(TextDirection::Rtl)
        .writing_mode(WritingMode::HorizontalTb);
    assert!(s.important.is_empty());
    let ComputedStyle {
        box_sizing: _,
        margin_trim: _,
        contain_intrinsic_width: _,
        contain_intrinsic_height: _,
        text_direction: _,
        writing_mode: _,
        ..
    } = ComputedStyle::initial();
    let tokens = style::parse::tokenize("block").unwrap();
    assert!(style::parse::values::parse_margin_trim(&tokens).is_some());

    let sheet = Stylesheet::new()
        .rule(
            "*, ::before, ::after",
            TuiStyle::new().box_sizing(BoxSizing::BorderBox),
        )
        .unwrap();
    let mut dom: TuiDom = TuiDom::new();
    let div = dom.create_element("div");
    dom.node_mut(div).set_box_sizing(BoxSizing::BorderBox);
    dom.cascade(&sheet);
}

/// C5G-RTL-SCROLL, C5G-INT-CLAMP-SITE, C5G-ATOM-BOX: the signed
/// `scrollLeft`, the wide integer token, and the line boxes' rows.
#[test]
fn scroll_token_and_line_hints() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    dom.node_mut(div).set_scroll(-2, 0);
    let x: i32 = dom.node(div).ext().unwrap().scroll_x;
    let _as_usize = x.max(0) as usize;

    let tokens = style::parse::tokenize("99999999999").unwrap();
    let [style::parse::Token::Number(n)] = tokens.as_slice() else {
        panic!("one integer token");
    };
    assert_eq!(*n, 99_999_999_999_i64);
    assert!(i32::try_from(*n).is_err());

    // The line rows themselves are read through the constructors now
    // (C6G-LINEBOX-API, below).
    let line = render::LineBox::new(Vec::new(), 0, 2);
    assert_eq!(line.text_row(), 2);
    let _ = div;
}

/// C6G-LINEBOX-API (superseding 0.5.0's `..Default::default()` hint):
/// `LineBox` and `InlineFragment` are `#[non_exhaustive]` — no struct
/// literal outside rdom-tui — and built by constructor, or for a line
/// from `LineBox::default()` with its public fields set.
#[test]
fn line_box_construction_hints() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    dom.append_child(root, p).unwrap();
    let t = dom.create_text_node("héllo");
    dom.append_child(p, t).unwrap();

    let text = render::InlineFragment::text(p, t, 0, 0, "héllo");
    assert_eq!((text.width, text.height, text.atomic), (5, 1, false));
    let atom = render::InlineFragment::atom(p, 6, 3, 2);
    assert_eq!(
        (atom.x, atom.width, atom.height, atom.atomic),
        (6, 3, 2, true)
    );

    let line = render::LineBox::new(vec![text, atom], 9, 0);
    assert_eq!((line.top, line.height, line.baseline), (0, 1, 0));
    let mut tall = render::LineBox::default();
    assert_eq!((tall.height, tall.width), (1, 0));
    tall.top = 1;
    tall.height = 3;
    tall.baseline = 1;
    tall.fragments = line.fragments.clone();
    assert_eq!((tall.text_row(), tall.bottom()), (2, 4));
}

/// C6-MARGIN-SIDES: `margin` / `padding` are per-side longhands on
/// `TuiStyle`, with a bit per side; `Margin` / `Padding` convert to and
/// from `Sides`.
#[test]
fn spacing_side_hints() {
    let mut style = TuiStyle::new().margin(1).padding(Padding::all(2));
    assert_eq!(
        style.margin.top,
        Some(Value::Specified(MarginValue::Cells(1)))
    );
    style.padding = Sides::from(Padding::all(3)).map(|v| Some(Value::Specified(v)));
    let back = Padding::from(style.padding.clone().map(|v| match v {
        Some(Value::Specified(p)) => p,
        _ => PaddingValue::default(),
    }));
    assert_eq!(back, Padding::all(3));
    let mask = ImportantMask::MARGIN_TOP
        | ImportantMask::MARGIN_RIGHT
        | ImportantMask::MARGIN_BOTTOM
        | ImportantMask::MARGIN_LEFT
        | ImportantMask::PADDING_TOP;
    assert!(!style.important.intersects(mask));
    let _ = Margin::from(Sides::all(MarginValue::Auto));
}

/// C6-MARGIN-SIDES, C6G-SIDE-SETTERS: the two shapes a 0.5 consumer
/// (rdom-virtualtable) breaks on — a whole `Margin` assigned to
/// `TuiStyle::margin`, and `padding.is_none()` — and their per-side
/// forms, through the per-side setters.
#[test]
fn spacing_assignment_hints() {
    // `style.margin = Some(Value::Specified(margin))` → the shorthand
    // builder, or the four longhands from `Sides`.
    let margin = Margin::new(
        MarginValue::Cells(0),
        MarginValue::Auto,
        MarginValue::Cells(0),
        MarginValue::Cells(1),
    );
    let built = TuiStyle::new().margin(margin.clone());
    let mut assigned = TuiStyle::new();
    assigned.margin = Sides::from(margin).map(|v| Some(Value::Specified(v)));
    assert_eq!(built, assigned);
    // One side: the field, or its setter.
    let mut style = TuiStyle::new();
    style.margin.left = Some(Value::Specified(MarginValue::Auto));
    assert_eq!(style, TuiStyle::new().margin_left(MarginValue::Auto));
    // `style.padding.is_none()` → no side declared, or one side.
    let style = TuiStyle::new();
    assert!(style.padding.each().iter().all(|side| side.is_none()));
    assert!(style.padding.top.is_none());
    let style = TuiStyle::new().padding_top(1);
    assert!(!style.padding.each().iter().all(|side| side.is_none()));
}

/// C6-DISPLAY-KEYWORDS: the new `Display` / `Flow` variants, the
/// `list_item` field and the `display` grammar.
#[test]
fn display_keyword_hints() {
    for d in [Display::Contents, Display::Block] {
        let _box_less = matches!(d, Display::Contents);
    }
    assert!(Flow::FlowRoot.is_block_flow() && Flow::Block.is_block_flow());
    let s = TuiStyle::new();
    assert!(s.list_item.is_none());
    let ComputedStyle { list_item, .. } = ComputedStyle::initial();
    assert!(!list_item);
    let tokens = style::parse::tokenize("inline list-item").unwrap();
    let (display, flow, item) = style::parse::values::parse_display(&tokens).unwrap();
    assert_eq!(
        style::parse::values::serialize_display(display, flow, item),
        "inline list-item"
    );
    assert!(ImportantMask::LIST_ITEM.intersects(ImportantMask::all()));
}

/// C6-VISIBILITY: the `visibility` field, its value type, builder and
/// bit, and the animatable property.
#[test]
fn visibility_hints() {
    let s = TuiStyle::new().visibility(Visibility::Hidden);
    assert_eq!(s.visibility, Some(Value::Specified(Visibility::Hidden)));
    let ComputedStyle { visibility, .. } = ComputedStyle::initial();
    assert!(visibility.is_visible());
    assert!(ImportantMask::VISIBILITY.intersects(ImportantMask::all()));
    let _ = style::transition::AnimatableProperty::Visibility;
}

/// C6-ORDER: the `order` field, builder, bit and parser.
#[test]
fn order_hints() {
    let s = TuiStyle::new().order(-1);
    assert_eq!(s.order, Some(Value::Specified(-1)));
    let ComputedStyle { order, .. } = ComputedStyle::initial();
    assert_eq!(order, 0);
    let tokens = style::parse::tokenize("3").unwrap();
    assert_eq!(style::parse::values::parse_order(&tokens), Some(3));
    assert!(ImportantMask::ORDER.intersects(ImportantMask::all()));
}

/// C6-DIRECTION-REVERSE: `flex_reverse`, the reversed builder, the
/// two-bit `flex-direction` mask, and the signed `scroll_y`.
#[test]
fn reverse_and_signed_scroll_top_hints() {
    let s = TuiStyle::new().direction_reverse(Direction::Column);
    assert_eq!(s.flex_reverse, Some(Value::Specified(true)));
    let ComputedStyle { flex_reverse, .. } = ComputedStyle::initial();
    assert!(!flex_reverse);
    assert_eq!(
        style::property_dispatch::property_mask("flex-direction"),
        Some(ImportantMask::FLEX_DIRECTION | ImportantMask::FLEX_REVERSE)
    );
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    dom.node_mut(div).set_scroll(0, -2);
    let y: i32 = dom.node(div).ext().unwrap().scroll_y;
    assert_eq!(usize::try_from(y).unwrap_or(0), 0);
}

/// C6-FLEX-LONGHANDS: `flex` sets the three longhands (not `width`),
/// `flex_grow`, `FlexBasis::Intrinsic`, the parser.
#[test]
fn flex_longhand_hints() {
    let mut s = TuiStyle::new().flex_grow(1.0);
    style::property_dispatch::set("flex", "2 1 0", &mut s).unwrap();
    assert_eq!(s.flex_grow, Some(Value::Specified(2.0)));
    assert!(s.width.is_none());
    let ComputedStyle { flex_grow, .. } = ComputedStyle::initial();
    assert_eq!(flex_grow, 0.0);
    let _ = FlexBasis::Intrinsic(IntrinsicSize::MinContent);
    let tokens = style::parse::tokenize("content").unwrap();
    assert_eq!(
        style::parse::values::parse_flex_basis(&tokens),
        Some(FlexBasis::Content)
    );
    assert!(ImportantMask::FLEX_GROW.intersects(ImportantMask::all()));
}

/// C6-GAP: the two gap fields and bits, `GapValue::Normal`, the
/// shorthand parser.
#[test]
fn gap_hints() {
    let s = TuiStyle::new().gap(1).column_gap(GapValue::Normal);
    assert_eq!(s.row_gap, Some(Value::Specified(GapValue::Cells(1))));
    let ComputedStyle {
        row_gap,
        column_gap,
        ..
    } = ComputedStyle::initial();
    assert_eq!((row_gap, column_gap), (GapValue::Normal, GapValue::Normal));
    let tokens = style::parse::tokenize("1 2").unwrap();
    assert!(style::parse::values::parse_gap_shorthand(&tokens).is_some());
    assert!((ImportantMask::ROW_GAP | ImportantMask::COLUMN_GAP).intersects(ImportantMask::all()));
}

/// C6-FLEX-DIRECTION-INITIAL: `flex-direction`'s initial value is `row`
/// — `Direction`'s default, `ComputedStyle::initial()`'s — and the
/// builders that ask for a column.
#[test]
fn flex_direction_initial_hints() {
    assert_eq!(Direction::default(), Direction::Row);
    assert_eq!(ComputedStyle::initial().direction, Direction::Row);
    let s = TuiStyle::new().flex_column();
    assert_eq!(s.direction, Some(Value::Specified(Direction::Column)));
    let _ = TuiStyle::new().flex().direction(Direction::Column);
}

/// C6-WRAP: `FlexWrap`, the `flex_wrap` fields and bit, the builders,
/// and the `flex-flow` parser.
#[test]
fn flex_wrap_hints() {
    let s = TuiStyle::new().flex_wrap(FlexWrap::Wrap);
    assert_eq!(s.flex_wrap, Some(Value::Specified(FlexWrap::Wrap)));
    let _ = TuiStyle::new().flex_wrap_important(FlexWrap::WrapReverse);
    let ComputedStyle { flex_wrap, .. } = ComputedStyle::initial();
    assert_eq!(flex_wrap, FlexWrap::NoWrap);
    let tokens = style::parse::tokenize("column wrap").unwrap();
    assert!(style::parse::values::parse_flex_flow(&tokens).is_some());
    assert!(ImportantMask::FLEX_WRAP.intersects(ImportantMask::all()));
}

/// C6-JUSTIFY: `Align`'s Box Alignment keywords, `Alignment` (with
/// `OverflowAlign`), the `justify_content` fields and bit, the parser.
#[test]
fn alignment_hints() {
    assert_eq!(Align::default(), Align::Normal);
    let safe = Alignment::safe(Align::Center);
    assert_eq!(safe.overflow, OverflowAlign::Safe);
    let s = TuiStyle::new().justify_content(Align::SpaceBetween);
    assert_eq!(
        s.justify_content,
        Some(Value::Specified(Alignment::new(Align::SpaceBetween)))
    );
    let ComputedStyle {
        justify_content, ..
    } = ComputedStyle::initial();
    assert_eq!(justify_content, Alignment::NORMAL);
    let tokens = style::parse::tokenize("safe end").unwrap();
    assert!(style::parse::values::parse_justify_content(&tokens).is_some());
    assert!(ImportantMask::JUSTIFY_CONTENT.intersects(ImportantMask::all()));
}

/// C6-ALIGN: the `align_items` / `align_self` fields, bits, builders and
/// parsers.
#[test]
fn align_items_hints() {
    let s = TuiStyle::new()
        .align_items(Align::Center)
        .align_self(Alignment::AUTO);
    assert_eq!(s.align_self, Some(Value::Specified(Alignment::AUTO)));
    let ComputedStyle {
        align_items,
        align_self,
        ..
    } = ComputedStyle::initial();
    assert_eq!(
        (align_items, align_self),
        (Alignment::NORMAL, Alignment::AUTO)
    );
    let tokens = style::parse::tokenize("last baseline").unwrap();
    assert!(style::parse::values::parse_align_items(&tokens).is_some());
    assert!(style::parse::values::parse_align_self(&tokens).is_some());
    assert!(
        (ImportantMask::ALIGN_ITEMS | ImportantMask::ALIGN_SELF).intersects(ImportantMask::all())
    );
}

/// C6-ALIGN-CONTENT: the `align_content` fields, bit, builders, parser.
#[test]
fn align_content_hints() {
    let s = TuiStyle::new().align_content(Align::SpaceEvenly);
    assert_eq!(
        s.align_content,
        Some(Value::Specified(Alignment::new(Align::SpaceEvenly)))
    );
    let ComputedStyle { align_content, .. } = ComputedStyle::initial();
    assert_eq!(align_content, Alignment::NORMAL);
    let tokens = style::parse::tokenize("safe center").unwrap();
    assert!(style::parse::values::parse_align_content(&tokens).is_some());
    assert!(ImportantMask::ALIGN_CONTENT.intersects(ImportantMask::all()));
}

/// C6-PLACE: the `justify_items` / `justify_self` fields, bits and
/// builders, `Alignment::LEGACY`, and the `place-*` parsers.
#[test]
fn place_hints() {
    let s = TuiStyle::new()
        .justify_items(Alignment::LEGACY)
        .justify_self(Align::Center);
    assert_eq!(s.justify_items, Some(Value::Specified(Alignment::LEGACY)));
    let ComputedStyle {
        justify_items,
        justify_self,
        ..
    } = ComputedStyle::initial();
    assert_eq!(
        (justify_items, justify_self),
        (Alignment::LEGACY, Alignment::AUTO)
    );
    let tokens = style::parse::tokenize("center end").unwrap();
    assert!(style::parse::values::parse_place_content(&tokens).is_some());
    assert!(style::parse::values::parse_place_items(&tokens).is_some());
    assert!(style::parse::values::parse_place_self(&tokens).is_some());
    assert!(
        (ImportantMask::JUSTIFY_ITEMS | ImportantMask::JUSTIFY_SELF)
            .intersects(ImportantMask::all())
    );
}

/// C6G-ALIGN-API: the alignment builders take `impl Into<Alignment>`
/// (a bare keyword, no `.into()`), `Align` is `#[non_exhaustive]` (a
/// match needs a `_` arm), the grammar check, and `flex-direction` as
/// one value.
#[test]
fn alignment_api_hints() {
    let s = TuiStyle::new()
        .justify_content(Align::Center)
        .align_self(Alignment::safe(Align::End));
    assert_eq!(
        s.justify_content,
        Some(Value::Specified(Alignment::new(Align::Center)))
    );
    let edge = |a: Align| match a {
        Align::Start | Align::FlexStart => 0,
        Align::End | Align::FlexEnd => 2,
        _ => 1,
    };
    assert_eq!(edge(Align::Center), 1);
    assert!(!Alignment::new(Align::SpaceBetween).is_valid_for(AlignProperty::AlignSelf));
    let s = TuiStyle::new().flex_direction(FlexDirection::RowReverse);
    assert_eq!(s.flex_reverse, Some(Value::Specified(true)));
    let _ = TuiStyle::new().direction_reverse_important(Direction::Column);
    assert_eq!(
        ComputedStyle::initial().flex_direction(),
        FlexDirection::Row
    );
}
