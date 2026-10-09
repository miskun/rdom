//! `position-area` (CSS Anchor Positioning 1 §3.1): the keywords of its
//! vocabularies, the value as written, and the tracks it spans in the
//! 3 × 3 grid of a containing block and an anchor.

/// One keyword of a `<position-area>` (§3.1).
///
/// Open (`#[non_exhaustive]`): Anchor Positioning has grown the grammar
/// (the `self-*` and `span-*` forms) and may again; an unknown keyword
/// reads as `span-all`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AreaKeyword {
    Left,
    Right,
    Top,
    Bottom,
    Center,
    SpanLeft,
    SpanRight,
    SpanTop,
    SpanBottom,
    SpanAll,
    XStart,
    XEnd,
    SpanXStart,
    SpanXEnd,
    XSelfStart,
    XSelfEnd,
    SpanXSelfStart,
    SpanXSelfEnd,
    YStart,
    YEnd,
    SpanYStart,
    SpanYEnd,
    YSelfStart,
    YSelfEnd,
    SpanYSelfStart,
    SpanYSelfEnd,
    BlockStart,
    BlockEnd,
    SpanBlockStart,
    SpanBlockEnd,
    InlineStart,
    InlineEnd,
    SpanInlineStart,
    SpanInlineEnd,
    SelfBlockStart,
    SelfBlockEnd,
    SpanSelfBlockStart,
    SpanSelfBlockEnd,
    SelfInlineStart,
    SelfInlineEnd,
    SpanSelfInlineStart,
    SpanSelfInlineEnd,
    Start,
    End,
    SpanStart,
    SpanEnd,
    SelfStart,
    SelfEnd,
    SpanSelfStart,
    SpanSelfEnd,
}

/// Which axis a `<position-area>` keyword names (§3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AreaAxis {
    /// `left` … `x-*`: the horizontal axis.
    X,
    /// `top` … `y-*`: the vertical axis.
    Y,
    /// `block-*`, `self-block-*`.
    Block,
    /// `inline-*`, `self-inline-*`.
    Inline,
    /// `start` … `self-end`: the block axis first, the inline second.
    Either,
    /// `center`, `span-all`: whichever.
    Any,
}

/// Where a keyword's side is counted from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    /// The physical start (left / top).
    Physical,
    /// The containing block's writing mode and direction.
    Container,
    /// The positioned box's own.
    SelfBox,
}

/// A `<position-area>` keyword's tracks in its axis: start, center, end
/// (`0`, `1`, `2`), counted from its side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Tracks(u8, u8);

