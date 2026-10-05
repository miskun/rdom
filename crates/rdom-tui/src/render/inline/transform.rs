//! `text-transform` (CSS Text 3 §2.1, CSS Text 4 §2.1, MathML Core §4.2):
//! what a grapheme of text renders as. Applied after white space
//! collapsing and before line breaking (§2.1, Appendix A), so the
//! transformed text is what is measured and broken (a full-width letter
//! breaks like an ideograph); the source text stays what the caret,
//! selection and copy read (`source_map`).
//!
//! - Case: the full Unicode mappings (`char::to_uppercase` /
//!   `to_lowercase` include SpecialCasing's unconditional ones — `ß` →
//!   `SS`), Final_Sigma for a lowercased `Σ`, and titlecase for
//!   `capitalize` (the characters whose titlecase is not their uppercase
//!   are tabled).
//! - `capitalize` words: a letter starts a word unless a letter, digit or
//!   word-internal apostrophe precedes it (so `don't` stays one word and
//!   `well-known` is two; Gecko and Blink split at the hyphen too).
//! - `full-width`: the `<wide>` forms of ASCII and its signs, U+3000 for a
//!   preserved space, and the fullwidth forms of halfwidth katakana.
//! - `full-size-kana`: the Text 4 §2.1 table.
//! - `math-auto`: a text node of one character, by MathML Core's italic
//!   table.

use std::borrow::Cow;

use crate::layout::{TextCase, TextTransform};

/// The context case mapping reads across graphemes.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CaseContext {
    /// The text before is inside a word: a letter here does not start one.
    pub(crate) in_word: bool,
    /// The character before is a cased letter (Final_Sigma's condition).
    pub(crate) after_cased: bool,
}

impl CaseContext {
    /// The context after the text `g` (rendered or not).
    pub(crate) fn after(self, g: &str) -> Self {
        let mut ctx = self;
        for c in g.chars() {
            ctx.in_word = c.is_alphanumeric() || (ctx.in_word && is_apostrophe(c)) || is_mark(c);
            ctx.after_cased = is_cased(c) || (ctx.after_cased && is_mark(c));
        }
        ctx
    }

    /// White space or a forced break: no word continues across it.
    pub(crate) fn break_word(&mut self) {
        *self = CaseContext::default();
    }
}

/// The text the grapheme `g` renders as under `transform`, `ctx` the
/// context before it and `next` the character after it in its text node;
/// `None` when it renders as itself.
pub(crate) fn apply<'a>(
    g: &'a str,
    transform: TextTransform,
    ctx: CaseContext,
    next: Option<char>,
) -> Option<Cow<'a, str>> {
    if transform.is_none() {
        return None;
    }
    let mut out: Cow<'a, str> = Cow::Borrowed(g);
    match transform.case {
        TextCase::None => {}
        TextCase::Uppercase => out = map_chars(&out, |c, s| s.extend(c.to_uppercase())),
        TextCase::Lowercase => {
            out = map_chars(&out, |c, s| {
                if c == 'Σ' {
                    // Unicode Final_Sigma: after a cased letter and not
                    // before one.
                    let fin = ctx.after_cased && !next.is_some_and(is_cased);
                    s.push(if fin { 'ς' } else { 'σ' });
                } else {
                    s.extend(c.to_lowercase());
                }
            });
        }
        TextCase::Capitalize => {
            if !ctx.in_word
                && let Some(first) = out.chars().next()
                && first.is_alphanumeric()
            {
                let mut s = String::with_capacity(out.len() + 2);
                titlecase(first, &mut s);
                s.push_str(&out[first.len_utf8()..]);
                out = Cow::Owned(s);
            }
        }
    }
    if transform.full_width {
        out = map_chars(&out, |c, s| s.push(full_width(c)));
    }
    if transform.full_size_kana {
        out = map_chars(&out, |c, s| s.push(full_size_kana(c)));
    }
    (out != g).then_some(out)
}

/// MathML Core §4.2 `math-auto`: the italic form of `c`, a text node's one
/// character, from Appendix C.1's table.
pub(crate) fn math_italic(c: char) -> Option<char> {
    let offset = |base: u32| char::from_u32(c as u32 + base);
    match c {
        'h' => Some('\u{210E}'),
        'A'..='Z' => offset(0x1D3F3),
        'a'..='z' => offset(0x1D3ED),
        'ı' => Some('\u{1D6A4}'),
        'ȷ' => Some('\u{1D6A5}'),
        '\u{0391}'..='\u{03A1}' | '\u{03A3}'..='\u{03A9}' => offset(0x1D351),
        'ϴ' => Some('\u{1D6F3}'),
        '∇' => Some('\u{1D6FB}'),
        '\u{03B1}'..='\u{03C9}' => offset(0x1D34B),
        '∂' => Some('\u{1D715}'),
        'ϵ' => Some('\u{1D716}'),
        'ϑ' => Some('\u{1D717}'),
        'ϰ' => Some('\u{1D718}'),
        'ϕ' => Some('\u{1D719}'),
        'ϱ' => Some('\u{1D71A}'),
        'ϖ' => Some('\u{1D71B}'),
        _ => None,
    }
}

/// `text` with each character mapped by `f`, borrowed when unchanged.
fn map_chars<'a>(text: &Cow<'a, str>, f: impl Fn(char, &mut String)) -> Cow<'a, str> {
    let mut s = String::with_capacity(text.len());
    for c in text.chars() {
        f(c, &mut s);
    }
    if s == text.as_ref() {
        text.clone()
    } else {
        Cow::Owned(s)
    }
}

