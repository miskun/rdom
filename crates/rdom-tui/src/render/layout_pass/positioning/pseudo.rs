//! An absolutely or fixed positioned `::before` / `::after` (CSS Pseudo 4
//! §2: rendered "as if it were a real element", its `position` included):
//! a box of its own — `items::AnonymousItem`, the box every other
//! generated box is — placed by phase 2 as a positioned element is
//! (`place::compute_placed_rect`): its containing block from its host up
//! (`containing::absolute_containing_block`; a grid area, a scrolled
//! scroll container), its size by CSS 2.1 §10.3.7 / §10.6.4 with its
//! content's intrinsic sizes, an axis with both insets `auto` at its
//! static position ([`static_position`]). The box is kept on its host
//! (`TuiExt::positioned_pseudos`); paint and hit-testing take it from its
//! stacking context's layers (`stacking::collect`), the scrollable
//! overflow from `positioned_overflow`.

use rdom_core::{Dom, NodeId};

use super::absolute_containing_block;
use super::place::{Placed, compute_placed_rect};
use crate::ext::{PseudoSlot, StaticPosition, TuiExt};
use crate::layout::{Display, LayoutRect};
use crate::node::TuiNodeExt;
use crate::render::inline::InlineLayout;
use crate::render::layout_pass::items::AnonymousItem;
use crate::style::ComputedStyle;

/// Whether `host`'s `slot` pseudo-element is an absolutely or fixed
/// positioned box: it has `content` (CSS 2.1 §12.1), is not `display:
/// none`, and its `position` takes it out of flow (§9.3.1).
pub(in crate::render::layout_pass) fn is_positioned_box(
    dom: &Dom<TuiExt>,
    host: NodeId,
    slot: PseudoSlot,
) -> bool {
    slot != PseudoSlot::Marker
        && dom.node(host).computed_pseudo(slot).is_some_and(|c| {
            crate::render::inline::generated::is_out_of_flow(c)
                && c.display != Display::None
                && c.content.is_some()
        })
}

/// Place `host`'s positioned `slot` pseudo-element against its
/// containing block and lay its content out inside it, keeping the box on
/// `host`.
pub(in crate::render::layout_pass) fn place(
    dom: &mut Dom<TuiExt>,
    host: NodeId,
    slot: PseudoSlot,
    viewport: LayoutRect,
) {
    let Some(item) = AnonymousItem::pseudo(dom, host, slot) else {
        return;
    };
    let style = item.style_rc();
    // A pseudo-element is its host's child (CSS Pseudo 4 §4): the walk
    // for its containing block starts at the host.
    let cb = absolute_containing_block(dom, Some(host), &style, viewport);
    let rect = compute_placed_rect(
        dom,
        Placed::Generated {
            host,
            slot,
            item: &item,
        },
        &style,
        cb,
    );
    let laid_out = item.lay_out(dom, rect, cb.width);
    if let Some(ext) = dom.node_mut(host).ext_mut() {
        ext.positioned_pseudos
            .get_or_insert_with(Default::default)
            .push(laid_out);
    }
}

/// The static position of `host`'s positioned `slot` pseudo-element
/// styled `style` (CSS 2.1 §10.3.7 / §10.6.4): where its box would have
/// been in flow. A `::before` is its host's first child: the start of the
/// host's first line when it is inline-level (where `text-align` and
/// `text-indent` put the line's content), else of the host's content. A `::after` is its last: after the host's content — on
/// its last line's end when the pseudo-element is inline-level, else at
/// the start of the line below. In a flex or grid container both sit at
/// the content box's start, as an absolutely positioned child does. An
/// inline host lays its content out in its block container's lines: its
/// first fragment's start, or its last fragment's end.
pub(super) fn static_position(
    dom: &Dom<TuiExt>,
    host: NodeId,
    slot: PseudoSlot,
    style: &ComputedStyle,
) -> Option<StaticPosition> {
    let inline_level = matches!(style.display, Display::Inline | Display::InlineBlock);
    let node = dom.node(host);
    let hc = node.computed()?;
    if hc.display == Display::Inline {
        return inline_host_position(dom, host, slot);
    }
    let origin = crate::render::inline::scrolled_content_rect(dom, host)?;
    let at = |x: i32, y: i32| Some(StaticPosition { x, y });
    if hc.flow.is_flex_or_grid() {
        return at(origin.x, origin.y);
    }
    let Some(ext) = node.ext() else {
        return at(origin.x, origin.y);
    };
    if slot == PseudoSlot::Before {
        // The host's first inline box: where `text-align` and
        // `text-indent` start its first line's content (C10G-MARKER-HIT).
        let first = ext
            .inline_layout
            .as_ref()
            .map(|il| (il, origin))
            .or_else(|| {
                ext.anonymous_blocks
                    .first()
                    .filter(|a| a.generated.is_none() && a.rect.y == origin.y)
                    .map(|a| (&a.inline_layout, a.rect))
            });
        return first
            .filter(|_| inline_level)
            .and_then(|(il, rect)| first_line_start(il, rect))
            .or_else(|| at(origin.x, origin.y));
    }
    if let Some(il) = ext.inline_layout.as_ref() {
        return after_lines(il, origin, inline_level).or_else(|| at(origin.x, origin.y));
    }
    // A block container's content: its block-level children and the
    // anonymous boxes of its inline runs (CSS 2.1 §9.2.1.1). After the
    // lowest; on the last line of an anonymous box that ends it.
    let mut bottom = origin.y;
    let mut last_lines: Option<(&InlineLayout, LayoutRect)> = None;
    for anon in &ext.anonymous_blocks {
        let b = anon.border_box().bottom();
        if b >= bottom {
            bottom = b;
            last_lines = anon
                .generated
                .is_none()
                .then_some((&anon.inline_layout, anon.rect));
        }
    }
    for child in crate::render::layout_pass::element_children_of(dom, host) {
        if !crate::render::layout_pass::is_in_flow(dom, child) {
            continue;
        }
        if let Some(r) = dom.node(child).layout_rect()
            && r.bottom() > bottom
        {
            bottom = r.bottom();
            last_lines = None;
        }
    }
    if let Some((il, rect)) = last_lines
        && let Some(p) = after_lines(il, rect, inline_level)
    {
        return Some(p);
    }
    at(origin.x, bottom)
}

