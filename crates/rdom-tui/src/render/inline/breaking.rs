//! Soft wrap opportunities between two pieces of text (CSS Text 3 §5):
//! the UAX #14 line breaking classes rdom distinguishes, and the rule
//! that decides whether a line may break between two adjacent
//! typographic character units under `word-break` (§5.2), `line-break`
//! (§5.3) and `hyphens` (§6.1). White space opportunities are the white
//! space processing rules' (`white_space`), and forced breaks the
//! packer's.
//!
//! ## Extent (a UAX #14 subset)
//!
//! rdom has no line breaking dictionary and no full UAX #14 tables. It
//! breaks:
//!
//! - around ideographic characters (class ID — CJK ideographs, kana,
//!   Hangul syllables, fullwidth letters, and any other two-cell
//!   grapheme), never inside a run of letters or digits (AL / NU);
//! - after hyphens (`-` HY, unless a digit follows; `‐` `–` `—` and
//!   friends, BA) and after a soft hyphen when `hyphens` allows;
//! - after a zero-width space (ZW, LB8), never around glue (GL / WJ:
//!   no-break spaces, the word joiner, LB11 / LB12);
//! - never before closing punctuation, exclamation, infix separators or
//!   non-starters (CL / CP / EX / IS / NS, LB13 / LB21) nor after opening
//!   punctuation (OP, LB14);
//!
//! with the CJK strictness rules of `line-break` (§5.3) applied when both
//! sides are CJK (rdom has no `lang`, so "the writing system is Chinese
//! or Japanese" is read off the characters). Southeast Asian scripts
//! (SA) do not break between words, as without a dictionary.

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::layout::{Hyphens, LineBreak, WordBreak};

/// The line breaking class of a grapheme, by its first character (a
/// subset of UAX #14's).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BreakClass {
    /// Letters, digits, symbols: no break between two (AL / NU / SA).
    Alphabetic,
    /// A digit (NU): no break between a hyphen and it.
    Numeric,
    /// Ideographic (ID): a break on either side.
    Ideographic,
    /// Small kana and the prolonged sound mark (CJ).
    SmallKana,
    /// Iteration marks (`々` `〻` `ゝ` `ゞ` `ヽ` `ヾ`).
    Iteration,
    /// Inseparable characters (IN: `‥` `…`).
    Inseparable,
    /// CJK hyphen-like characters `〜` `゠`.
    CjkHyphen,
    /// Hyphens that break before only under `loose` (`‐` `–`).
    LooseHyphen,
    /// `-` (HY): a break after it.
    Hyphen,
    /// Other characters a break may follow (BA: `—`, …).
    BreakAfter,
    /// A soft hyphen: a hyphenation opportunity after it.
    SoftHyphen,
    /// A zero-width space (ZW): a break after it.
    ZeroWidthSpace,
    /// Glue (GL / WJ): no break on either side.
    Glue,
    /// Closing punctuation, exclamation / interrogation, infix
    /// separators, non-starters (CL / CP / EX / IS / NS): no break
    /// before.
    Close,
    /// CJK centered punctuation (`・` `：` `；` `！` `？` …): no break
    /// before, except under `loose` in CJK text.
    Centered,
    /// Opening punctuation (OP): no break after.
    Open,
}

