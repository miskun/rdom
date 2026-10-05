//! Grid track lists (CSS Grid Layout 2 §7.2): `grid-template-columns`
//! / `grid-template-rows` — `none | <track-list> | <auto-track-list>` —
//! and the shortest serialization back, in the form browsers serialize
//! a specified value (`repeat()` kept, line names in brackets).

use super::numeric::{LengthPercentage, Range, cells_u16, components, length_percentage};
use crate::calc::CalcExpr;
use crate::layout::{
    GridTemplate, LineNameItem, LineNameList, RepeatCount, TrackBreadth, TrackList, TrackListItem,
    TrackRepeat, TrackSize,
};
use crate::parse::token::Token;

/// Parse `grid-template-columns` / `grid-template-rows` (§7.2):
/// `none`, or a track list — one with an automatic repetition must have
/// exactly one, and only fixed sizes (`<auto-track-list>`, §7.2.3.1).
pub fn parse_grid_template(value: &[Token]) -> Option<GridTemplate> {
    if let [Token::Ident(s)] = value
        && s.eq_ignore_ascii_case("none")
    {
        return Some(GridTemplate::None);
    }
    if let [Token::Ident(s), rest @ ..] = value
        && s.eq_ignore_ascii_case("subgrid")
    {
        return parse_line_name_list(rest).map(GridTemplate::Subgrid);
    }
    let list = parse_track_list(value)?;
    list.is_valid().then_some(GridTemplate::Tracks(list))
}

/// `<line-name-list>?` (§9): `[ <line-names> | <name-repeat> ]*` after
/// `subgrid`.
fn parse_line_name_list(value: &[Token]) -> Option<LineNameList> {
    if value.is_empty() {
        return Some(LineNameList::default());
    }
    let parts = components(value)?;
    let mut list = LineNameList::default();
    let mut i = 0;
    while i < parts.len() {
        if let Some((read, names)) = line_names(&parts[i..]) {
            list.items.push(LineNameItem::Names(names));
            i += read;
            continue;
        }
        let [Token::Function(f), inner @ .., Token::RParen] = parts[i] else {
            return None;
        };
        if !f.eq_ignore_ascii_case("repeat") {
            return None;
        }
        list.items.push(parse_name_repeat(inner)?);
        i += 1;
    }
    list.is_valid().then_some(list)
}

/// `<name-repeat>`'s arguments (§9): `[ <integer [1,∞]> | auto-fill ] ,
/// <line-names>+`.
fn parse_name_repeat(inner: &[Token]) -> Option<LineNameItem> {
    let [count, lines] = super::numeric::split_commas(inner)?[..] else {
        return None;
    };
    let count = match count {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("auto-fill") => RepeatCount::AutoFill,
        _ => {
            let n = super::numeric::integer(count)?;
            if n < 1 {
                return None;
            }
            RepeatCount::Count(u32::try_from(n).unwrap_or(u32::MAX))
        }
    };
    let parts = components(lines)?;
    let mut names = Vec::new();
    let mut i = 0;
    while i < parts.len() {
        let (read, n) = line_names(&parts[i..])?;
        names.push(n);
        i += read;
    }
    (!names.is_empty()).then_some(LineNameItem::Repeat { count, names })
}

/// `[ <line-names>? [ <track-size> | <track-repeat> ] ]+ <line-names>?`:
/// the components with the names between them. Two `<line-names>` in a
/// row, or a list with no track, is invalid.
pub(crate) fn parse_track_list(value: &[Token]) -> Option<TrackList> {
    let parts = components(value)?;
    let mut list = TrackList {
        line_names: Vec::new(),
        items: Vec::new(),
    };
    let mut names: Option<Vec<String>> = None;
    let mut i = 0;
    while i < parts.len() {
        if let Some((read, n)) = line_names(&parts[i..]) {
            if names.is_some() {
                return None;
            }
            names = Some(n);
            i += read;
            continue;
        }
        let item = match parts[i] {
            [Token::Function(f), inner @ .., Token::RParen] if f.eq_ignore_ascii_case("repeat") => {
                TrackListItem::Repeat(parse_repeat(inner)?)
            }
            part => TrackListItem::Size(parse_track_size(part)?),
        };
        list.line_names.push(names.take().unwrap_or_default());
        list.items.push(item);
        i += 1;
    }
    list.line_names.push(names.take().unwrap_or_default());
    (!list.items.is_empty()).then_some(list)
}

