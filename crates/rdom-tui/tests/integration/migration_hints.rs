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

/// C3G-MIN-AUTO, C2-PERCENT, C2G-MAX-NONE, C3G-API, C2-NUMBER, C2-RATIO,
/// C2G-LAYOUT-SAFETY: `MinSize::Auto` (and its `Default`),
/// `MaxSize::Cells`, the percentage constructors — the same variant shape
/// for `Size`, `MinSize` and `MaxSize` — the node setters' `impl Into<…>`
/// forms, `f32` flex factors, and `AspectRatio` behind accessors.
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
    let _: Option<Value<Option<AspectRatio>>> = TuiStyle::new().aspect_ratio;

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

/// C8G-PSEUDO-ATOMS: `GeneratedFragment` is `#[non_exhaustive]` — no
/// struct literal outside rdom-tui — and a run of generated text is built
/// with `GeneratedFragment::text`; an atomic pseudo-element's box is one
/// fragment read through `is_atom` / `atom_rows`.
#[test]
fn generated_fragment_construction_hints() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    dom.append_child(root, p).unwrap();
    let run = render::GeneratedFragment::text(p, ext::PseudoSlot::Before, -1, "• ");
    assert_eq!((run.x, run.width, run.text.as_str()), (-1, 2, "• "));
    assert!(!run.is_atom());
    assert_eq!(run.atom_rows(), None);
}

/// C6G-LINEBOX-API (superseding 0.5.0's `..Default::default()` hint),
/// C8-RTL-LINE-OVERFLOW:
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

    // C8-RTL-LINE-OVERFLOW: a column is an `i32`, negative left of the
    // content box.
    let left = render::InlineFragment::atom(p, -2, 1, 1);
    assert_eq!(left.x, -2i32);

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

/// C13-TFC: the table `display` values — `Display::TablePart`,
/// `Flow::Table` — and the table group (`table-layout`, `caption-side`).
#[test]
fn table_display_hints() {
    let d = Display::TablePart(TablePart::Cell);
    let _internal = match d {
        Display::TablePart(part) => part.is_block_container(),
        Display::Block
        | Display::Inline
        | Display::InlineBlock
        | Display::None
        | Display::Contents => false,
    };
    assert!(!Flow::Table.is_block_flow());
    let s = TuiStyle::new()
        .display(Display::TablePart(TablePart::Row))
        .table_layout(TableLayout::Fixed)
        .caption_side(CaptionSide::Bottom);
    assert!(s.table.table_layout.is_some());
    let ComputedStyle { table, .. } = ComputedStyle::initial();
    assert_eq!(table, TableStyle::default());
    let _: TableDeclarations = s.table;
    assert!(ImportantMask::TABLE_LAYOUT.intersects(ImportantMask::all()));
}

/// C13-TFC: no column-sync pass — a `<table>` sizes its columns as it
/// lays out; a fixed column is a `width` on its `<col>` (or its cells).
/// `TuiExt::table_used_width` is `table_tracks()` (C13G-TABLE-TRACKS).
#[test]
fn table_layout_hints() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let table = dom.create_element("table");
    let col = dom.create_element("col");
    let tr = dom.create_element("tr");
    let td = dom.create_element("td");
    dom.append_child(root, table).unwrap();
    dom.append_child(table, col).unwrap();
    dom.append_child(table, tr).unwrap();
    dom.append_child(tr, td).unwrap();
    dom.node_mut(col)
        .set_inline_style(TuiStyle::new().width(Size::Fixed(8)));
    dom.cascade(&Stylesheet::new());
    dom.layout_dom(render::Rect::new(0, 0, 20, 2));
    assert_eq!(dom.node(td).layout_rect().unwrap().width, 8);
    let tracks = rdom_tui::TuiAccessors::table_tracks(&dom.node(table)).unwrap();
    assert_eq!(tracks.columns()[0].len(), 8);
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
    let _ = style::transition::TransitionProperty::named("visibility");
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

/// C6-FLEX-LONGHANDS, C2G-FLEX-SHORTHAND: `flex` sets the three
/// longhands (not `width`), `flex_grow`, `FlexBasis::Intrinsic`, the
/// parsers (`parse_flex_shorthand` gives a `FlexShorthand`).
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
    let tokens = style::parse::tokenize("2").unwrap();
    let flex: style::parse::values::FlexShorthand =
        style::parse::values::parse_flex_shorthand(&tokens).unwrap();
    assert_eq!(flex.grow, 2.0);
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

/// C1G-TYPED-ERRORS, C2G-SUBSTITUTION-ERRORS: registration and
/// substitution report typed errors.
#[test]
fn typed_error_hints() {
    let syntax: std::result::Result<PropertySyntax, PropertySyntaxError> =
        PropertySyntax::parse("<nope>");
    assert!(syntax.is_err());
    let registration: std::result::Result<PropertyRegistration, RegisterPropertyError> =
        PropertyRegistration::new("no-dashes", "<color>", true, Some("red"));
    assert!(registration.is_err());
    fn _substitution(_: style::backend::SubstitutionError) {}
}

/// C4G-SEALED, C2-VIEWPORT, C3-SCHEME: the sealed extension traits are
/// called, not implemented — the document-level settings included.
#[test]
fn sealed_trait_hints() {
    let mut dom: TuiDom = TuiDom::new();
    dom.set_viewport(calc::Viewport::new(10, 5));
    dom.set_color_scheme(ColorScheme::Light);
    assert_eq!(dom.color_scheme(), ColorScheme::Light);
    let root = dom.root();
    let _ = dom.node(root).computed();
}

/// C6G-FRONTEND-API: `set_from_source` takes the text as an `Option` and
/// the declaration's `!important`; `SpannedTokens` is a struct.
#[test]
fn front_end_hints() {
    let mut s = TuiStyle::new();
    let value = style::parse::tokenize("red").unwrap();
    style::property_dispatch::set_from_source("color", &value, Some("red"), true, &mut s).unwrap();
    assert!(style::property_dispatch::is_important("color", &s));
    let style::parse::token::SpannedTokens {
        tokens,
        positions,
        spans,
    } = style::parse::token::tokenize_spans("a b", 1, 1).unwrap();
    assert_eq!((tokens.len(), positions.len(), spans.len()), (2, 2, 2));
}

