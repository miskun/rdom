//! The possible breaks of a laid-out flow (CSS Fragmentation 3 §4.1):
//! class A between block-level siblings, class B between line boxes, class
//! C between a block container's content edge and its children's outer
//! edges where a gap separates them (an explicit height past its content),
//! each
//! with the §4.4 rules it breaks and whether `break-before` /
//! `break-after` force it (§3.1, propagated from a first or last child,
//! §3.1's "propagates to its parent"). Monolithic boxes (§4.1) are not
//! looked into. A float is kept whole (monolithic, as rdom lays it out —
//! DIVERGENCES): a break inside its rows is marked [`FLOAT`], given up
//! after every §4.4 rule, so the float moves to the next fragmentainer when
//! a break before it fits and overflows its own otherwise.

use rdom_core::{Dom, NodeId};

use super::{Break, FLOAT, RULE_1, RULE_2, RULE_3, RULE_4};
use crate::ext::TuiExt;
use crate::layout::{BreakBetween, LayoutRect};
use crate::node::TuiNodeExt;
use crate::render::inline::InlineLayout;
use crate::style::ComputedStyle;

/// The breaks of `root`'s laid-out content, by the row they end at.
pub(in crate::render::layout_pass) fn collect(dom: &Dom<TuiExt>, root: NodeId) -> Vec<Break> {
    let mut out = Vec::new();
    let mut floats = Vec::new();
    flow(dom, root, Around::default(), &mut out, &mut floats);
    out.sort_by_key(|b| (b.end, b.resume));
    mark_floats(&mut out, floats);
    out
}

/// Mark each break strictly inside a float's rows `[top, bottom)` —
/// `O((b + f) log f)`: the floats sorted by top, a running deepest bottom.
fn mark_floats(breaks: &mut [Break], mut floats: Vec<(i32, i32)>) {
    if floats.is_empty() {
        return;
    }
    floats.sort_unstable();
    let deepest: Vec<i32> = floats
        .iter()
        .scan(i32::MIN, |d, &(_, b)| {
            *d = (*d).max(b);
            Some(*d)
        })
        .collect();
    for b in breaks {
        // The floats starting above the break; one of them reaching past it.
        let k = floats.partition_point(|&(top, _)| top < b.end);
        if k > 0 && deepest[k - 1] > b.end {
            b.violates |= FLOAT;
        }
    }
}

/// What a `break-before` / `-after` asks of the break at a box's edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Ask {
    Auto,
    Avoid,
    Force,
}

fn ask(v: BreakBetween) -> Ask {
    if v.forces_column() {
        Ask::Force
    } else if v.avoids_column() {
        Ask::Avoid
    } else {
        Ask::Auto
    }
}

/// A box of a block container's flow, in order.
enum Item {
    /// A block-level element and its border box.
    Element(NodeId, LayoutRect),
    /// An anonymous block box's lines (or a generated block box, whole),
    /// by index.
    Anonymous(usize, LayoutRect),
}

impl Item {
    fn rect(&self) -> LayoutRect {
        match self {
            Item::Element(_, r) | Item::Anonymous(_, r) => *r,
        }
    }
}

fn computed(dom: &Dom<TuiExt>, id: NodeId) -> Option<&ComputedStyle> {
    dom.node(id).tui_ext()?.computed.as_deref()
}

fn bottom(r: LayoutRect) -> i32 {
    r.y + i32::from(r.height)
}

/// What the boxes around a break give it: the §4.4 rules a break anywhere
/// in them breaks (a `break-inside: avoid` around), and the rows their
/// cloned edges add (`box-decoration-break: clone`, §5.4).
#[derive(Debug, Clone, Copy, Default)]
struct Around {
    violates: u8,
    tail: u16,
    lead: u16,
}

impl Around {
    /// A class A break.
    fn at(self, end: i32, resume: i32, forced: bool, violates: u8) -> Break {
        Break {
            end,
            resume,
            forced,
            violates: self.violates | violates,
            tail: self.tail,
            lead: self.lead,
        }
    }

