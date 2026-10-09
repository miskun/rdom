//! The possible breaks of a laid-out flow (CSS Fragmentation 3 §4.1):
//! class A between block-level siblings, class B between line boxes, each
//! with the §4.4 rules it breaks and whether `break-before` /
//! `break-after` force it (§3.1, propagated from a first or last child,
//! §3.1's "propagates to its parent"). Monolithic boxes (§4.1) are not
//! looked into.

use rdom_core::{Dom, NodeId};

use super::{Break, RULE_1, RULE_2, RULE_3};
use crate::ext::TuiExt;
use crate::layout::{BreakBetween, LayoutRect};
use crate::node::TuiNodeExt;
use crate::render::inline::InlineLayout;
use crate::style::ComputedStyle;

/// The breaks of `root`'s laid-out content, by the row they end at.
pub(in crate::render::layout_pass) fn collect(dom: &Dom<TuiExt>, root: NodeId) -> Vec<Break> {
    let mut out = Vec::new();
    flow(dom, root, 0, &mut out);
    out.sort_by_key(|b| (b.end, b.resume));
    out
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

/// The breaks inside `container`'s content, with `inside` the §4.4 rules a
/// break anywhere in it breaks already (a `break-inside: avoid` around).
fn flow(dom: &Dom<TuiExt>, container: NodeId, inside: u8, out: &mut Vec<Break>) {
    let Some(ext) = dom.node(container).tui_ext() else {
        return;
    };
    if let Some(il) = ext.inline_layout.as_ref() {
        let origin = crate::render::inline::scrolled_content_rect(dom, container)
            .unwrap_or(ext.content_layout);
        lines(dom, container, il, origin.y, inside, out);
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
            out.push(Break {
                end,
                resume: item.rect().y.max(end),
                forced: asked == Ask::Force,
                violates: inside | if asked == Ask::Avoid { RULE_1 } else { 0 },
            });
        }
        match item {
            Item::Element(id, _) if fragmentable(dom, *id) => {
                let avoid = computed(dom, *id)
                    .is_some_and(|c| c.fragmentation.break_inside.avoids_column());
                flow(dom, *id, inside | if avoid { RULE_2 } else { 0 }, out);
            }
            Item::Anonymous(i, _) => {
                let anon = &ext.anonymous_blocks[*i];
                if anon.generated.is_none() {
                    lines(
                        dom,
                        container,
                        &anon.inline_layout,
                        anon.rect.y,
                        inside,
                        out,
                    );
                }
            }
            Item::Element(..) => {}
        }
        prev = Some(item);
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
    inside: u8,
    out: &mut Vec<Break>,
) {
    let (orphans, widows) = computed(dom, block).map_or((2, 2), |c| {
        (c.fragmentation.orphans, c.fragmentation.widows)
    });
    let n = il.lines.len();
    for k in 1..n {
        let (before, after) = (k as u32, (n - k) as u32);
        let short = before < orphans || after < widows;
        out.push(Break {
            end: top + i32::from(il.lines[k - 1].bottom()),
            resume: top + i32::from(il.lines[k].top),
            forced: false,
            violates: inside | if short { RULE_3 } else { 0 },
        });
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
/// fragment nested fragmentation contexts) and not a line-clamp container.
/// Flex, grid and table boxes and atomic inlines are monolithic in rdom
/// (DIVERGENCES).
pub(in crate::render::layout_pass) fn fragmentable(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    let Some(c) = computed(dom, id) else {
        return false;
    };
    c.flow.is_block_flow()
        && c.display == crate::layout::Display::Block
        && !c.is_scroll_container()
        && !c.is_multicol_container()
        && !c.line_clamp_container
        && !super::super::containment::contains(dom, id, c, crate::layout::Direction::Column)
}
