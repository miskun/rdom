//! The CSS Basic User Interface 4 values: the outline (§5), the cursor
//! (§4.1), the caret's shape and animation (§6.2) — and [`UiStyle`], the
//! computed group of the user-interface properties.

use super::{BorderStyle, BorderWidth, PaintLength};

/// `outline-style` (CSS UI 4 §5.2): `auto | <outline-line-style>`, every
/// `<line-style>` but `hidden` (rdom's own `half-block` included in
/// neither). Not inherited; initial `none`.
///
/// Closed (DESIGN): the painter maps each one to a glyph set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum OutlineStyle {
    /// No outline (the initial value).
    #[default]
    None,
    /// The user agent's focus-ring look: in rdom a light single line with
    /// rounded corners, `outline-color: auto` drawing it in the accent
    /// color.
    Auto,
    Solid,
    Double,
    Dashed,
    Dotted,
    Ridge,
    Outset,
    Groove,
    Inset,
}

impl OutlineStyle {
    /// Every keyword with its CSS spelling.
    pub const KEYWORDS: &'static [(&'static str, OutlineStyle)] = &[
        ("none", OutlineStyle::None),
        ("auto", OutlineStyle::Auto),
        ("solid", OutlineStyle::Solid),
        ("double", OutlineStyle::Double),
        ("dashed", OutlineStyle::Dashed),
        ("dotted", OutlineStyle::Dotted),
        ("ridge", OutlineStyle::Ridge),
        ("outset", OutlineStyle::Outset),
        ("groove", OutlineStyle::Groove),
        ("inset", OutlineStyle::Inset),
    ];

    /// The keyword's CSS spelling.
    pub fn keyword(self) -> &'static str {
        Self::KEYWORDS
            .iter()
            .find(|(_, s)| *s == self)
            .map_or("none", |(k, _)| k)
    }

    /// The border line the ring draws with — `auto` a solid one — or
    /// `None` for `none`.
    pub fn line(self) -> Option<BorderStyle> {
        Some(match self {
            OutlineStyle::None => return None,
            OutlineStyle::Auto | OutlineStyle::Solid => BorderStyle::Solid,
            OutlineStyle::Double => BorderStyle::Double,
            OutlineStyle::Dashed => BorderStyle::Dashed,
            OutlineStyle::Dotted => BorderStyle::Dotted,
            OutlineStyle::Ridge => BorderStyle::Ridge,
            OutlineStyle::Outset => BorderStyle::Outset,
            OutlineStyle::Groove => BorderStyle::Groove,
            OutlineStyle::Inset => BorderStyle::Inset,
        })
    }
}

/// `outline-color` (CSS UI 4 §5.3): `auto | <color>`. `auto` is the
/// accent color under `outline-style: auto` and `currentcolor` otherwise;
/// a color stays a [`TuiColor`](crate::TuiColor) and resolves at paint
/// against the element, as `caret-color` does. Not inherited; initial
/// `auto`.
///
/// Closed (DESIGN): a color or the keyword.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum OutlineColor {
    #[default]
    Auto,
    Color(crate::TuiColor),
}

/// The computed CSS UI 4 properties of an element
/// ([`ComputedStyle::ui`](crate::ComputedStyle::ui)): the outline's four
/// longhands, which do not inherit, and `cursor`, which does.
///
/// Closed (DESIGN), as the other style groups: a new field fails a
/// destructuring pattern. `Default` is the initial values.
#[derive(Debug, Clone, PartialEq)]
pub struct UiStyle {
    /// `outline-style` (§5.2).
    pub outline_style: OutlineStyle,
    /// `outline-width` (§5.3): a `<line-width>`, viewport units resolved;
    /// it selects the ring's glyph weight as a border width does.
    pub outline_width: BorderWidth,
    /// `outline-color` (§5.3).
    pub outline_color: OutlineColor,
    /// `outline-offset` (§5.4), viewport units resolved: whole cells
    /// outside the border edge, a pixel length one cell its way
    /// ([`PaintLength::offset_cells`]).
    pub outline_offset: PaintLength,
    /// `cursor` (§4.1). Inherited.
    pub cursor: Cursor,
    /// `caret-shape` (§6.2.2). Inherited.
    pub caret_shape: CaretShape,
    /// `caret-animation` (§6.2.1). Inherited.
    pub caret_animation: CaretAnimation,
}

impl Default for UiStyle {
    fn default() -> Self {
        UiStyle {
            outline_style: OutlineStyle::None,
            outline_width: BorderWidth::Medium,
            outline_color: OutlineColor::Auto,
            outline_offset: PaintLength::Cells(0.0),
            cursor: Cursor::default(),
            caret_shape: CaretShape::Auto,
            caret_animation: CaretAnimation::Auto,
        }
    }
}

/// A `<cursor-predefined>` keyword, or `auto` / `default` / `none` (CSS
/// UI 4 §4.1.1): the pointer shape over an element.
///
/// Closed (DESIGN): the runtime maps each one to a terminal pointer shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum CursorKeyword {
    /// The UA's choice: the text pointer over text, `default` elsewhere
    /// (the initial value).
    #[default]
    Auto,
    Default,
    None,
    ContextMenu,
    Help,
    Pointer,
    Progress,
    Wait,
    Cell,
    Crosshair,
    Text,
    VerticalText,
    Alias,
    Copy,
    Move,
    NoDrop,
    NotAllowed,
    Grab,
    Grabbing,
    EResize,
    NResize,
    NeResize,
    NwResize,
    SResize,
    SeResize,
    SwResize,
    WResize,
    EwResize,
    NsResize,
    NeswResize,
    NwseResize,
    ColResize,
    RowResize,
    AllScroll,
    ZoomIn,
    ZoomOut,
}

