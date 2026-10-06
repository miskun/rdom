//! Dispatch tests for `content` (CSS Generated Content 3 §2; C10-CONTENT):
//! the full grammar parses, serializes back, and resolves.

use super::*;
use crate::content::QuoteKind;
use crate::{Content, ContentContext, TuiStyle, Value};

/// Set `content: css` on a fresh style; its serialization back.
fn round_trip(css: &str) -> Result<String, DispatchError> {
    let mut style = TuiStyle::new();
    set("content", css, &mut style)?;
    Ok(serialize("content", &style).expect("serializes"))
}

fn parsed(css: &str) -> Content {
    let mut style = TuiStyle::new();
    set("content", css, &mut style).expect("parses");
    match style.content {
        Some(Value::Specified(c)) => c,
        other => panic!("{css}: {other:?}"),
    }
}

/// §2: `normal | none | [ <content-replacement> | <content-list> ] [ /
/// [ <string> | <counter> | attr() ]+ ]?` — the list's `<string>`,
/// `<counter>` (`counter()` / `counters()`) and `<quote>` items, and the
/// alt text after `/`.
#[test]
fn content_takes_the_full_grammar() {
    for (css, out) in [
        ("none", "none"),
        (r#""a""#, r#""a""#),
        ("counters(sec, \".\")", "counters(sec, \".\")"),
        (
            "counters(sec, \"-\", upper-roman)",
            "counters(sec, \"-\", upper-roman)",
        ),
        ("open-quote", "open-quote"),
        ("CLOSE-QUOTE", "close-quote"),
        (
            "no-open-quote no-close-quote",
            "no-open-quote no-close-quote",
        ),
        (
            r#"open-quote "x" counter(n) close-quote"#,
            r#"open-quote "x" counter(n) close-quote"#,
        ),
        (r#""★" / "star""#, r#""★" / "star""#),
        (r#""★" / "a" counter(n)"#, r#""★" / "a" counter(n)"#),
    ] {
        assert_eq!(round_trip(css).as_deref(), Ok(out), "{css}");
    }
}

/// §2: what the grammar rejects — an empty alt, a quote in the alt text
/// (only strings, counters and `attr()`), `none` / `normal` with an alt,
/// `counters()` without its separator string, two slashes.
#[test]
fn content_rejects_what_the_grammar_does_not_allow() {
    for css in [
        r#""a" /"#,
        r#""a" / open-quote"#,
        r#"none / "x""#,
        "counters(sec)",
        "counters(sec, sep)",
        r#""a" / "b" / "c""#,
        r#"/ "alt""#,
    ] {
        assert!(round_trip(css).is_err(), "{css} must be rejected");
    }
}

/// A context with one counter, `sec`, nested 1 → 2 → 3, and a quote
/// depth that the quote items move.
struct Cx {
    depth: std::cell::Cell<u32>,
}

impl ContentContext for Cx {
    fn var(&self, _: &str) -> Option<String> {
        None
    }
    fn counter(&self, name: &str) -> i32 {
        if name == "sec" { 3 } else { 0 }
    }
    fn counters(&self, name: &str) -> Vec<i32> {
        if name == "sec" {
            vec![1, 2, 3]
        } else {
            vec![0]
        }
    }
    fn quote(&self, kind: QuoteKind) -> String {
        let d = self.depth.get();
        match kind {
            QuoteKind::Open => {
                self.depth.set(d + 1);
                format!("<{d}")
            }
            QuoteKind::Close if d > 0 => {
                self.depth.set(d - 1);
                format!("{}>", d - 1)
            }
            QuoteKind::NoOpen => {
                self.depth.set(d + 1);
                String::new()
            }
            QuoteKind::NoClose if d > 0 => {
                self.depth.set(d - 1);
                String::new()
            }
            _ => String::new(),
        }
    }
}

/// CSS Lists 3 §4.3 (`counters()` joins every counter of the name in
/// scope, outermost first, with the separator) and Generated Content 3
/// §2 (quote items in order; the alt text is not the content).
#[test]
fn content_resolves_counters_quotes_and_alt_text() {
    let cx = Cx {
        depth: std::cell::Cell::new(0),
    };
    let c = parsed(r#"counters(sec, ".") " " counters(sec, "-", lower-alpha)"#);
    assert_eq!(c.resolve(&cx).as_deref(), Some("1.2.3 a-b-c"));
    let q = parsed(r#"open-quote open-quote "x" close-quote close-quote close-quote"#);
    assert_eq!(q.resolve(&cx).as_deref(), Some("<0<1x1>0>"));
    let alt = parsed(r#""★" / "star " counter(sec)"#);
    assert_eq!(alt.resolve(&cx).as_deref(), Some("★"));
    assert_eq!(alt.resolve_alt(&cx).as_deref(), Some("star 3"));
    assert_eq!(parsed(r#""★""#).resolve_alt(&cx), None);
    assert!(parsed("open-quote").uses_quotes());
    assert!(!parsed(r#""a""#).uses_quotes());
    assert!(parsed(r#""a" / counter(x)"#).uses_counters());
}

// ── quotes (C10-QUOTES) ──────────────────────────────────────────────

/// Set `quotes: css` on a fresh style; its serialization back.
fn quotes_round_trip(css: &str) -> Result<String, DispatchError> {
    let mut style = TuiStyle::new();
    set("quotes", css, &mut style)?;
    Ok(serialize("quotes", &style).expect("serializes"))
}

/// CSS Generated Content 3 §2.1: `quotes: auto | none | match-parent |
/// [ <string> <string> ]+`, inherited, initial `auto`.
#[test]
fn quotes_takes_its_grammar() {
    for (css, out) in [
        ("auto", "auto"),
        ("NONE", "none"),
        ("match-parent", "match-parent"),
        (r#""<" ">""#, r#""<" ">""#),
        (r#""«" "»" "‹" "›""#, r#""«" "»" "‹" "›""#),
        ("inherit", "inherit"),
    ] {
        assert_eq!(quotes_round_trip(css).as_deref(), Ok(out), "{css}");
    }
    for css in [
        r#""<""#,
        r#""<" ">" "{""#,
        "auto none",
        r#"auto "<" ">""#,
        "x",
    ] {
        assert!(quotes_round_trip(css).is_err(), "{css} must be rejected");
    }
    assert!(inherits("quotes"));
}

/// §2.1 `auto`: "a typographically appropriate" system for the content
/// language — CLDR's delimiters, by primary language subtag, English as
/// the fallback; a level past the last pair repeats it.
#[test]
fn auto_quotes_follow_the_language() {
    use crate::Quotes;
    let auto = Quotes::Auto;
    let pair = |lang: Option<&str>, level| auto.pair(level, lang);
    assert_eq!(pair(None, 0), Some(("\u{201c}", "\u{201d}")));
    assert_eq!(pair(Some("en-GB"), 1), Some(("\u{2018}", "\u{2019}")));
    assert_eq!(pair(Some("de"), 0), Some(("\u{201e}", "\u{201c}")));
    assert_eq!(pair(Some("DE-ch"), 1), Some(("\u{201a}", "\u{2018}")));
    assert_eq!(pair(Some("fr"), 0), Some(("\u{ab}", "\u{bb}")));
    assert_eq!(pair(Some("fi"), 0), Some(("\u{201d}", "\u{201d}")));
    assert_eq!(pair(Some("ja"), 0), Some(("\u{300c}", "\u{300d}")));
    assert_eq!(pair(Some("ja"), 5), Some(("\u{300e}", "\u{300f}")));
    assert_eq!(pair(Some("tlh"), 0), Some(("\u{201c}", "\u{201d}")));
    assert_eq!(Quotes::None.pair(0, None), None);
}

// ── counter-reset / counter-set (C10-COUNTERS) ───────────────────────

/// Set `name: css` on a fresh style; its serialization back.
fn prop_round_trip(name: &str, css: &str) -> Result<String, DispatchError> {
    let mut style = TuiStyle::new();
    set(name, css, &mut style)?;
    Ok(serialize(name, &style).expect("serializes"))
}

/// CSS Lists 3 §4.2: `counter-reset: [ <counter-name> <integer>? |
/// reversed(<counter-name>) <integer>? ]+ | none`; §4.3: `counter-set:
/// [ <counter-name> <integer>? ]+ | none`, the integer defaulting to 0.
#[test]
fn counter_reset_takes_reversed_and_counter_set_parses() {
    for (name, css, out) in [
        ("counter-reset", "reversed(c)", "reversed(c)"),
        ("counter-reset", "reversed(c) 4 d", "reversed(c) 4 d 0"),
        (
            "counter-reset",
            "REVERSED(list-item) -2",
            "reversed(list-item) -2",
        ),
        ("counter-set", "c", "c 0"),
        ("counter-set", "c 5 d -1", "c 5 d -1"),
        ("counter-set", "none", "none"),
    ] {
        assert_eq!(
            prop_round_trip(name, css).as_deref(),
            Ok(out),
            "{name}: {css}"
        );
    }
    for (name, css) in [
        ("counter-set", "reversed(c)"),
        ("counter-increment", "reversed(c)"),
        ("counter-reset", "reversed(none)"),
        ("counter-reset", "reversed()"),
        ("counter-set", "none c"),
    ] {
        assert!(
            prop_round_trip(name, css).is_err(),
            "{name}: {css} must be rejected"
        );
    }
    assert!(!inherits("counter-set"));
}

// ── <counter-style>: names and symbols() (C10-COUNTER-STYLE) ─────────

/// CSS Counter Styles 3 §5: `symbols( <symbols-type>? <string>+ )` is a
/// `<counter-style>` — `symbolic` by default; `alphabetic` / `numeric`
/// need two symbols; an author name is any `<custom-ident>`.
#[test]
fn counter_takes_symbols_and_author_names() {
    for (css, out) in [
        (r#"counter(c, symbols("*"))"#, r#"counter(c, symbols("*"))"#),
        (
            r#"counter(c, symbols(cyclic "a" "b"))"#,
            r#"counter(c, symbols(cyclic "a" "b"))"#,
        ),
        (
            r#"counters(c, ".", symbols(numeric "0" "1"))"#,
            r#"counters(c, ".", symbols(numeric "0" "1"))"#,
        ),
        ("counter(c, thumbs)", "counter(c, thumbs)"),
        ("counter(c, Thumbs)", "counter(c, Thumbs)"),
    ] {
        assert_eq!(round_trip(css).as_deref(), Ok(out), "{css}");
    }
    for css in [
        r#"counter(c, symbols(alphabetic "a"))"#,
        r#"counter(c, symbols(additive "a"))"#,
        r#"counter(c, symbols(extends "a"))"#,
        "counter(c, symbols())",
        "counter(c, symbols(cyclic))",
        "counter(c, inherit)",
    ] {
        assert!(round_trip(css).is_err(), "{css} must be rejected");
    }
}
