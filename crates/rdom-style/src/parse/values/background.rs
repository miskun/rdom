//! Background values (CSS Backgrounds 3 §3): the `background`
//! shorthand's comma-separated layers (§3.10) and the per-layer
//! longhands `background-image` / `-position` / `-size` / `-repeat` /
//! `-attachment` / `-origin` / `-clip`.
//!
//! Every grammar is checked, so invalid CSS is dropped as on the web.
//! An image, position or size is kept as its CSS text (rdom draws no
//! images: DIVERGENCES §1); the keyword families are typed.

use super::color::parse_color;
use super::numeric::{Range, components, length_percentage, split_commas};
use super::render_keywords_lowercase;
use crate::TuiColor;
use crate::layout::{BackgroundAttachment, BackgroundRepeat, RepeatStyle, VisualBox};
use crate::parse::token::Token;

/// One background layer (CSS Backgrounds 3 §3.10 `<bg-layer>`), every
/// sub-value at its initial value unless the layer gave it.
#[derive(Debug, Clone, PartialEq)]
pub struct BackgroundLayer {
    /// `background-image`: `none`, `url("…")` or a gradient, as CSS text.
    pub image: String,
    /// `background-position`, as CSS text.
    pub position: String,
    /// `background-size`, as CSS text.
    pub size: String,
    pub repeat: BackgroundRepeat,
    pub attachment: BackgroundAttachment,
    pub origin: VisualBox,
    pub clip: VisualBox,
}

/// The initial `background-image`.
pub(crate) const INITIAL_IMAGE: &str = "none";
/// The initial `background-position` (§3.6).
pub(crate) const INITIAL_POSITION: &str = "0% 0%";
/// The initial `background-size` (§3.9).
pub(crate) const INITIAL_SIZE: &str = "auto";
/// The initial `background-origin` (§3.7).
pub(crate) const INITIAL_ORIGIN: VisualBox = VisualBox::PaddingBox;
/// The initial `background-clip` (§3.8).
pub(crate) const INITIAL_CLIP: VisualBox = VisualBox::BorderBox;

impl Default for BackgroundLayer {
    fn default() -> Self {
        Self {
            image: INITIAL_IMAGE.to_string(),
            position: INITIAL_POSITION.to_string(),
            size: INITIAL_SIZE.to_string(),
            repeat: BackgroundRepeat::default(),
            attachment: BackgroundAttachment::default(),
            origin: INITIAL_ORIGIN,
            clip: INITIAL_CLIP,
        }
    }
}

/// A parsed `background` shorthand: its layers, bottom-most last, and
/// the color the final layer gave (`None`: `transparent`, the initial
/// value).
#[derive(Debug, Clone, PartialEq)]
pub struct BackgroundShorthand {
    pub layers: Vec<BackgroundLayer>,
    pub color: Option<TuiColor>,
}

/// `background` (CSS Backgrounds 3 §3.10): `<bg-layer>#? ,
/// <final-bg-layer>` — each layer any of an image, a position with an
/// optional `/ <bg-size>`, a repeat style, an attachment and one or two
/// boxes (one box is the origin and the clip, two are origin then
/// clip), in any order, each at most once; only the final layer may
/// carry a color.
pub fn parse_background(value: &[Token]) -> Option<BackgroundShorthand> {
    let segments = split_commas(value)?;
    let last = segments.len() - 1;
    let mut layers = Vec::with_capacity(segments.len());
    let mut color = None;
    for (i, segment) in segments.into_iter().enumerate() {
        let (layer, c) = parse_layer(segment, i == last)?;
        layers.push(layer);
        if c.is_some() {
            color = c;
        }
    }
    Some(BackgroundShorthand { layers, color })
}

