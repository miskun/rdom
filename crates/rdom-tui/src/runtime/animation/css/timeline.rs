//! Progress-based timelines (Scroll-driven Animations 1 §2–§4): which
//! scroll container an animation's timeline follows — `scroll()`,
//! `view()` or a named timeline, through `timeline-scope` — and where the
//! container's scroll offset stands in the timeline's range and its named
//! ranges, all measured in cells from the scroll origin (CSSOM View §4:
//! the right or bottom edge where the axis runs reversed, so `rtl` and the
//! reversed flex axes count from there), against the unified scrollport
//! (`layout_pass::scrollport`) and the offsets of the last layout.
//!
//! Resolved each frame (a name can move to another element without this
//! one's style changing); a `timeline-scope` lookup searches the scope's
//! subtree, so it costs that subtree's size per frame.

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::Length;
use crate::style::{
    AnimationTimeline, ComputedStyle, RangeBoundary, TimelineAxis, TimelineInset,
    TimelineRangeName, TimelineScroller,
};

/// An animation's timeline at one moment.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Resolved {
    /// The document timeline: the clock.
    Document,
    /// No active timeline (`none`, no scroll container, nothing to
    /// scroll): the animation is idle.
    Inactive,
    Progress(ProgressTimeline),
}

/// A scroll or view progress timeline, in scroll offsets along its axis.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProgressTimeline {
    /// The scroll offset, from the scroll origin.
    offset: f64,
    /// The timeline's range: `0 ..= max` for a scroll timeline, `cover`
    /// for a view timeline.
    full: (f64, f64),
    /// The subject's geometry, for a view timeline's named ranges.
    view: Option<ViewGeometry>,
}

/// A view timeline's subject against its scrollport (§3.4), the insets
/// applied: `start` is the subject's near edge from the scroll origin,
/// `length` its size, `port` the inset scrollport's size.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ViewGeometry {
    start: f64,
    length: f64,
    port: f64,
}

impl ViewGeometry {
    /// A named range's offsets (§3.4).
    fn range(&self, name: TimelineRangeName) -> (f64, f64) {
        let (p, l, s) = (self.start, self.length, self.port);
        let cover = (p - s, p + l);
        let contain = if l <= s {
            (p + l - s, p)
        } else {
            (p, p + l - s)
        };
        match name {
            TimelineRangeName::Cover => cover,
            TimelineRangeName::Contain => contain,
            TimelineRangeName::Entry => (cover.0, contain.0),
            TimelineRangeName::Exit => (contain.1, cover.1),
            TimelineRangeName::EntryCrossing => (p - s, p - s + l),
            TimelineRangeName::ExitCrossing => (p, p + l),
        }
    }
}

impl ProgressTimeline {
    /// The offsets of the range `name` names — the whole timeline for
    /// `None`, and for any name on a scroll timeline, which has no named
    /// ranges.
    fn range(&self, name: Option<TimelineRangeName>) -> (f64, f64) {
        match (name, self.view) {
            (Some(name), Some(view)) => view.range(name),
            _ => self.full,
        }
    }

    /// The offset a range boundary stands at (§4.3): `normal` the
    /// timeline's start (`end`: its end); an offset into its range, a
    /// percentage of that range's length.
    fn boundary(&self, b: &RangeBoundary, end: bool) -> f64 {
        match b {
            RangeBoundary::Normal => {
                if end {
                    self.full.1
                } else {
                    self.full.0
                }
            }
            RangeBoundary::Offset { name, offset } => {
                let (a, z) = self.range(*name);
                a + resolve(offset, z - a)
            }
        }
    }

    /// Where the point `fraction` of the timeline range `name` falls in
    /// the attachment range `[start, end]` (Scroll-driven Animations 1
    /// §4.4) — `None` on a scroll timeline, which has no named ranges.
    pub(super) fn place(
        &self,
        name: TimelineRangeName,
        fraction: f64,
        (start, end): (&RangeBoundary, &RangeBoundary),
    ) -> Option<f64> {
        let view = self.view?;
        let (a, z) = view.range(name);
        let point = a + fraction * (z - a);
        let (r0, r1) = (self.boundary(start, false), self.boundary(end, true));
        (r1 > r0).then(|| (point - r0) / (r1 - r0))
    }

    /// Where the scroll offset stands in the attachment range
    /// `[start, end]` (§4.3): 0 at its start, 1 at its end, outside
    /// before and after it.
    pub(super) fn fraction(&self, start: &RangeBoundary, end: &RangeBoundary) -> f64 {
        let (a, z) = (self.boundary(start, false), self.boundary(end, true));
        if z > a {
            (self.offset - a) / (z - a)
        } else if self.offset < a {
            -1.0
        } else {
            2.0
        }
    }
}

