//! `serialize` for `display`, the flexbox properties (`flex-*`, `order`)
//! and box alignment (`justify-*`, `align-*`, `place-*`, the gaps).

use super::super::value_serializers::{serialize_flex_basis, serialize_gap, specified};
use crate::TuiStyle;

/// `name`'s serialization when it is one of this family's properties —
/// `Some(None)` when it is not set — else `None`.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    let out = match name {
        "display" => style.display.as_ref().and_then(specified).map(|d| {
            // Compat Standard §5: the legacy keywords read back as written.
            if style.webkit_box.as_ref().and_then(specified) == Some(&true) {
                let inline = *d == crate::layout::Display::Inline;
                return if inline {
                    "-webkit-inline-box"
                } else {
                    "-webkit-box"
                }
                .to_string();
            }
            let flow = style.flow.as_ref().and_then(specified).copied();
            let list_item = style.list_item.as_ref().and_then(specified).copied();
            crate::parse::values::serialize_display(
                *d,
                flow.unwrap_or_default(),
                list_item.unwrap_or(false),
            )
        }),
        "flex-direction" => style.direction.as_ref().and_then(specified).map(|d| {
            let reverse = style.flex_reverse.as_ref().and_then(specified) == Some(&true);
            crate::parse::values::serialize_flex_direction(*d, reverse).to_string()
        }),
        "flex-wrap" => style
            .flex_wrap
            .as_ref()
            .and_then(specified)
            .map(|w| crate::parse::values::serialize_flex_wrap(*w).to_string()),
        "justify-content" => style
            .justify_content
            .as_ref()
            .and_then(specified)
            .map(|a| crate::parse::values::serialize_alignment(*a)),
        "justify-items" => style
            .justify_items
            .as_ref()
            .and_then(specified)
            .map(|a| crate::parse::values::serialize_alignment(*a)),
        "justify-self" => style
            .justify_self
            .as_ref()
            .and_then(specified)
            .map(|a| crate::parse::values::serialize_alignment(*a)),
        // A shorthand serializes when both its longhands are set.
        "place-content" => {
            let a = style.align_content.as_ref().and_then(specified)?;
            let j = style.justify_content.as_ref().and_then(specified)?;
            Some(crate::parse::values::serialize_place(*a, *j, true))
        }
        "place-items" => {
            let a = style.align_items.as_ref().and_then(specified)?;
            let j = style.justify_items.as_ref().and_then(specified)?;
            Some(crate::parse::values::serialize_place(*a, *j, false))
        }
        "place-self" => {
            let a = style.align_self.as_ref().and_then(specified)?;
            let j = style.justify_self.as_ref().and_then(specified)?;
            Some(crate::parse::values::serialize_place(*a, *j, false))
        }
        "align-content" => style
            .align_content
            .as_ref()
            .and_then(specified)
            .map(|a| crate::parse::values::serialize_alignment(*a)),
        "align-items" => style
            .align_items
            .as_ref()
            .and_then(specified)
            .map(|a| crate::parse::values::serialize_alignment(*a)),
        "align-self" => style
            .align_self
            .as_ref()
            .and_then(specified)
            .map(|a| crate::parse::values::serialize_alignment(*a)),
        // The shorthand serializes only when both longhands are set
        // (CSSOM §6.7.2).
        "flex-flow" => {
            let d = style.direction.as_ref().and_then(specified)?;
            let w = style.flex_wrap.as_ref().and_then(specified)?;
            let reverse = style.flex_reverse.as_ref().and_then(specified) == Some(&true);
            Some(crate::parse::values::serialize_flex_flow(*d, reverse, *w))
        }
        // longhands are declared (CSS Flexbox §7.2).
        "flex" => match (
            style.flex_grow.as_ref().and_then(specified),
            style.flex_shrink.as_ref().and_then(specified),
            style.flex_basis.as_ref().and_then(specified),
        ) {
            (Some(grow), Some(shrink), Some(basis)) => {
                Some(format!("{grow} {shrink} {}", serialize_flex_basis(basis)))
            }
            _ => None,
        },
        "flex-grow" => style
            .flex_grow
            .as_ref()
            .and_then(specified)
            .map(|n| n.to_string()),
        "flex-basis" => style
            .flex_basis
            .as_ref()
            .and_then(specified)
            .map(serialize_flex_basis),
        "flex-shrink" => style
            .flex_shrink
            .as_ref()
            .and_then(specified)
            .map(|n| n.to_string()),
        "order" => style
            .order
            .as_ref()
            .and_then(specified)
            .map(|n| n.to_string()),

        // Layout — sizing
        // CSS Box Alignment 3 §8.3: one value when the two agree.
        "gap" => match (
            style.row_gap.as_ref().and_then(specified),
            style.column_gap.as_ref().and_then(specified),
        ) {
            (Some(row), Some(column)) if row == column => Some(serialize_gap(row)),
            (Some(row), Some(column)) => {
                Some(format!("{} {}", serialize_gap(row), serialize_gap(column)))
            }
            _ => None,
        },
        "row-gap" => style
            .row_gap
            .as_ref()
            .and_then(specified)
            .map(serialize_gap),
        "column-gap" => style
            .column_gap
            .as_ref()
            .and_then(specified)
            .map(serialize_gap),

        // CSS Box 3 §3.2 / §4.2: a shorthand when every side is set, in
        _ => return None,
    };
    Some(out)
}