/// C6G-PSEUDO-FLEX-ITEMS: `AnonymousIfc` is `#[non_exhaustive]`, built by
/// `AnonymousIfc::new`; a generated flex item's border box rides beside.
#[test]
fn anonymous_box_hints() {
    let layout = render::InlineLayout {
        lines: Vec::new(),
        content_width: 0,
    };
    let rect = LayoutRect::new(0, 0, 4, 1);
    let anon = ext::AnonymousIfc::new(rect, layout, (0, 1), None);
    assert_eq!(anon.border_box(), rect);
}

/// C7-GRID-CORE: the `grid_template_columns` / `grid_template_rows`
/// fields, their value types, builders, bits, parser and node setter;
/// `Flow::Grid` and the `display: grid` builders.
#[test]
fn grid_template_hints() {
    for f in [Flow::Grid, Flow::Flex] {
        let _items = f.is_flex_or_grid();
    }
    assert_eq!(
        (
            TuiStyle::new().grid().flow,
            TuiStyle::new().inline_grid().display
        ),
        (
            Some(Value::Specified(Flow::Grid)),
            Some(Value::Specified(Display::Inline))
        )
    );
    let s = TuiStyle::new()
        .grid_template_columns(TrackList::new([TrackSize::cells(2)]).repeat(
            RepeatCount::AutoFill,
            [TrackSize::minmax(4, TrackBreadth::Fr(1.0))],
        ))
        .grid_template_rows(vec![TrackSize::cells(1), TrackSize::AUTO]);
    assert!(s.grid_template_columns.is_some());
    let ComputedStyle {
        grid_template_columns,
        grid_template_rows,
        ..
    } = ComputedStyle::initial();
    assert_eq!(grid_template_columns, GridTemplate::None);
    assert!(grid_template_rows.tracks().is_none());
    let tokens = style::parse::tokenize("[a] 1fr repeat(2, 3)").unwrap();
    let t = style::parse::values::parse_grid_template(&tokens).unwrap();
    assert_eq!(
        style::parse::values::serialize_grid_template(&t),
        "[a] 1fr repeat(2, 3)"
    );
    assert!(
        (ImportantMask::GRID_TEMPLATE_COLUMNS | ImportantMask::GRID_TEMPLATE_ROWS)
            .intersects(ImportantMask::all())
    );
    let mut dom = TuiDom::new();
    let div = dom.create_element("div");
    dom.node_mut(div)
        .set_grid_template_columns(vec![TrackSize::cells(2)]);
    assert_eq!(
        dom.node(div)
            .inline_style()
            .and_then(|s| s.grid_template_columns.clone()),
        Some(Value::Specified(GridTemplate::from(vec![
            TrackSize::cells(2)
        ])))
    );
}

/// C7-GRID-AUTO: the `grid_auto_columns` / `grid_auto_rows` fields
/// (`Vec<TrackSize>`, initial `[auto]`), builders, bits, parser and node
/// setters.
#[test]
fn grid_auto_hints() {
    let s = TuiStyle::new()
        .grid_auto_rows([TrackSize::cells(1), TrackSize::fr(1.0)])
        .grid_auto_columns_important([TrackSize::AUTO]);
    assert!(s.grid_auto_rows.is_some());
    let ComputedStyle {
        grid_auto_columns,
        grid_auto_rows,
        ..
    } = ComputedStyle::initial();
    assert_eq!(
        (grid_auto_columns.len(), grid_auto_rows[0].clone()),
        (1, TrackSize::AUTO)
    );
    let tokens = style::parse::tokenize("1 minmax(2, 1fr)").unwrap();
    let sizes = style::parse::values::parse_track_sizes(&tokens).unwrap();
    assert_eq!(
        style::parse::values::serialize_track_sizes(&sizes),
        "1 minmax(2, 1fr)"
    );
    assert!(
        (ImportantMask::GRID_AUTO_COLUMNS | ImportantMask::GRID_AUTO_ROWS)
            .intersects(ImportantMask::all())
    );
    let mut dom = TuiDom::new();
    let div = dom.create_element("div");
    dom.node_mut(div)
        .set_grid_auto_rows([TrackSize::cells(2)])
        .set_grid_auto_columns([TrackSize::AUTO]);
    assert!(
        dom.node(div)
            .inline_style()
            .is_some_and(|s| s.grid_auto_rows.is_some() && s.grid_auto_columns.is_some())
    );
}

/// C7-GRID-PLACE: the placement fields (`GridLine`), `grid_auto_flow`
/// (`GridAutoFlow`), their builders, bits, parsers and node setters.
#[test]
fn grid_placement_hints() {
    let s = TuiStyle::new()
        .grid_row(GridLine::line(1), GridLine::span(2))
        .grid_column(GridLine::named("a"), GridLine::Auto)
        .grid_area(
            GridLine::nth_named(2, "x"),
            GridLine::line(-1),
            GridLine::span_named(1, "y"),
            GridLine::Auto,
        )
        .grid_auto_flow(GridAutoFlow::COLUMN.dense());
    assert!(s.grid_row_start.is_some() && s.grid_column_end.is_some());
    let ComputedStyle {
        grid_row_start,
        grid_auto_flow,
        ..
    } = ComputedStyle::initial();
    assert_eq!(
        (grid_row_start, grid_auto_flow),
        (GridLine::Auto, GridAutoFlow::ROW)
    );
    let tokens = style::parse::tokenize("a / span 2 b").unwrap();
    let (start, end) = style::parse::values::parse_grid_line_pair(&tokens).unwrap();
    assert_eq!(
        style::parse::values::serialize_grid_line_pair(&start, &end),
        "a / span 2 b"
    );
    assert!(
        (ImportantMask::GRID_ROW_START
            | ImportantMask::GRID_ROW_END
            | ImportantMask::GRID_COLUMN_START
            | ImportantMask::GRID_COLUMN_END
            | ImportantMask::GRID_AUTO_FLOW)
            .intersects(ImportantMask::all())
    );
    let mut dom = TuiDom::new();
    let div = dom.create_element("div");
    dom.node_mut(div)
        .set_grid_row(GridLine::line(2), GridLine::Auto)
        .set_grid_column(GridLine::span(2), GridLine::Auto)
        .set_grid_auto_flow(GridAutoFlow::ROW.dense());
    assert!(
        dom.node(div)
            .inline_style()
            .is_some_and(|s| s.grid_row_start.is_some() && s.grid_auto_flow.is_some())
    );
}

