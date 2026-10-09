//! The clip a `clip-path` draws (CSS Masking 1 §5, C15-CLIP-PATH): its
//! shape on its reference box — and the legacy `clip` rectangle of an
//! absolutely positioned box (CSS 2.1 §11.1.2, C15G-LEGACY-CLIP) — and
//! whether a cell is inside them: the one answer paint
//! (`paint_pass::effects`) and hit-testing share.

use rdom_core::{Dom, NodeId};
use rdom_style::layout::{BasicShape, ClipPath, GeometryBox, MarginValue};

use crate::ext::TuiExt;
use crate::layout::LayoutRect;
use crate::style::ComputedStyle;
use crate::style::effects::{clip_pathed, legacy_clipped};

/// What clips an element's cells: a `clip-path`'s shape on its reference
/// box, and a legacy `clip` rectangle — a cell is kept inside both.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Clip<'a> {
    /// The `clip-path`: its shape (`None`: the reference box itself) and
    /// its reference box `[left, top, width, height]` in cells.
    path: Option<(Option<&'a BasicShape>, [f64; 4])>,
    /// The `clip` rectangle, `[left, top, right, bottom]` in cells.
    rect: Option<[i32; 4]>,
}

/// The clip of the element `id` styled `c` — `None` when nothing clips it
/// (no `clip-path`, or `url()` / `path()`, which clip nothing, and no
/// legacy `clip` on an absolutely positioned box), or it has no box.
pub(crate) fn clip_of<'a>(dom: &Dom<TuiExt>, id: NodeId, c: &'a ComputedStyle) -> Option<Clip<'a>> {
    let path = path_of(dom, id, c);
    let rect = legacy_clipped(c)
        .then(|| {
            let b = dom.node(id).ext()?.layout;
            let [l, t, r, bo] = c
                .effects
                .clip
                .edges(i32::from(b.width), i32::from(b.height))?;
            Some([b.x + l, b.y + t, b.x + r, b.y + bo])
        })
        .flatten();
    (path.is_some() || rect.is_some()).then_some(Clip { path, rect })
}

/// A `clip-path`'s shape and reference box ([`Clip::path`]).
fn path_of<'a>(
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

/// Whether the cell `(x, y)` is inside a clip ([`clip_of`]): inside its
/// `clip` rectangle, and its centre inside the `clip-path` shape (or
/// reference box).
pub(crate) fn clip_contains(clip: Clip<'_>, x: i32, y: i32) -> bool {
    let in_rect = clip
        .rect
        .is_none_or(|[l, t, r, b]| x >= l && x < r && y >= t && y < b);
    in_rect && clip.path.is_none_or(|p| path_contains(p, x, y))
}

/// Whether the cell `(x, y)`'s centre is inside a `clip-path`'s shape, or
/// its reference box.
fn path_contains(path: (Option<&BasicShape>, [f64; 4]), x: i32, y: i32) -> bool {
    let (shape, reference) = path;
    let (cx, cy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
    match shape {
        Some(s) => s.contains(reference, cx, cy),
        None => {
            let [left, top, w, h] = reference;
            cx >= left && cx < left + w && cy >= top && cy < top + h
        }
    }
}
