//! Border values (CSS Backgrounds 3 §4): the line styles, the per-side
//! [`Border`] of styles, line widths and the lengths that only decorate
//! ([`PaintLength`]), corner styles and `border-collapse`.

/// Per-side border style — the full CSS `border-style` keyword set.
///
/// Per CSS 2.1 §8.5.3, `none` and `hidden` both produce a 0-width
/// border (the substrate honors this: `cells()` returns 0 for both).
/// The difference between them is only meaningful under
/// `border-collapse: collapse`: `hidden` is CSS Tables 3 §11.5's
/// kill-switch — wherever it appears in a border conflict, that
/// edge is suppressed entirely, regardless of any other contributor.
///
/// Style ranking on style tie (CSS 2.1 §17.6.2.1): `double > solid >
/// dashed > dotted > ridge > outset > groove > inset`. Higher rank
/// wins under collapse when widths and elements tie.
///
/// **Terminal-faithful degradation:** the substrate paints `None`,
/// `Hidden`, `Solid`, and `Double` with distinct glyphs (`│─┌┐└┘` /
/// `║═╔╗╚╝`). `Dashed`, `Dotted`, `Ridge`, `Outset`, `Groove`,
/// `Inset` parse and *rank* correctly in conflict resolution — the
/// data model is faithful — but render as `Solid` because rdom has
/// no distinct glyph set for them yet. Matches CSS's "render as
/// best you can on this medium" principle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum BorderStyle {
    /// No border. Zero width, no paint, no contribution in conflict
    /// resolution. CSS default.
    #[default]
    None,
    /// Invisible border. Zero width, no paint — same as `None` for
    /// layout. **Wins absolutely** in collapse-mode conflict
    /// resolution (CSS Tables 3 §11.5 rule 1: the kill-switch).
    Hidden,
    /// Single-line border. `│─┌┐└┘` (or `╭╮╰╯` with rounded corners).
    Solid,
    /// Double-line border. `║═╔╗╚╝`.
    Double,
    /// Dashed. Parses + ranks per CSS; renders as `Solid`.
    Dashed,
    /// Dotted. Parses + ranks per CSS; renders as `Solid`.
    Dotted,
    /// 3D ridge. Parses + ranks per CSS; renders as `Solid`.
    Ridge,
    /// 3D outset. Parses + ranks per CSS; renders as `Solid`.
    Outset,
    /// 3D groove. Parses + ranks per CSS; renders as `Solid`.
    Groove,
    /// 3D inset. Parses + ranks per CSS; renders as `Solid`.
    Inset,
    /// rdom-specific terminal-only style. Each border cell paints
    /// a half-block (`▄ ▀ ▌ ▐`) or quadrant (`▗ ▖ ▝ ▘`) glyph
    /// whose filled half-or-quarter points INWARD toward the
    /// element's content. The visible color spans roughly half a
    /// cell vertically (top/bottom edges) or horizontally
    /// (left/right edges), so a half-block-bordered element reads
    /// as a "pill" rather than a hard-edged rectangle. Pairs
    /// naturally with a `background-color`-filled interior to
    /// build a primary-CTA button style. Not a CSS-spec style;
    /// see DIVERGENCES.md.
    HalfBlock,
}

impl BorderStyle {
    /// Cells reserved for this border in layout. Per CSS 2.1 §8.5.3,
    /// `None` and `Hidden` produce 0 width; every other style → 1
    /// cell in rdom's terminal model.
    pub const fn cells(self) -> u16 {
        match self {
            BorderStyle::None | BorderStyle::Hidden => 0,
            _ => 1,
        }
    }

    /// True iff this style paints a glyph. Equivalent to `cells() == 1`
    /// in rdom (a border that reserves a cell also paints in it).
    pub const fn is_visible(self) -> bool {
        !matches!(self, BorderStyle::None | BorderStyle::Hidden)
    }

    /// True iff this is the `Hidden` kill-switch — used by collapse
    /// conflict resolution to suppress an edge regardless of other
    /// contributors.
    pub const fn is_hidden(self) -> bool {
        matches!(self, BorderStyle::Hidden)
    }

