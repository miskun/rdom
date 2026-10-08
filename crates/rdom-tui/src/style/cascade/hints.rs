//! Presentational hints (CSS Cascade 4 §6.4.4): attributes HTML maps to
//! CSS declarations of the author origin with specificity zero, before
//! every author rule. rdom maps the list attributes (HTML §15.3.8):
//!
//! - `<ol start=N>`: `counter-reset: list-item N-1`, or with `reversed`
//!   `counter-reset: reversed(list-item) N+1`;
//! - `<li value=N>`: `counter-set: list-item N`;
//! - `<ol type>` / `<li type>`: `1`, `a`, `A`, `i`, `I` → `list-style-type`
//!   `decimal`, `lower-alpha`, `upper-alpha`, `lower-roman`, `upper-roman`,
//!   matched case-sensitively; `<ul type>` / `<li type>`: `none`, `disc`,
//!   `circle`, `square`, matched ASCII case-insensitively.
//!
//! (`ol[reversed]` without `start` is the UA rule `counter-reset:
//! reversed(list-item)`. HTML writes the `type` mappings as UA rules with
//! the `s` attribute-selector flag; rdom matches `type` values
//! case-insensitively (DIVERGENCES §2), so no UA selector can tell `a`
//! from `A`, and they are hints — DIVERGENCES §2.)

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::ListStyleType;
use crate::style::{CounterOp, CounterStyle, TuiStyle};

/// The presentational hints of the element `id`, `None` when it has
/// none (the common case: one tag comparison).
pub(super) fn presentational_hints(dom: &Dom<TuiExt>, id: NodeId) -> Option<TuiStyle> {
    let node = dom.node(id);
    let tag = node.tag_name()?;
    let mut style: Option<TuiStyle> = None;
    let mut hint = |f: &dyn Fn(TuiStyle) -> TuiStyle| {
        style = Some(f(style.take().unwrap_or_default()));
    };
    match tag {
        "ol" => {
            if let Some(start) = node.get_attribute("start").and_then(parse_html_integer) {
                let op = if node.get_attribute("reversed").is_some() {
                    CounterOp::reversed("list-item", Some(start.saturating_add(1)))
                } else {
                    CounterOp::new("list-item", start.saturating_sub(1))
                };
                hint(&|s| s.counter_reset(vec![op.clone()]));
            }
        }
        "li" => {
            if let Some(value) = node.get_attribute("value").and_then(parse_html_integer) {
                hint(&|s| s.counter_set(vec![CounterOp::new("list-item", value)]));
            }
        }
        "ul" => {}
        _ => return None,
    }
    if let Some(list_type) = node.get_attribute("type").and_then(|t| list_type(tag, t)) {
        hint(&|s| s.list_style_type(list_type.clone()));
    }
    style
}

/// HTML §15.3.8's `type` attribute mapping for the list element `tag`: the
/// ordinal keywords on `ol` and `li` (case-sensitive), the bullet keywords
/// on `ul` and `li` (ASCII case-insensitive).
fn list_type(tag: &str, value: &str) -> Option<ListStyleType> {
    let named = |name| Some(ListStyleType::Style(CounterStyle::named(name)));
    let ordinal = match value {
        "1" => Some("decimal"),
        "a" => Some("lower-alpha"),
        "A" => Some("upper-alpha"),
        "i" => Some("lower-roman"),
        "I" => Some("upper-roman"),
        _ => None,
    };
    if let Some(name) = ordinal.filter(|_| matches!(tag, "ol" | "li")) {
        return named(name);
    }
    if !matches!(tag, "ul" | "li") {
        return None;
    }
    if value.eq_ignore_ascii_case("none") {
        return Some(ListStyleType::None);
    }
    ["disc", "circle", "square"]
        .into_iter()
        .find(|k| value.eq_ignore_ascii_case(k))
        .and_then(named)
}

/// HTML §2.3.4.1 "rules for parsing integers": leading ASCII whitespace
/// skipped, an optional `-` / `+`, then at least one ASCII digit; what
/// follows the digits is ignored. Clamped to `i32`.
pub(super) fn parse_html_integer(s: &str) -> Option<i32> {
    let s = s.trim_start_matches([' ', '\t', '\n', '\u{c}', '\r']);
    let (negative, digits) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let end = digits
        .bytes()
        .position(|b| !b.is_ascii_digit())
        .unwrap_or(digits.len());
    if end == 0 {
        return None;
    }
    let mut value: i64 = 0;
    for b in digits[..end].bytes() {
        value = (value * 10 + i64::from(b - b'0')).min(i64::from(i32::MAX) + 1);
    }
    let value = if negative { -value } else { value };
    Some(value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32)
}

#[cfg(test)]
mod tests {
    use super::parse_html_integer;

    /// HTML §2.3.4.1.
    #[test]
    fn html_integers_parse_leniently() {
        assert_eq!(parse_html_integer("5"), Some(5));
        assert_eq!(parse_html_integer("  -2xyz"), Some(-2));
        assert_eq!(parse_html_integer("+7"), Some(7));
        assert_eq!(parse_html_integer("x"), None);
        assert_eq!(parse_html_integer("-"), None);
        assert_eq!(parse_html_integer(""), None);
        assert_eq!(parse_html_integer("99999999999999"), Some(i32::MAX));
        assert_eq!(parse_html_integer("-99999999999999"), Some(i32::MIN));
    }
}