impl CursorKeyword {
    /// Every keyword with its CSS spelling.
    pub const KEYWORDS: &'static [(&'static str, CursorKeyword)] = &[
        ("auto", CursorKeyword::Auto),
        ("default", CursorKeyword::Default),
        ("none", CursorKeyword::None),
        ("context-menu", CursorKeyword::ContextMenu),
        ("help", CursorKeyword::Help),
        ("pointer", CursorKeyword::Pointer),
        ("progress", CursorKeyword::Progress),
        ("wait", CursorKeyword::Wait),
        ("cell", CursorKeyword::Cell),
        ("crosshair", CursorKeyword::Crosshair),
        ("text", CursorKeyword::Text),
        ("vertical-text", CursorKeyword::VerticalText),
        ("alias", CursorKeyword::Alias),
        ("copy", CursorKeyword::Copy),
        ("move", CursorKeyword::Move),
        ("no-drop", CursorKeyword::NoDrop),
        ("not-allowed", CursorKeyword::NotAllowed),
        ("grab", CursorKeyword::Grab),
        ("grabbing", CursorKeyword::Grabbing),
        ("e-resize", CursorKeyword::EResize),
        ("n-resize", CursorKeyword::NResize),
        ("ne-resize", CursorKeyword::NeResize),
        ("nw-resize", CursorKeyword::NwResize),
        ("s-resize", CursorKeyword::SResize),
        ("se-resize", CursorKeyword::SeResize),
        ("sw-resize", CursorKeyword::SwResize),
        ("w-resize", CursorKeyword::WResize),
        ("ew-resize", CursorKeyword::EwResize),
        ("ns-resize", CursorKeyword::NsResize),
        ("nesw-resize", CursorKeyword::NeswResize),
        ("nwse-resize", CursorKeyword::NwseResize),
        ("col-resize", CursorKeyword::ColResize),
        ("row-resize", CursorKeyword::RowResize),
        ("all-scroll", CursorKeyword::AllScroll),
        ("zoom-in", CursorKeyword::ZoomIn),
        ("zoom-out", CursorKeyword::ZoomOut),
    ];

    /// The keyword's CSS spelling.
    pub fn keyword(self) -> &'static str {
        Self::KEYWORDS
            .iter()
            .find(|(_, k)| *k == self)
            .map_or("auto", |(name, _)| name)
    }
}

/// One `<url> [<x> <y>]?` image of a `cursor` list (CSS UI 4 §4.1.1).
/// A terminal draws no image pointer, so rdom keeps it only to serialize
/// it back; the keyword after the images is the pointer it shows.
#[derive(Debug, Clone, PartialEq)]
pub struct CursorImage {
    /// The image's URL, as written.
    pub url: String,
    /// The hotspot, when given.
    pub hotspot: Option<(f32, f32)>,
}

/// `cursor` (CSS UI 4 §4.1): `[<url> [<x> <y>]?,]* <cursor-predefined>`.
/// Inherited; initial `auto`.
///
/// Closed (DESIGN): the images (inert in a terminal) and the keyword.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Cursor {
    /// The image fallbacks, in order — shared, so an inheriting element
    /// does not copy them.
    pub images: std::sync::Arc<[CursorImage]>,
    /// The keyword the pointer shows.
    pub keyword: CursorKeyword,
}

impl From<CursorKeyword> for Cursor {
    /// The keyword alone, no image fallbacks.
    fn from(keyword: CursorKeyword) -> Self {
        Cursor {
            images: std::sync::Arc::default(),
            keyword,
        }
    }
}

/// `caret-shape` (CSS UI 4 §6.2.2): `auto | bar | block | underscore`.
/// Inherited; initial `auto`, which rdom draws as `block` — the caret a
/// terminal's own text cursor shows.
///
/// Closed (DESIGN): the caret painter draws each one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum CaretShape {
    #[default]
    Auto,
    Bar,
    Block,
    Underscore,
}

impl CaretShape {
    /// Every keyword with its CSS spelling.
    pub const KEYWORDS: &'static [(&'static str, CaretShape)] = &[
        ("auto", CaretShape::Auto),
        ("bar", CaretShape::Bar),
        ("block", CaretShape::Block),
        ("underscore", CaretShape::Underscore),
    ];

    /// The keyword's CSS spelling.
    pub fn keyword(self) -> &'static str {
        Self::KEYWORDS
            .iter()
            .find(|(_, k)| *k == self)
            .map_or("auto", |(name, _)| name)
    }
}

/// `caret-animation` (CSS UI 4 §6.2.1): `auto | manual` — whether the UA
/// blinks the caret. Inherited; initial `auto`.
///
/// Closed (DESIGN): the blink is on or off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum CaretAnimation {
    /// The UA's blink (`App::with_caret_blink`).
    #[default]
    Auto,
    /// No blink: the caret stays shown, for an author animation.
    Manual,
}

impl CaretAnimation {
    /// Every keyword with its CSS spelling.
    pub const KEYWORDS: &'static [(&'static str, CaretAnimation)] = &[
        ("auto", CaretAnimation::Auto),
        ("manual", CaretAnimation::Manual),
    ];

    /// The keyword's CSS spelling.
    pub fn keyword(self) -> &'static str {
        match self {
            CaretAnimation::Auto => "auto",
            CaretAnimation::Manual => "manual",
        }
    }
}
