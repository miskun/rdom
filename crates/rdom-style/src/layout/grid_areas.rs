//! Named grid areas (CSS Grid 2 §7.3): the value of
//! `grid-template-areas` — [`GridTemplateAreas`], a matrix of cell
//! tokens whose names mark out rectangular [`NamedArea`]s.
//!
//! A value is valid by construction: [`GridTemplateAreas::new`] reads
//! the strings as §7.3 tokenizes them and refuses a matrix that is not
//! one (rows of unequal length, a trash token) or a name whose cells are
//! not a filled rectangle — "the declaration is invalid".

use std::ops::Range;

/// `grid-template-areas` (CSS Grid 2 §7.3): `none`, or one string per
/// row of the explicit grid, each cell a name or a null cell (`.`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GridTemplateAreas {
    /// The cells, row by row (`None` a null cell); every row has the
    /// same number of cells. Empty for `none`.
    rows: Vec<Vec<Option<String>>>,
}

/// One named grid area (§7.3): the rectangle of cells its name fills,
/// as track ranges of the explicit grid (0-based, end exclusive).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedArea {
    pub name: String,
    pub rows: Range<usize>,
    pub columns: Range<usize>,
}

impl GridTemplateAreas {
    /// `none`: no named areas (the initial value).
    pub const NONE: Self = Self { rows: Vec::new() };

    /// The areas the strings `rows` mark out, one string per row, or
    /// `None` when §7.3 makes the declaration invalid: no string, a
    /// string with no cell or a trash token (a character that is neither
    /// a name code point, `.` nor whitespace), rows with different
    /// numbers of cells, or a name whose cells do not form a filled
    /// rectangle.
    pub fn new<S: AsRef<str>>(rows: impl IntoIterator<Item = S>) -> Option<Self> {
        let rows = rows
            .into_iter()
            .map(|s| cell_tokens(s.as_ref()))
            .collect::<Option<Vec<_>>>()?;
        let columns = rows.first()?.len();
        if columns == 0 || rows.iter().any(|r| r.len() != columns) {
            return None;
        }
        let areas = Self { rows };
        areas.rectangles().map(|_| areas)
    }

    /// Whether this is `none`.
    pub fn is_none(&self) -> bool {
        self.rows.is_empty()
    }

    /// The cells, row by row; a null cell is `None`.
    pub fn rows(&self) -> &[Vec<Option<String>>] {
        &self.rows
    }

    /// How many rows the areas define (0 for `none`).
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// How many columns the areas define (0 for `none`).
    pub fn column_count(&self) -> usize {
        self.rows.first().map_or(0, Vec::len)
    }

    /// The named areas, in the order their names first appear (row by
    /// row, left to right).
    pub fn areas(&self) -> Vec<NamedArea> {
        self.rectangles().unwrap_or_default()
    }

    /// Each name's rectangle, or `None` when a name's cells are not one.
    fn rectangles(&self) -> Option<Vec<NamedArea>> {
        let mut out: Vec<NamedArea> = Vec::new();
        let mut counts: Vec<usize> = Vec::new();
        for (r, row) in self.rows.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                let Some(name) = cell else { continue };
                match out.iter().position(|a| &a.name == name) {
                    Some(k) => {
                        let a = &mut out[k];
                        a.rows.start = a.rows.start.min(r);
                        a.rows.end = a.rows.end.max(r + 1);
                        a.columns.start = a.columns.start.min(c);
                        a.columns.end = a.columns.end.max(c + 1);
                        counts[k] += 1;
                    }
                    None => {
                        out.push(NamedArea {
                            name: name.clone(),
                            rows: r..r + 1,
                            columns: c..c + 1,
                        });
                        counts.push(1);
                    }
                }
            }
        }
        // A name fills its bounding box exactly when it has as many
        // cells as the box: no other token can sit inside it then.
        out.iter()
            .zip(&counts)
            .all(|(a, &n)| a.rows.len() * a.columns.len() == n)
            .then_some(out)
    }
}

/// The cell tokens of one string (§7.3): a run of name code points is a
/// named cell, a run of `.` a null cell, whitespace separates them, and
/// anything else is a trash token — `None`.
fn cell_tokens(s: &str) -> Option<Vec<Option<String>>> {
    let mut out = Vec::new();
    let mut chars = s.chars().peekable();
    while let Some(&c) = chars.peek() {
        if is_whitespace(c) {
            chars.next();
        } else if c == '.' {
            while chars.next_if_eq(&'.').is_some() {}
            out.push(None);
        } else if is_name_code_point(c) {
            let mut name = String::new();
            while let Some(c) = chars.next_if(|&c| is_name_code_point(c)) {
                name.push(c);
            }
            out.push(Some(name));
        } else {
            return None;
        }
    }
    Some(out)
}

/// CSS whitespace (CSS Syntax 3 §4.2, after preprocessing).
fn is_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{c}')
}

/// An ident code point (CSS Syntax 3 §4.2): a letter, a digit, `-`,
/// `_`, or a non-ASCII code point.
fn is_name_code_point(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_' || !c.is_ascii()
}
