//! Intrinsic block sizes of block containers (CSS Sizing 3 §5.2: a box's
//! content contribution is the size its content takes laid out under the
//! constraint): the block flow run (`flow::run`) with each piece
//! *measured* — against a scratch exclusion area, the measured box the
//! root of its own formatting context (DIVERGENCES §2) — through the same
//! placement layout uses (`place`, `generated`): margins collapsing
//! (§8.3.1), clearance and formatting context roots beside floats (§9.5),
//! `box-sizing` and `min-*` / `max-*` (`resolve_block_height`,
//! `auto_height::used_content_height`), and a line-clamp container's
//! height ending at its clamp point (CSS Overflow 4 §4.4), its block
//! descendants' lines counted.
//!
//! A block child's height is its own measurement (`resolve_block_height`,
//! memoized per pass) unless floats are in the area or a clamp is
//! counting lines — then its content is measured here, in this area, its
//! lines beside the same floats, as layout lays a child that is no
//! formatting context root out in its parent's.

use rdom_core::{Dom, NodeId};

use super::flow::{self, FlowSink};
use super::generated::GeneratedPlace;
use super::inline_run::RunPlace;
use super::place::{self, BlockPlace, ChildPlaced};
use super::runs::Run;
use crate::ext::{PseudoSlot, TuiExt};
use crate::layout::{LayoutRect, Size};
use crate::render::box_tree::BoxItem;
use crate::render::inline::{InlineLayout, RunPseudos};
use crate::render::layout_pass::box_sizing::Sizer;
use crate::render::layout_pass::dispatch::ChildrenLayout;
use crate::render::layout_pass::float::lines::InlineFloats;
use crate::render::layout_pass::float::{ExclusionArea, Placement};
use crate::style::ComputedStyle;

/// The content height of the block container `id` (styled `computed`)
/// whose content box is `width` cells wide: its flow measured as laid
/// out, reaching its lowest float (CSS 2.1 §10.6.7).
pub(in crate::render::layout_pass) fn content_height(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    width: u16,
) -> u16 {
    let mut area = ExclusionArea::default();
    let mut clamps = Vec::new();
    let content = LayoutRect::new(0, 0, width, 0);
    let height = measure_children(dom, &mut area, &mut clamps, None, id, computed, content);
    let floats = area
        .lowest()
        .map_or(0, |b| b.clamp(0, i32::from(u16::MAX)) as u16);
    height.max(floats)
}

/// The rows of the first and last in-flow line boxes' text in the block
/// container `id` (styled `computed`), its content box `width` cells wide,
/// counted from its content top: its flow measured as for
/// [`content_height`], each line box recorded where it is packed — a
/// block-level child's own (`baselines::content_rows`) where layout lays
/// its lines out in its own context, else beside this flow's floats.
/// `None` with no line box (CSS 2.1 §10.8.1, CSS Box Alignment 3 §9.1).
pub(in crate::render::layout_pass) fn baselines(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    width: u16,
) -> Option<(i32, i32)> {
    let mut area = ExclusionArea::default();
    let mut clamps = Vec::new();
    let mut lines = LineRows::default();
    let content = LayoutRect::new(0, 0, width, 0);
    measure_children(
        dom,
        &mut area,
        &mut clamps,
        Some(&mut lines),
        id,
        computed,
        content,
    );
    Some((lines.first?, lines.last?))
}

/// The text rows of the line boxes a measurement met, in block order: the
/// first and the last.
#[derive(Debug, Default, Clone, Copy)]
struct LineRows {
    first: Option<i32>,
    last: Option<i32>,
}

impl LineRows {
    /// Record a box whose first and last baselines are `rows`, from `top`.
    fn rows(&mut self, rows: Option<(u16, u16)>, top: i32) {
        if let Some((first, last)) = rows {
            self.first.get_or_insert(top + i32::from(first));
            self.last = Some(top + i32::from(last));
        }
    }
}

/// A line-clamp container's count of the line boxes in its formatting
/// context, in block order (CSS Overflow 4 §4): the row after its Nth,
/// and whether content follows it.
struct Clamp {
    remaining: usize,
    bottom: Option<i32>,
    more: bool,
}

