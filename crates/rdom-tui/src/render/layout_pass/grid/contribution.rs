//! The grid items' contributions on one axis (CSS Grid 2 §11.5, CSS
//! Sizing 3 §5.2): each item's min-content, max-content and minimum
//! contributions — outer sizes, its margins on the axis included —
//! measured on demand, each at most once per sizing run, through the
//! layout pass's intrinsic measurements (whose per-pass memo serves the
//! later runs).

use rdom_core::Dom;

use super::Dimension;
use super::placement::Placed;
use super::sizing::Contributions;
use super::track::{MinFn, Span, TrackGrid};
use crate::ext::TuiExt;
use crate::layout::{MinSize, Size};
use crate::render::layout_pass::items::{Item, Suggestion, content_based_minimum};

/// §6.6: what an item's automatic minimum is on the axis.
#[derive(Debug, Clone, Copy)]
struct AutoMinimum {
    /// It is content-based: the item spans a track whose min track
    /// sizing function is `auto` and, spanning several, no flexible one.
    content_based: bool,
    /// When every spanned track has a fixed max track sizing function,
    /// their sum with the gutters between them — the grid area's maximum
    /// size the suggestion is clamped to.
    cap: Option<u32>,
}

/// The contributions of `placed` along `dimension`.
pub(super) struct Measured<'a> {
    dom: &'a Dom<TuiExt>,
    placed: &'a [Placed],
    dimension: Dimension,
    /// Per item: the extent its content is measured against on the other
    /// axis, and its containing block's width (percentage padding and
    /// margins) — for a row, its grid area's width; for a column, the
    /// container's cross budget and 0 (a cyclic percentage, CSS Sizing 3
    /// §5.2.1).
    budgets: Vec<(u16, u16)>,
    auto_min: Vec<AutoMinimum>,
    cache: Vec<[Option<u32>; 3]>,
}

impl<'a> Measured<'a> {
    /// The contributions of `placed` to `grid`'s tracks (its sizing
    /// functions read, before any is sized), each item measured against
    /// `budgets[i]`.
    pub(super) fn new(
        dom: &'a Dom<TuiExt>,
        placed: &'a [Placed],
        dimension: Dimension,
        grid: &TrackGrid,
        budgets: Vec<(u16, u16)>,
    ) -> Self {
        let auto_min = placed
            .iter()
            .map(|p| {
                let span = dimension.span(p);
                let tracks = &grid.tracks[span.tracks()];
                let flexible = tracks.iter().any(|t| t.flex().is_some());
                AutoMinimum {
                    content_based: tracks.iter().any(|t| t.min == MinFn::Auto)
                        && (span.len() == 1 || !flexible),
                    cap: tracks
                        .iter()
                        .map(|t| match t.max {
                            super::track::MaxFn::Fixed(n) => Some(n),
                            _ => None,
                        })
                        .sum::<Option<u32>>()
                        .map(|sum| sum + grid.inner_gutters(span)),
                }
            })
            .collect();
        Self {
            dom,
            placed,
            dimension,
            budgets,
            auto_min,
            cache: vec![[None; 3]; placed.len()],
        }
    }

    fn item(&self, i: usize) -> &Item {
        &self.placed[i].item
    }

    /// The item's margins on the axis, either sign (`auto` and trimmed
    /// ones are 0, `grid::margins`).
    fn margins(&self, i: usize) -> i32 {
        let m = super::margins(self.dom, &self.placed[i], self.budgets[i].1);
        match self.dimension {
            Dimension::Columns => m.left + m.right,
            Dimension::Rows => m.top + m.bottom,
        }
    }

    /// The item's outer contribution, min- or max-content: its border
    /// box and its margins.
    fn outer(&self, i: usize, max_content: bool) -> u32 {
        let (cross, cb) = self.budgets[i];
        let border_box =
            self.item(i)
                .contribution(self.dom, self.dimension.direction(), cross, cb, max_content);
        with_margins(border_box, self.margins(i))
    }

    /// §11.5 "minimum contribution": the outer size the item's used
    /// minimum size gives when its preferred size behaves as `auto` or
    /// depends on its grid area's size (a percentage) — its `min-*`, or
    /// for `min-*: auto` its automatic minimum (§6.6) — else its
    /// min-content contribution.
    fn measure_minimum(&mut self, i: usize) -> u32 {
        let dom = self.dom;
        let direction = self.dimension.direction();
        let (cross, cb) = self.budgets[i];
        let item = self.item(i).clone();
        let computed = item.computed(dom);
        let (preferred, min, max) = match self.dimension {
            Dimension::Columns => (&computed.width, &computed.min_width, &computed.max_width),
            Dimension::Rows => (&computed.height, &computed.min_height, &computed.max_height),
        };
        let behaves_auto = match preferred {
            Size::Auto | Size::Flex(_) | Size::Percent(_) => true,
            Size::Calc(e) => e.contains_percent(),
            Size::Fixed(_) | Size::Intrinsic(_) => false,
        };
        if !behaves_auto {
            return self.min_content(i);
        }
        let kw = item.keywords(dom, &computed, direction, cross, cb);
        let sizer = kw.sizer();
        let margins = self.margins(i);
        let used = match min {
            MinSize::Auto => {
                let rule = self.auto_min[i];
                if !rule.content_based {
                    sizer.chrome()
                } else {
                    let suggestion = content_based_minimum(
                        dom,
                        &item,
                        direction,
                        Suggestion {
                            basis: None,
                            available: cross,
                            cross_budget: cross,
                            cb_width: cb,
                        },
                    );
                    // Clamped by the definite maximum size, and when every
                    // spanned track's maximum is fixed, by the area's.
                    let limit = kw.max(max, None, cross);
                    let area = rule.cap.map(|c| {
                        (i64::from(c) - i64::from(margins)).clamp(0, i64::from(u16::MAX)) as u16
                    });
                    let capped = [limit, area]
                        .into_iter()
                        .flatten()
                        .fold(suggestion, u16::min);
                    sizer.floor(capped)
                }
            }
            m => sizer.floor(kw.min(m, None, cross).unwrap_or(0)),
        };
        with_margins(used, margins)
    }
}

#[cfg(test)]
thread_local! {
    /// Contributions measured (not served from a run's cache).
    pub(super) static MEASURES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Count a contribution measured (tests only).
fn measured() {
    #[cfg(test)]
    MEASURES.with(|c| c.set(c.get() + 1));
}

impl Contributions for Measured<'_> {
    fn min_content(&mut self, i: usize) -> u32 {
        if let Some(v) = self.cache[i][0] {
            return v;
        }
        measured();
        let v = self.outer(i, false);
        self.cache[i][0] = Some(v);
        v
    }

    fn max_content(&mut self, i: usize) -> u32 {
        if let Some(v) = self.cache[i][1] {
            return v;
        }
        measured();
        let v = self.outer(i, true);
        self.cache[i][1] = Some(v);
        v
    }

    fn minimum(&mut self, i: usize) -> u32 {
        if let Some(v) = self.cache[i][2] {
            return v;
        }
        measured();
        let v = self.measure_minimum(i);
        self.cache[i][2] = Some(v);
        v
    }
}

/// A border box plus its margins, at least 0.
fn with_margins(border_box: u16, margins: i32) -> u32 {
    u32::try_from((i32::from(border_box) + margins).max(0)).unwrap_or(0)
}

/// The spans of `placed` along `dimension`.
pub(super) fn spans(placed: &[Placed], dimension: Dimension) -> Vec<Span> {
    placed.iter().map(|p| dimension.span(p)).collect()
}