    /// True iff this is `None` (no border).
    pub const fn is_none(self) -> bool {
        matches!(self, BorderStyle::None)
    }

    /// Style-ranking score for CSS Tables 3 §11.5 conflict resolution
    /// (rule 4: "narrower borders are discarded in favor of wider
    /// ones; styles tie-break in this order").
    /// Higher number wins. `None` and `Hidden` get the lowest score
    /// because they're handled at higher-priority rules (1 + 2);
    /// `rank()` is only consulted when both participants are visible.
    pub const fn rank(self) -> u8 {
        match self {
            BorderStyle::Double => 7,
            BorderStyle::Solid => 6,
            BorderStyle::HalfBlock => 6,
            BorderStyle::Dashed => 5,
            BorderStyle::Dotted => 4,
            BorderStyle::Ridge => 3,
            BorderStyle::Outset => 2,
            BorderStyle::Groove => 1,
            BorderStyle::Inset => 0,
            BorderStyle::None | BorderStyle::Hidden => 0,
        }
    }
}

/// Per-side border state. CSS lets authors enable any combination
/// of `border-top` / `border-right` / `border-bottom` / `border-left`
/// independently — each side carries its own [`BorderStyle`].
/// `corner_style` only matters when all 4 sides paint — the
/// rounded-corner glyphs `╭╮╰╯` need both sides at a corner to share
/// a cell.
///
/// The `border` shorthand and the per-side longhands all write into
/// this struct via the cascade. `Border::default()` is "no border"
/// (all sides `BorderStyle::None`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Border {
    pub top: BorderStyle,
    pub right: BorderStyle,
    pub bottom: BorderStyle,
    pub left: BorderStyle,
    pub corner_style: CornerStyle,
}

impl Border {
    /// The used border: these styles with every side whose width is
    /// zero removed (`none`) — a zero-width border takes no space and
    /// draws nothing (CSS Backgrounds 3 §4.3). `hidden` stays: it is the
    /// collapse kill-switch, and zero-width anyway.
    pub fn with_widths(mut self, widths: &super::Sides<BorderWidth>) -> Self {
        let sides = [
            (&mut self.top, &widths.top),
            (&mut self.right, &widths.right),
            (&mut self.bottom, &widths.bottom),
            (&mut self.left, &widths.left),
        ];
        for (style, width) in sides {
            if !style.is_hidden() && width.weight().is_none() {
                *style = BorderStyle::None;
            }
        }
        self
    }

    /// All sides off (`BorderStyle::None`). Same as `Default`.
    pub const fn none() -> Self {
        Self {
            top: BorderStyle::None,
            right: BorderStyle::None,
            bottom: BorderStyle::None,
            left: BorderStyle::None,
            corner_style: CornerStyle::Square,
        }
    }
    /// All four sides solid, square corners. `border: solid`.
    pub const fn single() -> Self {
        Self::ring(BorderStyle::Solid)
    }
    /// All four sides solid, rounded corners. `border: rounded`.
    pub const fn rounded() -> Self {
        Self {
            top: BorderStyle::Solid,
            right: BorderStyle::Solid,
            bottom: BorderStyle::Solid,
            left: BorderStyle::Solid,
            corner_style: CornerStyle::Rounded,
        }
    }
    /// All four sides set to the same style, square corners.
    pub const fn ring(style: BorderStyle) -> Self {
        Self {
            top: style,
            right: style,
            bottom: style,
            left: style,
            corner_style: CornerStyle::Square,
        }
    }
    /// Top side only (solid). `border-top: solid` longhand without others.
    pub const fn top() -> Self {
        Self {
            top: BorderStyle::Solid,
            right: BorderStyle::None,
            bottom: BorderStyle::None,
            left: BorderStyle::None,
            corner_style: CornerStyle::Square,
        }
    }
    pub const fn bottom() -> Self {
        Self {
            top: BorderStyle::None,
            right: BorderStyle::None,
            bottom: BorderStyle::Solid,
            left: BorderStyle::None,
            corner_style: CornerStyle::Square,
        }
    }
    pub const fn left() -> Self {
        Self {
            top: BorderStyle::None,
            right: BorderStyle::None,
            bottom: BorderStyle::None,
            left: BorderStyle::Solid,
            corner_style: CornerStyle::Square,
        }
    }
    pub const fn right() -> Self {
        Self {
            top: BorderStyle::None,
            right: BorderStyle::Solid,
            bottom: BorderStyle::None,
            left: BorderStyle::None,
            corner_style: CornerStyle::Square,
        }
    }

