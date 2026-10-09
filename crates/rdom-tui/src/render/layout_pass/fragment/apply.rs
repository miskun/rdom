//! Moving a laid-out flow into its fragmentainers (CSS Fragmentation 3
//! §5): each box into the one its rows fall in, a box whose rows several
//! hold split into fragments ([`BoxFragments`]), each line box into its
//! own fragmentainer.

use rdom_core::{Dom, NodeId};

use super::{Plan, Slice};
use crate::ext::{AnonymousIfc, TuiExt};
use crate::layout::{Display, LayoutRect, Position};
use crate::render::inline::InlineLayout;

/// One fragment of a box split across fragmentainers (§5.4 `slice`): the
/// rows of its border box one fragmentainer holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BoxFragment {
    /// Those rows where they are drawn, relative to the box's `layout`
    /// origin (the fragments' bounding box).
    pub(crate) rect: LayoutRect,
    /// Where the whole, unfragmented border box sits for this fragment,
    /// relative to the same origin: the fragment draws that box, cut to
    /// its rows.
    pub(crate) origin: (i32, i32),
}

/// A split box's fragments, in flow order, with what drawing one needs.
/// Relative to the box's `layout` origin, so a subtree shift moves them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BoxFragments {
    /// The unfragmented border box's width and height.
    pub(crate) width: u16,
    pub(crate) height: u16,
    /// The unfragmented content box, relative to the border box's origin.
    pub(crate) content: LayoutRect,
    /// The fragments, at least two.
    pub(crate) list: Vec<BoxFragment>,
}

/// A fragment of a box as drawn: its whole border box and content box at
/// this fragment's place, and the rows of them it shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DrawnFragment {
    pub(crate) border_box: LayoutRect,
    pub(crate) content_box: LayoutRect,
    pub(crate) rows: LayoutRect,
}

impl BoxFragments {
    /// The fragments of the box laid out at `layout`, as drawn.
    pub(crate) fn drawn(&self, layout: LayoutRect) -> impl Iterator<Item = DrawnFragment> + '_ {
        self.list.iter().map(move |f| {
            let (x, y) = (layout.x + f.origin.0, layout.y + f.origin.1);
            DrawnFragment {
                border_box: LayoutRect::new(x, y, self.width, self.height),
                content_box: LayoutRect::new(
                    x + self.content.x,
                    y + self.content.y,
                    self.content.width,
                    self.content.height,
                ),
                rows: LayoutRect::new(
                    layout.x + f.rect.x,
                    layout.y + f.rect.y,
                    f.rect.width,
                    f.rect.height,
                ),
            }
        })
    }

    /// Whether the cell `(x, y)` is in one of the fragments.
    pub(crate) fn contains(&self, layout: LayoutRect, x: i32, y: i32) -> bool {
        self.drawn(layout).any(|f| {
            let r = f.rows;
            x >= r.x && x < r.x + i32::from(r.width) && y >= r.y && y < r.y + i32::from(r.height)
        })
    }
}

fn bottom(r: LayoutRect) -> i32 {
    r.y + i32::from(r.height)
}

fn union(a: Option<LayoutRect>, b: LayoutRect) -> LayoutRect {
    let Some(a) = a else {
        return b;
    };
    let (x, y) = (a.x.min(b.x), a.y.min(b.y));
    let right = (a.x + i32::from(a.width)).max(b.x + i32::from(b.width));
    let bot = bottom(a).max(bottom(b));
    LayoutRect::new(
        x,
        y,
        (right - x).clamp(0, i32::from(u16::MAX)) as u16,
        (bot - y).clamp(0, i32::from(u16::MAX)) as u16,
    )
}