impl Clamp {
    /// Count a line box ending at row `bottom`.
    fn line(&mut self, bottom: i32) {
        if self.remaining > 0 {
            self.remaining -= 1;
            if self.remaining == 0 {
                self.bottom = Some(bottom);
            }
        } else if self.bottom.is_some() {
            self.more = true;
        }
    }
}

/// Count the lines of `il`, whose rows start at row `top`, for every
/// clamp counting.
fn count_lines(clamps: &mut [Clamp], il: &InlineLayout, top: i32) {
    for line in &il.lines {
        for c in clamps.iter_mut() {
            c.line(top + i32::from(line.bottom()));
        }
    }
}

/// The content height of `id`'s children laid out in `content` (its
/// content box; its height unused) against `area`, by the formatting
/// context that lays them out (`dispatch::children_layout`): its lines
/// packed beside the floats, or its flow run; a line-clamp container's
/// ends at its clamp point.
fn measure_children(
    dom: &Dom<TuiExt>,
    area: &mut ExclusionArea,
    clamps: &mut Vec<Clamp>,
    mut lines: Option<&mut LineRows>,
    id: NodeId,
    computed: &ComputedStyle,
    content: LayoutRect,
) -> u16 {
    let clamp = computed
        .max_lines
        .filter(|_| computed.line_clamp_container)
        .and_then(|n| usize::try_from(n).ok());
    if let Some(n) = clamp {
        clamps.push(Clamp {
            remaining: n,
            bottom: None,
            more: false,
        });
    }
    let height = match crate::render::layout_pass::dispatch::children_layout(dom, id, computed) {
        ChildrenLayout::Inline | ChildrenLayout::TextLeaf => {
            let mut ex = InlineFloats::new(dom, area, content, content.y);
            let il = crate::render::inline::compute_inline_layout_around(
                dom,
                id,
                content.width,
                Some(&mut ex),
            );
            count_lines(clamps, &il, content.y);
            if let Some(lines) = lines.as_deref_mut() {
                lines.rows(il.baselines(), content.y);
            }
            il.height()
        }
        ChildrenLayout::Block => match flow::prepare(dom, id, None) {
            None => 0,
            Some(prepared) => {
                let container = flow::inset(dom, &prepared, computed, content);
                let mut sink = MeasureSink {
                    dom,
                    id,
                    area,
                    clamps,
                    lines,
                };
                let at = flow::FlowAt {
                    container,
                    scroll_x: 0,
                    scroll_y: 0,
                };
                flow::run(&mut sink, id, computed, &prepared, at)
                    .measurement
                    .content_height
            }
        },
        // A flex or grid container is a formatting context root: never
        // measured in its parent's flow (`same_context`).
        ChildrenLayout::Flex | ChildrenLayout::Grid => {
            debug_assert!(false, "a flex or grid container is measured on its own");
            0
        }
    };
    if clamp.is_some()
        && let Some(c) = clamps.pop()
        && let Some(bottom) = c.bottom
        && (c.more || content.y + i32::from(height) > bottom)
    {
        return (bottom - content.y).clamp(0, i32::from(u16::MAX)) as u16;
    }
    height
}

/// Whether the block-level `child` (styled `c`) lays its content out in
/// its parent's formatting context, sized by it: a block container that
/// is no formatting context root, its height `auto`.
fn same_context(dom: &Dom<TuiExt>, child: NodeId, c: &ComputedStyle) -> bool {
    c.flow.is_block_flow()
        && matches!(c.height, Size::Auto | Size::Intrinsic(_))
        && !super::establishes_bfc(dom, child, c)
}

/// [`FlowSink`] for measurement: each piece measured against `area`, the
/// line boxes recorded in `lines` when asked for ([`baselines`]).
struct MeasureSink<'a> {
    dom: &'a Dom<TuiExt>,
    id: NodeId,
    area: &'a mut ExclusionArea,
    clamps: &'a mut Vec<Clamp>,
    lines: Option<&'a mut LineRows>,
}