/// The class of the grapheme starting with `c`, `wide` when it takes two
/// cells.
pub(crate) fn class_of(c: char, wide: bool) -> BreakClass {
    use BreakClass as B;
    match c {
        '0'..='9' => B::Numeric,
        '-' => B::Hyphen,
        '\u{AD}' => B::SoftHyphen,
        '\u{200B}' => B::ZeroWidthSpace,
        '\u{A0}' | '\u{2007}' | '\u{202F}' | '\u{2060}' | '\u{FEFF}' | '\u{200D}' => B::Glue,
        '\u{2010}' | '\u{2013}' => B::LooseHyphen,
        '\u{2012}' | '\u{2014}' | '\u{058A}' | '\u{1680}' | '\u{2027}' | '|' => B::BreakAfter,
        '\u{301C}' | '\u{30A0}' => B::CjkHyphen,
        '\u{2025}' | '\u{2026}' => B::Inseparable,
        '\u{3005}' | '\u{303B}' | '\u{309D}' | '\u{309E}' | '\u{30FD}' | '\u{30FE}' => B::Iteration,
        '\u{30FB}'
        | '\u{FF1A}'
        | '\u{FF1B}'
        | '\u{FF65}'
        | '\u{203C}'
        | '\u{2047}'..='\u{2049}'
        | '\u{FF01}'
        | '\u{FF1F}' => B::Centered,
        ')' | ']' | '}' | ',' | '.' | ':' | ';' | '!' | '?' | '%' | '\u{3001}' | '\u{3002}'
        | '\u{FF0C}' | '\u{FF0E}' | '\u{FF09}' | '\u{FF3D}' | '\u{FF5D}' | '\u{300D}'
        | '\u{300F}' | '\u{3011}' | '\u{3015}' | '\u{3009}' | '\u{300B}' | '\u{3017}'
        | '\u{3019}' | '\u{301F}' | '\u{FF60}' | '\u{FF61}' | '\u{FF63}' | '\u{FF64}'
        | '\u{309B}' | '\u{309C}' => B::Close,
        '(' | '[' | '{' | '\u{FF08}' | '\u{FF3B}' | '\u{FF5B}' | '\u{300C}' | '\u{300E}'
        | '\u{3010}' | '\u{3014}' | '\u{3008}' | '\u{300A}' | '\u{3016}' | '\u{3018}'
        | '\u{301D}' | '\u{FF5F}' | '\u{FF62}' => B::Open,
        '\u{3041}'
        | '\u{3043}'
        | '\u{3045}'
        | '\u{3047}'
        | '\u{3049}'
        | '\u{3063}'
        | '\u{3083}'
        | '\u{3085}'
        | '\u{3087}'
        | '\u{308E}'
        | '\u{3095}'
        | '\u{3096}'
        | '\u{30A1}'
        | '\u{30A3}'
        | '\u{30A5}'
        | '\u{30A7}'
        | '\u{30A9}'
        | '\u{30C3}'
        | '\u{30E3}'
        | '\u{30E5}'
        | '\u{30E7}'
        | '\u{30EE}'
        | '\u{30F5}'
        | '\u{30F6}'
        | '\u{30FC}'
        | '\u{31F0}'..='\u{31FF}'
        | '\u{FF67}'..='\u{FF70}' => B::SmallKana,
        '\u{FF66}'..='\u{FF9D}' => B::Ideographic,
        _ if wide => B::Ideographic,
        _ => B::Alphabetic,
    }
}

/// The class of a source grapheme a transform rendered as `text`, maybe
/// several graphemes (`ß` → `SS`, `ß` → `ＳＳ`): its first grapheme's, by
/// that grapheme's own width (UAX #14 LB9) — not the whole rendering's.
pub(crate) fn class_of_rendered(text: &str) -> BreakClass {
    let g = text.graphemes(true).next().unwrap_or(" ");
    class_of(
        g.chars().next().unwrap_or(' '),
        UnicodeWidthStr::width(g) == 2,
    )
}

impl BreakClass {
    /// A letter or ideograph for `word-break` (§5.2: letters and the
    /// NU / AL / AI / ID classes).
    fn is_letter(self) -> bool {
        matches!(
            self,
            Self::Alphabetic | Self::Numeric | Self::Ideographic | Self::SmallKana
        )
    }

    /// Part of CJK text (ideographs, kana, CJK punctuation), for the
    /// `line-break` rules that apply "if the writing system is Chinese
    /// or Japanese".
    fn is_cjk(self) -> bool {
        matches!(
            self,
            Self::Ideographic
                | Self::SmallKana
                | Self::Iteration
                | Self::CjkHyphen
                | Self::Centered
                | Self::Inseparable
        )
    }
}

/// The line-breaking values in force at an opportunity: the text's
/// `word-break`, `line-break` and `hyphens`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BreakRules {
    pub(crate) word_break: WordBreak,
    pub(crate) line_break: LineBreak,
    pub(crate) hyphens: Hyphens,
}

impl Default for BreakRules {
    fn default() -> Self {
        BreakRules {
            word_break: WordBreak::Normal,
            line_break: LineBreak::Auto,
            hyphens: Hyphens::Manual,
        }
    }
}

