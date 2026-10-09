//! C15G-API — the Phase 15 API surface (gate API N5, N7, N8): the root
//! re-exports and prelude a consumer of `rdom_tui` alone names, the
//! builder argument shapes, and `client_rects()` against CSSOM View §6.1.

use super::{by_id, doc, rect, sheet, styled};
use rdom_tui::TuiAccessors;

/// The rule and declaration types `Stylesheet::position_try_rules()` and
/// `TuiStyle::anchor` hand out are nameable from the crate root, as their
/// `@keyframes` and effects siblings are.
#[test]
fn position_try_types_are_named_at_the_root() {
    let sheet = sheet("@position-try --above { bottom: anchor(top) }");
    let rules: &[rdom_tui::PositionTryRule] = sheet.position_try_rules();
    let declarations: &rdom_tui::AnchorDeclarations = &rules[0].declarations().anchor;
    assert!(declarations.position_area.is_none());
}

/// The prelude carries the Phase 14 / 15 builder arguments a typical app
/// passes: containment, multi-column, anchor positioning, transforms,
/// filters, compositing and clipping.
#[test]
fn the_prelude_names_the_phase_14_and_15_builder_arguments() {
    use rdom_tui::prelude::{
        BlendMode, BreakInside, ClipPath, ClipRect, ColumnCount, ColumnSpan, ContainerSize,
        ContainerType, FilterFunction, FilterList, Isolation, PositionAnchor, PositionArea,
        TransformList, Translate, TryFallback, TryTactic, TuiStyle,
    };
    let _style = TuiStyle::new()
        .container_type(ContainerType::new(ContainerSize::InlineSize))
        .column_count(ColumnCount::Count(2))
        .column_span(ColumnSpan::All)
        .break_inside(BreakInside::Avoid)
        .position_anchor(PositionAnchor::Name("--a".into()))
        .position_area(PositionArea::NONE)
        .position_try_fallbacks(vec![TryFallback::tactics(vec![TryTactic::FlipBlock])])
        .translate(Some(Translate::new(
            rdom_tui::prelude::Length::Cells(1),
            rdom_tui::prelude::Length::Cells(0),
        )))
        .transform(TransformList::none())
        .filter(FilterList::new(vec![FilterFunction::Grayscale(1.0)]))
        .mix_blend_mode(BlendMode::Normal)
        .isolation(Isolation::Isolate)
        .clip_path(ClipPath::None)
        .clip(ClipRect::Auto);
}

/// §3.1: `position-area: none` is a value of the type, as
/// `PositionAnchor::None` is — the builder takes it directly.
#[test]
fn position_area_none_is_a_value() {
    use rdom_tui::{PositionArea, TuiStyle};
    let none = PositionArea::NONE;
    assert!(none.is_none());
    assert_eq!(PositionArea::default(), none);
    assert!(none.keywords().is_none());
    let s = TuiStyle::new().position_area(none);
    assert!(s.anchor.position_area.is_some(), "declared");
}

/// CSSOM View §6.1 `getClientRects()` step 1: an element with no
/// associated layout box — `display: none`, under a `display: none`
/// ancestor, or `display: contents` — returns an empty list, the web's
/// "not rendered" idiom; a rendered box returns its border box.
#[test]
fn client_rects_is_empty_for_an_element_without_a_box() {
    let mut dom = doc(r#"<body><div id="shown">a</div><div id="gone">b</div>
           <div id="outer"><span id="inner">c</span></div>
           <div id="contents"><p>d</p></div></body>"#);
    styled(
        &mut dom,
        "#gone { display: none } #outer { display: none } #contents { display: contents }",
        20,
        5,
    );
    for id in ["gone", "inner", "contents"] {
        let rects = dom.node(by_id(&dom, id)).client_rects();
        assert!(rects.is_empty(), "#{id}: {rects:?}");
    }
    let shown: Vec<_> = dom
        .node(by_id(&dom, "shown"))
        .client_rects()
        .iter()
        .map(|r| (r.x, r.y, r.width, r.height))
        .collect();
    assert_eq!(shown, [rect(&dom, "shown")]);
}