/// One `<bg-layer>` (or, when `final_layer`, `<final-bg-layer>`).
fn parse_layer(tokens: &[Token], final_layer: bool) -> Option<(BackgroundLayer, Option<TuiColor>)> {
    let parts = components(tokens)?;
    let mut layer = BackgroundLayer::default();
    let (mut image, mut position, mut repeat, mut attachment) = (false, false, false, false);
    let mut boxes: Vec<VisualBox> = Vec::new();
    let mut color = None;
    let mut i = 0;
    while i < parts.len() {
        let part = parts[i];
        if !image && let Some(text) = image_text(part) {
            layer.image = text;
            image = true;
            i += 1;
            continue;
        }
        if !position && let Some((text, used)) = longest_position(&parts[i..]) {
            layer.position = text;
            position = true;
            i += used;
            // `<bg-position> [ / <bg-size> ]?`
            if matches!(parts.get(i), Some([Token::Delim('/')])) {
                let (size, used) = longest_size(&parts[i + 1..])?;
                layer.size = size;
                i += 1 + used;
            }
            continue;
        }
        if !repeat && let Some((r, used)) = leading_repeat(&parts[i..]) {
            layer.repeat = r;
            repeat = true;
            i += used;
            continue;
        }
        if !attachment && let Some(a) = attachment_keyword(part) {
            layer.attachment = a;
            attachment = true;
            i += 1;
            continue;
        }
        if boxes.len() < 2
            && let Some(b) = visual_box(part)
        {
            boxes.push(b);
            i += 1;
            continue;
        }
        if final_layer
            && color.is_none()
            && let Some(c) = parse_color(part)
        {
            color = Some(c);
            i += 1;
            continue;
        }
        return None;
    }
    match boxes.as_slice() {
        [] => {}
        [b] => (layer.origin, layer.clip) = (*b, *b),
        [o, c] => (layer.origin, layer.clip) = (*o, *c),
        _ => return None,
    }
    Some((layer, color))
}

/// `background-image`: `<bg-image>#`.
pub fn parse_background_image(value: &[Token]) -> Option<Vec<String>> {
    each_layer(value, |seg| match components(seg)?.as_slice() {
        [one] => image_text(one),
        _ => None,
    })
}

/// `background-position`: `<bg-position>#`.
pub fn parse_background_position(value: &[Token]) -> Option<Vec<String>> {
    each_layer(value, |seg| {
        let parts = components(seg)?;
        position_text(&parts)
    })
}

/// `background-size`: `<bg-size>#`.
pub fn parse_background_size(value: &[Token]) -> Option<Vec<String>> {
    each_layer(value, |seg| {
        let parts = components(seg)?;
        size_text(&parts)
    })
}

/// `background-repeat`: `<repeat-style>#`.
pub fn parse_background_repeat(value: &[Token]) -> Option<Vec<BackgroundRepeat>> {
    each_layer(value, |seg| {
        let parts = components(seg)?;
        match leading_repeat(&parts)? {
            (r, used) if used == parts.len() => Some(r),
            _ => None,
        }
    })
}

/// `background-attachment`: `<attachment>#`.
pub fn parse_background_attachment(value: &[Token]) -> Option<Vec<BackgroundAttachment>> {
    each_layer(value, attachment_keyword)
}

/// `background-origin` / `background-clip`: `<visual-box>#`.
/// `background-clip: text` (Backgrounds 4) is not accepted: a cell
/// cannot show a background through a glyph's shape (DIVERGENCES §1).
pub fn parse_visual_box_list(value: &[Token]) -> Option<Vec<VisualBox>> {
    each_layer(value, visual_box)
}

/// Parse every comma-separated layer of `value` with `one`.
fn each_layer<T>(value: &[Token], one: impl Fn(&[Token]) -> Option<T>) -> Option<Vec<T>> {
    split_commas(value)?.into_iter().map(one).collect()
}