/// C7-GRID-AREAS: `grid_template_areas` (`GridTemplateAreas`, valid by
/// construction), its builder, bit, parser and node setter.
#[test]
fn grid_template_areas_hints() {
    let areas = GridTemplateAreas::new(["a a", "b ."]).expect("rectangles");
    assert_eq!(
        areas.areas()[0],
        NamedArea {
            name: "a".into(),
            rows: 0..1,
            columns: 0..2
        }
    );
    let s = TuiStyle::new().grid_template_areas(areas.clone());
    assert!(s.grid_template_areas.is_some());
    assert_eq!(
        ComputedStyle::initial().grid_template_areas,
        GridTemplateAreas::NONE
    );
    let tokens = style::parse::tokenize("\"a a\" \"b .\"").unwrap();
    assert_eq!(
        style::parse::values::parse_grid_template_areas(&tokens),
        Some(areas.clone())
    );
    assert!(ImportantMask::GRID_TEMPLATE_AREAS.intersects(ImportantMask::all()));
    let mut dom = TuiDom::new();
    let div = dom.create_element("div");
    dom.node_mut(div).set_grid_template_areas(areas);
    assert!(
        dom.node(div)
            .inline_style()
            .is_some_and(|s| s.grid_template_areas.is_some())
    );
}

/// C8-Z-INDEX: `ZIndex::Value` holds an `i32`; an `i16` level converts
/// with `.into()`.
#[test]
fn z_index_hints() {
    let level: i16 = 3;
    let z = layout::ZIndex::Value(level.into());
    let n: i32 = match z {
        layout::ZIndex::Value(n) => n,
        layout::ZIndex::Auto => 0,
    };
    assert_eq!(n, 3);
    let s = TuiStyle::new().z_index(layout::ZIndex::Value(40_000));
    assert!(s.z_index.is_some());
}

/// C8-OVERFLOW-CLIP: `Overflow::Clip` is a new arm; `OverflowClipMargin`
/// and the scroll-container predicates are new.
#[test]
fn overflow_hints() {
    let clips = |o: layout::Overflow| match o {
        layout::Overflow::Visible => false,
        layout::Overflow::Hidden
        | layout::Overflow::Clip
        | layout::Overflow::Scroll
        | layout::Overflow::Auto => true,
    };
    assert!(clips(layout::Overflow::Clip));
    let s = TuiStyle::new()
        .overflow(layout::Overflow::Clip)
        .overflow_clip_margin(layout::OverflowClipMargin::new(
            layout::VisualBox::ContentBox,
            1,
        ));
    assert!(s.overflow_clip_margin.is_some());
    let c = ComputedStyle::initial();
    assert!(!c.is_scroll_container() && !c.clips_overflow());
}

/// C8-SCROLLBAR: `ScrollbarGutter::StableBothEdges` is a new arm (or ask
/// `is_stable()`); `ScrollbarWidth` and `ScrollbarColor` are new.
#[test]
fn scrollbar_hints() {
    let reserves = |g: layout::ScrollbarGutter| match g {
        layout::ScrollbarGutter::Auto => false,
        layout::ScrollbarGutter::Stable | layout::ScrollbarGutter::StableBothEdges => true,
    };
    assert!(reserves(layout::ScrollbarGutter::StableBothEdges));
    assert!(layout::ScrollbarGutter::StableBothEdges.is_stable());
    let s = TuiStyle::new()
        .scrollbar_width(layout::ScrollbarWidth::Thin)
        .scrollbar_color(layout::ScrollbarColor::Auto);
    assert!(s.scrollbar_width.is_some() && s.scrollbar_color.is_some());
}

/// C8G-API-TYPES: `scroll-padding-*` / `scroll-margin-*` are `Sides`, as
/// `margin` / `padding` are (`style.scroll_padding.top`, the per-side
/// builders unchanged); the Phase 8 value types are at the root, beside
/// the other style values; `FocusOptions` (C8G-FOCUS-SCROLL) and the
/// `TuiTimers` trait — `request_animation_frame` on an event context — are
/// at the root and in the prelude.
#[test]
fn scroll_sides_and_root_hints() {
    let s = TuiStyle::new()
        .scroll_padding_top(ScrollPadding::Auto)
        .scroll_margin_left(2);
    assert!(s.scroll_padding.top.is_some() && s.scroll_padding.bottom.is_none());
    assert_eq!(s.scroll_margin.left, Some(Value::Specified(2)));
    let c = ComputedStyle::initial();
    assert_eq!(c.scroll_padding.top, ScrollPadding::Auto);
    assert_eq!(c.scroll_margin, Sides::new(0, 0, 0, 0));
    // Each Phase 8 value type, named from the root.
    fn named<T>() {}
    named::<OverscrollBehavior>();
    named::<ScrollSnapType>();
    named::<ScrollSnapAxis>();
    named::<ScrollSnapStrictness>();
    named::<SnapAlign>();
    named::<ScrollSnapAlign>();
    named::<ScrollSnapStop>();
    named::<Float>();
    named::<FloatSide>();
    named::<Clear>();
    named::<TextOverflowSide>();
    named::<BlockEllipsis>();
    named::<Continue>();
    named::<BoxOrient>();
    named::<ScrollbarGutter>();
    named::<ScrollbarWidth>();
    named::<ScrollbarColor>();
    named::<ZIndex>();
    let _ = FocusOptions::new().prevent_scroll(true);
    fn _timers<T: TuiTimers>() {}
    fn _prelude() {
        use rdom_tui::prelude::{FocusOptions, TuiTimers};
        let _ = FocusOptions::new();
        fn _t<T: TuiTimers>() {}
    }
}

/// C9-WHITE-SPACE: `white-space` is the shorthand of
/// `white-space-collapse` and `text-wrap-mode` (CSS Text 4 §3) — the
/// `TuiStyle::text` / `ComputedStyle::text` groups, the builder that sets
/// both, the two bits, and the keyword the longhands spell.
#[test]
fn white_space_hints() {
    let s = TuiStyle::new().white_space(WhiteSpace::PreLine);
    assert_eq!(
        s.text.white_space_collapse,
        Some(Value::Specified(WhiteSpaceCollapse::PreserveBreaks))
    );
    assert_eq!(
        s.text.text_wrap_mode,
        Some(Value::Specified(TextWrapMode::Wrap))
    );
    let _: &TextDeclarations = &s.text;
    let ComputedStyle { text, .. } = ComputedStyle::initial();
    let _: &TextStyle = &text;
    assert_eq!(text.white_space(), Some(WhiteSpace::Normal));
    assert_eq!(
        WhiteSpace::BreakSpaces.longhands(),
        (WhiteSpaceCollapse::BreakSpaces, TextWrapMode::Wrap)
    );
    let both = ImportantMask::WHITE_SPACE_COLLAPSE | ImportantMask::TEXT_WRAP_MODE;
    assert!(both.intersects(ImportantMask::all()));
    assert_eq!(ImportantMask::WHITE_SPACE, both);
}

