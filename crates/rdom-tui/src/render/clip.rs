//! The clip a `clip-path` draws (CSS Masking 1 §5, C15-CLIP-PATH): its
//! shape on its reference box, and whether a cell is inside it — the one
//! answer paint (`paint_pass::effects`) and hit-testing share.

use rdom_core::{Dom, NodeId};
use rdom_style::layout::{BasicShape, ClipPath, GeometryBox, MarginValue};

use crate::ext::TuiExt;
use crate::layout::LayoutRect;
use crate::style::ComputedStyle;
use crate::style::effects::clip_pathed;

/// The clip a `clip-path` draws on the element `id` styled `c`: its shape
/// (`None`: the reference box itself) and its reference box `[left, top,
/// width, height]` in cells — `None` when it clips nothing (`none`,
/// `url()`, `path()`, or no box).
pub(crate) fn clip_of<'a>(
    dom: &Dom<TuiExt>,
    id: NodeId,
    c: &'a ComputedStyle,
) -> Option<(Option<&'a BasicShape>, [f64; 4])> {
    let ClipPath::Shape { shape, reference } = &c.effects.clip_path else {
        return None;
    };
    if !clip_pathed(c) || matches!(shape.as_deref(), Some(BasicShape::Path { .. })) {
        return None;
    }
    let ext = dom.node(id).ext()?;
    let border = ext.layout;
    let rect = match reference {
        GeometryBox::ContentBox => ext.content_layout,
        GeometryBox::PaddingBox => crate::layout::compute_padding_box(border, c.border),
        GeometryBox::MarginBox => {
            let basis = crate::render::box_tree::box_parent(dom, id)
                .and_then(|p| dom.node(p).ext().map(|e| i32::from(e.content_layout.width)))
                .unwrap_or(0);
            let m = |v: &MarginValue| match v {
                MarginValue::Cells(n) => i32::from(*n),
                MarginValue::Calc(e) => e.resolve(&rdom_style::calc::ResolveCtx::new(basis)),
                // A used `auto` margin is layout's alone: 0 here.
                MarginValue::Auto => 0,
            };
            let (t, r, b, l) = (
                m(&c.margin.top),
                m(&c.margin.right),
                m(&c.margin.bottom),
                m(&c.margin.left),
            );
            LayoutRect::new(
                border.x - l,
                border.y - t,
                (i32::from(border.width) + l + r).clamp(0, i32::from(u16::MAX)) as u16,
                (i32::from(border.height) + t + b).clamp(0, i32::from(u16::MAX)) as u16,
            )
        }
        _ => border,
    };
    Some((
        shape.as_deref(),
        [
            f64::from(rect.x),
            f64::from(rect.y),
            f64::from(rect.width),
            f64::from(rect.height),
        ],
    ))
}

/// Whether the cell `(x, y)` is inside a clip ([`clip_of`]): its centre
/// is inside the shape, or the reference box.
pub(crate) fn clip_contains(clip: (Option<&BasicShape>, [f64; 4]), x: i32, y: i32) -> bool {
    let (shape, reference) = clip;
    let (cx, cy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
    match shape {
        Some(s) => s.contains(reference, cx, cy),
        None => {
            let [left, top, w, h] = reference;
            cx >= left && cx < left + w && cy >= top && cy < top + h
        }
    }
}
