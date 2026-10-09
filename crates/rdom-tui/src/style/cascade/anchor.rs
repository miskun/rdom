//! The anchor positioning values at computed-value time (CSS Anchor
//! Positioning 1): an anchor function in a box that is not absolutely
//! positioned is invalid at computed-value time (§5.1, §5.2) — its
//! fallback stands in, or the property takes its initial value — and a
//! `position-try-fallbacks` entry naming a `@position-try` rule (§4.1)
//! carries that rule's declarations, found in the run's sheets.

use std::sync::Arc;

use super::sheets::Sheets;
use crate::layout::{Length, MarginValue, MaxSize, MinSize, Position, Size};
use crate::style::ComputedStyle;

/// Finish `working`'s anchor positioning values (module doc).
pub(super) fn finalize_anchor(working: &mut ComputedStyle, sheets: &Sheets<'_>) {
    if !matches!(working.position, Position::Absolute | Position::Fixed) {
        strip_anchor_functions(working);
    }
    if working
        .anchor
        .position_try_fallbacks
        .iter()
        .any(|f| f.name.is_some())
    {
        let fallbacks = std::mem::take(&mut working.anchor.position_try_fallbacks);
        working.anchor.position_try_fallbacks = fallbacks
            .into_iter()
            .map(|f| {
                let declarations = f
                    .name
                    .as_deref()
                    .and_then(|n| sheets.position_try_rule(n))
                    .map(|r| Arc::clone(r.declarations()));
                f.with_declarations(declarations)
            })
            .collect();
    }
}

/// The expression without its anchor functions (each its fallback);
/// `None` when one has none.
fn without_anchors(e: &rdom_style::calc::CalcExpr) -> Option<rdom_style::calc::CalcExpr> {
    e.substitute_anchors(&mut |_| None)
}

/// Replace every anchor function of a box that is not absolutely
/// positioned: by its fallback, or the whole value by the property's
/// initial value.
fn strip_anchor_functions(c: &mut ComputedStyle) {
    for inset in [&mut c.top, &mut c.right, &mut c.bottom, &mut c.left] {
        if let Length::Calc(e) = inset
            && e.contains_anchor()
        {
            *inset = without_anchors(e).map_or(Length::Auto, Length::calc);
        }
    }
    for size in [&mut c.width, &mut c.height] {
        if let Size::Calc(e) = size
            && e.contains_anchor()
        {
            *size = without_anchors(e).map_or(Size::Auto, Size::calc);
        }
    }
    for min in [&mut c.min_width, &mut c.min_height] {
        if let MinSize::Calc(e) = min
            && e.contains_anchor()
        {
            *min = without_anchors(e).map_or(MinSize::Auto, MinSize::calc);
        }
    }
    for max in [&mut c.max_width, &mut c.max_height] {
        if let MaxSize::Calc(e) = max
            && e.contains_anchor()
        {
            *max = without_anchors(e).map_or(MaxSize::None, MaxSize::calc);
        }
    }
    let m = &mut c.margin;
    for margin in [&mut m.top, &mut m.right, &mut m.bottom, &mut m.left] {
        if let MarginValue::Calc(e) = margin
            && e.contains_anchor()
        {
            *margin = without_anchors(e).map_or(MarginValue::Cells(0), MarginValue::calc);
        }
    }
}