/// C9G-TYPES (changes to APIs added after 0.5): `FontStretch`'s keywords
/// are typed, `FontVariant` takes a `_` arm, and `text_align` takes a
/// `TextAlign` or `justify-all`.
#[test]
fn font_type_hints() {
    let k = FontStretch::Keyword(FontStretchKeyword::Condensed);
    assert_eq!(k, FontStretch::Keyword(FontStretchKeyword::ALL[2].0));
    assert_eq!(
        FontStretchKeyword::from_keyword("condensed"),
        Some(FontStretchKeyword::Condensed)
    );
    let name = match FontVariant::SmallCaps {
        FontVariant::Normal => "normal",
        FontVariant::SmallCaps => "small-caps",
        _ => "normal",
    };
    assert_eq!(name, "small-caps");
    let _ = TuiStyle::new()
        .text_align(TextAlign::Center)
        .text_align(TextAlignKeyword::JustifyAll);
}

/// C9-DECORATION: `text-decoration` is a group of four longhands — the
/// builder's one-line form unchanged, the line read off the group — and
/// the decorations on an element's text are `applied_decorations`;
/// `render::Style` / `SgrState` carry the underline color.
#[test]
fn text_decoration_hints() {
    let s = TuiStyle::new().text_decoration(TextDecoration::Underline);
    assert_eq!(
        s.text_decoration.line,
        Some(Value::Specified(TextDecorationLine::UNDERLINE))
    );
    let _: &TextDecorationDeclarations = &s.text_decoration;
    assert!(ImportantMask::TEXT_DECORATION.contains(ImportantMask::TEXT_DECORATION_LINE));
    let computed = ComputedStyle::initial();
    assert_eq!(computed.applied_decorations, AppliedDecorations::NONE);
    let _: &TextDecorations = &computed.text_decoration;
    let tokens = style::parse::tokenize("underline").unwrap();
    let shorthand: Option<style::parse::values::TextDecorationShorthand> =
        style::parse::values::parse_text_decoration(&tokens);
    assert_eq!(
        shorthand.map(|s| s.line),
        Some(TextDecorationLine::UNDERLINE)
    );
    let _ = render::Style::new().underline_color(Color::Rgb(255, 0, 0));
    let _ = render::SgrState {
        underline_color: Color::Reset,
        ..render::SgrState::RESET
    };
}

/// C9-FONT: `font-weight` / `font-style` are the font group's longhands;
/// the two-state builders and the 0.5 mask names still work.
#[test]
fn font_hints() {
    let s = TuiStyle::new().bold(true).italic(true);
    assert_eq!(s.font.weight, Some(Value::Specified(FontWeight::Bold)));
    assert_eq!(s.font.style, Some(Value::Specified(FontStyle::Italic)));
    let _: &FontDeclarations = &s.font;
    assert_eq!(ImportantMask::BOLD, ImportantMask::FONT_WEIGHT);
    assert_eq!(ImportantMask::ITALIC, ImportantMask::FONT_STYLE);
    let computed = ComputedStyle::initial();
    let _: &Font = &computed.font;
    assert_eq!(computed.font.weight(), 400.0);
}

/// C10G-INHERIT-COST, C9-CARRY-INDENT: every `calc()` payload is an
/// `Arc<CalcExpr>` (was a `Box`), built with the value's `calc()`
/// constructor — `Size`, `MinSize`, `MaxSize`, `GapValue`, `PaddingValue`,
/// `MarginValue`, `Spacing`, `LineHeight`, `VerticalAlign`, `PaintLength`,
/// `FlexBasis`, `TrackBreadth` and `Length` (`Length::Calc(Box::new(e))` →
/// `Length::calc(e)`) — and a match on `Calc(e)` reads `e` as before. The
/// list strings and `block-ellipsis` hold `Arc<str>`.
#[test]
fn calc_payload_hints() {
    use calc::{CalcExpr, CalcOp};
    let e = || CalcExpr::binary(CalcOp::Sub, CalcExpr::Percent(50.0), CalcExpr::Length(2));
    let size = Size::calc(e());
    assert!(matches!(&size, Size::Calc(expr) if expr.contains_percent()));
    let _ = (MinSize::calc(e()), MaxSize::calc(e()), GapValue::calc(e()));
    let _ = (
        PaddingValue::calc(e()),
        MarginValue::calc(e()),
        Spacing::calc(e()),
    );
    let _ = (
        LineHeight::calc(e()),
        VerticalAlign::calc(e()),
        PaintLength::calc(e()),
    );
    let _ = (
        FlexBasis::calc(e()),
        TrackBreadth::calc(e()),
        Length::calc(e()),
    );
    assert!(matches!(Length::calc(e()), Length::Calc(expr) if expr.contains_percent()));
    let _ = ListStyleType::String("→ ".into());
    let _ = ListStyleImage::Image("url(a.png)".into());
    let _ = BlockEllipsis::Str("…".into());
}

/// C10G-TUIEXT-SIDE: `TuiExt`'s rarely set pseudo-element fields moved
/// into one boxed `PseudoStyles`; read them through the accessors named
/// as the fields were (`ext.computed_backdrop` → `ext.computed_backdrop()`,
/// an `Option<&Rc<ComputedStyle>>`), the transition overrides through
/// `presentation_for(ext::StyleSlot::Before)`, the record through
/// `pseudo_styles()`.
#[test]
fn tui_ext_pseudo_hints() {
    let ext = TuiExt::default();
    assert!(ext.computed_backdrop().is_none());
    assert!(ext.computed_scrollbar().is_none());
    assert!(ext.computed_scrollbar_thumb_vertical().is_none());
    assert!(ext.computed_scrollbar_thumb_horizontal().is_none());
    assert!(ext.computed_before_prev().is_none() && ext.computed_after_prev().is_none());
    assert!(ext.computed_marker().is_none() && ext.computed_first_line().is_none());
    assert!(ext.computed_first_letter().is_none() && ext.computed_details_content().is_none());
    assert!(ext.presentation_for(ext::StyleSlot::Before).is_none());
    let _: Option<&ext::PseudoStyles> = ext.pseudo_styles();
}