/// `<bg-image>` (§3.3): `none` or an `<image>` — `url()` or a gradient
/// function — as CSS text; a URL serializes as a string (`url("a.png")`,
/// CSSOM §6.7.2).
fn image_text(part: &[Token]) -> Option<String> {
    match part {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("none") => Some(INITIAL_IMAGE.to_string()),
        // An unquoted URL is one `<url-token>`, its text raw (CSS Syntax
        // 3 §4.3.6); a quoted one a `url(` function holding a string.
        [Token::Url(url)] => Some(url_text(url)),
        [Token::Function(f), Token::String(url), Token::RParen]
            if f.eq_ignore_ascii_case("url") =>
        {
            Some(url_text(url))
        }
        [Token::Function(f), .., Token::RParen] if is_gradient(f) => {
            Some(render_keywords_lowercase(part))
        }
        _ => None,
    }
}

/// A URL as CSSOM serializes it, a string (`url("a.png")`, CSSOM §6.7.2).
fn url_text(url: &str) -> String {
    format!("url({})", rdom_core::css_syntax::serialize_string(url))
}

/// The CSS Images 3 gradient functions.
fn is_gradient(name: &str) -> bool {
    [
        "linear-gradient",
        "radial-gradient",
        "conic-gradient",
        "repeating-linear-gradient",
        "repeating-radial-gradient",
        "repeating-conic-gradient",
    ]
    .iter()
    .any(|g| g.eq_ignore_ascii_case(name))
}

/// The longest prefix of `parts` (at most four components) that is a
/// `<bg-position>`, as CSS text, and how many components it took.
fn longest_position(parts: &[&[Token]]) -> Option<(String, usize)> {
    (1..=parts.len().min(4))
        .rev()
        .find_map(|n| position_text(&parts[..n]).map(|t| (t, n)))
}

/// What a `<bg-position>` component is.
#[derive(Clone, Copy, PartialEq)]
enum Pos {
    Left,
    Right,
    Top,
    Bottom,
    Center,
    Offset,
}

fn pos_component(part: &[Token]) -> Option<Pos> {
    if let [Token::Ident(s)] = part {
        return [
            ("left", Pos::Left),
            ("right", Pos::Right),
            ("top", Pos::Top),
            ("bottom", Pos::Bottom),
            ("center", Pos::Center),
        ]
        .iter()
        .find(|(k, _)| s.eq_ignore_ascii_case(k))
        .map(|(_, p)| *p);
    }
    length_percentage(part, Range::Any).map(|_| Pos::Offset)
}

/// `<bg-position>` (CSS Backgrounds 3 §3.6, the `<position>` of Values
/// 4 §9.1 plus the three-value form) over exactly `parts`, as CSS text.
fn position_text(parts: &[&[Token]]) -> Option<String> {
    use Pos::*;
    let p: Vec<Pos> = parts
        .iter()
        .map(|c| pos_component(c))
        .collect::<Option<_>>()?;
    let horizontal = |x: Pos| matches!(x, Left | Center | Right | Offset);
    let vertical = |y: Pos| matches!(y, Top | Center | Bottom | Offset);
    let valid = match p.as_slice() {
        [_] => true,
        [x, y] => {
            (horizontal(*x) && vertical(*y))
                || (matches!(x, Top | Bottom | Center) && matches!(y, Left | Right | Center))
        }
        // `[ center | [ left | right ] <lp>? ] && [ center | [ top |
        // bottom ] <lp>? ]`: two keyword groups, one per axis.
        _ => keyword_offset_groups(&p),
    };
    valid.then(|| join_components(parts))
}

/// The three- and four-value `<bg-position>`: two groups, each `center`
/// or a side keyword with an optional offset, one horizontal and one
/// vertical, at least one with an offset.
fn keyword_offset_groups(p: &[Pos]) -> bool {
    use Pos::*;
    let mut groups: Vec<(Pos, bool)> = Vec::new();
    let mut i = 0;
    while i < p.len() {
        let side = p[i];
        let offset = side != Center && p.get(i + 1) == Some(&Offset);
        if side == Offset {
            return false;
        }
        groups.push((side, offset));
        i += 1 + usize::from(offset);
    }
    let axis = |s: Pos| match s {
        Left | Right => Some(true),
        Top | Bottom => Some(false),
        _ => None,
    };
    match groups.as_slice() {
        [(a, _), (b, _)] => match (axis(*a), axis(*b)) {
            (Some(x), Some(y)) => x != y,
            // `center` with a side of either axis.
            _ => true,
        },
        _ => false,
    }
}