    /// A class B or C break: a `break-inside: avoid` around it breaks rule
    /// 4, not rule 2.
    fn at_bc(self, end: i32, resume: i32, violates: u8) -> Break {
        let inside = if self.violates & RULE_2 != 0 {
            RULE_4
        } else {
            0
        };
        Break {
            violates: (self.violates & !RULE_2) | inside | violates,
            ..self.at(end, resume, false, 0)
        }
    }

    /// Inside the fragmentable box `id` too.
    fn into(self, dom: &Dom<TuiExt>, id: NodeId) -> Around {
        let Some(c) = computed(dom, id) else {
            return self;
        };
        let mut around = self;
        if c.fragmentation.break_inside.avoids_column() {
            around.violates |= RULE_2;
        }
        if c.fragmentation.box_decoration_break == crate::layout::BoxDecorationBreak::Clone
            && let Some(e) = dom.node(id).tui_ext()
        {
            let (l, cl) = (e.layout, e.content_layout);
            let top = (cl.y - l.y).clamp(0, i32::from(u16::MAX)) as u16;
            let bottom = (bottom(l) - bottom(cl)).clamp(0, i32::from(u16::MAX)) as u16;
            around.lead = around.lead.saturating_add(top);
            around.tail = around.tail.saturating_add(bottom);
        }
        around
    }
}

/// The breaks inside `container`'s content, `around` what the boxes around
/// it give each.
fn flow(
    dom: &Dom<TuiExt>,
    container: NodeId,
    around: Around,
    out: &mut Vec<Break>,
    floats: &mut Vec<(i32, i32)>,
) {
    let Some(ext) = dom.node(container).tui_ext() else {
        return;
    };
    let content_bottom = bottom(ext.content_layout);
    floats.extend(
        ext.floated_pseudos
            .iter()
            .flat_map(|v| v.iter())
            .map(|a| a.border_box())
            .chain(
                super::super::element_children_of(dom, container)
                    .into_iter()
                    .filter(|&c| super::super::float::float_side(dom, c).is_some())
                    .filter_map(|c| Some(dom.node(c).tui_ext()?.layout)),
            )
            .filter(|r| r.height > 0)
            .map(|r| (r.y, bottom(r))),
    );
    if let Some(il) = ext.inline_layout.as_ref() {
        let origin = crate::render::inline::scrolled_content_rect(dom, container)
            .unwrap_or(ext.content_layout);
        lines(dom, container, il, origin.y, around, out);
        // Class C: below the last line, a gap to the content edge.
        if let Some(last) = il.lines.last() {
            let end = origin.y + i32::from(last.bottom());
            if content_bottom > end {
                out.push(around.at_bc(end, end, 0));
            }
        }
        return;
    }
    let mut items: Vec<Item> = super::super::element_children_of(dom, container)
        .into_iter()
        .filter(|&c| {
            super::super::is_in_flow(dom, c) && super::super::block::is_block_level(dom, c)
        })
        .filter_map(|c| Some(Item::Element(c, dom.node(c).tui_ext()?.layout)))
        .collect();
    items.extend(
        ext.anonymous_blocks
            .iter()
            .enumerate()
            .map(|(i, a)| Item::Anonymous(i, a.border_box())),
    );
    items.sort_by_key(|i| i.rect().y);
    let mut prev: Option<&Item> = None;
    for item in &items {
        if let Some(p) = prev {
            let after = match p {
                Item::Element(id, _) => edge(dom, *id, false),
                Item::Anonymous(..) => Ask::Auto,
            };
            let before = match item {
                Item::Element(id, _) => edge(dom, *id, true),
                Item::Anonymous(..) => Ask::Auto,
            };
            let asked = after.max(before);
            let end = bottom(p.rect());
            out.push(around.at(
                end,
                item.rect().y.max(end),
                asked == Ask::Force,
                if asked == Ask::Avoid { RULE_1 } else { 0 },
            ));
        }
        match item {
            Item::Element(id, _) if fragmentable(dom, *id) => {
                flow(dom, *id, around.into(dom, *id), out, floats);
            }
            Item::Anonymous(i, _) => {
                let anon = &ext.anonymous_blocks[*i];
                if anon.generated.is_none() {
                    lines(
                        dom,
                        container,
                        &anon.inline_layout,
                        anon.rect.y,
                        around,
                        out,
                    );
                }
            }
            Item::Element(..) => {}
        }
        prev = Some(item);
    }
    // Class C: between the last child's margin edge and the content edge,
    // where a gap separates them; the child's margin is truncated at it
    // (§5.2), as at an unforced break.
    if let Some(last) = items.iter().max_by_key(|i| bottom(i.rect())) {
        let end = bottom(last.rect());
        let margin = match last {
            Item::Element(id, _) => computed(dom, *id).map_or(0, |c| {
                i32::from(c.margin.bottom.resolve(ext.content_layout.width)).max(0)
            }),
            Item::Anonymous(..) => 0,
        };
        if content_bottom > end + margin {
            out.push(around.at_bc(end, end + margin, 0));
        }
    }
}