/// Move `root`'s laid-out content into the fragmentainers of `plan`: its
/// lines, whose origin moves from `old` to `new` (the content box's
/// scrolled origin before and after), its anonymous and floated boxes,
/// and its children's subtrees — but the children `skip` (spanners, which
/// the caller lays out again).
pub(in crate::render::layout_pass) fn apply(
    dom: &mut Dom<TuiExt>,
    root: NodeId,
    plan: &Plan,
    old: (i32, i32),
    new: (i32, i32),
    skip: &[NodeId],
) {
    if let Some(ext) = dom.node_mut(root).ext_mut() {
        if let Some(il) = ext.inline_layout.as_mut() {
            rebase(il, old, new, plan);
        }
        move_anonymous(&mut ext.anonymous_blocks, plan);
        move_floated(ext.floated_pseudos.as_deref_mut(), plan);
    }
    for child in super::super::element_children_of(dom, root) {
        if !skip.contains(&child) {
            place(dom, child, plan);
        }
    }
}

/// Move `id`'s box into its fragmentainer, or split it across several.
fn place(dom: &mut Dom<TuiExt>, id: NodeId, plan: &Plan) {
    let Some(ext) = dom.node(id).ext() else {
        return;
    };
    let Some(c) = ext.computed.as_deref() else {
        return;
    };
    if c.display == Display::None {
        return;
    }
    // An absolutely positioned box is placed after (`positioning`), from
    // its static position — in flow, so in a fragmentainer.
    if matches!(c.position, Position::Absolute | Position::Fixed) {
        if let Some(sp) = ext.static_position {
            let s = plan.slices[plan.index(sp.y)];
            if let Some(p) = dom
                .node_mut(id)
                .ext_mut()
                .and_then(|e| e.static_position.as_mut())
            {
                p.x += s.dx;
                p.y += s.dy;
            }
        }
        return;
    }
    // An inline box has no rect of its own (its text is its container's
    // lines): its atoms and floats move by their own rows.
    if c.display == Display::Inline && !c.is_atomic_inline() {
        let y = ext.layout.y;
        let s = plan.slices[plan.index(y)];
        if let Some(e) = dom.node_mut(id).ext_mut() {
            e.layout.x += s.dx;
            e.layout.y += s.dy;
            e.content_layout.x += s.dx;
            e.content_layout.y += s.dy;
        }
        for child in super::super::element_children_of(dom, id) {
            place(dom, child, plan);
        }
        return;
    }
    let r = ext.layout;
    let (first, last) = plan.span(r.y, r.height);
    if first == last || !super::candidates::fragmentable(dom, id) {
        let s = plan.slices[first];
        super::super::tree::shift_subtree(dom, id, s.dx, s.dy);
        return;
    }
    split(dom, id, plan, first, last);
}

/// Split `id`, whose border box the slices `first ..= last` of `plan`
/// hold, into fragments; then place its content.
fn split(dom: &mut Dom<TuiExt>, id: NodeId, plan: &Plan, first: usize, last: usize) {
    let Some(ext) = dom.node(id).ext() else {
        return;
    };
    let (r, cl) = (ext.layout, ext.content_layout);
    let mut pieces: Vec<(Slice, LayoutRect)> = Vec::new();
    let mut outer = None;
    let mut inner = None;
    for (k, s) in plan.slices[first..=last].iter().enumerate() {
        let top = if k == 0 { r.y } else { r.y.max(s.start) };
        let bot = if first + k == last {
            bottom(r)
        } else {
            bottom(r).min(s.end)
        };
        if bot <= top {
            continue;
        }
        let rows = LayoutRect::new(r.x + s.dx, top + s.dy, r.width, (bot - top) as u16);
        outer = Some(union(outer, rows));
        let (ctop, cbot) = (cl.y.max(top), bottom(cl).min(bot));
        if cbot > ctop {
            let content = LayoutRect::new(cl.x + s.dx, ctop + s.dy, cl.width, (cbot - ctop) as u16);
            inner = Some(union(inner, content));
        }
        pieces.push((*s, rows));
    }
    let Some(outer) = outer.filter(|_| pieces.len() > 1) else {
        let s = plan.slices[first];
        super::super::tree::shift_subtree(dom, id, s.dx, s.dy);
        return;
    };
    let inner = inner.unwrap_or(LayoutRect::new(
        outer.x + (cl.x - r.x),
        outer.y,
        cl.width,
        0,
    ));
    let fragments = BoxFragments {
        width: r.width,
        height: r.height,
        content: LayoutRect::new(cl.x - r.x, cl.y - r.y, cl.width, cl.height),
        list: pieces
            .iter()
            .map(|(s, rows)| BoxFragment {
                rect: LayoutRect::new(rows.x - outer.x, rows.y - outer.y, rows.width, rows.height),
                origin: (r.x + s.dx - outer.x, r.y + s.dy - outer.y),
            })
            .collect(),
    };
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        // A fragmentable box is no scroll container: its lines sit at its
        // content box.
        if let Some(il) = ext.inline_layout.as_mut() {
            rebase(il, (cl.x, cl.y), (inner.x, inner.y), plan);
        }
        move_anonymous(&mut ext.anonymous_blocks, plan);
        move_floated(ext.floated_pseudos.as_deref_mut(), plan);
        ext.layout = outer;
        ext.content_layout = inner;
        ext.kept = Some(Box::new(crate::ext::KeptLayout::Fragments(fragments)));
    }
    for child in super::super::element_children_of(dom, id) {
        place(dom, child, plan);
    }
}