/// The longest prefix of `parts` (one or two components) that is a
/// `<bg-size>`.
fn longest_size(parts: &[&[Token]]) -> Option<(String, usize)> {
    (1..=parts.len().min(2))
        .rev()
        .find_map(|n| size_text(&parts[..n]).map(|t| (t, n)))
}

/// `<bg-size>` (§3.9): `[ <length-percentage [0,∞]> | auto ]{1,2} |
/// cover | contain`, as CSS text.
fn size_text(parts: &[&[Token]]) -> Option<String> {
    let keyword =
        |c: &[Token], k: &str| matches!(c, [Token::Ident(s)] if s.eq_ignore_ascii_case(k));
    let one =
        |c: &[Token]| keyword(c, "auto") || length_percentage(c, Range::NonNegative).is_some();
    let valid = match parts {
        [c] => keyword(c, "cover") || keyword(c, "contain") || one(c),
        [a, b] => one(a) && one(b),
        _ => false,
    };
    valid.then(|| join_components(parts))
}

/// A `<repeat-style>` (§3.4) at the start of `parts`: `repeat-x`,
/// `repeat-y`, or one or two of `repeat | space | round | no-repeat`
/// (one value is both axes).
fn leading_repeat(parts: &[&[Token]]) -> Option<(BackgroundRepeat, usize)> {
    let ident = |c: &[Token]| match c {
        [Token::Ident(s)] => Some(s.to_ascii_lowercase()),
        _ => None,
    };
    let first = ident(parts.first()?)?;
    match first.as_str() {
        "repeat-x" => {
            return Some((
                BackgroundRepeat {
                    x: RepeatStyle::Repeat,
                    y: RepeatStyle::NoRepeat,
                },
                1,
            ));
        }
        "repeat-y" => {
            return Some((
                BackgroundRepeat {
                    x: RepeatStyle::NoRepeat,
                    y: RepeatStyle::Repeat,
                },
                1,
            ));
        }
        _ => {}
    }
    let style = |s: &str| match s {
        "repeat" => Some(RepeatStyle::Repeat),
        "space" => Some(RepeatStyle::Space),
        "round" => Some(RepeatStyle::Round),
        "no-repeat" => Some(RepeatStyle::NoRepeat),
        _ => None,
    };
    let x = style(&first)?;
    match parts.get(1).and_then(|c| ident(c)).and_then(|s| style(&s)) {
        Some(y) => Some((BackgroundRepeat { x, y }, 2)),
        None => Some((BackgroundRepeat { x, y: x }, 1)),
    }
}

fn attachment_keyword(part: &[Token]) -> Option<BackgroundAttachment> {
    super::keyword::parse_keyword(
        part,
        &[
            ("scroll", BackgroundAttachment::Scroll),
            ("fixed", BackgroundAttachment::Fixed),
            ("local", BackgroundAttachment::Local),
        ],
    )
}

fn visual_box(part: &[Token]) -> Option<VisualBox> {
    super::keyword::parse_keyword(
        part,
        &[
            ("border-box", VisualBox::BorderBox),
            ("padding-box", VisualBox::PaddingBox),
            ("content-box", VisualBox::ContentBox),
        ],
    )
}

/// Component values as CSS text, one space apart, each rendered by
/// [`render_keywords_lowercase`] (which keeps a sign on its number,
/// `-5%`, and lowercases the keywords, CSSOM §6.7.2).
fn join_components(parts: &[&[Token]]) -> String {
    parts
        .iter()
        .map(|c| render_keywords_lowercase(c))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
#[path = "background_tests.rs"]
mod tests;
