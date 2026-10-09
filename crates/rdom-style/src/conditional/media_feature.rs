//! One media feature (Media Queries 4 §2.4, §3): `(name)`, `(name:
//! value)` or a range, parsed from its parenthesized block and evaluated
//! against a [`MediaEnvironment`] by the terminal mapping (Media Queries
//! 4 §4–§7, Media Queries 5 §12; DIVERGENCES §2 "Media features answer
//! for a terminal").

use crate::parse::Token;

use super::syntax::{Cv, Prelude};
use super::{MediaEnvironment, PointerAccuracy, Truth};

/// One media feature test: `(name)`, `(name: value)` or a range.
#[derive(Debug, Clone, PartialEq)]
pub struct MediaFeature {
    /// The feature's name, ASCII-lowercased, without a `min-` / `max-`
    /// prefix.
    name: String,
    test: Test,
}

#[derive(Debug, Clone, PartialEq)]
enum Test {
    /// `(name)`: the boolean context (§2.4.4).
    Boolean,
    /// `(name: value)`.
    Equals(Value),
    /// `feature <op> value`, each pair (one, or two for a double range).
    Range(Vec<(Cmp, Value)>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cmp {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
}

impl Cmp {
    /// The comparison with its operands swapped (`v < name` is `name > v`).
    fn flip(self) -> Self {
        match self {
            Cmp::Lt => Cmp::Gt,
            Cmp::Le => Cmp::Ge,
            Cmp::Gt => Cmp::Lt,
            Cmp::Ge => Cmp::Le,
            Cmp::Eq => Cmp::Eq,
        }
    }

    fn holds(self, a: f64, b: f64) -> bool {
        match self {
            Cmp::Lt => a < b,
            Cmp::Le => a <= b,
            Cmp::Gt => a > b,
            Cmp::Ge => a >= b,
            Cmp::Eq => a == b,
        }
    }

    fn text(self) -> &'static str {
        match self {
            Cmp::Lt => "<",
            Cmp::Le => "<=",
            Cmp::Gt => ">",
            Cmp::Ge => ">=",
            Cmp::Eq => "=",
        }
    }
}

/// An `<mf-value>`.
#[derive(Debug, Clone, PartialEq)]
enum Value {
    /// A `<number>` (an integer included); also a length in cells.
    Number(f64),
    /// A `ch` length: cells.
    Cells(f64),
    /// A length in a unit with no cell measure (`px`, `em`, …), or a
    /// `<resolution>`: kept as written, compared as unknown.
    Unmeasured(String),
    /// `<ratio>`: `a / b`.
    Ratio(f64, f64),
    Ident(String),
}

impl Value {
    fn text(&self) -> String {
        match self {
            Value::Number(n) => fmt_number(*n),
            Value::Cells(n) => format!("{}ch", fmt_number(*n)),
            Value::Unmeasured(t) => t.clone(),
            Value::Ratio(a, b) => format!("{} / {}", fmt_number(*a), fmt_number(*b)),
            Value::Ident(s) => s.clone(),
        }
    }
}

fn fmt_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

/// A `( … )` block as a media feature, if it is one.
pub(super) fn feature(prelude: &Prelude<'_>, cv: &Cv) -> Option<MediaFeature> {
    let Cv::Paren {
        inner,
        close: Some(_),
        ..
    } = cv
    else {
        return None;
    };
    MediaFeature::parse(prelude, inner)
}

impl MediaFeature {
    pub(super) fn parse(prelude: &Prelude<'_>, inner: &[Cv]) -> Option<Self> {
        // `(name)`.
        if let [only] = inner {
            let name = prelude.ident(only)?.to_ascii_lowercase();
            return Some(MediaFeature {
                name,
                test: Test::Boolean,
            });
        }
        // `(name: value)`, `min-` / `max-` a range.
        if let [name_cv, colon, value @ ..] = inner
            && let Some(name) = prelude.ident(name_cv)
            && matches!(colon, Cv::Token(i) if *prelude.token(*i) == Token::Colon)
        {
            let name = name.to_ascii_lowercase();
            let value = parse_value(prelude, value)?;
            return Some(if let Some(base) = name.strip_prefix("min-") {
                MediaFeature {
                    name: base.to_string(),
                    test: Test::Range(vec![(Cmp::Ge, value)]),
                }
            } else if let Some(base) = name.strip_prefix("max-") {
                MediaFeature {
                    name: base.to_string(),
                    test: Test::Range(vec![(Cmp::Le, value)]),
                }
            } else {
                MediaFeature {
                    name,
                    test: Test::Equals(value),
                }
            });
        }
        // A range: split at the comparison operators.
        let mut parts: Vec<&[Cv]> = Vec::new();
        let mut ops: Vec<Cmp> = Vec::new();
        let mut start = 0;
        let mut i = 0;
        while i < inner.len() {
            if let Some((cmp, width)) = comparison(prelude, &inner[i..]) {
                parts.push(&inner[start..i]);
                ops.push(cmp);
                i += width;
                start = i;
            } else {
                i += 1;
            }
        }
        parts.push(&inner[start..]);
        match (parts.as_slice(), ops.as_slice()) {
            // `name op value` / `value op name`.
            ([a, b], [op]) => {
                if let [n] = a
                    && let Some(name) = prelude.ident(n)
                    && parse_value(prelude, b).is_some_and(|v| !matches!(v, Value::Ident(_)))
                {
                    let value = parse_value(prelude, b)?;
                    return Some(MediaFeature {
                        name: name.to_ascii_lowercase(),
                        test: Test::Range(vec![(*op, value)]),
                    });
                }
                let [n] = b else { return None };
                let name = prelude.ident(n)?.to_ascii_lowercase();
                let value = parse_value(prelude, a)?;
                Some(MediaFeature {
                    name,
                    test: Test::Range(vec![(op.flip(), value)]),
                })
            }
            // `value op name op value`: both `<` or both `>` (§3).
            ([a, n, b], [op1, op2]) => {
                let lt = |c: Cmp| matches!(c, Cmp::Lt | Cmp::Le);
                let gt = |c: Cmp| matches!(c, Cmp::Gt | Cmp::Ge);
                if !((lt(*op1) && lt(*op2)) || (gt(*op1) && gt(*op2))) {
                    return None;
                }
                let [n] = n else { return None };
                let name = prelude.ident(n)?.to_ascii_lowercase();
                let low = parse_value(prelude, a)?;
                let high = parse_value(prelude, b)?;
                Some(MediaFeature {
                    name,
                    test: Test::Range(vec![(op1.flip(), low), (*op2, high)]),
                })
            }
            _ => None,
        }
    }