    /// True iff every side is `None` (no border at all).
    pub const fn is_empty(&self) -> bool {
        self.top.is_none() && self.right.is_none() && self.bottom.is_none() && self.left.is_none()
    }
    /// True iff every side paints a visible glyph. (Used to gate
    /// rounded-corner rendering — corners only round when all four
    /// sides participate.)
    pub const fn is_box(&self) -> bool {
        self.top.is_visible()
            && self.right.is_visible()
            && self.bottom.is_visible()
            && self.left.is_visible()
    }
}

/// Corner glyph style — applies when all 4 sides are drawn (per-side
/// borders don't form corners). `Square` uses `┌┐└┘`; `Rounded` uses
/// `╭╮╰╯`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CornerStyle {
    #[default]
    Square,
    Rounded,
}

/// CSS `border-collapse` (M5.5). Default is `Separate` — every box
/// draws its own border ring. `Collapse` makes adjacent borders
/// share their cells: parent + child meeting at an edge use **one**
/// cell of border, not two; sibling flex children sharing an edge
/// also share **one** cell. The paint pass walks the buffer after
/// element-by-element border painting and rewrites junction glyphs
/// (`├ ┤ ┬ ┴ ┼`) based on 4-neighbor connectivity.
///
/// **Deliberate divergence from CSS:** the spec restricts
/// `border-collapse: collapse` to `<table>` boxes only. rdom extends
/// it to any flex container — TUI grid layouts are too dominant an
/// idiom to gate behind table semantics.
///
/// Style-conflict resolution (when parent + child borders share an
/// edge with different `border-style`): "outermost wins" — parent's
/// style at the shared edge defeats the child's. Simplification of
/// CSS's full hidden > double > solid > … cascade. Tracked as
/// `M5-COLLAPSE-1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderCollapse {
    /// CSS initial value. Each box's border ring is independent.
    #[default]
    Separate,
    /// Adjacent borders share cells; paint joiner rewrites junction
    /// glyphs.
    Collapse,
}

/// A length that only decorates — it changes what a box paints, never
/// its geometry: a border's width (which picks the glyph weight), a
/// corner's radius (round or square), a shadow's offsets. rdom lays out
/// in whole cells, but these properties are where web CSS writes pixel
/// lengths (`border: 1px solid`, `border-radius: 4px`), so they also
/// take the absolute units and, at the CSS initial font size (16px),
/// `em` / `rem`, which no layout property does (DIVERGENCES §2).
#[derive(Debug, Clone, PartialEq)]
pub enum PaintLength {
    /// rdom's cell lengths: a bare number, `ch`, `lh`, resolved math.
    Cells(f32),
    /// An absolute (`px`, `cm`, `mm`, `Q`, `in`, `pt`, `pc`) or
    /// font-relative (`em`, `rem`) length, in CSS pixels.
    Px(f32),
    /// A math function or a length that needs the viewport (`vw`, …)
    /// or a percentage basis; resolves to cells.
    Calc(Box<crate::calc::CalcExpr>),
}

impl PaintLength {
    /// The length in cells when it is one (`Cells`, or `Calc` against
    /// `basis`); `None` for a pixel length.
    pub fn cells(&self, basis: i32) -> Option<f64> {
        match self {
            PaintLength::Cells(c) => Some(f64::from(*c)),
            PaintLength::Px(_) => None,
            PaintLength::Calc(e) => Some(e.resolve_f64(&crate::calc::ResolveCtx::new(basis))),
        }
    }

