//! Math serializers: a stored `CalcExpr` — `calc()`, the math
//! functions, dimensions and the anchor functions — as CSS text. Split
//! from `value_serializers.rs` (C15G-HYGIENE); every form here must parse
//! back to the same expression, as the round-trip tests in `tests.rs` pin.

/// A stored math expression as a value: a lone percentage, dimension or
/// math function as itself (`10%`, `50vw`, `min(50%, 30)`), anything
/// else wrapped in `calc()`.
pub(crate) fn serialize_math(expr: &crate::calc::CalcExpr) -> String {
    match expr {
        crate::calc::CalcExpr::Percent(_)
        | crate::calc::CalcExpr::Dimension { .. }
        | crate::calc::CalcExpr::Function { .. }
        | crate::calc::CalcExpr::Anchor(_) => serialize_calc(expr),
        _ => format!("calc({})", serialize_calc(expr)),
    }
}

/// Render a `CalcExpr` back to its source form. Used by
/// `serialize_size` / `serialize_length` for the cssText
/// round-trip + devtools / debug output.
fn serialize_calc(expr: &crate::calc::CalcExpr) -> String {
    use crate::calc::{CalcExpr, CalcOp};
    match expr {
        CalcExpr::Number(n) if n.is_nan() => "NaN".to_string(),
        CalcExpr::Number(n) if n.is_infinite() => {
            if *n > 0.0 { "infinity" } else { "-infinity" }.to_string()
        }
        CalcExpr::Number(n) => {
            if n.fract() == 0.0 && n.abs() < 1e15 {
                format!("{}", *n as i64)
            } else {
                format!("{n}")
            }
        }
        CalcExpr::Length(c) => format!("{c}"),
        CalcExpr::NoBound => "none".to_string(),
        CalcExpr::Anchor(f) => serialize_anchor(f),
        CalcExpr::Dimension { value, unit } => {
            format!(
                "{}{}",
                serialize_calc(&CalcExpr::Number(*value)),
                unit.css_name()
            )
        }
        CalcExpr::Function { func, args } => {
            let mut parts: Vec<String> = Vec::with_capacity(args.len() + 1);
            if let crate::calc::MathFunction::Round(strategy) = func
                && *strategy != crate::calc::RoundingStrategy::Nearest
            {
                parts.push(strategy.keyword().to_string());
            }
            parts.extend(args.iter().map(serialize_calc));
            format!("{}({})", func.name(), parts.join(", "))
        }
        CalcExpr::Percent(p) => {
            if p.fract() == 0.0 && p.abs() < 1e15 {
                format!("{}%", *p as i64)
            } else {
                format!("{p}%")
            }
        }
        CalcExpr::Binary { op, lhs, rhs } => {
            let op_str = match op {
                CalcOp::Add => "+",
                CalcOp::Sub => "-",
                CalcOp::Mul => "*",
                CalcOp::Div => "/",
            };
            // Parenthesize a sub-expression whenever re-parsing the
            // flat form would bind differently: a lower-precedence
            // child under `*` / `/`, or any binary right operand of
            // the non-associative `-` / `/`.
            let prec = |o: &CalcOp| match o {
                CalcOp::Add | CalcOp::Sub => 1,
                CalcOp::Mul | CalcOp::Div => 2,
            };
            let wrap = |child: &CalcExpr, is_rhs: bool| -> String {
                let text = serialize_calc(child);
                match child {
                    CalcExpr::Binary { op: child_op, .. } => {
                        let needs = prec(child_op) < prec(op)
                            || (is_rhs
                                && prec(child_op) == prec(op)
                                && matches!(op, CalcOp::Sub | CalcOp::Div));
                        if needs { format!("({text})") } else { text }
                    }
                    _ => text,
                }
            };
            format!("{} {} {}", wrap(lhs, false), op_str, wrap(rhs, true))
        }
    }
}

/// An anchor function as CSS text (CSS Anchor Positioning 1 §5).
fn serialize_anchor(f: &crate::calc::AnchorFunction) -> String {
    use crate::calc::{AnchorFunction, AnchorSide};
    let mut parts: Vec<String> = Vec::new();
    let mut first: Vec<String> = Vec::new();
    if let Some(name) = f.name() {
        first.push(name.to_string());
    }
    let fallback = f.fallback();
    let func = match f {
        AnchorFunction::Edge { side, .. } => {
            first.push(match side {
                AnchorSide::Percent(p) => serialize_calc(&crate::calc::CalcExpr::Percent(*p)),
                side => AnchorSide::KEYWORDS
                    .iter()
                    .find(|(_, s)| s == side)
                    .map_or("center", |(k, _)| k)
                    .to_string(),
            });
            "anchor"
        }
        AnchorFunction::Size { size, .. } => {
            if let Some(size) = size {
                first.push(size.keyword().to_string());
            }
            "anchor-size"
        }
    };
    if !first.is_empty() {
        parts.push(first.join(" "));
    }
    if let Some(fb) = fallback {
        parts.push(serialize_math_bare(fb));
    }
    format!("{func}({})", parts.join(", "))
}

/// A math expression inside a function's arguments: its sum, unwrapped.
fn serialize_math_bare(expr: &crate::calc::CalcExpr) -> String {
    serialize_calc(expr)
}