/// The start of the first line of `il` laid out at `origin`: its first
/// in-line content's cell (an outside marker beside it aside), on its text
/// row. `None` with no line or nothing on it.
fn first_line_start(il: &InlineLayout, origin: LayoutRect) -> Option<StaticPosition> {
    let line = il.lines.first()?;
    let start = line
        .fragments
        .iter()
        .map(|f| f.x)
        .chain(
            line.generated
                .iter()
                .filter(|g| g.outside.is_none())
                .map(|g| g.x),
        )
        .min()?;
    Some(StaticPosition {
        x: origin.x + start,
        y: origin.y + i32::from(line.text_row()),
    })
}

/// After the last line of `il` laid out at `origin`: on its row past its
/// content's end (`inline_level`), else at the start of the row below.
/// `None` with no line.
fn after_lines(
    il: &InlineLayout,
    origin: LayoutRect,
    inline_level: bool,
) -> Option<StaticPosition> {
    let line = il.lines.last()?;
    Some(if inline_level {
        let end = line
            .fragments
            .iter()
            .map(|f| f.x + i32::from(f.width))
            .chain(
                line.generated
                    .iter()
                    .filter(|g| g.outside.is_none())
                    .map(|g| g.x + i32::from(g.width)),
            )
            .max()
            .unwrap_or(0);
        StaticPosition {
            x: origin.x + end,
            y: origin.y + i32::from(line.text_row()),
        }
    } else {
        StaticPosition {
            x: origin.x,
            y: origin.y + i32::from(line.bottom()),
        }
    })
}

/// The static position of a pseudo-element of the inline element `host`,
/// whose content is laid out in the lines of its block container: the
/// start of its first fragment (`::before`) or the end of its last
/// (`::after`), on that fragment's row; the container's content start
/// when the host has none.
fn inline_host_position(
    dom: &Dom<TuiExt>,
    host: NodeId,
    slot: PseudoSlot,
) -> Option<StaticPosition> {
    let mut block = crate::render::box_tree::box_parent(dom, host);
    while let Some(b) = block {
        if dom
            .node(b)
            .computed()
            .is_some_and(|c| c.display != Display::Inline)
        {
            break;
        }
        block = crate::render::box_tree::box_parent(dom, b);
    }
    let block = block?;
    let ext = dom.node(block).ext()?;
    let own = crate::render::inline::scrolled_content_rect(dom, block)?;
    let flows = ext.inline_layout.iter().map(|il| (il, own)).chain(
        ext.anonymous_blocks
            .iter()
            .map(|a| (&a.inline_layout, a.rect)),
    );
    let mut found: Option<StaticPosition> = None;
    for (il, origin) in flows {
        for line in &il.lines {
            for f in line.fragments.iter().filter(|f| within(dom, host, f.node)) {
                let x = match slot {
                    PseudoSlot::After => f.x + i32::from(f.width),
                    _ => f.x,
                };
                let p = StaticPosition {
                    x: origin.x + x,
                    y: origin.y + i32::from(line.text_row()),
                };
                if slot != PseudoSlot::After {
                    return Some(p);
                }
                found = Some(p);
            }
        }
    }
    found.or(Some(StaticPosition { x: own.x, y: own.y }))
}

/// `node` is `host` or one of its descendants.
fn within(dom: &Dom<TuiExt>, host: NodeId, node: NodeId) -> bool {
    let mut cur = Some(node);
    while let Some(n) = cur {
        if n == host {
            return true;
        }
        cur = crate::render::box_tree::slot::parent(dom, n);
    }
    false
}

/// The box of `host`'s positioned `slot` pseudo-element as the last
/// layout placed it (tests).
#[cfg(test)]
fn positioned_box(
    dom: &Dom<TuiExt>,
    host: NodeId,
    slot: PseudoSlot,
) -> Option<&crate::ext::AnonymousIfc> {
    dom.node(host)
        .ext()?
        .positioned_pseudo_boxes()
        .iter()
        .find(|a| a.generated.is_some_and(|g| g.slot == slot))
}

#[cfg(test)]
#[path = "pseudo_tests.rs"]
mod tests;
