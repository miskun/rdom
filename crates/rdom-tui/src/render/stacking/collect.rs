//! [`collect_layers`]: one walk of a stacking context, stopping at nested
//! contexts, that gathers its layers (CSS 2.1 Appendix E steps 2, 8 and
//! 9) and — for its root's paint unit and each `z-index: auto` positioned
//! box's — the in-flow block-level boxes of the background phase (step 4)
//! and the floats (step 5). A float's and an atomic box's own phases are
//! gathered when they paint (`unit`); this walk still descends into them
//! for the positioned descendants, which belong to the context.

use rdom_core::{Dom, NodeId, NodeType};

use super::{
    BoxEntry, Generated, LayerEntry, Layers, children_clip, creates_stacking_context, is_float,
    is_layered, is_positioned, is_z_indexed_item, paints_atomically,
};
use crate::ext::{PseudoSlot, TuiExt};
use crate::layout::{Display, Position, ZIndex};
use crate::render::Rect;

/// An ancestor on the walk from the context root down: whether it is
/// positioned (a containing-block candidate), the clip its content paints
/// into and the clip it paints in (its captions', for a table —
/// `child_clip`).
#[derive(Clone, Copy)]
struct Frame {
    positioned: bool,
    /// It contains `fixed` descendants (layout or paint containment, a
    /// `will-change` naming one; `containment::contains_positioned`).
    contains_fixed: bool,
    content_clip: Rect,
    own_clip: Rect,
}

/// Gather the positioned descendants that belong to the stacking
/// context rooted at `root`, and its units' background-phase boxes and
/// floats. `content_clip` is the clip the root's content paints into,
/// `own_clip` the clip it paints in;
/// `viewport` the document's clip (the clip of `position: fixed`
/// descendants).
///
/// An `absolute` descendant whose containing block lies above `root`
/// takes `content_clip`: exact for the document root (its containing
/// block is the viewport) and the nearest available clip for an
/// `opacity` context, whose ancestors this walk does not see.
pub(crate) fn collect_layers(
    dom: &Dom<TuiExt>,
    root: NodeId,
    (content_clip, own_clip): (Rect, Rect),
    viewport: Rect,
) -> Layers {
    let mut layers = Layers::default();
    let root_style = dom.node(root).ext().and_then(|e| e.computed.as_ref());
    let chain = vec![Frame {
        positioned: root_style.is_some_and(|c| contains(c, false)),
        contains_fixed: root_style.is_some_and(|c| contains(c, true)),
        content_clip,
        own_clip,
    }];
    let mut walk = Walk {
        dom,
        viewport,
        chain,
        layers: &mut layers,
        order: 0,
    };
    walk.children(root, root, Some(0));
    layers.negative.sort_by_key(|e| (e.z, e.order));
    layers.positive.sort_by_key(|e| (e.z, e.order));
    // Stable: tree order within each unit.
    layers.boxes.sort_by_key(|s| s.unit);
    layers
}

/// The state of [`collect_layers`]' walk.
struct Walk<'a> {
    dom: &'a Dom<TuiExt>,
    viewport: Rect,
    chain: Vec<Frame>,
    layers: &'a mut Layers,
    order: usize,
}