/// Whether a line may break between a grapheme of class `before` and one
/// of class `after` (CSS Text 3 §5.2, §5.3, §6.1, UAX #14 as listed in the
/// module docs).
pub(crate) fn break_between(before: BreakClass, after: BreakClass, rules: BreakRules) -> bool {
    use BreakClass as B;
    let (word_break, line_break) = (rules.word_break, rules.line_break);
    // §5.3 `anywhere`: around every typographic character unit,
    // "disregarding any prohibition".
    if line_break == LineBreak::Anywhere {
        return true;
    }
    let loose = line_break == LineBreak::Loose;
    let strict = line_break == LineBreak::Strict;
    let cjk_before = before.is_cjk();
    // §5.2 `break-all`: letters break as ideographs.
    let as_id = |c: BreakClass| {
        if word_break == WordBreak::BreakAll && matches!(c, B::Alphabetic | B::Numeric) {
            B::Ideographic
        } else {
            c
        }
    };
    let (before, after) = (as_id(before), as_id(after));
    match (before, after) {
        // LB8: after a zero-width space; LB11 / LB12: never at glue.
        (B::ZeroWidthSpace, _) => true,
        (B::Glue, _) | (_, B::Glue) => false,
        // §6.1: a soft hyphen is a hyphenation opportunity unless
        // `hyphens: none`.
        (B::SoftHyphen, _) => rules.hyphens != crate::layout::Hyphens::None,
        (_, B::SoftHyphen) => false,
        // §5.2 `keep-all`: none between letters and ideographs,
        // "regardless of line-break settings other than anywhere".
        (b, a) if word_break == WordBreak::KeepAll && b.is_letter() && a.is_letter() => false,
        // LB13 / LB21: no break before closing punctuation or a hyphen;
        // LB14: none after opening punctuation.
        (_, B::Close | B::Hyphen | B::BreakAfter) | (B::Open, _) => false,
        // §5.3: centered punctuation, iteration marks, small kana and
        // inseparables break before only under `loose`, in CJK text.
        (_, B::Centered | B::Iteration | B::SmallKana) => loose && cjk_before,
        (B::Inseparable, B::Inseparable) => loose,
        (_, B::Inseparable) => false,
        // §5.3: `〜` `゠` break before under `normal` and `loose`; `‐` `–`
        // only under `loose`, after an ideograph.
        (_, B::CjkHyphen) => !strict && cjk_before,
        (_, B::LooseHyphen) => loose && before == B::Ideographic,
        // LB21 / LB25: after a hyphen, unless a digit follows.
        (B::Hyphen, B::Numeric) => false,
        (B::Hyphen | B::BreakAfter | B::LooseHyphen | B::CjkHyphen, _) => true,
        // UAX #14 LB31 for ideographs: a break on either side — after
        // closing punctuation only before one (LB29 / LB30 keep `a.b`,
        // `a)b` together).
        (B::Close, a) => a == B::Ideographic,
        (B::Ideographic | B::SmallKana | B::Iteration | B::Centered, _) => true,
        (_, B::Ideographic) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(word_break: WordBreak, line_break: LineBreak) -> BreakRules {
        BreakRules {
            word_break,
            line_break,
            hyphens: Hyphens::Manual,
        }
    }

    fn may_break(a: char, b: char, r: BreakRules) -> bool {
        let w = |c: char| unicode_width::UnicodeWidthChar::width(c) == Some(2);
        break_between(class_of(a, w(a)), class_of(b, w(b)), r)
    }

    /// UAX #14's core pairs under the initial values.
    #[test]
    fn the_initial_rules() {
        let r = BreakRules::default();
        assert!(!may_break('a', 'b', r));
        assert!(may_break('中', '文', r));
        assert!(may_break('a', '中', r));
        assert!(may_break('中', 'a', r));
        assert!(may_break('-', 'a', r));
        assert!(!may_break('-', '1', r));
        assert!(!may_break('a', '-', r));
        assert!(!may_break('中', '。', r));
        assert!(may_break('。', '中', r));
        assert!(!may_break('「', '中', r));
        assert!(!may_break('a', '\u{A0}', r));
        assert!(may_break('\u{200B}', 'a', r));
        assert!(may_break('\u{AD}', 'a', r));
        assert!(!may_break('キ', 'ャ', r));
        assert!(may_break('中', '〜', r));
        assert!(!may_break('a', ')', r));
    }

    /// §5.2 / §5.3: `break-all`, `keep-all`, `strict`, `loose`,
    /// `anywhere`, and `hyphens: none`.
    #[test]
    fn the_values_change_the_pairs() {
        let all = rules(WordBreak::BreakAll, LineBreak::Auto);
        assert!(may_break('a', 'b', all));
        assert!(!may_break('a', '.', all));
        let keep = rules(WordBreak::KeepAll, LineBreak::Auto);
        assert!(!may_break('中', '文', keep));
        assert!(!may_break('a', '中', keep));
        assert!(may_break('-', 'a', keep));
        let strict = rules(WordBreak::Normal, LineBreak::Strict);
        assert!(!may_break('中', '〜', strict));
        let loose = rules(WordBreak::Normal, LineBreak::Loose);
        assert!(may_break('キ', 'ャ', loose));
        assert!(may_break('中', '‐', loose));
        assert!(may_break('中', '・', loose));
        assert!(may_break('…', '…', loose));
        let anywhere = rules(WordBreak::Normal, LineBreak::Anywhere);
        assert!(may_break('a', 'b', anywhere));
        assert!(may_break('a', '\u{A0}', anywhere));
        let none = BreakRules {
            hyphens: Hyphens::None,
            ..BreakRules::default()
        };
        assert!(!may_break('\u{AD}', 'a', none));
    }
}