/// The class B breaks between the lines of `il`, whose rows count from
/// `top`, in the block container `block` (whose `orphans` / `widows`
/// apply, §3.3).
fn lines(
    dom: &Dom<TuiExt>,
    block: NodeId,
    il: &InlineLayout,
    top: i32,
    around: Around,
    out: &mut Vec<Break>,
) {
    let (orphans, widows) = computed(dom, block).map_or((2, 2), |c| {
        (c.fragmentation.orphans, c.fragmentation.widows)
    });
    let n = il.lines.len();
    for k in 1..n {
        let (before, after) = (k as u32, (n - k) as u32);
        let short = before < orphans || after < widows;
        out.push(around.at_bc(
            top + i32::from(il.lines[k - 1].bottom()),
            top + i32::from(il.lines[k].top),
            if short { RULE_3 } else { 0 },
        ));
    }
}

/// What `id`'s `break-before` (`before`) or `break-after` asks at its edge,
/// a value on its first (last) in-flow block child propagated up (§3.1):
/// the strongest of them.
fn edge(dom: &Dom<TuiExt>, id: NodeId, before: bool) -> Ask {
    let mut asked = Ask::Auto;
    let mut cur = Some(id);
    while let Some(id) = cur {
        let Some(c) = computed(dom, id) else {
            break;
        };
        let v = if before {
            c.fragmentation.break_before
        } else {
            c.fragmentation.break_after
        };
        asked = asked.max(ask(v));
        cur = if fragmentable(dom, id)
            && dom
                .node(id)
                .tui_ext()
                .is_some_and(|e| e.inline_layout.is_none())
        {
            let children = super::super::element_children_of(dom, id)
                .into_iter()
                .filter(|&c| {
                    super::super::is_in_flow(dom, c) && super::super::block::is_block_level(dom, c)
                });
            if before {
                children.min_by_key(|&c| dom.node(c).tui_ext().map_or(0, |e| e.layout.y))
            } else {
                children.max_by_key(|&c| dom.node(c).tui_ext().map_or(0, |e| bottom(e.layout)))
            }
        } else {
            None
        };
    }
    asked
}

/// Whether a break can fall inside `id` (§4.1): a block container in
/// block flow that is not monolithic — not a scroll container, not
/// size-contained, not a multi-column container of its own (rdom does not
/// fragment nested fragmentation contexts), not a line-clamp container and
/// not a float (kept whole).
/// Flex, grid and table boxes and atomic inlines are monolithic in rdom
/// (DIVERGENCES).
pub(in crate::render::layout_pass) fn fragmentable(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    let Some(c) = computed(dom, id) else {
        return false;
    };
    c.flow.is_block_flow()
        && c.display == crate::layout::Display::Block
        && super::super::float::float_side(dom, id).is_none()
        && !c.is_scroll_container()
        && !c.is_multicol_container()
        && !c.line_clamp_container
        && !super::super::containment::contains(dom, id, c, crate::layout::Direction::Column)
}
