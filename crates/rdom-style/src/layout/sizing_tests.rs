//! `layout::sizing` — sizes to cells, and the intrinsic keywords' conversions.

use super::sizing::*;
use crate::calc::{CalcExpr, CalcOp};

fn calc(lhs: CalcExpr, rhs: CalcExpr) -> Box<CalcExpr> {
    Box::new(CalcExpr::binary(CalcOp::Sub, lhs, rhs))
}

/// `C2G-CELLS-CONVERSIONS`: the one conversion of a size or an
/// inset to cells — percentages through `Size::percent_of`, `calc()`
/// against the same basis, `auto` (and a flex weight) left to the
/// caller. A size is an extent: clamped to `0..=u16::MAX` (CSS
/// Values 4 §10.12, a negative width is 0); an inset is signed.
#[test]
fn sizes_and_lengths_to_cells() {
    assert_eq!(Size::Fixed(7).cells(Some(80)), Some(7));
    assert_eq!(Size::Percent(12.5).cells(Some(80)), Some(10));
    let minus = calc(CalcExpr::Percent(50.0), CalcExpr::Number(50.0));
    assert_eq!(Size::Calc(minus.clone()).cells(Some(80)), Some(0));
    assert_eq!(Size::Percent(200.0).cells(Some(40_000)), Some(u16::MAX));
    assert_eq!(Size::Auto.cells(Some(80)), None);
    assert_eq!(Size::Flex(1.0).cells(Some(80)), None);
    assert_eq!(Length::Cells(-3).cells(80), Some(-3));
    assert_eq!(Length::Calc(minus.into()).cells(80), Some(-10));
    assert_eq!(Length::Auto.cells(80), None);
}

/// C3G-API: `Size`, `MinSize` and `MaxSize` share one convention —
/// a `u16` converts to cells, `percent(p: f32)` builds a percentage,
/// `cells(basis: Option<u16>) -> Option<u16>` resolves against the
/// containing block's extent (`None` when indefinite: a percentage
/// is then `auto` for a size, `0` for a minimum and `none` for a
/// maximum, CSS 2.1 §10.5 / §10.7), and each carries its keyword
/// (`Size::Auto`, `MinSize::Auto`, `MaxSize::None`).
#[test]
fn sizing_types_share_one_convention() {
    let (s, min, max): (Size, MinSize, MaxSize) = (20u16.into(), 5u16.into(), 40u16.into());
    assert_eq!(s.cells(None), Some(20));
    assert_eq!(min.cells(None), Some(5));
    assert_eq!(max.cells(None), Some(40));
    assert_eq!(Size::percent(50.0_f32).cells(Some(80)), Some(40));
    assert_eq!(MinSize::percent(50.0_f32).cells(Some(80)), Some(40));
    assert_eq!(MaxSize::percent(50.0_f32).cells(Some(80)), Some(40));
    assert_eq!(Size::percent(50.0).cells(None), None);
    assert_eq!(MinSize::percent(50.0).cells(None), Some(0));
    assert_eq!(MaxSize::percent(50.0).cells(None), None);
    assert_eq!(MaxSize::None.cells(Some(80)), None);
    assert_eq!(MaxSize::default(), MaxSize::None);
}

/// C5G-API-EDGES: an intrinsic keyword (CSS Sizing 3 §3.1) converts
/// into each of the three size types, as a `u16` does, and
/// `fit_content` builds `fit-content(<length-percentage>)`.
#[test]
fn intrinsic_keywords_convert_into_every_size() {
    let k = IntrinsicSize::MinContent;
    assert_eq!(Size::from(k.clone()), Size::Intrinsic(k.clone()));
    assert_eq!(MinSize::from(k.clone()), MinSize::Intrinsic(k.clone()));
    assert_eq!(MaxSize::from(k.clone()), MaxSize::Intrinsic(k));
    let limit = IntrinsicSize::fit_content(10);
    assert_eq!(
        limit,
        IntrinsicSize::FitContentLimit(Box::new(CalcExpr::Length(10)))
    );
    assert_eq!(limit.limit_cells(None), Some(10));
    let half = IntrinsicSize::fit_content_percent(50.0);
    assert_eq!(half.limit_cells(Some(80)), Some(40));
    assert_eq!(half.limit_cells(None), None);
}
