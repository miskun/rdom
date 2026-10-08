//! Attribute selectors (Selectors 4 §6): the value operators and the
//! case-sensitivity of the comparison.

use crate::selectors::AttrOp;

/// HTML §4.16.2 "case-sensitivity of selectors": attribute selectors on
/// an HTML element treat the values of these attributes as ASCII
/// case-insensitive (a `match`: this runs on every attribute-selector
/// test of the cascade). Every rdom element is an HTML element in an HTML
/// document. (HTML exempts `type` in its rendering section's `ol[type]`
/// rules via the `s` flag, which rdom does not parse.)
fn is_html_case_insensitive_attr(name: &str) -> bool {
    matches!(
        name,
        "accept"
            | "accept-charset"
            | "align"
            | "alink"
            | "axis"
            | "bgcolor"
            | "charset"
            | "checked"
            | "clear"
            | "codetype"
            | "color"
            | "compact"
            | "declare"
            | "defer"
            | "dir"
            | "direction"
            | "disabled"
            | "enctype"
            | "face"
            | "frame"
            | "hreflang"
            | "http-equiv"
            | "lang"
            | "language"
            | "link"
            | "media"
            | "method"
            | "multiple"
            | "nohref"
            | "noresize"
            | "noshade"
            | "nowrap"
            | "readonly"
            | "rel"
            | "rev"
            | "rules"
            | "scope"
            | "scrolling"
            | "selected"
            | "shape"
            | "target"
            | "text"
            | "type"
            | "valign"
            | "valuetype"
            | "vlink"
    )
}

pub(super) fn match_attribute(
    attrs: &std::collections::BTreeMap<String, String>,
    name: &str,
    op: Option<AttrOp>,
    want: Option<&str>,
) -> bool {
    let Some(have) = attrs.get(name) else {
        return false;
    };
    let Some(op) = op else { return true }; // `[name]` — presence only.
    let want = want.unwrap_or("");
    if is_html_case_insensitive_attr(name) {
        match_value::<AsciiCaseInsensitive>(op, have, want)
    } else {
        match_value::<CaseSensitive>(op, have, want)
    }
}

/// How [`match_value`] compares attribute-value bytes. Comparing bytes
/// is sound for UTF-8: ASCII case folding never touches the bytes of a
/// multi-byte character, so a match starts and ends on char boundaries
/// whenever `want` is valid UTF-8.
trait ValueCase {
    fn eq(a: &[u8], b: &[u8]) -> bool;

    /// `want` (non-empty) occurs in `have`.
    fn contains(have: &str, want: &str) -> bool {
        have.as_bytes()
            .windows(want.len())
            .any(|win| Self::eq(win, want.as_bytes()))
    }
}

struct CaseSensitive;
impl ValueCase for CaseSensitive {
    fn eq(a: &[u8], b: &[u8]) -> bool {
        a == b
    }

    fn contains(have: &str, want: &str) -> bool {
        have.contains(want)
    }
}

/// HTML §4.16.2: ASCII case-insensitive, without allocating.
struct AsciiCaseInsensitive;
impl ValueCase for AsciiCaseInsensitive {
    fn eq(a: &[u8], b: &[u8]) -> bool {
        a.eq_ignore_ascii_case(b)
    }
}

/// Selectors 4 §6.1 / §6.2: the attribute-value operators, comparing
/// per `C`.
fn match_value<C: ValueCase>(op: AttrOp, have: &str, want: &str) -> bool {
    let (h, w) = (have.as_bytes(), want.as_bytes());
    let starts = |h: &[u8]| h.get(..w.len()).is_some_and(|p| C::eq(p, w));
    match op {
        AttrOp::Exact => C::eq(h, w),
        AttrOp::Includes => have
            .split_ascii_whitespace()
            .any(|tok| C::eq(tok.as_bytes(), w)),
        AttrOp::DashMatch => C::eq(h, w) || (starts(h) && h.get(w.len()) == Some(&b'-')),
        AttrOp::Prefix => !w.is_empty() && starts(h),
        AttrOp::Suffix => !w.is_empty() && h.len() >= w.len() && C::eq(&h[h.len() - w.len()..], w),
        AttrOp::Substring => !w.is_empty() && C::contains(have, want),
    }
}