    /// The feature's name, lowercased, without `min-` / `max-`.
    pub fn name(&self) -> &str {
        &self.name
    }

    pub(super) fn write(&self, out: &mut String) {
        out.push('(');
        match &self.test {
            Test::Boolean => out.push_str(&self.name),
            Test::Equals(v) => {
                out.push_str(&self.name);
                out.push_str(": ");
                out.push_str(&v.text());
            }
            Test::Range(pairs) => match pairs.as_slice() {
                [(op, v)] => {
                    out.push_str(&self.name);
                    out.push(' ');
                    out.push_str(op.text());
                    out.push(' ');
                    out.push_str(&v.text());
                }
                [(op1, low), (op2, high)] => {
                    out.push_str(&low.text());
                    out.push(' ');
                    out.push_str(op1.flip().text());
                    out.push(' ');
                    out.push_str(&self.name);
                    out.push(' ');
                    out.push_str(op2.text());
                    out.push(' ');
                    out.push_str(&high.text());
                }
                _ => out.push_str(&self.name),
            },
        }
        out.push(')');
    }

    /// Evaluate against `env` (Media Queries 4 §4–§7, 5 §12), by the
    /// terminal mapping in the module doc of `media_env`.
    pub(crate) fn evaluate(&self, env: &MediaEnvironment) -> Truth {
        let p = &env.preferences;
        let vp = env.viewport;
        match self.name.as_str() {
            "width" | "device-width" => self.range(Kind::Length, f64::from(vp.cols)),
            "height" | "device-height" => self.range(Kind::Length, f64::from(vp.rows)),
            "aspect-ratio" | "device-aspect-ratio" => {
                self.range(Kind::Ratio, f64::from(vp.cols) / f64::from(vp.rows))
            }
            "color" => self.range(Kind::Integer, f64::from(p.color_bits)),
            "color-index" => self.range(Kind::Integer, 0.0),
            "monochrome" => self.range(Kind::Integer, if p.color_bits == 0 { 1.0 } else { 0.0 }),
            // §4.4: a grid device — a terminal is the spec's example.
            "grid" => self.range(Kind::Integer, 1.0),
            // A terminal has no pixel density: the concept does not exist
            // on the device, so the feature is false (§2.4).
            "resolution" => Truth::False,
            "orientation" => {
                let v = if vp.rows > vp.cols {
                    "portrait"
                } else {
                    "landscape"
                };
                self.discrete(v, &["portrait", "landscape"], None)
            }
            "update" => self.discrete("fast", &["none", "slow", "fast"], Some("none")),
            "overflow-block" => self.discrete("scroll", &["none", "scroll", "paged"], Some("none")),
            "overflow-inline" => self.discrete("scroll", &["none", "scroll"], Some("none")),
            "color-gamut" | "video-color-gamut" => {
                // `srgb` holds for a device that covers sRGB; the wider
                // gamuts do not.
                self.discrete("srgb", &["srgb", "p3", "rec2020"], None)
            }
            "dynamic-range" | "video-dynamic-range" => {
                self.discrete("standard", &["standard", "high"], None)
            }
            "environment-blending" => {
                self.discrete("opaque", &["opaque", "additive", "subtractive"], None)
            }
            "scripting" => self.discrete(
                "enabled",
                &["none", "initial-only", "enabled"],
                Some("none"),
            ),
            "display-mode" => self.discrete(
                "standalone",
                &[
                    "fullscreen",
                    "standalone",
                    "minimal-ui",
                    "browser",
                    "picture-in-picture",
                ],
                None,
            ),
            "hover" | "any-hover" => {
                let v = if p.hover { "hover" } else { "none" };
                self.discrete(v, &["none", "hover"], Some("none"))
            }
            "pointer" | "any-pointer" => {
                let v = match p.pointer {
                    PointerAccuracy::None => "none",
                    PointerAccuracy::Coarse => "coarse",
                    PointerAccuracy::Fine => "fine",
                };
                self.discrete(v, &["none", "coarse", "fine"], Some("none"))
            }
            "prefers-color-scheme" => {
                let v = match env.color_scheme {
                    crate::color::ColorScheme::Light => "light",
                    crate::color::ColorScheme::Dark => "dark",
                };
                self.discrete(v, &["light", "dark"], None)
            }
            "prefers-reduced-motion" => self.preference(p.reduced_motion),
            "prefers-reduced-transparency" => self.preference(p.reduced_transparency),
            "prefers-reduced-data" => self.preference(p.reduced_data),
            "prefers-contrast" => {
                let v = match p.contrast {
                    super::Contrast::NoPreference => "no-preference",
                    super::Contrast::More => "more",
                    super::Contrast::Less => "less",
                    super::Contrast::Custom => "custom",
                };
                self.discrete(
                    v,
                    &["no-preference", "more", "less", "custom"],
                    Some("no-preference"),
                )
            }
            "forced-colors" => {
                let v = if p.forced_colors { "active" } else { "none" };
                self.discrete(v, &["none", "active"], Some("none"))
            }
            "inverted-colors" => {
                let v = if p.inverted_colors {
                    "inverted"
                } else {
                    "none"
                };
                self.discrete(v, &["none", "inverted"], Some("none"))
            }
            _ => Truth::Unknown,
        }
    }

