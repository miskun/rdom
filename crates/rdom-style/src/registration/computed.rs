//! The computed value of a registered custom property (CSS Properties
//! and Values API 1 §2.4): a `<length>` is an absolute length — whole
//! cells, a viewport-percentage unit resolved against the viewport — a
//! `<length-percentage>` keeps its percentage beside the absolute part,
//! and every other component is as specified.

use super::{
    Multiplier, PropertySyntax, SyntaxComponent, consume, matches_term, top_level_segments,
};
use crate::CustomValue;
use crate::calc::{CalcExpr, ResolveCtx, Viewport, to_cells};
use crate::parse::token::Token;
use crate::parse::values::{LengthPercentage, Range, length_percentage};

impl PropertySyntax {
    /// The computed value of `value`, which matches this syntax, in a
    /// document presented in `viewport`; `None` when that is `value`
    /// itself. The first alternative `value` matches decides its type;
    /// a list computes item by item.
    pub fn computed(&self, value: &CustomValue, viewport: Viewport) -> Option<CustomValue> {
        let PropertySyntax::Alternatives(alternatives) = self else {
            return None;
        };
        let tokens = value.tokens()?;
        let (component, multiplier) = alternatives
            .iter()
            .find(|(c, m)| matches_term(c, *m, tokens))?;
        let percent = match component {
            SyntaxComponent::Length => false,
            SyntaxComponent::LengthPercentage => true,
            _ => return None,
        };
        let (items, separator) = match multiplier {
            Multiplier::One => (vec![tokens], ""),
            Multiplier::CommaList => (top_level_segments(tokens), ", "),
            Multiplier::SpaceList => (space_items(component, tokens), " "),
        };
        let computed: Vec<String> = items
            .into_iter()
            .map(|item| length(item, percent, viewport))
            .collect::<Option<_>>()?;
        let text = computed.join(separator);
        (text != value.as_str()).then(|| CustomValue::new(&text))
    }
}

/// The items of a space-separated list of `component`s.
fn space_items<'t>(component: &SyntaxComponent, tokens: &'t [Token]) -> Vec<&'t [Token]> {
    let mut items = Vec::new();
    let mut at = 0;
    while let Some(end) = consume(component, tokens, at).filter(|end| *end > at) {
        items.push(&tokens[at..end]);
        at = end;
    }
    items
}

/// One `<length>` (or, with `percent`, `<length-percentage>`) as its
/// computed value's text: whole cells, or — holding a percentage — that
/// percentage beside the cells (`calc(2 + 50%)`).
fn length(item: &[Token], percent: bool, viewport: Viewport) -> Option<String> {
    Some(match length_percentage(item, Range::Any)? {
        LengthPercentage::Integer(n) => n.to_string(),
        LengthPercentage::Cells(v) => to_cells(v).to_string(),
        LengthPercentage::Expr(e) => {
            let e = absolute(&e, viewport);
            if !e.contains_percent() {
                to_cells(e.resolve_f64(&ResolveCtx::new(0))).to_string()
            } else if !percent {
                return None;
            } else if let Some((cells, pct)) = e.linear_parts() {
                length_percentage_text(cells, pct)
            } else {
                crate::property_dispatch::serialize_math(&e)
            }
        }
    })
}

/// `expr` with every length unit in cells: the viewport-percentage ones
/// against `viewport`, `ch` / `lh` (a cell each) as they are.
fn absolute(expr: &CalcExpr, viewport: Viewport) -> CalcExpr {
    match expr {
        CalcExpr::Dimension { unit, .. } if unit.kind().is_length() => {
            CalcExpr::Number(expr.resolve_f64(&ResolveCtx::new(0).with_viewport(viewport)))
        }
        CalcExpr::Binary { op, lhs, rhs } => {
            CalcExpr::binary(*op, absolute(lhs, viewport), absolute(rhs, viewport))
        }
        CalcExpr::Function { func, args } => {
            CalcExpr::function(*func, args.iter().map(|a| absolute(a, viewport)).collect())
        }
        other => other.clone(),
    }
}

/// The text of `cells + percent%` as a computed `<length-percentage>`:
/// whole cells, then the percentage — `8`, `50%`, `calc(2 + 50%)`,
/// `calc(2 - 50%)` (CSS Values 4 §10.10.1 orders a number before a
/// percentage).
pub fn length_percentage_text(cells: f64, percent: f64) -> String {
    let cells = to_cells(cells);
    let percent = (percent * 1000.0).round() / 1000.0;
    if percent == 0.0 {
        cells.to_string()
    } else if cells == 0 {
        format!("{percent}%")
    } else if percent < 0.0 {
        format!("calc({cells} - {}%)", -percent)
    } else {
        format!("calc({cells} + {percent}%)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn computed(syntax: &str, value: &str) -> Option<String> {
        PropertySyntax::parse(syntax)
            .unwrap()
            .computed(&CustomValue::new(value), Viewport::new(80, 20))
            .map(|v| v.as_str().to_string())
    }

    /// §2.4: a `<length>` computes to an absolute length — `vw` / `vh`
    /// against the viewport (CSS Values 4 §6.1.2), `ch` and math folded to
    /// whole cells.
    #[test]
    fn length_is_absolute_cells() {
        assert_eq!(computed("<length>", "10vw").as_deref(), Some("8"));
        assert_eq!(computed("<length>", "50vh").as_deref(), Some("10"));
        assert_eq!(computed("<length>", "calc(2ch + 3)").as_deref(), Some("5"));
        assert_eq!(computed("<length>", "-2ch").as_deref(), Some("-2"));
        assert_eq!(computed("<length>", "7"), None, "already absolute");
    }

    /// §2.4: a `<length-percentage>` keeps its percentage; the length part
    /// is absolute.
    #[test]
    fn length_percentage_keeps_its_percentage() {
        assert_eq!(computed("<length-percentage>", "50%"), None);
        assert_eq!(computed("<length-percentage>", "2ch").as_deref(), Some("2"));
        assert_eq!(
            computed("<length-percentage>", "calc(10vw + 50%)").as_deref(),
            Some("calc(8 + 50%)")
        );
    }

    /// Lists compute per item; other components are as specified.
    #[test]
    fn lists_compute_per_item_and_other_types_stay() {
        assert_eq!(computed("<length>+", "10vw 2ch").as_deref(), Some("8 2"));
        assert_eq!(computed("<length>#", "10vw, 3").as_deref(), Some("8, 3"));
        assert_eq!(computed("<length> | auto", "auto"), None);
        assert_eq!(computed("<number>", "1.5"), None);
        assert_eq!(computed("*", "10vw"), None);
    }
}
