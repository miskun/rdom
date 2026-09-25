//! Per-value-type serializers: the CSS text form of each stored value
//! type (`TuiColor`, `Size`, `Length`, `CalcExpr`, transition lists,
//! counter ops, `Content`, …). Every function here must emit a form
//! the matching parser in `crate::parse::values` reads back to the
//! same value — the round-trip tests in `tests.rs` pin that.

use crate::layout::{Length, Overflow, Size};
use crate::transition::{TimingFunction, TransitionProperty};
use crate::{Color, Content, TuiColor, TuiStyle, Value};

pub(super) fn specified<T>(v: &Value<T>) -> Option<&T> {
    match v {
        Value::Specified(t) => Some(t),
        _ => None,
    }
}

pub(super) fn serialize_color(c: &TuiColor) -> String {
    match c {
        TuiColor::Literal(lit) => serialize_literal_color(lit),
        TuiColor::Var { name, fallback } => match fallback {
            Some(fb) => format!("var(--{name}, {})", serialize_color(fb)),
            None => format!("var(--{name})"),
        },
    }
}

/// Serialize a `Color` to a form `parse_color` will read back as
/// the same value:
/// - Reset → "Reset" (parses via `rdom_tui::style::parse_color`).
/// - Named ANSI → lowercase name.
/// - Rgb(r,g,b) → "rgb(r, g, b)".
/// - Indexed(n) → "indexed-N" — not currently round-trippable
///   through the CSS parser; emitted as a debug-friendly form.
pub(super) fn serialize_literal_color(c: &Color) -> String {
    match c {
        Color::Reset => "reset".to_string(),
        Color::Indexed(n) => format!("indexed-{n}"),
        Color::Rgb(r, g, b) => {
            // Prefer the terminal-palette spellings authors write most
            // (they are also the aliases `named::name_of` would not pick
            // first), then any CSS name for the triple, then `rgb()`.
            let preferred = match (r, g, b) {
                (0, 255, 255) => Some("cyan"),
                (255, 0, 255) => Some("magenta"),
                (128, 128, 128) => Some("gray"),
                (169, 169, 169) => Some("darkgray"),
                _ => None,
            };
            preferred
                .or_else(|| crate::color::named::name_of(*c))
                .map(str::to_string)
                .unwrap_or_else(|| format!("rgb({r}, {g}, {b})"))
        }
    }
}

pub(super) fn serialize_overflow(o: &Overflow) -> &'static str {
    match o {
        Overflow::Visible => "visible",
        Overflow::Hidden => "hidden",
        Overflow::Scroll => "scroll",
        Overflow::Auto => "auto",
    }
}

pub(super) fn serialize_size(s: &Size) -> String {
    match s {
        Size::Auto => "auto".to_string(),
        Size::Fixed(n) => n.to_string(),
        Size::Flex(n) => format!("{n}fr"),
        Size::Percent(p) => format!("{p}%"),
        Size::Calc(expr) => format!("calc({})", serialize_calc(expr)),
    }
}

pub(super) fn serialize_min_size(m: &crate::layout::MinSize) -> String {
    match m {
        crate::layout::MinSize::Auto => "auto".to_string(),
        crate::layout::MinSize::Cells(n) => n.to_string(),
    }
}

pub(super) fn serialize_margin_value(v: &crate::layout::MarginValue) -> String {
    match v {
        crate::layout::MarginValue::Auto => "auto".to_string(),
        crate::layout::MarginValue::Cells(n) => n.to_string(),
        crate::layout::MarginValue::Calc(expr) => format!("calc({})", serialize_calc(expr)),
    }
}

pub(super) fn border_style_keyword(s: crate::layout::BorderStyle) -> &'static str {
    use crate::layout::BorderStyle;
    match s {
        BorderStyle::None => "none",
        BorderStyle::Hidden => "hidden",
        BorderStyle::Solid => "solid",
        BorderStyle::Double => "double",
        BorderStyle::Dashed => "dashed",
        BorderStyle::Dotted => "dotted",
        BorderStyle::Ridge => "ridge",
        BorderStyle::Outset => "outset",
        BorderStyle::Groove => "groove",
        BorderStyle::Inset => "inset",
        BorderStyle::HalfBlock => "half-block",
    }
}

pub(super) fn serialize_padding_value(v: &crate::layout::PaddingValue) -> String {
    match v {
        crate::layout::PaddingValue::Cells(n) => n.to_string(),
        crate::layout::PaddingValue::Calc(expr) => format!("calc({})", serialize_calc(expr)),
    }
}

pub(super) fn serialize_length(l: &Length) -> String {
    match l {
        Length::Auto => "auto".to_string(),
        Length::Cells(n) => n.to_string(),
        Length::Calc(expr) => format!("calc({})", serialize_calc(expr)),
    }
}

