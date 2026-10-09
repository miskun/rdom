//! What an animated value change costs layout (C15G-TRANSLATE-GAPS).

use super::Longhand;
use crate::style::ComputedStyle;

/// Whether `l` going from `old` to `new` moves a box. A transform longhand
/// on an element moves it only when its whole-cell translation against the
/// box (`boxes`: its border and content box) changes — or whether it is
/// transformed, or its reference box — so a percentage `translate` lays out
/// on the frames its cell offset changes, not each frame its raw value
/// does (C15G-TRANSLATE-GAPS); otherwise `Longhand::moves_boxes`.
pub(super) fn moves(
    l: Longhand,
    old: &ComputedStyle,
    new: &ComputedStyle,
    boxes: Option<(crate::layout::LayoutRect, crate::layout::LayoutRect)>,
) -> bool {
    match (l.name(), boxes) {
        ("translate" | "rotate" | "scale" | "transform", Some((border, content))) => {
            let (a, b) = (&old.effects, &new.effects);
            a.is_transformed() != b.is_transformed()
                || a.transform_box != b.transform_box
                || crate::style::effects::translation(old, border, content)
                    != crate::style::effects::translation(new, border, content)
        }
        _ => l.moves_boxes(old, new),
    }
}
