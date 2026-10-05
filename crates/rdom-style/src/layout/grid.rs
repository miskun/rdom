//! Grid track lists (CSS Grid Layout 2 §7.2): the values of
//! `grid-template-columns` / `grid-template-rows` — [`GridTemplate`], a
//! [`TrackList`] of [`TrackSize`]s, `repeat()`s and line names.
//!
//! The types are the specified grammar, kept as written so CSSOM
//! serializes what the author wrote (`repeat(2, 1fr)` stays a
//! repetition); layout expands the repetitions (rdom-tui's
//! `layout_pass::grid`). A length is whole cells, as everywhere in rdom
//! (DIVERGENCES §1); a percentage or a `calc()` resolves against the grid
//! container's content box at layout time.

use crate::calc::CalcExpr;

/// `<track-breadth>` (CSS Grid 2 §7.2.1): one end of a track's sizing
/// function. `<inflexible-breadth>` is every variant but [`Fr`](Self::Fr);
/// `<fixed-breadth>` is a length or percentage ([`is_fixed`](Self::is_fixed)).
#[derive(Debug, Clone, PartialEq)]
pub enum TrackBreadth {
    /// A length in whole cells.
    Cells(u16),
    /// A `<percentage>` of the grid container's content box on the
    /// track's axis (`12.5%` is `12.5`); `auto` while that size is
    /// indefinite (§7.2.1).
    Percent(f32),
    /// A math function, resolved against the same basis as a percentage.
    Calc(Box<CalcExpr>),
    /// `<flex>` (§7.2.4): a share of the leftover space, `1fr`.
    Fr(f32),
    /// `min-content` — the largest min-content contribution of the
    /// track's items.
    MinContent,
    /// `max-content` — the largest max-content contribution.
    MaxContent,
    /// `auto` — as a minimum, the largest minimum size of the items
    /// (§6.6); as a maximum, `max-content`, stretched by
    /// `align-content` / `justify-content: normal` (§11.8).
    Auto,
}

impl TrackBreadth {
    /// A length or percentage: `<fixed-breadth>` (§7.2.2).
    pub fn is_fixed(&self) -> bool {
        matches!(
            self,
            TrackBreadth::Cells(_) | TrackBreadth::Percent(_) | TrackBreadth::Calc(_)
        )
    }

    /// `min-content`, `max-content` or `auto` (§11.1 "intrinsic sizing
    /// function").
    pub fn is_intrinsic(&self) -> bool {
        matches!(
            self,
            TrackBreadth::MinContent | TrackBreadth::MaxContent | TrackBreadth::Auto
        )
    }

    /// The flex factor of an `fr` breadth.
    pub fn flex(&self) -> Option<f32> {
        match self {
            TrackBreadth::Fr(f) => Some(*f),
            _ => None,
        }
    }

    /// This breadth in cells against `basis` (the grid container's
    /// content size on the axis, `None` while indefinite): a length, or
    /// a percentage / `calc()` against a definite basis. `None` for the
    /// keywords, `fr`, and a percentage against an indefinite basis.
    pub fn cells(&self, basis: Option<u16>) -> Option<u16> {
        match self {
            TrackBreadth::Cells(n) => Some(*n),
            TrackBreadth::Percent(p) => basis.map(|b| super::sizing::percent_cells(b, *p)),
            TrackBreadth::Calc(e) => match basis {
                Some(b) => Some(super::sizing::resolve_u16(e, b)),
                None if e.contains_percent() => None,
                None => Some(super::sizing::resolve_u16(e, 0)),
            },
            _ => None,
        }
    }
}

impl From<u16> for TrackBreadth {
    fn from(n: u16) -> Self {
        TrackBreadth::Cells(n)
    }
}

/// `auto`, the sizing function a lone `fr` or `fit-content()` takes as
/// its minimum.
const AUTO: TrackBreadth = TrackBreadth::Auto;
/// `max-content`, `fit-content()`'s maximum before its limit.
const MAX_CONTENT: TrackBreadth = TrackBreadth::MaxContent;

/// `<track-size>` (CSS Grid 2 §7.2.2): a track's sizing function — a
/// breadth, `minmax(<inflexible-breadth>, <track-breadth>)`, or
/// `fit-content(<length-percentage>)`.
#[derive(Debug, Clone, PartialEq)]
pub enum TrackSize {
    /// One breadth: both its minimum and maximum, except that a lone
    /// `fr` has the minimum `auto` (§7.2.4).
    Breadth(TrackBreadth),
    /// `minmax(min, max)` (§7.2.3.1). The parser rejects an `fr`
    /// minimum; built by hand, one lays out as `auto`.
    MinMax(TrackBreadth, TrackBreadth),
    /// `fit-content(limit)`: `max(minimum, min(limit, max-content))` —
    /// a minimum of `auto` and a maximum of `max-content` capped at the
    /// limit (a length or percentage).
    FitContent(TrackBreadth),
}