    /// True when the length is zero (or negative) against `basis`.
    pub fn is_zero(&self, basis: i32) -> bool {
        match self {
            PaintLength::Px(p) => *p <= 0.0,
            other => other.cells(basis).is_none_or(|c| c.is_nan() || c <= 0.0),
        }
    }
}

/// `<line-width>` (CSS Backgrounds 3 §4.3): a border side's width. Its
/// initial value is `medium`. Every rdom border is one cell wide; the
/// width selects the glyph weight ([`BorderWidth::weight`]), and a zero
/// width removes the side.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum BorderWidth {
    Thin,
    #[default]
    Medium,
    Thick,
    Length(PaintLength),
}

/// The glyph weight a border side draws with: the light box-drawing set
/// (`─│┌`) or the heavy one (`━┃┏`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub enum BorderWeight {
    #[default]
    Light,
    Heavy,
}

impl BorderWidth {
    /// `thick` in CSS pixels — the width from which a pixel length is
    /// heavy (browsers draw `thin` / `medium` / `thick` as 1 / 3 / 5px).
    pub const THICK_PX: f32 = 5.0;
    /// The cells from which a cell length is heavy: one cell is the
    /// border every rdom box draws.
    pub const HEAVY_CELLS: f64 = 2.0;

    /// The weight this width draws with, `None` for a zero width (no
    /// border, §4.3). `thin` and `medium` are light and `thick` heavy;
    /// a pixel length is light below `thick` (5px) and heavy from it,
    /// a cell length light below two cells (rounded onto the grid) and
    /// heavy from them; any non-zero length is at least light, as a
    /// browser draws a sub-pixel border one device pixel wide.
    pub fn weight(&self) -> Option<BorderWeight> {
        match self {
            BorderWidth::Thin | BorderWidth::Medium => Some(BorderWeight::Light),
            BorderWidth::Thick => Some(BorderWeight::Heavy),
            BorderWidth::Length(l) if l.is_zero(0) => None,
            BorderWidth::Length(PaintLength::Px(px)) => Some(if *px < Self::THICK_PX {
                BorderWeight::Light
            } else {
                BorderWeight::Heavy
            }),
            BorderWidth::Length(cells) => {
                let n = cells.cells(0).unwrap_or(0.0).round_ties_even();
                Some(if n < Self::HEAVY_CELLS {
                    BorderWeight::Light
                } else {
                    BorderWeight::Heavy
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CSS Backgrounds 3 §4.3: `thin` / `medium` light, `thick` heavy;
    /// pixel lengths light below `thick` (5px), cell lengths below two
    /// cells; zero is no border, any other length at least light.
    #[test]
    fn widths_map_onto_two_weights() {
        use BorderWeight::{Heavy, Light};
        let px = |p| BorderWidth::Length(PaintLength::Px(p));
        let cells = |c| BorderWidth::Length(PaintLength::Cells(c));
        for (width, weight) in [
            (BorderWidth::Thin, Some(Light)),
            (BorderWidth::Medium, Some(Light)),
            (BorderWidth::Thick, Some(Heavy)),
            (px(0.0), None),
            (px(0.5), Some(Light)),
            (px(4.9), Some(Light)),
            (px(5.0), Some(Heavy)),
            (cells(0.0), None),
            (cells(0.3), Some(Light)),
            (cells(1.0), Some(Light)),
            (cells(1.5), Some(Heavy)),
            (cells(2.0), Some(Heavy)),
        ] {
            assert_eq!(width.weight(), weight, "{width:?}");
        }
    }

    /// The used border drops zero-width sides but keeps `hidden`.
    #[test]
    fn used_border_drops_zero_width_sides() {
        let zero = BorderWidth::Length(PaintLength::Cells(0.0));
        let widths =
            super::super::Sides::new(zero.clone(), BorderWidth::Medium, zero, BorderWidth::Thick);
        let mut b = Border::single();
        b.bottom = BorderStyle::Hidden;
        let used = b.with_widths(&widths);
        assert_eq!(
            (used.top, used.right, used.bottom, used.left),
            (
                BorderStyle::None,
                BorderStyle::Solid,
                BorderStyle::Hidden,
                BorderStyle::Solid
            )
        );
    }
}
