//! Grid item placement values (CSS Grid 2 §7.7, §8.3): a
//! `grid-row-start` / `-end` / `grid-column-start` / `-end` value
//! ([`GridLine`]) and `grid-auto-flow` ([`GridAutoFlow`]).

use super::Direction;

/// `<grid-line>` (CSS Grid 2 §8.3): where an item's grid area starts or
/// ends on one axis.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum GridLine {
    /// `auto`: auto-placement, or a span of one (§8.3).
    #[default]
    Auto,
    /// `<custom-ident>`: the line named `<ident>-start` / `-end` (a
    /// named area's edge, §7.3.2) when there is one, else the first line
    /// named `<ident>` (§8.3.1).
    Name(String),
    /// `<integer> && <custom-ident>?`: the `index`th line — or the
    /// `index`th line named `name` — counting from the end of the
    /// explicit grid when negative. Never 0.
    Line { index: i32, name: Option<String> },
    /// `span && [ <integer [1,∞]> || <custom-ident> ]`: a span of
    /// `count` tracks — or to the `count`th line named `name` — from the
    /// other edge. `count` is at least 1.
    Span { count: u32, name: Option<String> },
}

impl GridLine {
    /// The `index`th line (`3`, `-1`).
    pub fn line(index: i32) -> Self {
        GridLine::Line { index, name: None }
    }

    /// The line `name` (`a`).
    pub fn named(name: &str) -> Self {
        GridLine::Name(name.to_string())
    }

    /// The `index`th line named `name` (`2 a`).
    pub fn nth_named(index: i32, name: &str) -> Self {
        GridLine::Line {
            index,
            name: Some(name.to_string()),
        }
    }

    /// `span <count>`.
    pub fn span(count: u32) -> Self {
        GridLine::Span { count, name: None }
    }

    /// `span <count> <name>` (`span a` is a count of 1).
    pub fn span_named(count: u32, name: &str) -> Self {
        GridLine::Span {
            count,
            name: Some(name.to_string()),
        }
    }

    /// The `<custom-ident>` of a lone name (§8.4: the one an omitted
    /// shorthand end copies).
    pub fn ident(&self) -> Option<&str> {
        match self {
            GridLine::Name(n) => Some(n),
            _ => None,
        }
    }

    /// Whether the value is in the grammar: no line `0`, no span below
    /// one, every name a `<custom-ident>` a line may take.
    pub fn is_valid(&self) -> bool {
        let named_ok = |n: &str| crate::parse::values::is_line_name(n);
        match self {
            GridLine::Auto => true,
            GridLine::Name(n) => named_ok(n),
            GridLine::Line { index, name } => *index != 0 && name.as_deref().is_none_or(named_ok),
            GridLine::Span { count, name } => *count >= 1 && name.as_deref().is_none_or(named_ok),
        }
    }
}

/// `grid-auto-flow` (CSS Grid 2 §7.7): the direction auto-placed items
/// fill — `row` fills each row before the next, `column` each column —
/// and whether the `dense` packing backfills earlier holes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridAutoFlow {
    pub direction: Direction,
    pub dense: bool,
}

impl GridAutoFlow {
    /// `row`, the initial value.
    pub const ROW: Self = GridAutoFlow {
        direction: Direction::Row,
        dense: false,
    };
    /// `column`.
    pub const COLUMN: Self = GridAutoFlow {
        direction: Direction::Column,
        dense: false,
    };

    /// This flow, `dense`.
    pub const fn dense(self) -> Self {
        GridAutoFlow {
            dense: true,
            ..self
        }
    }
}

impl Default for GridAutoFlow {
    fn default() -> Self {
        GridAutoFlow::ROW
    }
}
