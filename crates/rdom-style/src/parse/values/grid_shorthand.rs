//! The grid shorthands (CSS Grid Layout 2): `grid-template` (§7.4), the
//! explicit grid in one declaration, and `grid` (§7.8), which adds the
//! implicit grid's properties — with the shortest serializations back,
//! in the forms browsers give.

use super::grid::{parse_track_list, serialize_track_size};
use super::grid_areas::serialize_area_row;
use super::grid_placement::split_slashes;
use super::numeric::components;
use super::{parse_grid_template, parse_track_sizes, serialize_grid_template};
use crate::layout::{
    GridAutoFlow, GridTemplate, GridTemplateAreas, TrackList, TrackListItem, TrackSize,
};
use crate::parse::token::Token;

/// The three longhands `grid-template` sets (§7.4).
#[derive(Debug, Clone, PartialEq)]
pub struct GridTemplateShorthand {
    pub rows: GridTemplate,
    pub columns: GridTemplate,
    pub areas: GridTemplateAreas,
}

impl GridTemplateShorthand {
    /// `none`: every longhand `none`.
    pub const NONE: Self = Self {
        rows: GridTemplate::None,
        columns: GridTemplate::None,
        areas: GridTemplateAreas::NONE,
    };
}

/// The six longhands `grid` sets (§7.8); it does not reset the gutters.
#[derive(Debug, Clone, PartialEq)]
pub struct GridShorthand {
    pub template: GridTemplateShorthand,
    pub auto_rows: Vec<TrackSize>,
    pub auto_columns: Vec<TrackSize>,
    pub auto_flow: GridAutoFlow,
}

/// Parse `grid-template` (§7.4): `none | [ <'grid-template-rows'> /
/// <'grid-template-columns'> ] | [ <line-names>? <string> <track-size>?
/// <line-names>? ]+ [ / <explicit-track-list> ]?`. In the last form each
/// string is a row of `grid-template-areas` and its size a row of
/// `grid-template-rows` (`auto` when omitted), the names after one row
/// and before the next naming the same line; the columns are `none`
/// when omitted.
pub fn parse_grid_template_shorthand(value: &[Token]) -> Option<GridTemplateShorthand> {
    if let [Token::Ident(s)] = value
        && s.eq_ignore_ascii_case("none")
    {
        return Some(GridTemplateShorthand::NONE);
    }
    let parts = split_slashes(value)?;
    if value.iter().any(|t| matches!(t, Token::String(_))) {
        let (rows, areas) = parse_area_rows(parts[0])?;
        let columns = match parts[1..] {
            [] => GridTemplate::None,
            [columns] => GridTemplate::Tracks(parse_explicit_track_list(columns)?),
            _ => return None,
        };
        return Some(GridTemplateShorthand {
            rows: GridTemplate::Tracks(rows),
            columns,
            areas,
        });
    }
    let [rows, columns] = parts[..] else {
        return None;
    };
    Some(GridTemplateShorthand {
        rows: parse_grid_template(rows)?,
        columns: parse_grid_template(columns)?,
        areas: GridTemplateAreas::NONE,
    })
}

/// `[ <line-names>? <string> <track-size>? <line-names>? ]+`: the rows'
/// track list and the areas their strings mark out.
fn parse_area_rows(value: &[Token]) -> Option<(TrackList, GridTemplateAreas)> {
    let parts = components(value)?;
    let mut list = TrackList {
        line_names: vec![Vec::new()],
        items: Vec::new(),
    };
    let mut strings = Vec::new();
    let mut i = 0;
    while i < parts.len() {
        // Names before the string join the line the previous row's
        // trailing names are on.
        if let Some((read, names)) = super::grid::line_names(&parts[i..]) {
            list.line_names.last_mut()?.extend(names);
            i += read;
        }
        let [Token::String(s)] = parts.get(i)? else {
            return None;
        };
        strings.push(s.as_str());
        i += 1;
        let size = match parts.get(i) {
            Some(part) if !matches!(part, [Token::String(_)] | [Token::Delim('[')]) => {
                i += 1;
                super::parse_track_size(part)?
            }
            _ => TrackSize::AUTO,
        };
        list.items.push(TrackListItem::Size(size));
        list.line_names.push(Vec::new());
        if let Some((read, names)) = super::grid::line_names(&parts[i..]) {
            list.line_names.last_mut()?.extend(names);
            i += read;
        }
    }
    let areas = GridTemplateAreas::new(strings)?;
    Some((list, areas))
}

/// `<explicit-track-list>` (§7.2): a track list without `repeat()`.
fn parse_explicit_track_list(value: &[Token]) -> Option<TrackList> {
    let list = parse_track_list(value)?;
    (list.is_valid()
        && list
            .items
            .iter()
            .all(|i| matches!(i, TrackListItem::Size(_))))
    .then_some(list)
}

