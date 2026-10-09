//! Media queries (Media Queries 4 §2–§3, Media Queries 5): the
//! `<media-query-list>` of `@media`, `@import … <media-query-list>` and
//! `matchMedia()`, parsed and evaluated against a
//! [`MediaEnvironment`].
//!
//! Grammar (§3): `<media-query> = <media-condition> | [not | only]?
//! <media-type> [and <media-condition-without-or>]?`; a feature is
//! `(<mf-name>)`, `(<mf-name>: <mf-value>)` or a range (`(width >= 80)`,
//! `(40 <= width < 80)`), `min-` / `max-` the old range spelling. A query
//! that does not parse is `not all` (§3.2) — the list's other queries
//! stand; a feature with an unknown name or an invalid value, and any
//! `<general-enclosed>`, is unknown (§3.1).
//!
//! Lengths are cells (DESIGN "Pixel lengths select, cells measure"): a
//! unitless number or `ch` is columns or rows, as in every rdom length;
//! `px`, `em` and the other pixel and font units have no cell measure,
//! so a feature compared with one is unknown — it never matches, and
//! nor does its `not` — rather than guessing a pixel size.

use super::media_feature::feature;
use super::syntax::{Cv, Prelude, split_commas};
use super::{Condition, MediaEnvironment, MediaFeature, Truth};

/// A `<media-query-list>`: it matches when any of its queries does; an
/// empty list matches everything (Media Queries 4 §2.1).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MediaList {
    queries: Vec<MediaQuery>,
}

/// One `<media-query>`.
#[derive(Debug, Clone, PartialEq)]
pub struct MediaQuery {
    /// `not` before the media type: the query's result is negated.
    negated: bool,
    /// The media type (`all` when only a condition is written).
    media_type: MediaType,
    /// The `<media-condition>`, if any.
    condition: Option<Condition<MediaFeature>>,
    /// The query did not parse: it is `not all` (§3.2).
    invalid: bool,
}

/// A media type (Media Queries 4 §2.3).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MediaType {
    /// `all`, and a query with no type.
    All,
    /// `screen` — what a terminal is.
    Screen,
    /// `print` — never: rdom has no paged media.
    Print,
    /// Any other type (`tv`, `speech`, the deprecated ones): matches
    /// nothing (§2.3).
    Other(String),
}

impl MediaList {
    /// Parse a `<media-query-list>` (Media Queries 4 §3): comma-separated
    /// queries, each `not all` when it does not parse. Empty text is the
    /// empty list, which matches everything.
    pub fn parse(text: &str) -> Self {
        let Some(prelude) = Prelude::parse(text) else {
            return MediaList {
                queries: vec![MediaQuery::not_all()],
            };
        };
        let values = prelude.values();
        if values.is_empty() {
            return MediaList::default();
        }
        let queries = split_commas(&prelude, &values)
            .into_iter()
            .map(|q| MediaQuery::parse(&prelude, q).unwrap_or_else(MediaQuery::not_all))
            .collect();
        MediaList { queries }
    }

    /// The queries, in order.
    pub fn queries(&self) -> &[MediaQuery] {
        &self.queries
    }

    /// Whether the list matches `env`: any query does (§2.1).
    pub fn matches(&self, env: &MediaEnvironment) -> bool {
        self.queries.is_empty() || self.queries.iter().any(|q| q.matches(env))
    }

    /// The feature values of the list with no cell measure, as written
    /// (`30ex`, `50vw`, `2dppx`): each makes its feature unknown. A parser
    /// warns about them; `px` and `em` are measured (C14G-PX-BREAKPOINTS).
    pub fn unmeasured_values(&self) -> Vec<String> {
        self.queries
            .iter()
            .filter_map(|q| q.condition.as_ref())
            .flat_map(|c| c.leaves())
            .flat_map(MediaFeature::unmeasured)
            .collect()
    }

    /// Whether any query of the list reads the viewport (`width`,
    /// `height`, `aspect-ratio`, `orientation`, the `device-*` ones): only
    /// such a list can change when the terminal is resized.
    pub fn reads_viewport(&self) -> bool {
        self.queries.iter().any(|q| {
            q.condition.as_ref().is_some_and(|c| {
                c.leaves().iter().any(|f| {
                    matches!(
                        f.name(),
                        "width"
                            | "height"
                            | "device-width"
                            | "device-height"
                            | "aspect-ratio"
                            | "device-aspect-ratio"
                            | "orientation"
                    )
                })
            })
        })
    }
}