impl TrackSize {
    /// `auto`, the initial `grid-auto-columns` / `-rows` (§7.6).
    pub const AUTO: Self = TrackSize::Breadth(TrackBreadth::Auto);

    /// A track `n` cells wide.
    pub fn cells(n: u16) -> Self {
        TrackSize::Breadth(TrackBreadth::Cells(n))
    }

    /// A track of `factor` fr (`1fr`).
    pub fn fr(factor: f32) -> Self {
        TrackSize::Breadth(TrackBreadth::Fr(factor))
    }

    /// A track `p` percent of the grid container's content box.
    pub fn percent(p: f32) -> Self {
        TrackSize::Breadth(TrackBreadth::Percent(p))
    }

    /// `minmax(min, max)`.
    pub fn minmax(min: impl Into<TrackBreadth>, max: impl Into<TrackBreadth>) -> Self {
        TrackSize::MinMax(min.into(), max.into())
    }

    /// `fit-content(<cells>)`.
    pub fn fit_content(cells: u16) -> Self {
        TrackSize::FitContent(TrackBreadth::Cells(cells))
    }

    /// The min track sizing function (§11.1): the breadth, `minmax()`'s
    /// first, `auto` for a lone `fr` (§7.2.4) and for `fit-content()`.
    /// An `fr` minimum, which the grammar forbids, is `auto` too.
    pub fn min_sizing(&self) -> &TrackBreadth {
        let min = match self {
            TrackSize::Breadth(b) | TrackSize::MinMax(b, _) => b,
            TrackSize::FitContent(_) => &AUTO,
        };
        match min {
            TrackBreadth::Fr(_) => &AUTO,
            other => other,
        }
    }

    /// The max track sizing function (§11.1): the breadth, `minmax()`'s
    /// second, `max-content` for `fit-content()` (capped by
    /// [`fit_content_limit`](Self::fit_content_limit)).
    pub fn max_sizing(&self) -> &TrackBreadth {
        match self {
            TrackSize::Breadth(b) | TrackSize::MinMax(_, b) => b,
            TrackSize::FitContent(_) => &MAX_CONTENT,
        }
    }

    /// `fit-content()`'s argument.
    pub fn fit_content_limit(&self) -> Option<&TrackBreadth> {
        match self {
            TrackSize::FitContent(limit) => Some(limit),
            _ => None,
        }
    }

    /// `<fixed-size>` (§7.2.2): a fixed breadth, or a `minmax()` with at
    /// least one fixed end and no flexible minimum — the sizes an
    /// automatic repetition may hold.
    pub fn is_fixed_size(&self) -> bool {
        match self {
            TrackSize::Breadth(b) => b.is_fixed(),
            TrackSize::MinMax(min, max) => {
                (min.is_fixed() || max.is_fixed()) && min.flex().is_none()
            }
            TrackSize::FitContent(_) => false,
        }
    }

    /// Whether the size is in the grammar: no `fr` minimum, and a
    /// `fit-content()` limit that is a length or percentage.
    pub fn is_valid(&self) -> bool {
        match self {
            TrackSize::Breadth(_) => true,
            TrackSize::MinMax(min, _) => min.flex().is_none(),
            TrackSize::FitContent(limit) => limit.is_fixed(),
        }
    }
}

impl From<u16> for TrackSize {
    fn from(n: u16) -> Self {
        TrackSize::cells(n)
    }
}

impl From<TrackBreadth> for TrackSize {
    fn from(b: TrackBreadth) -> Self {
        TrackSize::Breadth(b)
    }
}

/// How many times a `repeat()` repeats (§7.2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatCount {
    /// `repeat(<integer [1,∞]>, …)`.
    Count(u32),
    /// `repeat(auto-fill, …)`: as many repetitions as fit the container
    /// (§7.2.3.2).
    AutoFill,
    /// `repeat(auto-fit, …)`: `auto-fill`, with the repetitions that hold
    /// no item collapsed (§7.2.3.2).
    AutoFit,
}

impl RepeatCount {
    /// `auto-fill` or `auto-fit`.
    pub fn is_auto(self) -> bool {
        !matches!(self, RepeatCount::Count(_))
    }
}

/// `repeat()` (§7.2.3): its tracks and the line names between them —
/// `line_names` has one entry per line, so one more than `sizes`.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackRepeat {
    pub count: RepeatCount,
    /// The names of each line of one repetition: before the first track,
    /// between the tracks, after the last (each empty when unnamed).
    pub line_names: Vec<Vec<String>>,
    pub sizes: Vec<TrackSize>,
}