impl Walk<'_> {
    /// The clip the current box's content paints into.
    fn content_clip(&self) -> Rect {
        self.chain
            .last()
            .expect("the context root frame is always present")
            .content_clip
    }

    /// Walk the children of `id`; `box_parent` is the element whose
    /// content paint reaches them (`id`, or the element above a
    /// fragment), `unit` the paint unit they belong to — `None` inside a
    /// float or an atomic box, whose own paint gathers its phases
    /// (`unit::for_each_unit_box`).
    fn children(&mut self, id: NodeId, box_parent: NodeId, unit: Option<usize>) {
        let dom = self.dom;
        if id == box_parent {
            self.generated(id, unit, PseudoSlot::Before);
        }
        // A flex container's items in order-modified document order (CSS
        // Flexbox §5.4: `order` affects painting).
        let kids = if id == box_parent {
            crate::render::box_tree::paint_order_children(dom, id)
        } else {
            crate::render::box_tree::PaintOrder::tree(dom, id)
        };
        for cid in kids {
            super::visit();
            let child = dom.node(cid);
            match child.node_type() {
                // A box-less element's children are its parent box's
                // (CSS Display 3 §2.5), as a fragment's are.
                NodeType::Fragment => {
                    self.children(cid, box_parent, unit);
                    continue;
                }
                NodeType::Element if crate::render::box_tree::is_contents(dom, cid) => {
                    self.children(cid, box_parent, unit);
                    continue;
                }
                NodeType::Element => {}
                _ => continue,
            }
            // A top-layer element and its subtree render in the top
            // layer, after the whole document (CSS Position 4,
            // `paint_pass::top_layer`), not in any stacking context.
            if dom.is_in_top_layer(cid) {
                continue;
            }
            let frame = *self.chain.last().expect("the context root frame");
            let current =
                super::child_clip(dom, box_parent, cid, frame.content_clip, frame.own_clip);
            let Some(c) = child.ext().and_then(|e| e.computed.as_ref()) else {
                // Not cascaded: an in-flow box with nothing to clip.
                self.background(cid, box_parent, unit, current);
                self.chain.push(Frame {
                    positioned: false,
                    contains_fixed: false,
                    content_clip: current,
                    own_clip: current,
                });
                self.children(cid, cid, unit);
                self.chain.pop();
                continue;
            };
            if c.display == Display::None {
                continue;
            }
            if !is_positioned(c) && !is_z_indexed_item(dom, box_parent, c) && is_float(dom, cid) {
                // A float paints atomically in its unit's step 5; inside a
                // float or an atomic box (`unit` `None`) that unit's own
                // paint finds it. Its positioned descendants belong to
                // this context; its in-flow boxes and floats to its own
                // phases.
                let context = creates_stacking_context(dom, box_parent, c);
                if let Some(owner) = unit {
                    self.layers.floats.push(LayerEntry {
                        id: cid,
                        z: 0,
                        order: self.order,
                        context,
                        clip: current,
                        generated: None,
                        owner,
                    });
                    self.order += 1;
                }
                if !context {
                    self.chain.push(Frame {
                        positioned: contains(c, false),
                        contains_fixed: contains(c, true),
                        content_clip: children_clip(dom, cid, c, current),
                        own_clip: current,
                    });
                    self.children(cid, cid, None);
                    self.chain.pop();
                }
            } else if is_layered(dom, cid, box_parent, c) {
                let clip = self.positioned_clip(c.position, current);
                let context = creates_stacking_context(dom, box_parent, c);
                let entry = self.layer(cid, c, context, clip, None);
                if !context {
                    // `z-index: auto`: its positioned descendants belong
                    // to this context, clipped by its own content clip;
                    // its in-flow boxes and floats to its own paint unit.
                    self.chain.push(Frame {
                        positioned: true,
                        contains_fixed: contains(c, true),
                        content_clip: children_clip(dom, cid, c, clip),
                        own_clip: clip,
                    });
                    self.children(cid, cid, Some(entry.unit()));
                    self.chain.pop();
                }
            } else if creates_stacking_context(dom, box_parent, c) {
                // `opacity < 1` on an in-flow box: painted atomically in
                // place; nothing inside it belongs to this context.
            } else {
                // An atomic box paints whole at its turn, its own phases
                // gathered then; only its positioned descendants are this
                // context's.
                let atomic = paints_atomically(dom, box_parent, c);
                if !atomic {
                    self.background(cid, box_parent, unit, current);
                }
                self.chain.push(Frame {
                    positioned: contains(c, false),
                    contains_fixed: contains(c, true),
                    content_clip: children_clip(dom, cid, c, current),
                    own_clip: current,
                });
                self.children(cid, cid, if atomic { None } else { unit });
                self.chain.pop();
            }
        }
        if id == box_parent {
            self.generated(id, unit, PseudoSlot::After);
        }
    }

    /// The clip a positioned box styled `position`, met where the
    /// content clip is `current`, paints into (CSS 2.1 §11.1.1): the
    /// content clip at its containing block — for `fixed`, the nearest
    /// box containing fixed boxes (CSS Containment 2 §3.2, §3.4), else
    /// the viewport — and `current` for an in-flow position.
    fn positioned_clip(&self, position: Position, current: Rect) -> Rect {
        match position {
            Position::Fixed => self
                .chain
                .iter()
                .rev()
                .find(|f| f.contains_fixed)
                .map_or(self.viewport, |f| f.content_clip),
            Position::Absolute => self
                .chain
                .iter()
                .rev()
                .find(|f| f.positioned)
                .map_or(self.chain[0].content_clip, |f| f.content_clip),
            Position::Relative | Position::Sticky | Position::Static => current,
        }
    }

    /// Put the positioned box `id` styled `c` — or its `generated` box —
    /// on its layer by its `z-index` (Appendix E steps 2, 8, 9): a child
    /// context (`context`) by its value, a plain box at 0.
    fn layer(
        &mut self,
        id: NodeId,
        c: &crate::style::ComputedStyle,
        context: bool,
        clip: Rect,
        generated: Option<Generated>,
    ) -> LayerEntry {
        let z = match c.z_index {
            ZIndex::Auto => 0,
            ZIndex::Value(n) => n,
        };
        let entry = LayerEntry {
            id,
            z,
            order: self.order,
            context,
            clip,
            generated,
            owner: 0,
        };
        self.order += 1;
        match z {
            _ if !context => self.layers.zero_auto.push(entry),
            z if z < 0 => self.layers.negative.push(entry),
            0 => self.layers.zero_auto.push(entry),
            _ => self.layers.positive.push(entry),
        }
        entry
    }

    /// `id`'s absolutely or fixed positioned `slot` pseudo-element, a
    /// positioned child of `id` (CSS Pseudo 4 §4): on its layer, ahead of
    /// `id`'s children for `::before`, after them for `::after`. It is a
    /// stacking context of its own by its own `z-index` or `opacity`, as
    /// an element is; it holds no descendants.
    fn positioned_pseudo(&mut self, id: NodeId, slot: PseudoSlot) {
        let dom = self.dom;
        let Some(ext) = dom.node(id).ext() else {
            return;
        };
        for (k, anon) in ext.positioned_pseudo_boxes().iter().enumerate() {
            let Some(g) = anon.generated.filter(|g| g.slot == slot) else {
                continue;
            };
            let Some(c) = ext.computed_pseudo(g.slot) else {
                continue;
            };
            let clip = self.positioned_clip(c.position, self.content_clip());
            let context = !matches!(c.z_index, ZIndex::Auto) || c.opacity < 1.0;
            self.layer(id, c, context, clip, Some(Generated::Positioned(k)));
        }
    }

    /// The in-flow, non-atomic element `id` (reached through
    /// `box_parent`'s content paint) in its unit's background phase, when
    /// that content paint draws it as a box.
    fn background(&mut self, id: NodeId, box_parent: NodeId, unit: Option<usize>, clip: Rect) {
        if let Some(unit) = unit
            && crate::render::paint_pass::paints_child_box(self.dom, box_parent, id)
        {
            self.layers.boxes.push(BoxEntry {
                id,
                generated: None,
                clip,
                unit,
            });
        }
    }

    /// `id`'s generated boxes on the `slot` side of its children: its
    /// block-level `::before` (`::after`) in the background phase of
    /// `unit`, and the floated pseudo-elements its formatting context run
    /// placed — its own `::before` ahead of its children's floats, the
    /// rest after them — on the float layer of `unit`. Inside a float or
    /// an atomic box (`unit` `None`) its own paint finds them.
    fn generated(&mut self, id: NodeId, unit: Option<usize>, slot: PseudoSlot) {
        // A positioned pseudo-element belongs to the context whatever
        // unit paints its host.
        if slot == PseudoSlot::Before {
            self.positioned_pseudo(id, slot);
        }
        self.unit_generated(id, unit, slot);
        if slot == PseudoSlot::After {
            self.positioned_pseudo(id, slot);
        }
    }

    /// [`Self::generated`]'s in-flow boxes: block-level and floated.
    fn unit_generated(&mut self, id: NodeId, unit: Option<usize>, slot: PseudoSlot) {
        let Some(unit) = unit else {
            return;
        };
        let Some(ext) = self.dom.node(id).ext() else {
            return;
        };
        let clip = self.content_clip();
        let leading = slot == PseudoSlot::Before;
        // A flex or grid container's pseudo-element boxes are items,
        // painted atomically with its content.
        if !crate::render::box_tree::is_flex_or_grid_container(self.dom, id) {
            for (k, anon) in ext.anonymous_blocks.iter().enumerate() {
                if anon.generated.is_some_and(|g| g.slot == slot) {
                    self.layers.boxes.push(BoxEntry {
                        id,
                        generated: Some(k),
                        clip,
                        unit,
                    });
                }
            }
        }
        for (k, g) in ext.floated_pseudos().iter().enumerate() {
            let own_before = g
                .generated
                .is_some_and(|g| g.host == id && g.slot == PseudoSlot::Before);
            if own_before != leading {
                continue;
            }
            self.layers.floats.push(LayerEntry {
                id,
                z: 0,
                order: self.order,
                context: false,
                clip,
                generated: Some(Generated::Floated(k)),
                owner: unit,
            });
            self.order += 1;
        }
    }
}

/// Whether a box styled `c` contains absolutely positioned descendants
/// (`fixed`: fixed ones): a non-static `position` (absolute only) or
/// containment (`containment::contains_positioned`).
fn contains(c: &crate::style::ComputedStyle, fixed: bool) -> bool {
    (!fixed && is_positioned(c)) || crate::style::containment::contains_positioned(c, fixed)
}