/// Render a `CalcExpr` back to its source form. Used by
/// `serialize_size` / `serialize_length` for the cssText
/// round-trip + devtools / debug output.
pub(super) fn serialize_calc(expr: &crate::calc::CalcExpr) -> String {
    use crate::calc::{CalcExpr, CalcOp};
    match expr {
        CalcExpr::Number(n) => {
            if n.fract() == 0.0 {
                format!("{}", *n as i64)
            } else {
                format!("{n}")
            }
        }
        CalcExpr::Length(c) => format!("{c}"),
        CalcExpr::Percent(p) => {
            if p.fract() == 0.0 {
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

pub(super) fn serialize_transition_property(p: &TransitionProperty) -> String {
    use crate::transition::AnimatableProperty;
    match p {
        TransitionProperty::All => "all".to_string(),
        TransitionProperty::None => "none".to_string(),
        TransitionProperty::Discrete(name) => name.clone(),
        TransitionProperty::Named(a) => match a {
            AnimatableProperty::Color => "color",
            AnimatableProperty::BackgroundColor => "background-color",
            AnimatableProperty::BorderColor => "border-color",
            AnimatableProperty::Width => "width",
            AnimatableProperty::Height => "height",
            AnimatableProperty::Padding => "padding",
            AnimatableProperty::Gap => "gap",
            AnimatableProperty::Top => "top",
            AnimatableProperty::Right => "right",
            AnimatableProperty::Bottom => "bottom",
            AnimatableProperty::Left => "left",
            AnimatableProperty::ZIndex => "z-index",
        }
        .to_string(),
    }
}

/// `counter-reset` / `counter-increment` value: `name value` pairs.
pub(super) fn serialize_counter_ops(ops: &[crate::counters::CounterOp]) -> String {
    if ops.is_empty() {
        return "none".to_string();
    }
    ops.iter()
        .map(|op| format!("{} {}", op.name, op.value))
        .collect::<Vec<_>>()
        .join(" ")
}

/// `content` value serialization; `None` for the cascade-internal
/// `Var` form, which no CSS source produces.
pub(super) fn serialize_content(c: &Content) -> Option<String> {
    match c {
        Content::Str(s) => Some(format!("\"{s}\"")),
        Content::Attr(a) => Some(format!("attr({a})")),
        Content::Counter { name, style } => Some(match style {
            crate::counters::CounterStyle::Decimal => format!("counter({name})"),
            other => format!("counter({name}, {})", other.as_str()),
        }),
        Content::Concat(parts) => {
            let mut out = Vec::with_capacity(parts.len());
            for p in parts {
                out.push(serialize_content(p)?);
            }
            Some(out.join(" "))
        }
        Content::None => Some("none".to_string()),
        Content::Var(_) => None,
    }
}

pub(super) fn serialize_timing_function(f: &TimingFunction) -> String {
    use crate::transition::StepPosition;
    match f {
        TimingFunction::Linear => "linear".to_string(),
        TimingFunction::Ease => "ease".to_string(),
        TimingFunction::EaseIn => "ease-in".to_string(),
        TimingFunction::EaseOut => "ease-out".to_string(),
        TimingFunction::EaseInOut => "ease-in-out".to_string(),
        TimingFunction::CubicBezier { x1, y1, x2, y2 } => {
            format!("cubic-bezier({x1}, {y1}, {x2}, {y2})")
        }
        TimingFunction::Steps { count, position } => {
            let pos = match position {
                StepPosition::Start => "jump-start",
                StepPosition::End => "jump-end",
                StepPosition::JumpNone => "jump-none",
                StepPosition::JumpBoth => "jump-both",
            };
            format!("steps({count}, {pos})")
        }
    }
}

/// Serialize the `transition` shorthand from the four longhand
/// vectors. Pads shorter vectors by repeating the last element
/// (matches CSS's "repeat shorter list" rule), then emits one
/// comma-separated piece per rule.
pub(super) fn serialize_transition_shorthand(style: &TuiStyle) -> Option<String> {
    let props = style.transition_property.as_ref().and_then(specified)?;
    let durs = style.transition_duration.as_ref().and_then(specified)?;
    let timings = style
        .transition_timing_function
        .as_ref()
        .and_then(specified)?;
    let delays = style.transition_delay.as_ref().and_then(specified)?;
    let n = props.len();
    if n == 0 || durs.is_empty() || timings.is_empty() || delays.is_empty() {
        return None;
    }
    let pad_dur = |i: usize| durs[i.min(durs.len() - 1)];
    let pad_timing = |i: usize| &timings[i.min(timings.len() - 1)];
    let pad_delay = |i: usize| delays[i.min(delays.len() - 1)];
    let mut parts = Vec::with_capacity(n);
    for (i, p) in props.iter().enumerate() {
        parts.push(format!(
            "{} {}ms {} {}ms",
            serialize_transition_property(p),
            pad_dur(i),
            serialize_timing_function(pad_timing(i)),
            pad_delay(i),
        ));
    }
    Some(parts.join(", "))
}

pub(super) fn join_csv<I, F, T>(iter: I, f: F) -> String
where
    I: Iterator<Item = T>,
    F: Fn(T) -> String,
{
    let mut out = String::new();
    for (i, item) in iter.enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&f(item));
    }
    out
}