/// The titlecase mapping of `c` (UnicodeData's titlecase column and
/// SpecialCasing's titlecase): the uppercase mapping except for the
/// digraphs and ligatures listed.
fn titlecase(c: char, out: &mut String) {
    let special = match c {
        'Ǆ' | 'ǅ' | 'ǆ' => "ǅ",
        'Ǉ' | 'ǈ' | 'ǉ' => "ǈ",
        'Ǌ' | 'ǋ' | 'ǌ' => "ǋ",
        'Ǳ' | 'ǲ' | 'ǳ' => "ǲ",
        'ß' => "Ss",
        'ﬀ' => "Ff",
        'ﬁ' => "Fi",
        'ﬂ' => "Fl",
        'ﬃ' => "Ffi",
        'ﬄ' => "Ffl",
        'ﬅ' | 'ﬆ' => "St",
        'և' => "Եւ",
        'ﬓ' => "Մն",
        'ﬔ' => "Մե",
        'ﬕ' => "Մի",
        'ﬖ' => "Վն",
        'ﬗ' => "Մխ",
        _ => {
            out.extend(c.to_uppercase());
            return;
        }
    };
    out.push_str(special);
}

/// The full-width form of `c` (its `<wide>` compatibility form), or `c`.
fn full_width(c: char) -> char {
    const HALFWIDTH_KATAKANA: &str = "。「」、・ヲァィゥェォャュョッーアイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワン゛゜";
    match c {
        ' ' => '\u{3000}',
        '!'..='~' => char::from_u32(c as u32 - 0x21 + 0xFF01).unwrap_or(c),
        '¢' => '￠',
        '£' => '￡',
        '¬' => '￢',
        '¯' => '￣',
        '¦' => '￤',
        '¥' => '￥',
        '₩' => '￦',
        '\u{FF61}'..='\u{FF9F}' => HALFWIDTH_KATAKANA
            .chars()
            .nth((c as u32 - 0xFF61) as usize)
            .unwrap_or(c),
        _ => c,
    }
}

/// The full-size form of a small kana `c` (CSS Text 4 §2.1's table), or
/// `c`.
fn full_size_kana(c: char) -> char {
    const SMALL: &str = "ぁぃぅぇぉゕゖっゃゅょゎァィゥェォヵㇰヶㇱㇲッㇳㇴㇵㇶㇷㇸㇹㇺャュョㇻㇼㇽㇾㇿヮｧｨｩｪｫｯｬｭｮ\u{1B132}\u{1B150}\u{1B151}\u{1B152}\u{1B155}\u{1B164}\u{1B165}\u{1B166}\u{1B167}";
    const FULL: &str = "あいうえおかけつやゆよわアイウエオカクケシスツトヌハヒフヘホムヤユヨラリルレロワｱｲｳｴｵﾂﾔﾕﾖこゐゑをコヰヱヲン";
    SMALL
        .chars()
        .position(|s| s == c)
        .and_then(|i| FULL.chars().nth(i))
        .unwrap_or(c)
}

/// A cased letter (Unicode `Cased`, approximated by having a case).
fn is_cased(c: char) -> bool {
    c.is_lowercase() || c.is_uppercase()
}

/// An apostrophe that may sit inside a word (`don't`).
fn is_apostrophe(c: char) -> bool {
    matches!(c, '\'' | '\u{2019}')
}

/// A combining mark, which continues the letter before it.
fn is_mark(c: char) -> bool {
    matches!(c, '\u{0300}'..='\u{036F}' | '\u{1AB0}'..='\u{1AFF}' | '\u{1DC0}'..='\u{1DFF}' | '\u{20D0}'..='\u{20FF}' | '\u{FE20}'..='\u{FE2F}')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(case: TextCase) -> TextTransform {
        TextTransform {
            case,
            ..TextTransform::NONE
        }
    }

    /// The tables: a sample of each.
    #[test]
    fn the_tables() {
        assert_eq!(full_width('A'), 'Ａ');
        assert_eq!(full_width('~'), '～');
        assert_eq!(full_width('ｶ'), 'カ');
        assert_eq!(full_width('ﾟ'), '゜');
        assert_eq!(full_width('中'), '中');
        assert_eq!(full_size_kana('ㇿ'), 'ロ');
        assert_eq!(full_size_kana('ｮ'), 'ﾖ');
        assert_eq!(full_size_kana('\u{1B167}'), 'ン');
        assert_eq!(full_size_kana('あ'), 'あ');
        assert_eq!(math_italic('Z'), Some('\u{1D44D}'));
        assert_eq!(math_italic('ω'), Some('\u{1D714}'));
        assert_eq!(math_italic('1'), None);
    }

    /// Case mappings with their context.
    #[test]
    fn case_mapping() {
        let ctx = CaseContext::default();
        assert_eq!(
            apply("ß", t(TextCase::Uppercase), ctx, None).as_deref(),
            Some("SS")
        );
        assert_eq!(
            apply("ﬁ", t(TextCase::Capitalize), ctx, None).as_deref(),
            Some("Fi")
        );
        let cased = ctx.after("Ο");
        assert_eq!(
            apply("Σ", t(TextCase::Lowercase), cased, None).as_deref(),
            Some("ς")
        );
        assert_eq!(
            apply("Σ", t(TextCase::Lowercase), cased, Some('Α')).as_deref(),
            Some("σ")
        );
        assert_eq!(
            apply("Σ", t(TextCase::Lowercase), ctx, None).as_deref(),
            Some("σ")
        );
        let word = ctx.after("don'");
        assert_eq!(apply("t", t(TextCase::Capitalize), word, None), None);
        assert_eq!(apply("a", TextTransform::NONE, ctx, None), None);
    }
}