/// Move a box's anonymous block boxes: each whole into its fragmentainer,
/// or its lines each into theirs (an anonymous box draws nothing of its
/// own, so its rect is its lines' bounding box). A generated block box
/// (`::before` / `::after`) is monolithic.
fn move_anonymous(anons: &mut [AnonymousIfc], plan: &Plan) {
    for anon in anons {
        if let Some(g) = anon.generated.as_mut() {
            let s = plan.slices[plan.index(g.border_box.y)];
            g.border_box.x += s.dx;
            g.border_box.y += s.dy;
            anon.rect.x += s.dx;
            anon.rect.y += s.dy;
            continue;
        }
        let r = anon.rect;
        let (first, last) = plan.span(r.y, r.height);
        if first == last {
            let s = plan.slices[first];
            anon.rect.x += s.dx;
            anon.rect.y += s.dy;
            continue;
        }
        let mut at = None;
        for line in &anon.inline_layout.lines {
            let top = r.y + i32::from(line.top);
            let s = plan.slices[plan.index(top)];
            at = Some(union(
                at,
                LayoutRect::new(r.x + s.dx, top + s.dy, r.width, line.height),
            ));
        }
        let Some(at) = at else {
            continue;
        };
        rebase(&mut anon.inline_layout, (r.x, r.y), (at.x, at.y), plan);
        anon.rect = at;
    }
}

/// Move a formatting context's floated `::before` / `::after` (monolithic)
/// into their fragmentainers.
fn move_floated(floated: Option<&mut Vec<AnonymousIfc>>, plan: &Plan) {
    for anon in floated.into_iter().flatten() {
        let y = anon.border_box().y;
        let s = plan.slices[plan.index(y)];
        anon.rect.x += s.dx;
        anon.rect.y += s.dy;
        if let Some(g) = anon.generated.as_mut() {
            g.border_box.x += s.dx;
            g.border_box.y += s.dy;
        }
    }
}

/// Move each line of `il` — whose rows and columns count from `old` — into
/// its fragmentainer, counting from `new` after: its row, its fragments'
/// and generated runs' columns, its float band, and the fragmentainer it
/// is in ([`LineBox::column`](crate::render::LineBox)).
fn rebase(il: &mut InlineLayout, old: (i32, i32), new: (i32, i32), plan: &Plan) {
    let width = il.content_width;
    for line in &mut il.lines {
        let top = old.1 + i32::from(line.top);
        let s = plan.slices[plan.index(top)];
        line.top = (top + s.dy - new.1).clamp(0, i32::from(u16::MAX)) as u16;
        let dx = old.0 + s.dx - new.0;
        for f in &mut line.fragments {
            f.x += dx;
        }
        for g in &mut line.generated {
            g.x += dx;
        }
        if let Some((start, w)) = line.band {
            line.band = Some((start + dx, w));
        }
        line.column = Some((dx, width));
    }
}