/// `repeat( <integer [1,∞]> | auto-fill | auto-fit , [ <line-names>?
/// <track-size> ]+ <line-names>? )` (§7.2.3). The integer is clamped to
/// `u32` (layout clamps the grid itself).
fn parse_repeat(inner: &[Token]) -> Option<TrackRepeat> {
    let [count, tracks] = super::numeric::split_commas(inner)?[..] else {
        return None;
    };
    let count = match count {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("auto-fill") => RepeatCount::AutoFill,
        [Token::Ident(s)] if s.eq_ignore_ascii_case("auto-fit") => RepeatCount::AutoFit,
        _ => {
            let n = super::numeric::integer(count)?;
            if n < 1 {
                return None;
            }
            RepeatCount::Count(u32::try_from(n).unwrap_or(u32::MAX))
        }
    };
    let parts = components(tracks)?;
    let mut repeat = TrackRepeat {
        count,
        line_names: Vec::new(),
        sizes: Vec::new(),
    };
    let mut names: Option<Vec<String>> = None;
    let mut i = 0;
    while i < parts.len() {
        if let Some((read, n)) = line_names(&parts[i..]) {
            if names.is_some() {
                return None;
            }
            names = Some(n);
            i += read;
            continue;
        }
        repeat.line_names.push(names.take().unwrap_or_default());
        repeat.sizes.push(parse_track_size(parts[i])?);
        i += 1;
    }
    repeat.line_names.push(names.take().unwrap_or_default());
    (!repeat.sizes.is_empty()).then_some(repeat)
}

/// `'[' <custom-ident>* ']'` at the start of `parts`: how many
/// components it takes and the names. `None` when `parts` does not start
/// with `[`, or the brackets hold anything but valid identifiers.
pub(crate) fn line_names(parts: &[&[Token]]) -> Option<(usize, Vec<String>)> {
    if parts.first()? != &[Token::Delim('[')] {
        return None;
    }
    let mut names = Vec::new();
    for (k, part) in parts.iter().enumerate().skip(1) {
        match part {
            [Token::Delim(']')] => return Some((k + 1, names)),
            [Token::Ident(name)] if is_line_name(name) => names.push(name.clone()),
            _ => return None,
        }
    }
    None
}

/// A `<custom-ident>` a line can be named (§7.2: excluding `span` and
/// `auto`; CSS Values 4 §4.2 excludes the CSS-wide keywords and
/// `default`), ASCII case-insensitively.
pub(crate) fn is_line_name(name: &str) -> bool {
    const RESERVED: &[&str] = &[
        "span",
        "auto",
        "initial",
        "inherit",
        "unset",
        "revert",
        "revert-layer",
        "default",
    ];
    !RESERVED.iter().any(|r| name.eq_ignore_ascii_case(r))
}

/// `grid-auto-columns` / `grid-auto-rows` (§7.6): `<track-size>+`.
pub fn parse_track_sizes(value: &[Token]) -> Option<Vec<TrackSize>> {
    let sizes = components(value)?
        .into_iter()
        .map(parse_track_size)
        .collect::<Option<Vec<_>>>()?;
    (!sizes.is_empty()).then_some(sizes)
}

/// The text of a `<track-size>+` list.
pub fn serialize_track_sizes(sizes: &[TrackSize]) -> String {
    sizes
        .iter()
        .map(serialize_track_size)
        .collect::<Vec<_>>()
        .join(" ")
}

/// `<track-size>` (§7.2.2): `<track-breadth> | minmax(
/// <inflexible-breadth> , <track-breadth> ) | fit-content(
/// <length-percentage [0,∞]> )`.
pub fn parse_track_size(value: &[Token]) -> Option<TrackSize> {
    match value {
        [Token::Function(f), inner @ .., Token::RParen] if f.eq_ignore_ascii_case("minmax") => {
            let [min, max] = super::numeric::split_commas(inner)?[..] else {
                return None;
            };
            let min = parse_breadth(min).filter(|b| b.flex().is_none())?;
            Some(TrackSize::MinMax(min, parse_breadth(max)?))
        }
        [Token::Function(f), inner @ .., Token::RParen]
            if f.eq_ignore_ascii_case("fit-content") =>
        {
            let limit = parse_breadth(inner).filter(TrackBreadth::is_fixed)?;
            Some(TrackSize::FitContent(limit))
        }
        _ => parse_breadth(value).map(TrackSize::Breadth),
    }
}

