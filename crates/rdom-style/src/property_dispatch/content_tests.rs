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
