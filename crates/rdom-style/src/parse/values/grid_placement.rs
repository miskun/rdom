//! Grid placement (CSS Grid 2 §8.3, §8.4, §7.7): `grid-row-start` /
//! `-end`, `grid-column-start` / `-end` (`<grid-line>`), their
//! shorthands `grid-row`, `grid-column` and `grid-area`, and
//! `grid-auto-flow`; with the shortest serializations back.

use super::grid::is_line_name;
use super::numeric::components;
use crate::layout::{Direction, GridAutoFlow, GridLine};
use crate::parse::token::Token;

/// Parse a `<grid-line>` (§8.3): `auto | <custom-ident> | [ <integer>
/// && <custom-ident>? ] | [ span && [ <integer [1,∞]> || <custom-ident>
/// ] ]`, keywords ASCII case-insensitive. The integer is never 0, a
/// span's never below one; it is clamped to `i32` (layout clamps the
/// grid itself).
pub fn parse_grid_line(value: &[Token]) -> Option<GridLine> {
    let parts = components(value)?;
    if let [[Token::Ident(s)]] = parts.as_slice()
        && s.eq_ignore_ascii_case("auto")
    {
        return Some(GridLine::Auto);
    }
    let (mut span, mut integer, mut name) = (false, None, None);
    for part in parts {
        match part {
            [Token::Ident(s)] if s.eq_ignore_ascii_case("span") && !span => span = true,
            [Token::Ident(s)] if name.is_none() && is_line_name(s) => name = Some(s.clone()),
            _ if integer.is_none() => {
                integer = Some(
                    super::numeric::integer(part)?.clamp(i64::from(i32::MIN), i64::from(i32::MAX))
                        as i32,
                )
            }
            _ => return None,
        }
    }
    match (span, integer, name) {
        (true, Some(n), name) if n >= 1 => Some(GridLine::Span {
            count: n as u32,
            name,
        }),
        (true, None, Some(name)) => Some(GridLine::Span {
            count: 1,
            name: Some(name),
        }),
        (false, Some(index), name) if index != 0 => Some(GridLine::Line { index, name }),
        (false, None, Some(name)) => Some(GridLine::Name(name)),
        _ => None,
    }
}

/// The shortest text of a `<grid-line>` (CSSOM §6.7.2): the integer
/// before the name, `span` first, a span of one to a name without its
/// count.
pub fn serialize_grid_line(line: &GridLine) -> String {
    match line {
        GridLine::Auto => "auto".to_string(),
        GridLine::Name(n) => n.clone(),
        GridLine::Line { index, name: None } => index.to_string(),
        GridLine::Line {
            index,
            name: Some(n),
        } => format!("{index} {n}"),
        GridLine::Span { count, name: None } => format!("span {count}"),
        GridLine::Span {
            count: 1,
            name: Some(n),
        } => format!("span {n}"),
        GridLine::Span {
            count,
            name: Some(n),
        } => format!("span {count} {n}"),
    }
}

/// Split a value at its top-level `/` delimiters. `None` when a part is
/// empty.
fn split_slashes(value: &[Token]) -> Option<Vec<&[Token]>> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (i, t) in value.iter().enumerate() {
        match t {
            Token::Function(_) | Token::LParen => depth += 1,
            Token::RParen => depth = depth.checked_sub(1)?,
            Token::Delim('/') if depth == 0 => {
                out.push(&value[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&value[start..]);
    out.iter().all(|s| !s.is_empty()).then_some(out)
}

/// The end an omitted shorthand component takes from its pair (§8.4):
/// a lone `<custom-ident>` itself, anything else `auto`.
fn omitted(pair: &GridLine) -> GridLine {
    match pair {
        GridLine::Name(n) => GridLine::Name(n.clone()),
        _ => GridLine::Auto,
    }
}

/// `grid-row` / `grid-column` (§8.4): `<grid-line> [ / <grid-line> ]?`,
/// as `(start, end)`.
pub fn parse_grid_line_pair(value: &[Token]) -> Option<(GridLine, GridLine)> {
    let parts = split_slashes(value)?;
    let lines = parts
        .into_iter()
        .map(parse_grid_line)
        .collect::<Option<Vec<_>>>()?;
    match lines.as_slice() {
        [start] => Some((start.clone(), omitted(start))),
        [start, end] => Some((start.clone(), end.clone())),
        _ => None,
    }
}

/// `grid-area` (§8.4): `<grid-line> [ / <grid-line> ]{0,3}`, as
/// `(row-start, column-start, row-end, column-end)`.
pub fn parse_grid_area(value: &[Token]) -> Option<[GridLine; 4]> {
    let parts = split_slashes(value)?;
    if parts.len() > 4 {
        return None;
    }
    let mut lines = parts
        .into_iter()
        .map(parse_grid_line)
        .collect::<Option<Vec<_>>>()?;
    // An omitted column-start copies row-start's ident; an omitted
    // row-end row-start's; an omitted column-end column-start's.
    if lines.len() < 2 {
        lines.push(omitted(&lines[0]));
    }
    if lines.len() < 3 {
        lines.push(omitted(&lines[0]));
    }
    if lines.len() < 4 {
        lines.push(omitted(&lines[1]));
    }
    lines.try_into().ok()
}

/// `grid-row` / `grid-column`'s shortest text: the end dropped when the
/// start alone gives it back.
pub fn serialize_grid_line_pair(start: &GridLine, end: &GridLine) -> String {
    if *end == omitted(start) {
        serialize_grid_line(start)
    } else {
        format!(
            "{} / {}",
            serialize_grid_line(start),
            serialize_grid_line(end)
        )
    }
}

/// `grid-area`'s shortest text: trailing components dropped while the
/// shorter form gives them back.
pub fn serialize_grid_area(area: &[GridLine; 4]) -> String {
    let [row_start, column_start, row_end, column_end] = area;
    let mut keep = 4;
    if *column_end == omitted(column_start) {
        keep = 3;
        if *row_end == omitted(row_start) {
            keep = 2;
            if *column_start == omitted(row_start) {
                keep = 1;
            }
        }
    }
    area[..keep]
        .iter()
        .map(serialize_grid_line)
        .collect::<Vec<_>>()
        .join(" / ")
}

/// `grid-auto-flow` (§7.7): `[ row | column ] || dense`.
pub fn parse_grid_auto_flow(value: &[Token]) -> Option<GridAutoFlow> {
    let (mut direction, mut dense) = (None, false);
    for t in value {
        match t {
            Token::Ident(s) if s.eq_ignore_ascii_case("row") && direction.is_none() => {
                direction = Some(Direction::Row)
            }
            Token::Ident(s) if s.eq_ignore_ascii_case("column") && direction.is_none() => {
                direction = Some(Direction::Column)
            }
            Token::Ident(s) if s.eq_ignore_ascii_case("dense") && !dense => dense = true,
            _ => return None,
        }
    }
    (direction.is_some() || dense).then_some(GridAutoFlow {
        direction: direction.unwrap_or(Direction::Row),
        dense,
    })
}

/// `grid-auto-flow`'s shortest text: `row` omitted beside `dense`.
pub fn serialize_grid_auto_flow(flow: GridAutoFlow) -> String {
    match (flow.direction, flow.dense) {
        (Direction::Row, false) => "row",
        (Direction::Row, true) => "dense",
        (Direction::Column, false) => "column",
        (Direction::Column, true) => "column dense",
    }
    .to_string()
}