/// C10-COUNTERS: `CounterStyle` is a counter style name, no longer a
/// closed `Copy` enum — `CounterStyle::UpperRoman` → `CounterStyle::named(
/// "upper-roman")`, `Decimal` → `decimal()`, `as_str()` → `name()` (an
/// `Option`: `None` for `symbols()`), a match on the old variants compares
/// `name()`, and a copy is a `.clone()`. It and the list types are at the
/// root, so a list style is built with one import.
#[test]
fn counter_style_hints() {
    let upper = CounterStyle::named("upper-roman");
    assert_eq!(upper.name(), Some("upper-roman"));
    assert_eq!(upper.format(4), "IV");
    assert_eq!(CounterStyle::decimal(), CounterStyle::default());
    assert_eq!(CounterStyle::parse("lower-alpha").unwrap().format(2), "b");
    let roman = matches!(upper.name(), Some("upper-roman" | "lower-roman"));
    assert!(roman);
    let copy = upper.clone();
    let stars = CounterStyle::symbols(style::counters::System::Cyclic, &["*"]).unwrap();
    assert_eq!((stars.name(), stars.format(3).as_str()), (None, "*"));
    let _: CounterStyleName = match copy {
        CounterStyle::Name(n) => n,
        _ => unreachable!("a named style"),
    };
    let _ = TuiStyle::new()
        .list_style_type(ListStyleType::Style(CounterStyle::named("upper-roman")))
        .list_style_position(ListStylePosition::Inside)
        .list_style_image(ListStyleImage::None)
        .marker_side(MarkerSide::MatchParent);
}

/// C10-COUNTERS: `CounterOp` is `#[non_exhaustive]` — `CounterOp { name,
/// value }` → `CounterOp::new(name, value)`, `CounterOp::reversed(name,
/// Some(n))` for `counter-reset: reversed(name) n` — and
/// `parse_counter_ops` takes a third argument, whether `reversed()` is
/// allowed (`counter-reset` only).
#[test]
fn counter_op_hints() {
    let op = CounterOp::new("item", 0);
    assert_eq!((op.name.as_str(), op.value), ("item", 0));
    let rev = CounterOp::reversed("item", Some(5));
    assert!(!rev.is_auto_reversed());
    assert!(CounterOp::reversed("item", None).is_auto_reversed());
    let _ = TuiStyle::new()
        .counter_reset(vec![op.clone()])
        .counter_increment(vec![CounterOp::new("item", 1)]);
    let tokens = style::parse::tokenize("reversed(item) 5").unwrap();
    let ops = style::parse::values::parse_counter_ops(&tokens, 0, true).unwrap();
    assert_eq!(ops, [rev]);
    assert!(style::parse::values::parse_counter_ops(&tokens, 1, false).is_none());
}

/// C10-CONTENT, C10-LIST-ITEM: `Content` is `#[non_exhaustive]` (a match
/// adds a `_` arm) and `content: normal` is `Content::Normal`, not
/// `Content::None` — a match treating `None` as "no content" adds
/// `Normal`.
#[test]
fn generated_content_hints() {
    fn generates(c: &Content) -> bool {
        match c {
            Content::None | Content::Normal => false,
            Content::Str(s) => !s.is_empty(),
            _ => true,
        }
    }
    let mut style = TuiStyle::new();
    style::property_dispatch::set("content", "normal", &mut style).unwrap();
    assert_eq!(style.content, Some(Value::Specified(Content::Normal)));
    assert!(!generates(&Content::Normal));
    assert!(generates(&Content::Quote(QuoteKind::Open)));
}

/// C10-HIGHLIGHT: `PseudoElementTarget` is no longer `Copy` (its
/// `Highlight` variant names a `::highlight()`): `.clone()` where a copy
/// was taken; a match adds `Highlight(_)` under its `_` arm.
#[test]
fn pseudo_element_target_hints() {
    let target = PseudoElementTarget::Highlight("search".into());
    let copy = target.clone();
    let name = match target {
        PseudoElementTarget::Before => "before",
        PseudoElementTarget::Highlight(name) => {
            assert_eq!(&*name, "search");
            "highlight"
        }
        _ => "other",
    };
    assert_eq!(name, "highlight");
    assert_eq!(copy, PseudoElementTarget::Highlight("search".into()));
}

/// C10G-HIGHLIGHT-API (changes to APIs added after 0.5): a `Highlight`'s
/// members are methods — `priority()` / `set_priority`, `kind()` /
/// `set_kind` (the web's `type`), the `with_kind` builder (was
/// `with_type`), `len()` / `is_empty()` (was `size()`) — and
/// `highlights_mut()` is a `HighlightsMut` guard that reads and changes as
/// the registry and reports the change when it goes.
#[test]
fn highlight_hints() {
    let mut dom = TuiDom::new();
    let t = dom.create_text_node("hello");
    dom.append_child(dom.root(), t).unwrap();
    let r = dom
        .range_between(Position::new(t, 0), Position::new(t, 2))
        .unwrap();
    let h = Highlight::new([r])
        .with_priority(1)
        .with_kind(HighlightType::SpellingError);
    assert_eq!(
        (h.priority(), h.kind(), h.len()),
        (1, HighlightType::SpellingError, 1)
    );
    dom.highlights_mut().set("search", h);
    let mut registry: HighlightsMut<'_, TuiExt> = dom.highlights_mut();
    let _: &mut HighlightRegistry = &mut registry;
    registry.get_mut("search").unwrap().set_priority(2);
    drop(registry);
    assert_eq!(dom.highlights().get("search").unwrap().priority(), 2);
    assert_eq!(dom.descendants(dom.root()).collect::<Vec<_>>(), [t]);
}