/// The shortest text of `grid-template` for its longhands (CSSOM
/// §6.7.2), or `None` when no form of the shorthand gives them back:
/// `none` when all three are; `<rows> / <columns>` without areas; with
/// areas, each row's names, string and size (`auto` left out) and then
/// `/ <columns>` unless the columns are `none` — possible only when the
/// rows are a plain list of one size per area row and neither axis
/// repeats.
#[deny(clippy::wildcard_enum_match_arm)]
pub fn serialize_grid_template_shorthand(t: &GridTemplateShorthand) -> Option<String> {
    if t.areas.is_none() {
        if t.rows == GridTemplate::None && t.columns == GridTemplate::None {
            return Some("none".to_string());
        }
        return Some(format!(
            "{} / {}",
            serialize_grid_template(&t.rows),
            serialize_grid_template(&t.columns)
        ));
    }
    let rows = t.rows.tracks()?;
    let plain = |list: &TrackList| {
        list.items
            .iter()
            .all(|i| matches!(i, TrackListItem::Size(_)))
    };
    if rows.items.len() != t.areas.row_count() || !plain(rows) {
        return None;
    }
    let names = |k: usize, out: &mut Vec<String>| {
        if let Some(n) = rows.line_names.get(k).filter(|n| !n.is_empty()) {
            out.push(format!("[{}]", n.join(" ")));
        }
    };
    let mut out = Vec::new();
    names(0, &mut out);
    for (k, item) in rows.items.iter().enumerate() {
        out.push(serialize_area_row(&t.areas, k));
        if let TrackListItem::Size(size) = item
            && *size != TrackSize::AUTO
        {
            out.push(serialize_track_size(size));
        }
        names(k + 1, &mut out);
    }
    match &t.columns {
        GridTemplate::None => {}
        GridTemplate::Tracks(columns) if plain(columns) => {
            out.push("/".to_string());
            out.push(serialize_grid_template(&t.columns));
        }
        GridTemplate::Tracks(_) | GridTemplate::Subgrid(_) => return None,
    }
    Some(out.join(" "))
}

/// Parse `grid` (§7.8): `<'grid-template'> | <'grid-template-rows'> / [
/// auto-flow && dense? ] <'grid-auto-columns'>? | [ auto-flow && dense?
/// ] <'grid-auto-rows'>? / <'grid-template-columns'>`. Every longhand it
/// does not name is reset to its initial value: the `auto-flow` forms
/// set the other axis's template, the areas and the other implicit size
/// to theirs.
pub fn parse_grid_shorthand(value: &[Token]) -> Option<GridShorthand> {
    let initial = |template| GridShorthand {
        template,
        auto_rows: vec![TrackSize::AUTO],
        auto_columns: vec![TrackSize::AUTO],
        auto_flow: GridAutoFlow::ROW,
    };
    if let Some(template) = parse_grid_template_shorthand(value) {
        return Some(initial(template));
    }
    let [first, second] = split_slashes(value)?[..] else {
        return None;
    };
    if let Some((dense, sizes)) = auto_flow(second) {
        let mut out = initial(GridTemplateShorthand {
            rows: parse_grid_template(first)?,
            ..GridTemplateShorthand::NONE
        });
        out.auto_flow = flow(GridAutoFlow::COLUMN, dense);
        if let Some(sizes) = sizes {
            out.auto_columns = sizes;
        }
        return Some(out);
    }
    let (dense, sizes) = auto_flow(first)?;
    let mut out = initial(GridTemplateShorthand {
        columns: parse_grid_template(second)?,
        ..GridTemplateShorthand::NONE
    });
    out.auto_flow = flow(GridAutoFlow::ROW, dense);
    if let Some(sizes) = sizes {
        out.auto_rows = sizes;
    }
    Some(out)
}

fn flow(direction: GridAutoFlow, dense: bool) -> GridAutoFlow {
    if dense { direction.dense() } else { direction }
}

/// `[ auto-flow && dense? ] <track-size>*`: whether `dense` is given and
/// the sizes, if any. `None` when `value` does not start with the
/// keywords.
fn auto_flow(value: &[Token]) -> Option<(bool, Option<Vec<TrackSize>>)> {
    let parts = components(value)?;
    let keyword = |k: usize, word: &str| matches!(parts.get(k), Some([Token::Ident(s)]) if s.eq_ignore_ascii_case(word));
    let (dense, read) = match (keyword(0, "auto-flow"), keyword(0, "dense")) {
        (true, _) if keyword(1, "dense") => (true, 2),
        (true, _) => (false, 1),
        (false, true) if keyword(1, "auto-flow") => (true, 2),
        _ => return None,
    };
    let rest: Vec<Token> = parts[read..]
        .iter()
        .flat_map(|p| p.iter().cloned())
        .collect();
    if rest.is_empty() {
        return Some((dense, None));
    }
    Some((dense, Some(parse_track_sizes(&rest)?)))
}

/// The shortest text of `grid` for its longhands (CSSOM §6.7.2), or
/// `None` when no form gives them back: the `grid-template` form when
/// the implicit properties are initial; `<rows> / auto-flow [dense]
/// [<auto-columns>]` for a column flow with no columns, areas or
/// implicit row sizes; `auto-flow [dense] [<auto-rows>] / <columns>` for
/// a row flow with no rows, areas or implicit column sizes.
pub fn serialize_grid_shorthand(g: &GridShorthand) -> Option<String> {
    let auto = |sizes: &[TrackSize]| sizes == [TrackSize::AUTO];
    let keywords = |sizes: &[TrackSize]| {
        let mut out = String::from("auto-flow");
        if g.auto_flow.dense {
            out.push_str(" dense");
        }
        if !auto(sizes) {
            out.push(' ');
            out.push_str(&super::serialize_track_sizes(sizes));
        }
        out
    };
    let t = &g.template;
    if auto(&g.auto_rows) && auto(&g.auto_columns) && g.auto_flow == GridAutoFlow::ROW {
        return serialize_grid_template_shorthand(t);
    }
    if !t.areas.is_none() {
        return None;
    }
    let column_flow = g.auto_flow.direction == crate::layout::Direction::Column;
    if column_flow && t.columns == GridTemplate::None && auto(&g.auto_rows) {
        return Some(format!(
            "{} / {}",
            serialize_grid_template(&t.rows),
            keywords(&g.auto_columns)
        ));
    }
    if !column_flow && t.rows == GridTemplate::None && auto(&g.auto_columns) {
        return Some(format!(
            "{} / {}",
            keywords(&g.auto_rows),
            serialize_grid_template(&t.columns)
        ));
    }
    None
}
