//! Presentational hints (CSS Cascade 4 §6.4.4): attributes HTML maps to
//! CSS declarations of the author origin with specificity zero, before
//! every author rule. rdom maps the list attributes (HTML §15.3.8):
//!
//! - `<ol start=N>`: `counter-reset: list-item N-1`, or with `reversed`
//!   `counter-reset: reversed(list-item) N+1`;
//! - `<li value=N>`: `counter-set: list-item N`.
//!
//! (`ol[reversed]` without `start` is the UA rule `counter-reset:
//! reversed(list-item)`.)

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::style::{CounterOp, TuiStyle};

/// The presentational hints of the element `id`, `None` when it has
/// none (the common case: one tag comparison).
pub(super) fn presentational_hints(dom: &Dom<TuiExt>, id: NodeId) -> Option<TuiStyle> {
    let node = dom.node(id);
    match node.tag_name()? {
        "ol" => {
            let start = parse_html_integer(node.get_attribute("start")?)?;
            let op = if node.get_attribute("reversed").is_some() {
                CounterOp::reversed("list-item", Some(start.saturating_add(1)))
            } else {
                CounterOp::new("list-item", start.saturating_sub(1))
            };
            Some(TuiStyle::new().counter_reset(vec![op]))
        }
        "li" => {
            let value = parse_html_integer(node.get_attribute("value")?)?;
            Some(TuiStyle::new().counter_set(vec![CounterOp::new("list-item", value)]))
        }
        _ => None,
    }
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
