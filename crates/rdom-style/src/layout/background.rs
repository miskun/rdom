//! Background values (CSS Backgrounds 3 §3): the keyword families of
//! the per-layer longhands. A layer's image, position and size are kept
//! as their validated CSS text — rdom draws no images (DIVERGENCES §1),
//! so nothing reads them but serialization. Only the background color
//! paints, clipped by the final layer's `background-clip`.

/// `<visual-box>` (CSS Box 4 §3.1): which box of an element a
/// `background-origin` or `background-clip` names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum VisualBox {
    /// The border box: the whole element, border cells included.
    /// `background-clip`'s initial value.
    #[default]
    BorderBox,
    /// The padding box: inside the border. `background-origin`'s
    /// initial value.
    PaddingBox,
    /// The content box: inside the padding.
    ContentBox,
}

impl VisualBox {
    /// The keyword's CSS spelling.
    pub const fn keyword(self) -> &'static str {
        match self {
            VisualBox::BorderBox => "border-box",
            VisualBox::PaddingBox => "padding-box",
            VisualBox::ContentBox => "content-box",
        }
    }
}

/// One axis of a `<repeat-style>` (CSS Backgrounds 3 §3.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum RepeatStyle {
    #[default]
    Repeat,
    Space,
    Round,
    NoRepeat,
}

impl RepeatStyle {
    /// The keyword's CSS spelling.
    pub const fn keyword(self) -> &'static str {
        match self {
            RepeatStyle::Repeat => "repeat",
            RepeatStyle::Space => "space",
            RepeatStyle::Round => "round",
            RepeatStyle::NoRepeat => "no-repeat",
        }
    }
}

/// One layer's `background-repeat` (CSS Backgrounds 3 §3.4): the
/// horizontal and vertical repeat. `repeat-x` is `{ x: Repeat, y:
/// NoRepeat }`, `repeat-y` the reverse. Inert: there is no image to
/// repeat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct BackgroundRepeat {
    pub x: RepeatStyle,
    pub y: RepeatStyle,
}

/// One layer's `background-attachment` (CSS Backgrounds 3 §3.5).
/// Inert: there is no image to attach.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum BackgroundAttachment {
    #[default]
    Scroll,
    Fixed,
    Local,
}

impl BackgroundAttachment {
    /// The keyword's CSS spelling.
    pub const fn keyword(self) -> &'static str {
        match self {
            BackgroundAttachment::Scroll => "scroll",
            BackgroundAttachment::Fixed => "fixed",
            BackgroundAttachment::Local => "local",
        }
    }
}