impl TrackRepeat {
    /// `repeat(count, sizes…)` with unnamed lines.
    pub fn new(count: RepeatCount, sizes: impl IntoIterator<Item = TrackSize>) -> Self {
        let sizes: Vec<TrackSize> = sizes.into_iter().collect();
        Self {
            count,
            line_names: vec![Vec::new(); sizes.len() + 1],
            sizes,
        }
    }
}

/// One component of a track list: a track, or a `repeat()`.
#[derive(Debug, Clone, PartialEq)]
pub enum TrackListItem {
    Size(TrackSize),
    Repeat(TrackRepeat),
}

/// `<track-list>` / `<auto-track-list>` (§7.2): the components and the
/// line names between them — `line_names` has one entry per gap between
/// components and one at each end, so one more than `items`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TrackList {
    /// The names written before the first component, between the
    /// components, and after the last (each empty when none is written).
    pub line_names: Vec<Vec<String>>,
    pub items: Vec<TrackListItem>,
}

impl TrackList {
    /// A list of `sizes` with unnamed lines.
    pub fn new(sizes: impl IntoIterator<Item = TrackSize>) -> Self {
        let items: Vec<TrackListItem> = sizes.into_iter().map(TrackListItem::Size).collect();
        Self {
            line_names: vec![Vec::new(); items.len() + 1],
            items,
        }
    }

    /// Append `size`. Chainable.
    pub fn track(mut self, size: impl Into<TrackSize>) -> Self {
        self.push(TrackListItem::Size(size.into()));
        self
    }

    /// Append `repeat(count, sizes…)`. Chainable.
    pub fn repeat(
        mut self,
        count: RepeatCount,
        sizes: impl IntoIterator<Item = TrackSize>,
    ) -> Self {
        self.push(TrackListItem::Repeat(TrackRepeat::new(count, sizes)));
        self
    }

    /// Name the line after the components so far (`[name]`). Chainable.
    pub fn line(mut self, name: &str) -> Self {
        if self.line_names.is_empty() {
            self.line_names.push(Vec::new());
        }
        if let Some(last) = self.line_names.last_mut() {
            last.push(name.to_string());
        }
        self
    }

    fn push(&mut self, item: TrackListItem) {
        if self.line_names.is_empty() {
            self.line_names.push(Vec::new());
        }
        self.items.push(item);
        self.line_names.push(Vec::new());
    }

    /// Whether the list is in the grammar (§7.2): at least one
    /// component, the line names one per line, every size valid, every
    /// repetition non-empty, and — when one repetition is automatic —
    /// exactly one, with every size of the list fixed (`<auto-track-list>`).
    pub fn is_valid(&self) -> bool {
        let repeats = || {
            self.items.iter().filter_map(|i| match i {
                TrackListItem::Repeat(r) => Some(r),
                TrackListItem::Size(_) => None,
            })
        };
        let sizes = || {
            self.items.iter().flat_map(|i| match i {
                TrackListItem::Size(s) => std::slice::from_ref(s).iter(),
                TrackListItem::Repeat(r) => r.sizes.iter(),
            })
        };
        let autos = repeats().filter(|r| r.count.is_auto()).count();
        !self.items.is_empty()
            && self.line_names.len() == self.items.len() + 1
            && repeats().all(|r| {
                !r.sizes.is_empty()
                    && r.line_names.len() == r.sizes.len() + 1
                    && r.count != RepeatCount::Count(0)
            })
            && sizes().all(TrackSize::is_valid)
            && (autos == 0 || (autos == 1 && sizes().all(TrackSize::is_fixed_size)))
    }
}

/// `grid-template-columns` / `grid-template-rows` (CSS Grid 2 §7.2).
/// `#[non_exhaustive]`: `subgrid` (§9) and later grammars add variants;
/// a reader that meets one it does not know has no explicit tracks.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum GridTemplate {
    /// `none`: no explicit grid on the axis (the initial value).
    #[default]
    None,
    /// A track list.
    Tracks(TrackList),
}

impl GridTemplate {
    /// The explicit track list, if any.
    pub fn tracks(&self) -> Option<&TrackList> {
        match self {
            GridTemplate::Tracks(list) => Some(list),
            GridTemplate::None => None,
        }
    }

    /// Whether the value is in the grammar ([`TrackList::is_valid`]).
    pub fn is_valid(&self) -> bool {
        self.tracks().is_none_or(TrackList::is_valid)
    }
}

impl From<TrackList> for GridTemplate {
    fn from(list: TrackList) -> Self {
        GridTemplate::Tracks(list)
    }
}

impl From<Vec<TrackSize>> for GridTemplate {
    fn from(sizes: Vec<TrackSize>) -> Self {
        GridTemplate::Tracks(TrackList::new(sizes))
    }
}
