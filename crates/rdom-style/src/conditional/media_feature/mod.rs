//! One media feature (Media Queries 4 §2.4, §3): `(name)`, `(name:
//! value)` or a range, parsed from its parenthesized block, serialized,
//! and evaluated (`eval`) against a
//! [`MediaEnvironment`](super::MediaEnvironment) by the terminal mapping
//! (Media Queries 4 §4–§7, Media Queries 5 §12; DIVERGENCES §2 "Media
//! features answer for a terminal").

use crate::parse::Token;

use super::syntax::{Cv, Prelude};

mod eval;

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
    /// A pixel length — `px`, `em` / `rem` (16px, the initial font size,
    /// §1.3), or an absolute unit (CSS Values 4 §6.2) — kept as written,
    /// compared at 8px a column and 16px a row (DESIGN "Pixel lengths
    /// select, cells measure", C14G-PX-BREAKPOINTS).
    Pixels {
        px: f64,
        text: String,
    },
    /// A length in a unit with no cell measure (`ex`, `lh`, a viewport
    /// unit, …), or a `<resolution>`: kept as written, compared as unknown.
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
            Value::Pixels { text, .. } | Value::Unmeasured(text) => text.clone(),
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
    /// The values of this feature with no cell measure, as written
    /// (`Value::Unmeasured`): what a parser warns about. `resolution` is
    /// left out — a terminal has none, so it is false whatever its value.
    pub(crate) fn unmeasured(&self) -> Vec<String> {
        if self.name == "resolution" {
            return Vec::new();
        }
        let values: Vec<&Value> = match &self.test {
            Test::Boolean => Vec::new(),
            Test::Equals(v) => vec![v],
            Test::Range(pairs) => pairs.iter().map(|(_, v)| v).collect(),
        };
        values
            .into_iter()
            .filter_map(|v| match v {
                Value::Unmeasured(t) => Some(t.clone()),
                _ => None,
            })
            .collect()
    }

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

/// A dimension as an `<mf-value>`: `ch` is cells; `px`, `em` / `rem` (the
/// initial font size, 16px — Media Queries 4 §1.3) and the absolute units
/// (CSS Values 4 §6.2: 96px an inch) are pixels; the other font-relative,
/// the viewport and the resolution units are kept unmeasured; any other
/// unit is invalid.
fn dimension(value: f64, unit: &str, text: &str) -> Option<Value> {
    let unit = unit.to_ascii_lowercase();
    let px_per = match unit.as_str() {
        "ch" => return Some(Value::Cells(value)),
        "px" => 1.0,
        "em" | "rem" | "pc" => 16.0,
        "in" => 96.0,
        "cm" => 96.0 / 2.54,
        "mm" => 96.0 / 25.4,
        "q" => 96.0 / 101.6,
        "pt" => 96.0 / 72.0,
        "ex" | "rex" | "cap" | "rcap" | "ic" | "ric" | "lh" | "rlh" | "dpi" | "dpcm" | "dppx"
        | "x" | "vw" | "vh" | "vi" | "vb" | "vmin" | "vmax" | "svw" | "svh" | "svi" | "svb"
        | "svmin" | "svmax" | "lvw" | "lvh" | "lvi" | "lvb" | "lvmin" | "lvmax" | "dvw" | "dvh"
        | "dvi" | "dvb" | "dvmin" | "dvmax" => {
            return Some(Value::Unmeasured(text.to_string()));
        }
        _ => return None,
    };
    Some(Value::Pixels {
        px: value * px_per,
        text: text.to_string(),
    })
}
