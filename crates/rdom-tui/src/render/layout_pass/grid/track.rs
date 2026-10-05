//! One axis's tracks as the track sizing algorithm sees them (CSS Grid 2
//! §11.1, §11.4): each track's min and max track sizing functions,
//! resolved for this layout, its base size and growth limit, and the
//! gutters between the tracks.

use crate::layout::{TrackBreadth, TrackSize};

/// A min track sizing function (§11.1), a length resolved to cells.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum MinFn {
    Fixed(u32),
    MinContent,
    MaxContent,
    Auto,
}

/// A max track sizing function (§11.1). `Auto` sizes like `MaxContent`
/// but is the one §11.8 stretches; `FitContent` is `max-content` capped
/// at its argument (§7.2.2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum MaxFn {
    Fixed(u32),
    MinContent,
    MaxContent,
    Auto,
    Flex(f32),
    FitContent(u32),
}

impl MinFn {
    /// `min-content`, `max-content` or `auto` (§11.1 "intrinsic sizing
    /// function").
    pub(super) fn is_intrinsic(self) -> bool {
        !matches!(self, MinFn::Fixed(_))
    }
}

impl MaxFn {
    /// An intrinsic max track sizing function: `min-content`,
    /// `max-content`, `auto` or `fit-content()`.
    pub(super) fn is_intrinsic(self) -> bool {
        matches!(
            self,
            MaxFn::MinContent | MaxFn::MaxContent | MaxFn::Auto | MaxFn::FitContent(_)
        )
    }

    /// A `max-content` max track sizing function: `max-content`, `auto`,
    /// and `fit-content()` below its argument (§11.5.1).
    pub(super) fn is_max_content(self) -> bool {
        matches!(self, MaxFn::MaxContent | MaxFn::Auto | MaxFn::FitContent(_))
    }

    /// The fixed size this function caps a contribution at (§11.5
    /// "limited contribution"): a length, or `fit-content()`'s argument.
    pub(super) fn fixed_limit(self) -> Option<u32> {
        match self {
            MaxFn::Fixed(n) | MaxFn::FitContent(n) => Some(n),
            _ => None,
        }
    }
}

/// One track being sized.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Track {
    pub(super) min: MinFn,
    pub(super) max: MaxFn,
    /// The base size (§11.4).
    pub(super) base: u32,
    /// The growth limit; `None` is infinite (§11.4).
    pub(super) limit: Option<u32>,
    /// §11.5 step 3.5's "infinitely growable" flag, for the next step.
    pub(super) growable: bool,
}

impl Track {
    /// A track sized by `size`, its lengths and percentages against
    /// `basis` — the grid container's content size on the axis, `None`
    /// while indefinite, which makes a percentage `auto` (§7.2.1).
    /// Initialized per §11.4: a fixed minimum is the base size (else 0),
    /// a fixed maximum the growth limit (else infinite), and the growth
    /// limit is at least the base size.
    pub(super) fn new(size: &TrackSize, basis: Option<u16>) -> Self {
        let fixed = |b: &TrackBreadth| b.cells(basis).map(u32::from);
        let min = match size.min_sizing() {
            TrackBreadth::MinContent => MinFn::MinContent,
            TrackBreadth::MaxContent => MinFn::MaxContent,
            b => fixed(b).map_or(MinFn::Auto, MinFn::Fixed),
        };
        let max = match (size.max_sizing(), size.fit_content_limit()) {
            // `fit-content()` against an indefinite percentage is
            // `max-content` (CSS Sizing 3 §3.1).
            (_, Some(limit)) => fixed(limit).map_or(MaxFn::MaxContent, MaxFn::FitContent),
            (TrackBreadth::Fr(f), None) => MaxFn::Flex(*f),
            (TrackBreadth::MinContent, None) => MaxFn::MinContent,
            (TrackBreadth::MaxContent, None) => MaxFn::MaxContent,
            (b, None) => fixed(b).map_or(MaxFn::Auto, MaxFn::Fixed),
        };
        Self::with(min, max)
    }

