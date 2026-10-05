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

    /// Whether the breadth is in the grammar: a percentage
    /// `<percentage [0,∞]>` and an `fr` `<flex [0,∞]>` (§7.2.1), so
    /// neither negative, NaN nor infinite.
    pub fn is_valid(&self) -> bool {
        match self {
            TrackBreadth::Percent(v) | TrackBreadth::Fr(v) => v.is_finite() && *v >= 0.0,
            _ => true,
        }
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

    /// The one-track list `auto`: the initial value of `grid-auto-columns`
    /// / `grid-auto-rows` (CSS Grid 2 §7.6), shared — a
    /// [`ComputedStyle`](crate::ComputedStyle) borrows it, so building one
    /// allocates nothing for grid.
    pub const AUTO_LIST: &'static [Self] = &[Self::AUTO];

    /// A track `n` cells wide.
    pub fn cells(n: u16) -> Self {
        TrackSize::Breadth(TrackBreadth::Cells(n))
    }

    /// A track of `factor` fr (`1fr`). `<flex>` is `[0,∞]` (§7.2.4): a
    /// negative or NaN factor is 0 and an infinite one the largest finite
    /// factor, as for `flex-grow` ([`valid_flex_factor`](super::valid_flex_factor)).
    pub fn fr(factor: f32) -> Self {
        TrackSize::Breadth(TrackBreadth::Fr(super::sizing::valid_flex_factor(factor)))
    }

    /// A track `p` percent of the grid container's content box, kept in
    /// `<percentage [0,∞]>` (§7.2.1) as [`fr`](Self::fr) keeps its factor.
    pub fn percent(p: f32) -> Self {
        TrackSize::Breadth(TrackBreadth::Percent(super::sizing::valid_flex_factor(p)))
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

    /// Whether the size is in the grammar: every breadth valid
    /// ([`TrackBreadth::is_valid`]), no `fr` minimum, and a
    /// `fit-content()` limit that is a length or percentage.
    pub fn is_valid(&self) -> bool {
        match self {
            TrackSize::Breadth(b) => b.is_valid(),
            TrackSize::MinMax(min, max) => min.flex().is_none() && min.is_valid() && max.is_valid(),
            TrackSize::FitContent(limit) => limit.is_fixed() && limit.is_valid(),
        }
    }

    /// Whether `sizes` is a `<track-size>+` (`grid-auto-columns` /
    /// `grid-auto-rows`, §7.6): at least one size, each valid.
    pub fn is_valid_list(sizes: &[Self]) -> bool {
        !sizes.is_empty() && sizes.iter().all(Self::is_valid)
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
    /// `repeat(count, sizes…)` with unnamed lines. The count is
    /// `<integer [1,∞]>` (§7.2.3): `RepeatCount::Count(0)` is 1.
    pub fn new(count: RepeatCount, sizes: impl IntoIterator<Item = TrackSize>) -> Self {
        let sizes: Vec<TrackSize> = sizes.into_iter().collect();
        let count = match count {
            RepeatCount::Count(0) => RepeatCount::Count(1),
            other => other,
        };
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

/// One component of a subgrid's `<line-name-list>` (CSS Grid 2 §9):
/// the names of one line, or a `<name-repeat>` of several lines' names.
#[derive(Debug, Clone, PartialEq)]
pub enum LineNameItem {
    /// `[ <custom-ident>* ]`: one line's names (possibly none, `[]`).
    Names(Vec<String>),
    /// `repeat( <integer [1,∞]> | auto-fill , <line-names>+ )`: the
    /// lines' names `count` times — for `auto-fill`, as many times as the
    /// subgrid's span has lines left for (§9). `count` is never
    /// `auto-fit`.
    Repeat {
        count: RepeatCount,
        names: Vec<Vec<String>>,
    },
}

/// `<line-name-list>` (CSS Grid 2 §9): the names a subgrid gives its
/// lines, in order from its first line.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LineNameList {
    pub items: Vec<LineNameItem>,
}

impl LineNameList {
    /// Whether the list is in the grammar: every repetition non-empty
    /// and counted (`auto-fill`, or at least once — never `auto-fit`),
    /// and at most one `auto-fill`.
    pub fn is_valid(&self) -> bool {
        let mut auto_fill = 0;
        self.items.iter().all(|item| match item {
            LineNameItem::Names(_) => true,
            LineNameItem::Repeat { count, names } => {
                if *count == RepeatCount::AutoFill {
                    auto_fill += 1;
                }
                !names.is_empty() && !matches!(count, RepeatCount::Count(0) | RepeatCount::AutoFit)
            }
        }) && auto_fill <= 1
    }

    /// How many lines the list names without its `auto-fill`
    /// repetition: what an auto-placed subgrid spans one fewer tracks
    /// than (§9).
    pub fn explicit_lines(&self) -> usize {
        self.items
            .iter()
            .map(|item| match item {
                LineNameItem::Names(_) => 1,
                LineNameItem::Repeat {
                    count: RepeatCount::Count(n),
                    names,
                } => (*n as usize).saturating_mul(names.len()),
                LineNameItem::Repeat { .. } => 0,
            })
            .fold(0usize, usize::saturating_add)
    }

    /// The names of each of `lines` lines, the list written out from the
    /// first line: a `repeat()` expanded, `auto-fill` repeated as many
    /// whole times as the lines the rest of the list leaves allow, names
    /// past the last line dropped.
    pub fn expand(&self, lines: usize) -> Vec<Vec<String>> {
        let fill = lines.saturating_sub(self.explicit_lines());
        let mut out: Vec<Vec<String>> = Vec::with_capacity(lines);
        for item in &self.items {
            match item {
                LineNameItem::Names(n) => out.push(n.clone()),
                LineNameItem::Repeat { count, names } => {
                    let times = match count {
                        RepeatCount::Count(n) => *n as usize,
                        _ => fill / names.len().max(1),
                    };
                    for _ in 0..times {
                        if out.len() >= lines {
                            break;
                        }
                        out.extend(names.iter().cloned());
                    }
                }
            }
            if out.len() >= lines {
                break;
            }
        }
        out.resize(lines, Vec::new());
        out
    }
}

/// `grid-template-columns` / `grid-template-rows` (CSS Grid 2 §7.2, §9).
/// `#[non_exhaustive]`: later grammars add variants; a reader that meets
/// one it does not know has no explicit tracks.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum GridTemplate {
    /// `none`: no explicit grid on the axis (the initial value).
    #[default]
    None,
    /// A track list.
    Tracks(TrackList),
    /// `subgrid <line-name-list>?` (§9): a grid item's grid that takes
    /// its parent grid's tracks on this axis. Where the element is no
    /// subgrid (its parent is not a grid), it is `none`.
    Subgrid(LineNameList),
}

impl GridTemplate {
    /// The explicit track list, if any.
    pub fn tracks(&self) -> Option<&TrackList> {
        match self {
            GridTemplate::Tracks(list) => Some(list),
            GridTemplate::None | GridTemplate::Subgrid(_) => None,
        }
    }

    /// The line names of a `subgrid`, if it is one.
    pub fn subgrid(&self) -> Option<&LineNameList> {
        match self {
            GridTemplate::Subgrid(names) => Some(names),
            _ => None,
        }
    }

    /// Whether the value is in the grammar ([`TrackList::is_valid`],
    /// [`LineNameList::is_valid`]).
    pub fn is_valid(&self) -> bool {
        match self {
            GridTemplate::Tracks(list) => list.is_valid(),
            GridTemplate::Subgrid(names) => names.is_valid(),
            GridTemplate::None => true,
        }
    }
}

/// A list of no tracks is `none` (no explicit grid, §7.2), so a list
/// built from a possibly empty collection is always a value.
impl From<TrackList> for GridTemplate {
    fn from(list: TrackList) -> Self {
        if list.items.is_empty() {
            GridTemplate::None
        } else {
            GridTemplate::Tracks(list)
        }
    }
}

/// An empty `Vec` is `none`, as for [`TrackList`].
impl From<Vec<TrackSize>> for GridTemplate {
    fn from(sizes: Vec<TrackSize>) -> Self {
        TrackList::new(sizes).into()
    }
}