/// A `<length-percentage>` in cells, its percentage of `basis`.
fn resolve(length: &Length, basis: f64) -> f64 {
    match length {
        Length::Auto => 0.0,
        Length::Cells(n) => f64::from(*n),
        Length::Calc(e) => e.resolve_f64(&rdom_style::calc::ResolveCtx::new(basis.round() as i32)),
    }
}

/// The timeline `timeline` of the element `id` (an animation of one of
/// its pseudo-elements looks up from the element).
pub(crate) fn resolve_timeline(
    dom: &Dom<TuiExt>,
    id: NodeId,
    timeline: &AnimationTimeline,
) -> Resolved {
    match timeline {
        AnimationTimeline::Auto => Resolved::Document,
        AnimationTimeline::None => Resolved::Inactive,
        AnimationTimeline::Scroll { scroller, axis } => {
            let sc = match scroller {
                TimelineScroller::Nearest => nearest_scroller(dom, id),
                TimelineScroller::Root => Some(dom.document_element().id()),
                TimelineScroller::SelfElement => Some(id),
            };
            sc.map_or(Resolved::Inactive, |sc| scroll(dom, sc, *axis))
        }
        AnimationTimeline::View { axis, inset } => view(dom, id, *axis, inset),
        AnimationTimeline::Named(name) => named(dom, id, name),
        // `AnimationTimeline` is `#[non_exhaustive]`; a timeline added
        // upstream is unknown here: the document's.
        #[allow(unreachable_patterns)]
        _ => Resolved::Document,
    }
}

fn style(dom: &Dom<TuiExt>, id: NodeId) -> Option<&ComputedStyle> {
    dom.node(id).ext()?.computed.as_deref()
}

fn is_scroll_container(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    style(dom, id)
        .is_some_and(|c| c.is_scroll_container() && c.display != crate::layout::Display::None)
}

/// The nearest scroll container above `id` (its box ancestors).
fn nearest_scroller(dom: &Dom<TuiExt>, id: NodeId) -> Option<NodeId> {
    let mut at = crate::render::box_tree::box_parent(dom, id);
    while let Some(a) = at {
        if is_scroll_container(dom, a) {
            return Some(a);
        }
        at = crate::render::box_tree::box_parent(dom, a);
    }
    None
}

/// Whether `axis` is the vertical one (`block` is `y` in `horizontal-tb`,
/// rdom's only writing mode).
fn vertical(axis: TimelineAxis) -> bool {
    matches!(axis, TimelineAxis::Block | TimelineAxis::Y)
}

/// `sc`'s scroll progress timeline on `axis` (§2): inactive when it is no
/// scroll container or has nothing to scroll on the axis.
fn scroll(dom: &Dom<TuiExt>, sc: NodeId, axis: TimelineAxis) -> Resolved {
    let Some(g) = axis_geometry(dom, sc, axis) else {
        return Resolved::Inactive;
    };
    if g.range <= 0.0 {
        return Resolved::Inactive;
    }
    Resolved::Progress(ProgressTimeline {
        offset: g.offset,
        full: (0.0, g.range),
        view: None,
    })
}

/// A scroll container's state on one axis, from its scroll origin.
struct AxisGeometry {
    /// The current offset.
    offset: f64,
    /// The scroll range.
    range: f64,
    /// The scrollport's size.
    port: f64,
    /// Whether the origin is the far (right / bottom) edge.
    origin_at_end: bool,
    /// The offset (from the origin) the last layout placed the content at.
    laid_out: f64,
}

fn axis_geometry(dom: &Dom<TuiExt>, sc: NodeId, axis: TimelineAxis) -> Option<AxisGeometry> {
    if !is_scroll_container(dom, sc) {
        return None;
    }
    let ext = dom.node(sc).ext()?;
    let bounds = crate::runtime::scrollbar::scroll_bounds(dom, sc)?;
    let port = crate::render::layout_pass::scrollport::scrollport(dom, sc)?;
    let laid = crate::runtime::scrollbar::state::laid_out(ext);
    let v = vertical(axis);
    let (offset, laid, min, max, size, at_end) = if v {
        let (min, max) = (bounds.min_y, bounds.max_y);
        (
            ext.scroll_y,
            laid.1,
            min,
            max,
            port.height,
            bounds.origin_at_end.1,
        )
    } else {
        let (min, max) = (bounds.min_x, bounds.max_x);
        (
            ext.scroll_x,
            laid.0,
            min,
            max,
            port.width,
            bounds.origin_at_end.0,
        )
    };
    let from_origin = |o: i32| f64::from(if at_end { -o } else { o });
    Some(AxisGeometry {
        offset: from_origin(offset),
        range: f64::from(max - min),
        port: f64::from(size),
        origin_at_end: at_end,
        laid_out: from_origin(laid),
    })
}