/// The serialization (CSSOM §4.2.2 "serialize a media query list"):
/// the queries joined by `", "`, each in canonical form.
impl std::fmt::Display for MediaList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, q) in self.queries.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{q}")?;
        }
        Ok(())
    }
}

impl MediaQuery {
    fn not_all() -> Self {
        MediaQuery {
            negated: true,
            media_type: MediaType::All,
            condition: None,
            invalid: true,
        }
    }

    fn parse(prelude: &Prelude<'_>, values: &[Cv]) -> Option<Self> {
        let first = values.first()?;
        // `<media-condition>`: starts with `not (` or a block.
        let typed = match prelude.ident(first) {
            Some(word) if word.eq_ignore_ascii_case("not") => {
                values.get(1).is_some_and(|v| prelude.ident(v).is_some())
            }
            Some(_) => true,
            None => false,
        };
        if !typed {
            let condition = prelude.condition(values, true, &feature)?;
            return Some(MediaQuery {
                negated: false,
                media_type: MediaType::All,
                condition: Some(condition),
                invalid: false,
            });
        }
        let mut rest = values;
        let mut negated = false;
        match prelude.ident(first) {
            Some(w) if w.eq_ignore_ascii_case("not") => {
                negated = true;
                rest = &rest[1..];
            }
            Some(w) if w.eq_ignore_ascii_case("only") => rest = &rest[1..],
            _ => {}
        }
        let (type_cv, rest) = rest.split_first()?;
        let name = prelude.ident(type_cv)?;
        // §3: these are not media types.
        if ["only", "not", "and", "or", "layer"]
            .iter()
            .any(|r| name.eq_ignore_ascii_case(r))
        {
            return None;
        }
        let media_type = match name.to_ascii_lowercase().as_str() {
            "all" => MediaType::All,
            "screen" => MediaType::Screen,
            "print" => MediaType::Print,
            other => MediaType::Other(other.to_string()),
        };
        let condition = match rest.split_first() {
            None => None,
            Some((and, cond)) if prelude.is_ident(and, "and") => {
                Some(prelude.condition(cond, false, &feature)?)
            }
            Some(_) => return None,
        };
        Some(MediaQuery {
            negated,
            media_type,
            condition,
            invalid: false,
        })
    }

    /// Whether the query matches `env` (§2.2): the type and the
    /// condition, an unknown result false — before `not` too.
    pub fn matches(&self, env: &MediaEnvironment) -> bool {
        if self.invalid {
            return false;
        }
        let type_ok = Truth::from_bool(matches!(
            self.media_type,
            MediaType::All | MediaType::Screen
        ));
        let result = match &self.condition {
            Some(c) => type_ok.and(c.evaluate(&mut |f| f.evaluate(env))),
            None => type_ok,
        };
        if self.negated {
            // §3.2: an unknown condition makes the query false, negated
            // or not.
            match result {
                Truth::Unknown => false,
                r => !r.holds(),
            }
        } else {
            result.holds()
        }
    }

    /// The media type.
    pub fn media_type(&self) -> &MediaType {
        &self.media_type
    }
}

impl std::fmt::Display for MediaQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.invalid {
            return f.write_str("not all");
        }
        let mut out = String::new();
        if self.negated {
            out.push_str("not ");
        }
        let type_name = match &self.media_type {
            MediaType::All => "all",
            MediaType::Screen => "screen",
            MediaType::Print => "print",
            MediaType::Other(s) => s,
        };
        let write_feature = |l: &MediaFeature, out: &mut String| l.write(out);
        match &self.condition {
            None => out.push_str(type_name),
            Some(c) => {
                if self.negated || self.media_type != MediaType::All {
                    out.push_str(type_name);
                    out.push_str(" and ");
                    c.write(&mut out, &write_feature, false);
                } else {
                    c.write(&mut out, &write_feature, true);
                }
            }
        }
        f.write_str(&out)
    }
}
