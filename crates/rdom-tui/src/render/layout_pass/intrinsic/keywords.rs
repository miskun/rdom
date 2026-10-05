//! Intrinsic size keywords (CSS Sizing 3 §3.1–§3.3) resolved for one
//! box on one axis, and every declared size through one door:
//! [`Keywords`] turns a `width` / `height`, `min-*` or `max-*` into the
//! border box layout stores — a length or percentage through the box's
//! [`Sizer`] (`box-sizing`), a keyword by measuring the content.
//!
//! On the inline axis (`Direction::Row`) `min-content` is the box's
//! min-content size and `max-content` its max-content size (§5.1, the
//! content's own, whatever the box declares); `fit-content` clamps the
//! available space between them and `fit-content(<limit>)` the limit.
//! On the block axis every keyword is "equivalent to [the box's]
//! automatic size" (§3.1): its content height at its width.

use rdom_core::{Dom, NodeId};

use super::{Measure, content_max_size, content_min_size, intrinsic_size_inner};
use crate::ext::TuiExt;
use crate::layout::{Direction, IntrinsicSize, MaxSize, MinSize, Size};
use crate::render::layout_pass::box_sizing::Sizer;
use crate::render::layout_pass::items::AnonymousItem;
use crate::style::ComputedStyle;

/// What a keyword measures: an element's subtree, or the content of an
/// anonymous or generated flex item (`layout_pass::items`), which has no node.
#[derive(Clone, Copy)]
enum Subject<'a> {
    Node(NodeId),
    Run(&'a AnonymousItem),
}

/// One box's sizing on one axis.
pub(crate) struct Keywords<'a> {
    dom: &'a Dom<TuiExt>,
    subject: Subject<'a>,
    direction: Direction,
    /// The perpendicular extent the content is measured against: for
    /// the block axis the box's own (border-box) width, which its text
    /// wraps to; for the inline axis the container's cross size.
    cross_budget: u16,
    /// The containing block's width, the basis of padding percentages.
    cb_width: u16,
    sizer: Sizer,
}

impl<'a> Keywords<'a> {
    pub(crate) fn new(
        dom: &'a Dom<TuiExt>,
        id: NodeId,
        computed: &ComputedStyle,
        direction: Direction,
        cross_budget: u16,
        cb_width: u16,
    ) -> Self {
        Self {
            dom,
            subject: Subject::Node(id),
            direction,
            cross_budget,
            cb_width,
            sizer: Sizer::along(computed, direction, cb_width),
        }
    }

    /// The keywords of an anonymous or generated flex item's box, styled
    /// by its own computed style.
    pub(in crate::render::layout_pass) fn for_run(
        dom: &'a Dom<TuiExt>,
        run: &'a AnonymousItem,
        direction: Direction,
        cross_budget: u16,
        cb_width: u16,
    ) -> Self {
        Self {
            dom,
            subject: Subject::Run(run),
            direction,
            cross_budget,
            cb_width,
            sizer: Sizer::along(run.style(), direction, cb_width),
        }
    }

    /// The box's `box-sizing` conversion on this axis.
    pub(crate) fn sizer(&self) -> Sizer {
        self.sizer
    }

    /// The border box `size` makes: a length or percentage (against
    /// `basis`) through the sizer, a keyword measured with `available`
    /// as its stretch-fit size. `None` for `auto` and a flex weight,
    /// which the caller sizes, and for a percentage against an
    /// indefinite `basis`.
    pub(crate) fn size(&self, size: &Size, basis: Option<u16>, available: u16) -> Option<u16> {
        match size {
            Size::Intrinsic(k) => Some(self.keyword(k, basis, available)),
            other => self.sizer.outer_opt(other.cells(basis)),
        }
    }

    /// A `min-*` floor as a border box; `None` for `auto`.
    pub(crate) fn min(&self, min: &MinSize, basis: Option<u16>, available: u16) -> Option<u16> {
        match min {
            MinSize::Intrinsic(k) => Some(self.keyword(k, basis, available)),
            other => self.sizer.outer_opt(other.cells(basis)),
        }
    }

    /// A `max-*` limit as a border box; `None` for `none`.
    pub(crate) fn max(&self, max: &MaxSize, basis: Option<u16>, available: u16) -> Option<u16> {
        match max {
            MaxSize::Intrinsic(k) => Some(self.keyword(k, basis, available)),
            other => self.sizer.outer_opt(other.cells(basis)),
        }
    }

    /// The border box keyword `k` sizes (CSS Sizing 3 §3.1).
    pub(crate) fn keyword(&self, k: &IntrinsicSize, basis: Option<u16>, available: u16) -> u16 {
        let (cross, cb) = (self.cross_budget, self.cb_width);
        let dom = self.dom;
        let id = match self.subject {
            Subject::Node(id) => id,
            Subject::Run(run) => {
                let measure = |max| run.content_size(dom, self.direction, cross, max, cb);
                if self.direction == Direction::Column {
                    return measure(true);
                }
                return match k {
                    IntrinsicSize::MinContent => measure(false),
                    IntrinsicSize::MaxContent => measure(true),
                    IntrinsicSize::FitContent => measure(true).min(measure(false).max(available)),
                    IntrinsicSize::FitContentLimit(_) => match k.limit_cells(basis) {
                        Some(limit) => {
                            measure(true).min(measure(false).max(self.sizer.outer(limit)))
                        }
                        None => measure(true),
                    },
                };
            }
        };
        if self.direction == Direction::Column {
            return intrinsic_size_inner(
                dom,
                id,
                Direction::Column,
                cross,
                cb,
                super::IntrinsicMode::ContentOnly,
                Measure::MaxContent,
            );
        }
        let min = || content_min_size(dom, id, Direction::Row, cross, cb);
        let max = || content_max_size(dom, id, Direction::Row, cross, cb);
        match k {
            IntrinsicSize::MinContent => min(),
            IntrinsicSize::MaxContent => max(),
            IntrinsicSize::FitContent => max().min(min().max(available)),
            IntrinsicSize::FitContentLimit(_) => match k.limit_cells(basis) {
                // The limit is a size like `width`'s, so `box-sizing`
                // applies to it.
                Some(limit) => max().min(min().max(self.sizer.outer(limit))),
                None => max(),
            },
        }
    }
}