/// The view progress timeline of the subject `id` on `axis` (§3): its
/// crossing of its nearest scroll container's scrollport, the insets
/// applied.
fn view(dom: &Dom<TuiExt>, id: NodeId, axis: TimelineAxis, inset: &TimelineInset) -> Resolved {
    let Some(sc) = nearest_scroller(dom, id) else {
        return Resolved::Inactive;
    };
    let Some(g) = axis_geometry(dom, sc, axis) else {
        return Resolved::Inactive;
    };
    let (Some(subject), Some(port)) = (
        dom.node(id).ext().map(|e| e.layout),
        crate::render::layout_pass::scrollport::scrollport(dom, sc),
    ) else {
        return Resolved::Inactive;
    };
    let v = vertical(axis);
    let (near, length, port_near) = if v {
        (subject.y, f64::from(subject.height), port.y)
    } else {
        (subject.x, f64::from(subject.width), port.x)
    };
    // The subject's near edge from the scrolling area's start, where the
    // last layout placed it (its offset then added back).
    let in_port = f64::from(near - port_near);
    let start = if g.origin_at_end {
        // Measured from the far edge: the area is the port plus the range.
        g.port + g.range - (in_port + (g.range - g.laid_out)) - length
    } else {
        in_port + g.laid_out
    };
    // §3.2.3: the insets shrink the scrollport; `auto` is its
    // `scroll-padding` on that side.
    let padding = style(dom, sc).map(|c| c.scroll_padding.clone());
    let side = |l: &Length, near_side: bool| match l {
        Length::Auto => padding.as_ref().map_or(0.0, |p| {
            let s = match (v, near_side != g.origin_at_end) {
                (true, true) => &p.top,
                (true, false) => &p.bottom,
                (false, true) => &p.left,
                (false, false) => &p.right,
            };
            f64::from(s.resolve(g.port as u16))
        }),
        other => resolve(other, g.port),
    };
    let (is, ie) = (side(&inset.start, true), side(&inset.end, false));
    let geometry = ViewGeometry {
        start: start - is,
        length,
        port: g.port - is - ie,
    };
    Resolved::Progress(ProgressTimeline {
        offset: g.offset,
        full: geometry.range(TimelineRangeName::Cover),
        view: Some(geometry),
    })
}

/// The named timeline `name` as `id` sees it (§4.1–§4.2): the nearest
/// element at or above it declaring it — a scroll timeline before a view
/// timeline of the name — or the one descendant declaring it of the
/// nearest ancestor whose `timeline-scope` takes it (none or several:
/// inactive).
fn named(dom: &Dom<TuiExt>, id: NodeId, name: &str) -> Resolved {
    let mut at = Some(id);
    while let Some(a) = at {
        if let Some(found) = declared(dom, a, name) {
            return found;
        }
        if style(dom, a).is_some_and(|c| c.timeline_scope.covers(name)) {
            let mut found = Vec::new();
            descendants_declaring(dom, a, name, &mut found);
            return match found.as_slice() {
                [only] => declared(dom, *only, name).unwrap_or(Resolved::Inactive),
                _ => Resolved::Inactive,
            };
        }
        at = dom
            .node(a)
            .parent_node()
            .filter(|p| p.node_type() == NodeType::Element)
            .map(|p| p.id());
    }
    Resolved::Inactive
}

/// The timeline `a` declares under `name`, if it declares one.
fn declared(dom: &Dom<TuiExt>, a: NodeId, name: &str) -> Option<Resolved> {
    let c = style(dom, a)?;
    let index =
        |names: &[crate::style::TimelineName]| names.iter().rposition(|n| n.name() == Some(name));
    let cycled = |list: &[TimelineAxis], i: usize| {
        if list.is_empty() {
            TimelineAxis::Block
        } else {
            list[i % list.len()]
        }
    };
    if let Some(i) = index(&c.scroll_timeline_name) {
        return Some(scroll(dom, a, cycled(&c.scroll_timeline_axis, i)));
    }
    let i = index(&c.view_timeline_name)?;
    let inset = if c.view_timeline_inset.is_empty() {
        TimelineInset::default()
    } else {
        c.view_timeline_inset[i % c.view_timeline_inset.len()].clone()
    };
    Some(view(dom, a, cycled(&c.view_timeline_axis, i), &inset))
}

/// The elements under `a` declaring a timeline named `name`, in tree
/// order — walked with an explicit stack, so a deep subtree cannot
/// overflow the call stack (C12G-MISC).
fn descendants_declaring(dom: &Dom<TuiExt>, a: NodeId, name: &str, out: &mut Vec<NodeId>) {
    let children = |id: NodeId| {
        let mut ids: Vec<NodeId> = dom.node(id).children().map(|c| c.id()).collect();
        ids.reverse();
        ids
    };
    let mut stack = children(a);
    while let Some(c) = stack.pop() {
        if dom.node(c).node_type() != NodeType::Element {
            continue;
        }
        if style(dom, c).is_some_and(|s| {
            s.scroll_timeline_name
                .iter()
                .chain(&s.view_timeline_name)
                .any(|n| n.name() == Some(name))
        }) {
            out.push(c);
        }
        stack.extend(children(c));
    }
}