/// C10G-API-SMALL (C10-PSEUDO-UNIFY's row from 0.5; the counter errors
/// after 0.5): `positioned_pseudos()` yields a `PositionedPseudo` per
/// absolutely positioned pseudo-element — its slot, border box and lines —
/// a fragment reads where it is drawn, and the `@counter-style` checks
/// return typed errors.
#[test]
fn generated_box_read_hints() {
    let ext = TuiExt::default();
    let none: Vec<ext::PositionedPseudo<'_>> = ext.positioned_pseudos().collect();
    assert!(none.is_empty());
    let dom: TuiDom = TuiDom::new();
    let g = render::GeneratedFragment::text(dom.root(), ext::PseudoSlot::Before, 3, "x");
    assert_eq!(
        (g.offset(), g.drawn_at(), g.is_outside_marker()),
        ((0, 0), (3, 0), false)
    );
    let rule = style::counters::CounterStyleRule::default();
    let e: style::counters::CounterStyleRuleError =
        style::counters::check_rule("x", &rule).unwrap_err();
    assert_eq!(e.to_string(), "the symbols do not suit the system");
    let mut rule = rule;
    let d: style::counters::DescriptorError =
        style::counters::apply_descriptor(&mut rule, "colour", &[]).unwrap_err();
    assert_eq!(d, style::counters::DescriptorError::Unknown);
}

/// C11G-POPOVER-BOUND: `runtime::builtins::dialog::show` / `show_modal`
/// return a `Result` — HTML's `InvalidStateError` cases are
/// `DomError::InvalidState` (`show` on a modal dialog; `show_modal` on an
/// open, disconnected or popover-showing one). Add `?`, or `.ok()` where
/// the dialog is known to be closed and connected.
#[test]
fn dialog_show_hints() {
    use runtime::builtins::dialog;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let d = dom.create_element("dialog");
    dom.append_child(root, d).unwrap();
    dialog::show_modal(&mut dom, d).unwrap();
    dialog::show_modal(&mut dom, d).unwrap(); // already modal: nothing to do
    assert!(matches!(
        dialog::show(&mut dom, d),
        Err(DomError::InvalidState(_))
    ));
    dialog::close(&mut dom, d, "");
    let loose = dom.create_element("dialog");
    assert!(dialog::show_modal(&mut dom, loose).is_err());
}

/// C11-ATTR-FLAGS: `SimpleSelector::Attribute` gains `case: AttrCase` (the
/// Selectors 4 §6.3 flag). Build it with `case: AttrCase::Default` — the
/// 0.5 behaviour, HTML §4.16.2's case-insensitive list deciding — and
/// match it with `{ name, op, value, .. }`.
#[test]
fn selector_hints() {
    use core_api::selectors::{
        AttrOp, ComplexSelector, CompoundSelector, SelectorList, SimpleSelector,
    };
    let attr = SimpleSelector::Attribute {
        name: "type".into(),
        op: Some(AttrOp::Exact),
        value: Some("checkbox".into()),
        case: AttrCase::Default,
    };
    let SimpleSelector::Attribute { ref name, .. } = attr else {
        unreachable!("built as an attribute selector")
    };
    assert_eq!(name, "type");
    let list = SelectorList(vec![ComplexSelector {
        subject: CompoundSelector {
            simples: vec![attr.clone()],
        },
        ancestors: Vec::new(),
    }]);
    let mut dom = TuiDom::new();
    let root = dom.root();
    let input = dom.create_element("input");
    dom.set_attribute(input, "type", "CheckBox").unwrap();
    dom.append_child(root, input).unwrap();
    assert!(dom.matches_list(input, &list), "`type` is case-insensitive");
}

/// C11G-API: a `ControlStateHook` returns `Option<bool>` — `None` for a
/// question the backend does not keep, which the substrate's default then
/// answers. A 0.5-era `bool` hook wraps its answers in `Some` and turns
/// its `_ => false` arm into `_ => None`.
#[test]
fn control_state_hook_hints() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let check = dom.create_element("input");
    dom.set_attribute(check, "type", "checkbox").unwrap();
    dom.set_attribute(check, "checked", "").unwrap();
    dom.append_child(root, check).unwrap();
    let hook: core_api::ControlStateHook<TuiExt> = |_, _, state| match state {
        ControlState::UserValidity => Some(false),
        _ => None,
    };
    dom.set_control_state_hook(Some(hook));
    assert!(dom.control_state(check, ControlState::DefaultChecked));
    assert!(!dom.control_state(check, ControlState::UserValidity));
}

/// C11G-API: Phase 11's vocabularies are at the root — `TopLayerKind`,
/// `Directionality`, `ControlState`, `AttrCase` and `PopoverState` — so
/// `use rdom_tui::*;` names what `top_layer_kind`, `directionality`,
/// `control_state`, an attribute selector and `popover_state` return.
#[test]
fn phase11_reexport_hints() {
    use runtime::builtins::{dialog, popover};
    let mut dom = TuiDom::new();
    let root = dom.root();
    let d = dom.create_element("dialog");
    dom.append_child(root, d).unwrap();
    dialog::show_modal(&mut dom, d).unwrap();
    let kind: Option<TopLayerKind> = dom.top_layer_kind(d);
    assert_eq!(kind, Some(TopLayerKind::ModalDialog));
    let dir: Directionality = dom.directionality(d);
    assert_eq!(dir, Directionality::Ltr);
    let p = dom.create_element("div");
    dom.set_attribute(p, "popover", "hint").unwrap();
    dom.append_child(root, p).unwrap();
    let state: Option<PopoverState> = popover::popover_state(&dom, p);
    assert_eq!(state, Some(PopoverState::Hint));
    let _ = (ControlState::UserValidity, AttrCase::AsciiInsensitive);
}

/// C12-ANIMATABLE: `transition-property` names any property — a
/// `TransitionProperty::Named` holds its canonical name, a `PropertyName`
/// built only from a known one (C12G-API-HYGIENE; `AnimatableProperty`
/// is gone), `Discrete(name)` is `Other(name)` for a custom or unknown
/// name; the engine's `AnimatedProp` / `AnimatedValue` are a
/// `style::animation::Longhand` and two whole styles
/// (`ActiveAnimation::from` / `to`); `PresentationStyle` holds no values
/// — a running transition's values are in `TuiExt::computed`, the
/// cascade's own style (the base value) in `TuiExt::base_computed_for` /
/// `node.base_computed()` (C12G-COMPUTED-DOCS).
#[test]
fn transition_property_hints() {
    use style::transition::TransitionProperty;
    assert_eq!(
        TransitionProperty::named("color"),
        TransitionProperty::Named(PropertyName::new("color").unwrap())
    );
    if let TransitionProperty::Named(n) = TransitionProperty::named("color") {
        assert_eq!(n.as_str(), "color");
    }
    assert_eq!(
        TransitionProperty::named("--x"),
        TransitionProperty::Other("--x".into())
    );
    let l = style::animation::Longhand::from_name("padding-top").unwrap();
    assert_eq!(l.css_name(), "padding-top");
    assert_eq!(
        style::animation::animation_type("display"),
        Some(style::animation::AnimationType::Discrete)
    );
    let _: Option<&Longhand> = None;
    let ext = TuiExt::default();
    assert!(ext.base_computed_for(ext::StyleSlot::Host).is_none());
    let mut dom: TuiDom = TuiDom::new();
    let el = dom.create_element("div");
    assert!(dom.node(el).base_computed().is_none());
}

