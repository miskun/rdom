//! Value grammars constraint validation checks: HTML's valid e-mail
//! address, a simplified absolute URL, valid floating-point numbers and
//! non-negative integers.

/// HTML §4.10.5.1.5 "valid e-mail address":
/// `1*( atext / "." ) "@" label *( "." label )`, where `atext` is
/// `A-Z a-z 0-9 ! # $ % & ' * + / = ? ^ _ ` { | } ~ -` and a label is
/// 1–63 letters, digits or hyphens that neither starts nor ends with a
/// hyphen.
pub(super) fn is_valid_email(s: &str) -> bool {
    let Some((local, domain)) = s.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && local
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ".!#$%&'*+/=?^_`{|}~-".contains(c))
        && domain.split('.').all(is_valid_label)
}

fn is_valid_label(label: &str) -> bool {
    let b = label.as_bytes();
    (1..=63).contains(&b.len())
        && b.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'-')
        && b[0] != b'-'
        && b[b.len() - 1] != b'-'
}

/// A simplified HTML "valid absolute URL" (URL Standard §4.3): after
/// stripping ASCII whitespace at the ends (value sanitization), a
/// scheme (`ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )`), `:`, and a
/// non-empty remainder without ASCII whitespace. The URL parser's
/// host, port and percent-encoding rules are not applied (DIVERGENCES).
pub(super) fn is_valid_absolute_url(s: &str) -> bool {
    let s = s.trim_ascii();
    let Some((scheme, rest)) = s.split_once(':') else {
        return false;
    };
    let mut chars = scheme.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
        && !rest.is_empty()
        && !rest.chars().any(|c| c.is_ascii_whitespace())
}

/// HTML §2.3.4.3 "valid floating-point number":
/// `-? ( digits | digits "." digits | "." digits ) ( [eE] [+-]? digits )?`,
/// parsed to its value. `None` for anything else (`+1`, `1.`, `inf`,
/// `NaN`, surrounding whitespace).
pub(super) fn parse_float(s: &str) -> Option<f64> {
    let b = s.as_bytes();
    let mut i = 0;
    let digits = |i: &mut usize| {
        let start = *i;
        while *i < b.len() && b[*i].is_ascii_digit() {
            *i += 1;
        }
        *i > start
    };
    if b.first() == Some(&b'-') {
        i += 1;
    }
    let int = digits(&mut i);
    if b.get(i) == Some(&b'.') {
        i += 1;
        if !digits(&mut i) {
            return None;
        }
    } else if !int {
        return None;
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(b.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        if !digits(&mut i) {
            return None;
        }
    }
    if i != b.len() {
        return None;
    }
    s.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// HTML §2.3.4.2 "rules for parsing non-negative integers": skip
/// leading ASCII whitespace, an optional `+`, then at least one digit;
/// trailing characters are ignored.
pub(super) fn parse_non_negative(s: &str) -> Option<usize> {
    let s = s.trim_start_matches(|c: char| c.is_ascii_whitespace());
    let s = s.strip_prefix('+').unwrap_or(s);
    let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    s[..end].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats_follow_the_html_grammar() {
        for (s, v) in [
            ("1", Some(1.0)),
            ("-2.5", Some(-2.5)),
            (".5", Some(0.5)),
            ("1e3", Some(1000.0)),
            ("1E-1", Some(0.1)),
            ("+1", None),
            ("1.", None),
            ("", None),
            ("-", None),
            ("inf", None),
            ("NaN", None),
            (" 1", None),
            ("1e", None),
            ("1e999", None),
        ] {
            assert_eq!(parse_float(s), v, "{s:?}");
        }
    }

    #[test]
    fn non_negative_integers_follow_the_html_rules() {
        assert_eq!(parse_non_negative(" +12px"), Some(12));
        assert_eq!(parse_non_negative("0"), Some(0));
        assert_eq!(parse_non_negative("-1"), None);
        assert_eq!(parse_non_negative("x"), None);
    }
}