    /// Evaluate as a container size feature (CSS Conditional 5 §6.5)
    /// against a query container's content box: `width` / `inline-size`
    /// read `width`, `height` / `block-size` read `height`, `aspect-ratio`
    /// and `orientation` read both — `None`, an axis the container does
    /// not answer on, is unknown, as is any other feature.
    pub(crate) fn evaluate_size(&self, width: Option<f64>, height: Option<f64>) -> Truth {
        let unknown = |v: Option<f64>| v.is_none();
        match self.name.as_str() {
            "width" | "inline-size" => match width {
                Some(w) => self.range(Kind::Length, w),
                None => Truth::Unknown,
            },
            "height" | "block-size" => match height {
                Some(h) => self.range(Kind::Length, h),
                None => Truth::Unknown,
            },
            "aspect-ratio" | "orientation" if unknown(width) || unknown(height) => Truth::Unknown,
            "aspect-ratio" => self.range(Kind::Ratio, width.unwrap_or(0.0) / height.unwrap_or(0.0)),
            "orientation" => {
                let v = if height > width {
                    "portrait"
                } else {
                    "landscape"
                };
                self.discrete(v, &["portrait", "landscape"], None)
            }
            _ => Truth::Unknown,
        }
    }

    /// A `no-preference | reduce` preference.
    fn preference(&self, reduce: bool) -> Truth {
        let v = if reduce { "reduce" } else { "no-preference" };
        self.discrete(v, &["no-preference", "reduce"], Some("no-preference"))
    }

    /// A discrete feature whose value is `actual`, one of `values`;
    /// `falsy`: the value that is false in the boolean context (else
    /// every value is true). A range test, or a value not among
    /// `values`, is unknown.
    fn discrete(&self, actual: &str, values: &[&str], falsy: Option<&str>) -> Truth {
        match &self.test {
            Test::Boolean => Truth::from_bool(falsy != Some(actual)),
            Test::Equals(Value::Ident(v)) => {
                let v = v.to_ascii_lowercase();
                if values.contains(&v.as_str()) {
                    Truth::from_bool(v == actual)
                } else {
                    Truth::Unknown
                }
            }
            _ => Truth::Unknown,
        }
    }