impl AreaKeyword {
    /// Every keyword with its CSS spelling.
    pub const KEYWORDS: &'static [(&'static str, AreaKeyword)] = &[
        ("left", AreaKeyword::Left),
        ("right", AreaKeyword::Right),
        ("top", AreaKeyword::Top),
        ("bottom", AreaKeyword::Bottom),
        ("center", AreaKeyword::Center),
        ("span-left", AreaKeyword::SpanLeft),
        ("span-right", AreaKeyword::SpanRight),
        ("span-top", AreaKeyword::SpanTop),
        ("span-bottom", AreaKeyword::SpanBottom),
        ("span-all", AreaKeyword::SpanAll),
        ("x-start", AreaKeyword::XStart),
        ("x-end", AreaKeyword::XEnd),
        ("span-x-start", AreaKeyword::SpanXStart),
        ("span-x-end", AreaKeyword::SpanXEnd),
        ("x-self-start", AreaKeyword::XSelfStart),
        ("x-self-end", AreaKeyword::XSelfEnd),
        ("span-x-self-start", AreaKeyword::SpanXSelfStart),
        ("span-x-self-end", AreaKeyword::SpanXSelfEnd),
        ("y-start", AreaKeyword::YStart),
        ("y-end", AreaKeyword::YEnd),
        ("span-y-start", AreaKeyword::SpanYStart),
        ("span-y-end", AreaKeyword::SpanYEnd),
        ("y-self-start", AreaKeyword::YSelfStart),
        ("y-self-end", AreaKeyword::YSelfEnd),
        ("span-y-self-start", AreaKeyword::SpanYSelfStart),
        ("span-y-self-end", AreaKeyword::SpanYSelfEnd),
        ("block-start", AreaKeyword::BlockStart),
        ("block-end", AreaKeyword::BlockEnd),
        ("span-block-start", AreaKeyword::SpanBlockStart),
        ("span-block-end", AreaKeyword::SpanBlockEnd),
        ("inline-start", AreaKeyword::InlineStart),
        ("inline-end", AreaKeyword::InlineEnd),
        ("span-inline-start", AreaKeyword::SpanInlineStart),
        ("span-inline-end", AreaKeyword::SpanInlineEnd),
        ("self-block-start", AreaKeyword::SelfBlockStart),
        ("self-block-end", AreaKeyword::SelfBlockEnd),
        ("span-self-block-start", AreaKeyword::SpanSelfBlockStart),
        ("span-self-block-end", AreaKeyword::SpanSelfBlockEnd),
        ("self-inline-start", AreaKeyword::SelfInlineStart),
        ("self-inline-end", AreaKeyword::SelfInlineEnd),
        ("span-self-inline-start", AreaKeyword::SpanSelfInlineStart),
        ("span-self-inline-end", AreaKeyword::SpanSelfInlineEnd),
        ("start", AreaKeyword::Start),
        ("end", AreaKeyword::End),
        ("span-start", AreaKeyword::SpanStart),
        ("span-end", AreaKeyword::SpanEnd),
        ("self-start", AreaKeyword::SelfStart),
        ("self-end", AreaKeyword::SelfEnd),
        ("span-self-start", AreaKeyword::SpanSelfStart),
        ("span-self-end", AreaKeyword::SpanSelfEnd),
    ];

    /// The keyword's CSS spelling.
    pub fn keyword(self) -> &'static str {
        Self::KEYWORDS
            .iter()
            .find(|(_, k)| *k == self)
            .map_or("span-all", |(name, _)| name)
    }

    /// Its axis, its side, and its tracks counted from that side.
    fn parts(self) -> (AreaAxis, Side, Tracks) {
        use AreaAxis as A;
        use AreaKeyword as K;
        use Side as S;
        let (start, end, span_start, span_end) =
            (Tracks(0, 0), Tracks(2, 2), Tracks(0, 1), Tracks(1, 2));
        match self {
            K::Center => (A::Any, S::Physical, Tracks(1, 1)),
            K::SpanAll => (A::Any, S::Physical, Tracks(0, 2)),
            K::Left => (A::X, S::Physical, start),
            K::Right => (A::X, S::Physical, end),
            K::SpanLeft => (A::X, S::Physical, span_start),
            K::SpanRight => (A::X, S::Physical, span_end),
            K::Top => (A::Y, S::Physical, start),
            K::Bottom => (A::Y, S::Physical, end),
            K::SpanTop => (A::Y, S::Physical, span_start),
            K::SpanBottom => (A::Y, S::Physical, span_end),
            K::XStart => (A::X, S::Container, start),
            K::XEnd => (A::X, S::Container, end),
            K::SpanXStart => (A::X, S::Container, span_start),
            K::SpanXEnd => (A::X, S::Container, span_end),
            K::XSelfStart => (A::X, S::SelfBox, start),
            K::XSelfEnd => (A::X, S::SelfBox, end),
            K::SpanXSelfStart => (A::X, S::SelfBox, span_start),
            K::SpanXSelfEnd => (A::X, S::SelfBox, span_end),
            K::YStart => (A::Y, S::Container, start),
            K::YEnd => (A::Y, S::Container, end),
            K::SpanYStart => (A::Y, S::Container, span_start),
            K::SpanYEnd => (A::Y, S::Container, span_end),
            K::YSelfStart => (A::Y, S::SelfBox, start),
            K::YSelfEnd => (A::Y, S::SelfBox, end),
            K::SpanYSelfStart => (A::Y, S::SelfBox, span_start),
            K::SpanYSelfEnd => (A::Y, S::SelfBox, span_end),
            K::BlockStart => (A::Block, S::Container, start),
            K::BlockEnd => (A::Block, S::Container, end),
            K::SpanBlockStart => (A::Block, S::Container, span_start),
            K::SpanBlockEnd => (A::Block, S::Container, span_end),
            K::InlineStart => (A::Inline, S::Container, start),
            K::InlineEnd => (A::Inline, S::Container, end),
            K::SpanInlineStart => (A::Inline, S::Container, span_start),
            K::SpanInlineEnd => (A::Inline, S::Container, span_end),
            K::SelfBlockStart => (A::Block, S::SelfBox, start),
            K::SelfBlockEnd => (A::Block, S::SelfBox, end),
            K::SpanSelfBlockStart => (A::Block, S::SelfBox, span_start),
            K::SpanSelfBlockEnd => (A::Block, S::SelfBox, span_end),
            K::SelfInlineStart => (A::Inline, S::SelfBox, start),
            K::SelfInlineEnd => (A::Inline, S::SelfBox, end),
            K::SpanSelfInlineStart => (A::Inline, S::SelfBox, span_start),
            K::SpanSelfInlineEnd => (A::Inline, S::SelfBox, span_end),
            K::Start => (A::Either, S::Container, start),
            K::End => (A::Either, S::Container, end),
            K::SpanStart => (A::Either, S::Container, span_start),
            K::SpanEnd => (A::Either, S::Container, span_end),
            K::SelfStart => (A::Either, S::SelfBox, start),
            K::SelfEnd => (A::Either, S::SelfBox, end),
            K::SpanSelfStart => (A::Either, S::SelfBox, span_start),
            K::SpanSelfEnd => (A::Either, S::SelfBox, span_end),
        }
    }

    pub(crate) fn axis(self) -> AreaAxis {
        self.parts().0
    }

    /// Whether the keyword counts from the box's own writing mode and
    /// direction (`self-*`).
    pub(crate) fn is_self(self) -> bool {
        self.parts().1 == Side::SelfBox
    }
}

