//! White-space processing (CSS Text 3 §4.1, CSS Text 4 §4.1–§4.3): what
//! each document white space character is under the
//! `white-space-collapse` of the text holding it, and the segment break
//! transformation rules. The packer applies Phase I (collapsing) as it
//! takes the text in and Phase II (trimming at line edges, hanging) as it
//! breaks lines (`packer`).

use crate::layout::WhiteSpaceCollapse;

/// What a grapheme is to white-space processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WhiteSpaceClass {
    /// A collapsible space, tab or segment break (§4.1.1): it collapses
    /// with its neighbours into one separator, or none at a line edge.
    /// `segment_break` marks a segment break, which the segment break
    /// transformation rules may remove (§4.1.3, [`segment_break_removed`]).
    Collapsible { segment_break: bool },
    /// A preserved segment break: a forced line break.
    ForcedBreak,
    /// A preserved space (a segment break under `preserve-spaces`, a
    /// carriage return), one cell.
    PreservedSpace,
    /// A preserved tab (§4.2).
    PreservedTab,
    /// A control character other than these: not rendered.
    Control,
    /// Anything else: text.
    Text,
}

/// Classify the grapheme `g` under `collapse` (CSS Text 3 §4, §4.1.1).
/// A carriage return is treated identically to a space (§4); a CRLF
/// pair is one segment break.
pub(super) fn classify(g: &str, collapse: WhiteSpaceCollapse) -> WhiteSpaceClass {
    use WhiteSpaceClass as C;
    match g {
        " " | "\r" if collapse.collapses_spaces() => C::Collapsible {
            segment_break: false,
        },
        "\t" if collapse.collapses_spaces() => C::Collapsible {
            segment_break: false,
        },
        " " | "\r" => C::PreservedSpace,
        "\t" => C::PreservedTab,
        "\n" | "\r\n" => match collapse {
            WhiteSpaceCollapse::Collapse => C::Collapsible {
                segment_break: true,
            },
            WhiteSpaceCollapse::PreserveSpaces => C::PreservedSpace,
            _ => C::ForcedBreak,
        },
        _ if g.chars().next().is_some_and(char::is_control) => C::Control,
        _ => C::Text,
    }
}

/// Whether the character `c` is collapsible white space under
/// `collapse` — what a run of such text renders as nothing at a line
/// edge (CSS 2.1 §9.2.1.1: whitespace-only text that collapses away
/// generates no line box; HTML §3.2.7's rendered text collection).
pub(crate) fn is_collapsible_white_space(c: char, collapse: WhiteSpaceCollapse) -> bool {
    match c {
        ' ' | '\t' | '\r' => collapse.collapses_spaces(),
        '\n' => collapse == WhiteSpaceCollapse::Collapse,
        _ => false,
    }
}

/// The segment break transformation rules (CSS Text 3 §4.1.3, UA-defined
/// there and in Text 4 §4.3; rdom follows the rule earlier drafts gave,
/// as Gecko does): a collapsible segment break between `before` and
/// `after` — spaces and tabs around it already removed — is removed
/// when either is a zero-width space, or when both are East Asian Wide,
/// Fullwidth or Halfwidth and neither is Hangul, so Chinese or Japanese
/// text broken across source lines joins with no space; otherwise it
/// becomes a space.
pub(super) fn segment_break_removed(before: char, after: char) -> bool {
    if before == '\u{200B}' || after == '\u{200B}' {
        return true;
    }
    is_east_asian_wide(before)
        && is_east_asian_wide(after)
        && !is_hangul(before)
        && !is_hangul(after)
}

/// East Asian Width F, W or H (UAX #11): the wide characters (two cells
/// in `unicode-width`'s table) and the halfwidth forms.
fn is_east_asian_wide(c: char) -> bool {
    unicode_width::UnicodeWidthChar::width(c) == Some(2)
        || matches!(c, '\u{FF61}'..='\u{FFDC}' | '\u{FFE8}'..='\u{FFEE}')
}

/// A Hangul letter: Jamo, compatibility Jamo, syllables, halfwidth Jamo.
fn is_hangul(c: char) -> bool {
    matches!(
        c,
        '\u{1100}'..='\u{11FF}'
            | '\u{3130}'..='\u{318F}'
            | '\u{A960}'..='\u{A97F}'
            | '\u{AC00}'..='\u{D7AF}'
            | '\u{D7B0}'..='\u{D7FF}'
            | '\u{FFA0}'..='\u{FFDC}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CSS Text 3 §4.1.3: a break between two CJK ideographs goes, one
    /// between Latin letters or beside Hangul becomes a space.
    #[test]
    fn segment_breaks_vanish_between_wide_non_hangul_characters() {
        assert!(segment_break_removed('段', '在'));
        assert!(segment_break_removed('。', 'ｱ'));
        assert!(segment_break_removed('a', '\u{200B}'));
        assert!(!segment_break_removed('a', 'b'));
        assert!(!segment_break_removed('段', 'b'));
        assert!(!segment_break_removed('한', '국'));
    }

    /// CSS Text 4 §4.1: which characters each `white-space-collapse`
    /// collapses.
    #[test]
    fn classification_follows_white_space_collapse() {
        use WhiteSpaceClass as C;
        use WhiteSpaceCollapse as W;
        let sp = C::Collapsible {
            segment_break: false,
        };
        let br = C::Collapsible {
            segment_break: true,
        };
        assert_eq!(classify(" ", W::Collapse), sp);
        assert_eq!(classify("\n", W::Collapse), br);
        assert_eq!(classify("\r\n", W::Collapse), br);
        assert_eq!(classify("\t", W::PreserveBreaks), sp);
        assert_eq!(classify("\n", W::PreserveBreaks), C::ForcedBreak);
        assert_eq!(classify("\n", W::PreserveSpaces), C::PreservedSpace);
        assert_eq!(classify("\t", W::PreserveSpaces), C::PreservedTab);
        assert_eq!(classify(" ", W::Preserve), C::PreservedSpace);
        assert_eq!(classify("\r", W::BreakSpaces), C::PreservedSpace);
        assert_eq!(classify("\n", W::BreakSpaces), C::ForcedBreak);
        assert_eq!(classify("\u{7}", W::Preserve), C::Control);
        assert_eq!(classify("\u{A0}", W::Collapse), C::Text);
        assert!(!is_collapsible_white_space('\u{A0}', W::Collapse));
        assert!(!is_collapsible_white_space('\n', W::PreserveBreaks));
        assert!(is_collapsible_white_space(' ', W::PreserveBreaks));
    }
}