/// `<track-breadth>` (§7.2.1): `<length-percentage [0,∞]> | <flex [0,∞]>
/// | min-content | max-content | auto`.
fn parse_breadth(value: &[Token]) -> Option<TrackBreadth> {
    match value {
        [Token::Ident(s)] if s.eq_ignore_ascii_case("auto") => Some(TrackBreadth::Auto),
        [Token::Ident(s)] if s.eq_ignore_ascii_case("min-content") => {
            Some(TrackBreadth::MinContent)
        }
        [Token::Ident(s)] if s.eq_ignore_ascii_case("max-content") => {
            Some(TrackBreadth::MaxContent)
        }
        [Token::Dimension { value, unit, .. }] if unit.eq_ignore_ascii_case("fr") => {
            (*value >= 0.0 && *value <= f64::from(f32::MAX))
                .then_some(TrackBreadth::Fr(*value as f32))
        }
        _ => Some(match length_percentage(value, Range::NonNegative)? {
            LengthPercentage::Integer(n) => TrackBreadth::Cells(u16::try_from(n).ok()?),
            LengthPercentage::Cells(v) => TrackBreadth::Cells(cells_u16(v)),
            LengthPercentage::Expr(CalcExpr::Percent(p)) => {
                (p >= 0.0 && p <= f64::from(u16::MAX)).then_some(TrackBreadth::Percent(p as f32))?
            }
            LengthPercentage::Expr(e) => TrackBreadth::Calc(Box::new(e)),
        }),
    }
}

/// The text of a `grid-template-*` value (CSSOM §6.7.2): `none`, or the
/// track list as written — line names in brackets, `repeat()` kept.
pub fn serialize_grid_template(value: &GridTemplate) -> String {
    match value {
        GridTemplate::Tracks(list) => serialize_track_list(list),
        GridTemplate::Subgrid(names) => serialize_line_name_list(names),
        GridTemplate::None => "none".to_string(),
    }
}

/// `subgrid` and its line names as written.
fn serialize_line_name_list(list: &LineNameList) -> String {
    let names = |n: &[String]| format!("[{}]", n.join(" "));
    let mut out = vec!["subgrid".to_string()];
    for item in &list.items {
        out.push(match item {
            LineNameItem::Names(n) => names(n),
            LineNameItem::Repeat {
                count,
                names: lines,
            } => {
                let count = match count {
                    RepeatCount::Count(n) => n.to_string(),
                    _ => "auto-fill".to_string(),
                };
                let lines: Vec<String> = lines.iter().map(|n| names(n)).collect();
                format!("repeat({count}, {})", lines.join(" "))
            }
        });
    }
    out.join(" ")
}

fn serialize_track_list(list: &TrackList) -> String {
    let mut out = Vec::with_capacity(list.items.len() * 2 + 1);
    for (k, item) in list.items.iter().enumerate() {
        push_names(&mut out, list.line_names.get(k));
        out.push(match item {
            TrackListItem::Size(s) => serialize_track_size(s),
            TrackListItem::Repeat(r) => serialize_repeat(r),
        });
    }
    push_names(&mut out, list.line_names.get(list.items.len()));
    out.join(" ")
}

fn serialize_repeat(r: &TrackRepeat) -> String {
    let count = match r.count {
        RepeatCount::Count(n) => n.to_string(),
        RepeatCount::AutoFill => "auto-fill".to_string(),
        RepeatCount::AutoFit => "auto-fit".to_string(),
    };
    let mut out = Vec::with_capacity(r.sizes.len() * 2 + 1);
    for (k, size) in r.sizes.iter().enumerate() {
        push_names(&mut out, r.line_names.get(k));
        out.push(serialize_track_size(size));
    }
    push_names(&mut out, r.line_names.get(r.sizes.len()));
    format!("repeat({count}, {})", out.join(" "))
}

/// `[a b]`, when the line has names.
fn push_names(out: &mut Vec<String>, names: Option<&Vec<String>>) {
    if let Some(names) = names.filter(|n| !n.is_empty()) {
        out.push(format!("[{}]", names.join(" ")));
    }
}

/// The text of a `<track-size>`.
pub fn serialize_track_size(size: &TrackSize) -> String {
    match size {
        TrackSize::Breadth(b) => serialize_breadth(b),
        TrackSize::MinMax(min, max) => {
            format!(
                "minmax({}, {})",
                serialize_breadth(min),
                serialize_breadth(max)
            )
        }
        TrackSize::FitContent(limit) => format!("fit-content({})", serialize_breadth(limit)),
    }
}

fn serialize_breadth(b: &TrackBreadth) -> String {
    match b {
        TrackBreadth::Cells(n) => n.to_string(),
        TrackBreadth::Percent(p) => format!("{p}%"),
        TrackBreadth::Calc(e) => crate::property_dispatch::serialize_math(e),
        TrackBreadth::Fr(f) => format!("{f}fr"),
        TrackBreadth::MinContent => "min-content".to_string(),
        TrackBreadth::MaxContent => "max-content".to_string(),
        TrackBreadth::Auto => "auto".to_string(),
    }
}

#[cfg(test)]
#[path = "grid_tests.rs"]
mod tests;