/// `position-area` (§3.1): `none | <position-area>`, one keyword per axis
/// as written (one keyword alone completed by §3.1's rule: `span-all` for
/// an axis-specific keyword, itself again otherwise).
///
/// `none` is a value of the type ([`PositionArea::NONE`], the initial
/// value and `Default`), as `PositionAnchor::None` is of its own.
///
/// Closed (DESIGN): a value record read through [`tracks`](Self::tracks).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PositionArea {
    /// The keywords as written; `None` is `none`.
    keywords: Option<(AreaKeyword, Option<AreaKeyword>)>,
}

/// The tracks a `position-area` spans in the 3 × 3 grid of its
/// containing block and anchor (§3.1.1): columns and rows, `(first,
/// last)` from the left and top, each `0` (before the anchor), `1` (the
/// anchor's) or `2` (after it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaTracks {
    pub columns: (u8, u8),
    pub rows: (u8, u8),
}

impl PositionArea {
    /// `none`: the box is not placed in the grid (the initial value).
    pub const NONE: PositionArea = PositionArea { keywords: None };

    /// `first` and, when written, `second` — `None` when the pair is not
    /// one of §3.1's (two keywords of one axis, or of two vocabularies).
    pub fn new(first: AreaKeyword, second: Option<AreaKeyword>) -> Option<Self> {
        if let Some(second) = second {
            use AreaAxis as A;
            let same_vocabulary = first.is_self() == second.is_self();
            let ok = match (first.axis(), second.axis()) {
                // `center` and `span-all` belong to every vocabulary.
                (A::Any, _) | (_, A::Any) => true,
                (A::X, A::Y) | (A::Y, A::X) => true,
                (A::Block, A::Inline) | (A::Inline, A::Block) => same_vocabulary,
                (A::Either, A::Either) => same_vocabulary,
                _ => false,
            };
            if !ok {
                return None;
            }
        }
        Some(PositionArea {
            keywords: Some((first, second)),
        })
    }

    /// Whether this is `none`.
    pub fn is_none(&self) -> bool {
        self.keywords.is_none()
    }

    /// The keywords as written; `None` for `none`.
    pub fn keywords(&self) -> Option<(AreaKeyword, Option<AreaKeyword>)> {
        self.keywords
    }

    /// The columns and rows the area spans, for a containing block whose
    /// `direction` is `rtl` when `cb_rtl` and a box whose own is when
    /// `self_rtl` (rdom lays out `horizontal-tb`: block is vertical);
    /// `None` for `none`, which spans no area.
    pub fn tracks(&self, cb_rtl: bool, self_rtl: bool) -> Option<AreaTracks> {
        use AreaAxis as A;
        // Which physical axis each keyword lands on: a horizontal one
        // (`true`) or the vertical.
        let horizontal =
            |k: AreaKeyword, position: usize, other: Option<AreaKeyword>| match k.axis() {
                A::X | A::Inline => true,
                A::Y | A::Block => false,
                // The first of two generic keywords is the block axis.
                A::Either => position == 1,
                A::Any => match other.map(AreaKeyword::axis) {
                    Some(A::X | A::Inline) => false,
                    Some(A::Y | A::Block) => true,
                    // Two generic keywords or `center` / `span-all` pairs:
                    // the first the block axis.
                    _ => position == 1,
                },
            };
        let physical = |k: AreaKeyword, horizontal: bool| -> (u8, u8) {
            let (_, side, Tracks(a, b)) = k.parts();
            let flip = horizontal
                && match side {
                    Side::Physical => false,
                    Side::Container => cb_rtl,
                    Side::SelfBox => self_rtl,
                };
            if flip { (2 - b, 2 - a) } else { (a, b) }
        };
        let (first, second) = self.keywords?;
        let mut columns = (0, 2);
        let mut rows = (0, 2);
        let h1 = horizontal(first, 0, second);
        let t1 = physical(first, h1);
        if h1 {
            columns = t1;
        } else {
            rows = t1;
        }
        match second {
            Some(s) => {
                let h2 = horizontal(s, 1, Some(first));
                let t2 = physical(s, h2);
                if h2 {
                    columns = t2;
                } else {
                    rows = t2;
                }
            }
            // §3.1: one keyword — `span-all` in the other axis when it is
            // specific to its axis, itself again otherwise.
            None => match first.axis() {
                A::X | A::Y | A::Block | A::Inline => {}
                A::Either | A::Any => {
                    columns = physical(first, true);
                    rows = physical(first, false);
                }
            },
        }
        Some(AreaTracks { columns, rows })
    }
}
