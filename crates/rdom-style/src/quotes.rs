//! `quotes` (CSS Generated Content 3 §2.1): the quotation marks that
//! `open-quote` / `close-quote` insert, one pair per nesting level.

use std::sync::Arc;

/// One level's quotation marks.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QuotePair {
    /// The mark `open-quote` inserts.
    pub open: String,
    /// The mark `close-quote` inserts.
    pub close: String,
}

/// The `quotes` value (§2.1). Inherited; initial `auto`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Quotes {
    /// The marks of the content language ([`auto_quotes`]).
    #[default]
    Auto,
    /// No marks: `open-quote` / `close-quote` insert nothing (the quote
    /// depth still moves, §2.2).
    None,
    /// The parent's marks. The cascade computes it away: the parent's
    /// value, an `auto` one made explicit in the parent's language.
    MatchParent,
    /// `[<string> <string>]+`: the outermost level's pair first.
    Pairs(Arc<[QuotePair]>),
}

impl Quotes {
    /// The pair of nesting `level` (0 = outermost; a level past the last
    /// pair uses the last), `None` under `quotes: none`. `auto` takes the
    /// marks of `lang`, the content language (a BCP 47 tag);
    /// `match-parent` left uncomputed reads as `auto`.
    pub fn pair(&self, level: u32, lang: Option<&str>) -> Option<(&str, &str)> {
        let at = |len: usize| (level as usize).min(len - 1);
        match self {
            Quotes::None => None,
            Quotes::Pairs(pairs) => {
                let p = &pairs[at(pairs.len())];
                Some((&p.open, &p.close))
            }
            Quotes::Auto | Quotes::MatchParent => {
                let pairs = auto_quotes(lang);
                Some(pairs[at(pairs.len())])
            }
        }
    }

    /// `auto` made explicit for `lang`; any other value as it is — the
    /// computed value `match-parent` takes from a parent (§2.1).
    pub fn explicit(&self, lang: Option<&str>) -> Quotes {
        match self {
            Quotes::Auto | Quotes::MatchParent => Quotes::Pairs(
                auto_quotes(lang)
                    .iter()
                    .map(|(open, close)| QuotePair {
                        open: (*open).to_string(),
                        close: (*close).to_string(),
                    })
                    .collect(),
            ),
            other => other.clone(),
        }
    }
}

/// The marks `quotes: auto` uses for a content language, outermost
/// level first: CLDR's quotation and alternate quotation delimiters,
/// chosen by the tag's primary language subtag (ASCII case-insensitive);
/// English for no language or one not in the table.
pub fn auto_quotes(lang: Option<&str>) -> &'static [(&'static str, &'static str)] {
    const EN: &[(&str, &str)] = &[("\u{201c}", "\u{201d}"), ("\u{2018}", "\u{2019}")];
    const LOW9: &[(&str, &str)] = &[("\u{201e}", "\u{201c}"), ("\u{201a}", "\u{2018}")];
    const GUILLEMETS: &[(&str, &str)] = &[("\u{ab}", "\u{bb}"), ("\u{ab}", "\u{bb}")];
    const GUILLEMETS_EN: &[(&str, &str)] = &[("\u{ab}", "\u{bb}"), ("\u{201c}", "\u{201d}")];
    const GUILLEMETS_LOW9: &[(&str, &str)] = &[("\u{ab}", "\u{bb}"), ("\u{201e}", "\u{201c}")];
    const RIGHT: &[(&str, &str)] = &[("\u{201d}", "\u{201d}"), ("\u{2019}", "\u{2019}")];
    const CJK: &[(&str, &str)] = &[("\u{300c}", "\u{300d}"), ("\u{300e}", "\u{300f}")];
    const PL: &[(&str, &str)] = &[("\u{201e}", "\u{201d}"), ("\u{ab}", "\u{bb}")];
    const NB: &[(&str, &str)] = &[("\u{ab}", "\u{bb}"), ("\u{2018}", "\u{2019}")];
    let primary = lang
        .and_then(|l| l.split(['-', '_']).next())
        .map(str::to_ascii_lowercase);
    match primary.as_deref() {
        Some("de" | "cs" | "sk" | "lt" | "bg" | "is") => LOW9,
        Some("fr") => GUILLEMETS,
        Some("es" | "it" | "ca" | "el") => GUILLEMETS_EN,
        Some("ru" | "uk" | "be") => GUILLEMETS_LOW9,
        Some("fi" | "sv") => RIGHT,
        Some("ja") => CJK,
        Some("pl") => PL,
        Some("nb" | "no" | "nn") => NB,
        _ => EN,
    }
}