    /// A range feature whose value is `actual`.
    fn range(&self, kind: Kind, actual: f64) -> Truth {
        let value = |v: &Value| -> Option<f64> {
            match (kind, v) {
                (Kind::Length, Value::Number(n) | Value::Cells(n)) => Some(*n),
                (Kind::Ratio, Value::Ratio(a, b)) => Some(a / b),
                (Kind::Ratio, Value::Number(n)) => Some(*n),
                (Kind::Integer, Value::Number(n)) if n.fract() == 0.0 => Some(*n),
                _ => None,
            }
        };
        match &self.test {
            Test::Boolean => Truth::from_bool(actual != 0.0 && !actual.is_nan()),
            Test::Equals(v) => match value(v) {
                Some(n) => Truth::from_bool(actual == n),
                None => Truth::Unknown,
            },
            Test::Range(pairs) => {
                let mut result = Truth::True;
                for (op, v) in pairs {
                    result = result.and(match value(v) {
                        Some(n) => Truth::from_bool(op.holds(actual, n)),
                        None => Truth::Unknown,
                    });
                }
                result
            }
        }
    }
}

/// What a range feature's values are.
#[derive(Clone, Copy)]
enum Kind {
    /// Cells: a number or `ch`.
    Length,
    /// `<ratio>` (or a number, `n / 1`).
    Ratio,
    /// `<integer>`.
    Integer,
}

/// A comparison operator at the start of `values`, and how many
/// component values it spans: `<`, `<=`, `>`, `>=`, `=` (§3: no space
/// inside `<=` / `>=`).
fn comparison(prelude: &Prelude<'_>, values: &[Cv]) -> Option<(Cmp, usize)> {
    let Cv::Token(i) = values.first()? else {
        return None;
    };
    let Token::Delim(c) = *prelude.token(*i) else {
        return None;
    };
    let eq_next = matches!(values.get(1), Some(Cv::Token(j)) if *prelude.token(*j) == Token::Delim('=') && prelude.adjacent(*i));
    match (c, eq_next) {
        ('<', true) => Some((Cmp::Le, 2)),
        ('<', false) => Some((Cmp::Lt, 1)),
        ('>', true) => Some((Cmp::Ge, 2)),
        ('>', false) => Some((Cmp::Gt, 1)),
        ('=', _) => Some((Cmp::Eq, 1)),
        _ => None,
    }
}

/// An `<mf-value>`: a number, a dimension, an identifier or a ratio.
fn parse_value(prelude: &Prelude<'_>, values: &[Cv]) -> Option<Value> {
    let number = |values: &[Cv]| -> Option<f64> {
        match values {
            [Cv::Token(i)] => match prelude.token(*i) {
                Token::Number(n) => Some(*n as f64),
                Token::Float(f) => Some(*f),
                _ => None,
            },
            [Cv::Token(s), Cv::Token(i)] if *prelude.token(*s) == Token::Delim('-') => {
                match prelude.token(*i) {
                    Token::Number(n) => Some(-(*n as f64)),
                    Token::Float(f) => Some(-*f),
                    _ => None,
                }
            }
            _ => None,
        }
    };
    if let Some(n) = number(values) {
        return Some(Value::Number(n));
    }
    // `<ratio>`: `a / b`.
    if let Some(slash) = values
        .iter()
        .position(|v| matches!(v, Cv::Token(i) if *prelude.token(*i) == Token::Delim('/')))
    {
        let a = number(&values[..slash])?;
        let b = number(&values[slash + 1..])?;
        return (a >= 0.0 && b >= 0.0).then_some(Value::Ratio(a, b));
    }
    match values {
        [cv @ Cv::Token(i)] => match prelude.token(*i) {
            Token::Ident(s) => Some(Value::Ident(s.clone())),
            Token::Dimension { value, unit, .. } => dimension(*value, unit, prelude.text_of(cv)),
            _ => None,
        },
        [Cv::Token(s), cv @ Cv::Token(i)] if *prelude.token(*s) == Token::Delim('-') => {
            match prelude.token(*i) {
                Token::Dimension { value, unit, .. } => {
                    dimension(-value, unit, &format!("-{}", prelude.text_of(cv)))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// A dimension as an `<mf-value>`: `ch` is cells; a pixel, font-relative
/// or resolution unit is kept unmeasured; any other unit is invalid.
fn dimension(value: f64, unit: &str, text: &str) -> Option<Value> {
    let unit = unit.to_ascii_lowercase();
    match unit.as_str() {
        "ch" => Some(Value::Cells(value)),
        "px" | "cm" | "mm" | "q" | "in" | "pt" | "pc" | "em" | "rem" | "ex" | "rex" | "cap"
        | "rcap" | "ic" | "ric" | "lh" | "rlh" | "dpi" | "dpcm" | "dppx" | "x" => {
            Some(Value::Unmeasured(text.to_string()))
        }
        _ => None,
    }
}