    /// A track with these sizing functions, initialized (§11.4).
    pub(super) fn with(min: MinFn, max: MaxFn) -> Self {
        let base = match min {
            MinFn::Fixed(n) => n,
            _ => 0,
        };
        let limit = match max {
            MaxFn::Fixed(n) => Some(n.max(base)),
            _ => None,
        };
        Self {
            min,
            max,
            base,
            limit,
            growable: false,
        }
    }

    /// A collapsed track (§7.2.3.2 `auto-fit`): fixed at zero.
    pub(super) fn collapsed() -> Self {
        Self::with(MinFn::Fixed(0), MaxFn::Fixed(0))
    }

    /// The flex factor of a flexible track (§7.2.4).
    pub(super) fn flex(&self) -> Option<f32> {
        match self.max {
            MaxFn::Flex(f) => Some(f),
            _ => None,
        }
    }

    /// The growth limit, an infinite one read as the base size (§11.5.1
    /// "treat an infinite growth limit as the base size").
    pub(super) fn limit_or_base(&self) -> u32 {
        self.limit.unwrap_or(self.base)
    }
}

/// `[start, end)` — the tracks an item spans, by index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Span {
    pub(super) start: usize,
    pub(super) end: usize,
}

impl Span {
    pub(super) fn new(start: usize, end: usize) -> Self {
        debug_assert!(start < end, "a span covers at least one track");
        Self { start, end }
    }

    /// How many tracks it spans.
    pub(super) fn len(self) -> usize {
        self.end - self.start
    }

    pub(super) fn tracks(self) -> std::ops::Range<usize> {
        self.start..self.end
    }
}

/// One axis's tracks and the gutters between them (§11.1: "gutters are
/// treated as empty fixed-size tracks of the specified size").
#[derive(Debug, Clone, PartialEq)]
pub(super) struct TrackGrid {
    pub(super) tracks: Vec<Track>,
    /// The gutter after each track but the last: the gap, or none beside
    /// a collapsed track (§7.2.3.2 — the gutters on either side of it
    /// collapse into one).
    pub(super) gutters: Vec<u32>,
}

impl TrackGrid {
    /// `tracks` with a `gap` between each pair, none where a collapsed
    /// track (`collapsed[i]`) leaves nothing on one side to separate.
    pub(super) fn new(tracks: Vec<Track>, collapsed: &[bool], gap: u32) -> Self {
        let n = tracks.len();
        let mut seen_open = false;
        let mut gutters = Vec::with_capacity(n.saturating_sub(1));
        for i in 0..n {
            seen_open |= !collapsed.get(i).copied().unwrap_or(false);
            if i + 1 < n {
                let next_open = !collapsed.get(i + 1).copied().unwrap_or(false);
                gutters.push(if seen_open && next_open { gap } else { 0 });
            }
        }
        Self { tracks, gutters }
    }

    /// The gutters inside `span`.
    pub(super) fn inner_gutters(&self, span: Span) -> u32 {
        self.gutters[span.start..span.end - 1].iter().sum()
    }

    /// `span`'s extent by `size` of each track, its inner gutters
    /// included.
    pub(super) fn span_size(&self, span: Span, size: impl Fn(&Track) -> u32) -> u32 {
        self.tracks[span.tracks()].iter().map(size).sum::<u32>() + self.inner_gutters(span)
    }

    /// The whole grid's extent: the base sizes and every gutter.
    pub(super) fn total(&self) -> u32 {
        self.tracks.iter().map(|t| t.base).sum::<u32>() + self.gutters.iter().sum::<u32>()
    }

    /// Each track's start and end offset from the grid's start edge, in
    /// cells: a track ends at its start plus its base size, and the next
    /// starts past the gutter.
    pub(super) fn extents(&self) -> Vec<(u32, u32)> {
        let mut out = Vec::with_capacity(self.tracks.len());
        let mut at = 0u32;
        for (i, t) in self.tracks.iter().enumerate() {
            out.push((at, at + t.base));
            at += t.base + self.gutters.get(i).copied().unwrap_or(0);
        }
        out
    }
}