/// C12-ANIMATABLE: `Size` gains `CalcSize` (`calc-size()`, CSS Values 5
/// §10) — a match adds an arm, sizing it by `basis_size()` or `resolve()`
/// — and `interpolate-size` is a property (`InterpolateSize`).
#[test]
fn calc_size_hints() {
    let c = CalcSize::new(CalcSizeBasis::Auto, 0.5, calc::CalcExpr::Number(2.0));
    assert_eq!(c.basis_size(), Size::Auto);
    assert_eq!(c.resolve(10, 0), 7);
    let size = Size::CalcSize(std::sync::Arc::new(c));
    let described = match size {
        Size::CalcSize(c) => c.resolve(4, 0),
        _ => 0,
    };
    assert_eq!(described, 4);
    let s = TuiStyle::new().interpolate_size(InterpolateSize::AllowKeywords);
    assert_eq!(
        s.interpolate_size,
        Some(Value::Specified(InterpolateSize::AllowKeywords))
    );
    assert_eq!(
        ComputedStyle::initial().interpolate_size,
        InterpolateSize::NumericOnly
    );
    // `min-*`, `max-*` and `flex-basis` take `calc-size()` too, a
    // `flex-basis` one over `content` (`CalcSizeBasis::Content`).
    let c = CalcSize::new(CalcSizeBasis::Content, 1.0, calc::CalcExpr::Number(3.0));
    assert_eq!(c.basis_flex_basis(), FlexBasis::Content);
    let min = MinSize::CalcSize(std::sync::Arc::new(c.clone()));
    let floor = match &min {
        MinSize::CalcSize(c) => c.resolve(5, 0),
        _ => 0,
    };
    assert_eq!(floor, 8);
    let max = MaxSize::CalcSize(std::sync::Arc::new(c.clone()));
    assert!(matches!(max, MaxSize::CalcSize(_)));
    assert!(matches!(
        FlexBasis::CalcSize(std::sync::Arc::new(c)),
        FlexBasis::CalcSize(_)
    ));
}

/// C12-TIMING: `TimingFunction` is not `Copy`, has `linear()`'s
/// `LinearStops` and is `#[non_exhaustive]` (C12G-API-HYGIENE); delays are signed (`transition_delay: Vec<i32>`);
/// `parse_duration_list` reads durations, `parse_time_list` delays.
#[test]
fn timing_hints() {
    use style::transition::{LinearStop, TimingFunction};
    let f = TimingFunction::LinearStops(
        vec![LinearStop::new(0.0, 0.0), LinearStop::new(1.0, 1.0)].into(),
    );
    let g = f.clone();
    assert_eq!(g.ease(0.5), 0.5);
    // C12G-API-HYGIENE: `#[non_exhaustive]` — a match keeps a `_` arm.
    let name = match g {
        TimingFunction::Linear => "linear",
        TimingFunction::LinearStops(_) => "linear()",
        _ => "other",
    };
    assert_eq!(name, "linear()");
    let s = TuiStyle::new().transition_delay(vec![-500]);
    assert_eq!(s.transition_delay, Some(Value::Specified(vec![-500])));
    let tokens = style::parse::tokenize("1s, 2s").unwrap();
    assert_eq!(
        style::parse::values::parse_duration_list(&tokens),
        Some(vec![1000, 2000])
    );
}

/// C12G-API-HYGIENE: the `effective_*` helpers are gone — read the
/// computed style, which holds the running values; `ActiveAnimation` is
/// the engine's own (inspect with `App::get_animations`); rdom-style's
/// tokenizer cursor is `parse::SourceCursor`; `linear()` stops are built
/// output first, as CSS writes them; `Appearance` is `#[non_exhaustive]`;
/// the UI setters take `impl Into` of their value.
#[test]
fn animation_api_hygiene_hints() {
    let mut dom: TuiDom = TuiDom::new();
    let el = dom.create_element("div");
    let fg = dom.node(el).computed().map(|c| c.fg);
    assert_eq!(fg, None, "not cascaded yet");
    let mut cursor = style::parse::SourceCursor::new("a");
    assert_eq!(cursor.bump(), Some('a'));
    let stop = LinearStop::new(0.25, 0.75);
    assert_eq!((stop.output, stop.input), (0.25, 0.75));
    let chrome = match layout::Appearance::None {
        layout::Appearance::None => "none",
        layout::Appearance::Auto => "auto",
        _ => "auto (an unknown keyword)",
    };
    assert_eq!(chrome, "none");
    let s = TuiStyle::new().cursor(layout::CursorKeyword::Pointer);
    assert!(s.ui.cursor.is_some());
    let s = TuiStyle::new().timeline_scope(TimelineScope::All);
    assert!(s.timeline_scope.is_some());
}

/// C12G-APP-CONFIG: every construction-time `App` option is a consuming
/// `with_*` builder — `tick_rate` is `with_tick_rate`, `on_tick` is
/// `with_tick_handler`, `set_animation_frame_rate` is
/// `with_animation_frame_rate`, `set_import_loader` is
/// `with_import_loader`; `set_color_scheme` stays for run-time changes.
#[test]
fn app_config_hints() {
    let terminal = Terminal::new(TestBackend::new(10, 2)).unwrap();
    let mut app = App::with_backend(TuiDom::new(), Stylesheet::new(), terminal)
        .unwrap()
        .with_tick_rate(std::time::Duration::from_millis(20))
        .with_tick_handler(|_| ControlFlow::Continue)
        .with_animation_frame_rate(30)
        .with_import_loader(|_: &str| Ok(String::new()));
    app.set_color_scheme(ColorScheme::Light);
    assert_eq!(app.color_scheme(), ColorScheme::Light);
}