impl MeasureSink<'_> {
    /// The height of the block-level `child`, `placed` in a containing
    /// block `cb` wide: its own measurement, or — floats in the area, or
    /// a clamp counting lines — its content measured here when it lays
    /// out in this formatting context, its `auto` height that content
    /// clamped by its `min-*` / `max-*` (`used_content_height`) plus its
    /// padding and border.
    fn height_of(&mut self, child: NodeId, placed: &ChildPlaced, cb: u16) -> u16 {
        let c = &placed.computed;
        if (self.area.is_empty() && self.clamps.is_empty()) || !same_context(self.dom, child, c) {
            if let Some(lines) = self.lines.as_deref_mut() {
                let rows = crate::render::layout_pass::baselines::content_rows(
                    self.dom,
                    child,
                    c,
                    placed.rect.width,
                    cb,
                );
                lines.rows(rows, placed.rect.y);
            }
            return placed.rect.height;
        }
        let (h, v) = (Sizer::horizontal(c, cb), Sizer::vertical(c, cb));
        let left = c
            .padding
            .left
            .resolve(cb)
            .saturating_add(c.border.left.cells());
        let top = c
            .padding
            .top
            .resolve(cb)
            .saturating_add(c.border.top.cells());
        let content = LayoutRect::new(
            placed.rect.x + i32::from(left),
            placed.rect.y + i32::from(top),
            placed.rect.width.saturating_sub(h.chrome()),
            0,
        );
        let inner = measure_children(
            self.dom,
            self.area,
            self.clamps,
            self.lines.as_deref_mut(),
            child,
            c,
            content,
        );
        crate::render::layout_pass::auto_height::used_content_height(self.dom, child, c, cb, inner)
            .saturating_add(v.chrome())
    }
}

impl FlowSink for MeasureSink<'_> {
    fn dom(&self) -> &Dom<TuiExt> {
        self.dom
    }

    fn float(&mut self, item: BoxItem, at: Placement) {
        crate::render::layout_pass::float::place(self.dom, self.area, item, at);
    }

    fn block(&mut self, child: NodeId, mut place: BlockPlace<'_>) -> i32 {
        let (y_cursor, cb) = (place.y_cursor, place.containing_block_width);
        let mut placed = place::place_block_child(self.dom, self.area, child, &mut place);
        let before = self.lines.as_deref().copied();
        let mut height = self.height_of(child, &placed, cb);
        // §9.5: a formatting context root beside floats, taller than
        // placed, is placed again at its height (its lines recorded
        // there instead).
        if let Some(rect) =
            place::replace_beside_floats(self.dom, self.area, child, &placed, height)
        {
            placed.rect = rect;
            if let (Some(lines), Some(before)) = (self.lines.as_deref_mut(), before) {
                *lines = before;
            }
            height = self.height_of(child, &placed, cb).max(rect.height);
        }
        place::advance(&placed, placed.rect.y, height, y_cursor, place.margin_acc)
    }

    fn generated(&mut self, host: NodeId, slot: PseudoSlot, at: GeneratedPlace<'_>) -> Option<i32> {
        let cb = at.cb_width;
        let placed = super::generated::place(self.dom, self.area, host, slot, at)?;
        if let Some(lines) = self.lines.as_deref_mut()
            && let Some(item) =
                crate::render::layout_pass::items::AnonymousItem::pseudo(self.dom, host, slot)
        {
            lines.rows(
                item.content_rows(self.dom, placed.rect.width, cb),
                placed.rect.y,
            );
        }
        Some(placed.bottom)
    }

    fn inline_run(&mut self, run: &Run, pseudos: RunPseudos, place: RunPlace) -> u16 {
        let mut ex = InlineFloats::new(self.dom, self.area, place.at, place.content_top);
        let il = crate::render::inline::pack_run(
            self.dom,
            self.id,
            &run.children,
            pseudos,
            place.at.width,
            Some(&mut ex),
        );
        count_lines(self.clamps, &il, place.at.y);
        if let Some(lines) = self.lines.as_deref_mut() {
            lines.rows(il.baselines(), place.at.y);
        }
        il.height()
    }
}