/// C13G-COLUMN-MATCH / C13G-MISC — two items new since 0.5 reshaped:
/// `CellSpan`'s spans are read through `columns()` / `rows()` (its fields
/// are private, so every value is clamped to HTML's ranges), and the
/// HTML attributes are read off an element by `cell_span_of` /
/// `column_span_of`; `UnitContext` is `#[non_exhaustive]` — build it with
/// `UnitContext::new` and set its public fields.
#[test]
fn table_span_and_unit_context_hints() {
    use rdom_tui::core_api::table::{CellSpan, assign_slots, cell_span_of, column_span_of};
    let span = CellSpan::new(5000, 2);
    assert_eq!((span.columns(), span.rows()), (1000, 2));
    let slots = assign_slots(&[vec![vec![span]]]);
    assert_eq!(slots.columns, 1000);
    let mut dom = TuiDom::new();
    let td = dom.create_element("td");
    dom.set_attribute(td, "colspan", "3").unwrap();
    assert_eq!(cell_span_of(&dom, td).columns(), 3);
    let col = dom.create_element("col");
    dom.set_attribute(col, "span", "2").unwrap();
    assert_eq!(column_span_of(&dom, col), 2);
    let mut ctx = rdom_style::calc::UnitContext::new(Viewport::new(80, 24));
    ctx.lh = 2.0;
    assert_eq!(ctx.lh, 2.0);
}

/// C14G-READ-COUNTERS: the thread-wide `calc::viewport_reads()` /
/// `container_reads()` counts are gone; a unit resolver returns what it
/// read (`calc::UnitReads`) beside the value.
#[test]
fn unit_reads_hints() {
    let cx = calc::UnitContext::new(Viewport::new(80, 24)).with_container(Some(40.0), None);
    let vw = calc::CalcExpr::Dimension {
        value: 10.0,
        unit: calc::CalcUnit::parse("vw").unwrap(),
    };
    let (absolute, reads) = vw.absolutize_in(&cx);
    assert_eq!(absolute, calc::CalcExpr::Number(8.0));
    assert!(reads.viewport && !reads.container);
    let mut style = ComputedStyle::initial();
    style.width = Size::calc(calc::CalcExpr::Dimension {
        value: 50.0,
        unit: calc::CalcUnit::parse("cqw").unwrap(),
    });
    assert_eq!(style.resolve_context_units(&cx), calc::UnitReads::CONTAINER);
    assert_eq!(style.width, Size::Fixed(20));
}

/// C14G-API-NAMING: `WillChange::has` (was `names`), the web's
/// `ContentVisibilityAutoStateChange` detail with its `as_<variant>`
/// accessor, and `ContainerType` built by its constructors (it is
/// `#[non_exhaustive]`: Anchor Positioning 2 adds `anchored`).
#[test]
fn containment_naming_hints() {
    let will_change = WillChange::new(["transform".into()]);
    assert!(will_change.has("transform"));
    assert!(!will_change.has("opacity"));
    let detail = EventDetail::ContentVisibilityAutoStateChange { skipped: true };
    assert_eq!(detail.as_content_visibility_auto_state_change(), Some(true));
    let ty = ContainerType::new(ContainerSize::InlineSize).with_scroll_state(true);
    assert_eq!(
        (ty.size, ty.scroll_state),
        (ContainerSize::InlineSize, true)
    );
    assert_eq!(ty.css(), "inline-size scroll-state");
    let _ = TuiStyle::new().container_type(ContainerType::new(ContainerSize::Size));
}

/// C15-COLUMNS, C15-ANCHOR: `TuiStyle` / `ComputedStyle` gain the
/// `multicol`, `fragmentation` and `anchor` groups (a pattern adds them,
/// or `..`), and `CalcExpr` gains `Anchor` — an exhaustive walker adds the
/// arm, reading the function's fallback (or resolving it,
/// `substitute_anchors`).
#[test]
fn multicol_and_anchor_hints() {
    let c = ComputedStyle::initial();
    let ComputedStyle {
        multicol,
        fragmentation,
        anchor,
        ..
    } = c;
    assert!(!multicol.is_multicol());
    assert_eq!(fragmentation.orphans, 2);
    assert!(!anchor.is_anchored());
    let _ = TuiStyle::new()
        .column_count(ColumnCount::Count(2))
        .position_anchor(PositionAnchor::Name("--a".into()));
    fn leaves(e: &calc::CalcExpr) -> usize {
        match e {
            calc::CalcExpr::Number(_)
            | calc::CalcExpr::Length(_)
            | calc::CalcExpr::Percent(_)
            | calc::CalcExpr::Dimension { .. }
            | calc::CalcExpr::NoBound => 1,
            calc::CalcExpr::Binary { lhs, rhs, .. } => leaves(lhs) + leaves(rhs),
            calc::CalcExpr::Function { args, .. } => args.iter().map(leaves).sum(),
            calc::CalcExpr::Anchor(f) => f.fallback().map_or(1, leaves),
        }
    }
    let e = calc::CalcExpr::Anchor(Box::new(calc::AnchorFunction::Edge {
        name: None,
        side: calc::AnchorSide::Bottom,
        fallback: Some(calc::CalcExpr::Length(2)),
    }));
    assert_eq!(leaves(&e), 1);
    assert_eq!(
        e.substitute_anchors(&mut |_| Some(5)),
        Some(calc::CalcExpr::Length(5))
    );
}

/// C15G-STRETCH: `IntrinsicSize` (new since 0.5) gained `Stretch` (CSS
/// Sizing 4 §3.1 `stretch`, `-webkit-fill-available`): an exhaustive
/// `match` adds its arm; it serializes as `stretch`.
#[test]
fn stretch_hints() {
    let describe = |k: &IntrinsicSize| match k {
        IntrinsicSize::MinContent => "min",
        IntrinsicSize::MaxContent => "max",
        IntrinsicSize::FitContent | IntrinsicSize::FitContentLimit(_) => "fit",
        IntrinsicSize::Stretch => "stretch",
    };
    assert_eq!(describe(&IntrinsicSize::Stretch), "stretch");
    let s = TuiStyle::new().width(Size::Intrinsic(IntrinsicSize::Stretch));
    assert_eq!(
        s.width,
        Some(Value::Specified(Size::Intrinsic(IntrinsicSize::Stretch)))
    );
}
