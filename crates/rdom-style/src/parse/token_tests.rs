//! Tokenizer tests: numbers and their type flag, dimensions, escapes,
//! strings, url tokens.
use super::*;

/// C4G-SERIALIZE — CSS Syntax 3 §4.3.4 / §4.3.6: an unquoted
/// `url(` is one `<url-token>` holding the raw text (so `0001.png`
/// keeps its zeros), whitespace around it dropped and escapes
/// decoded; a quoted one is a function with a string; whitespace
/// inside, a quote or a `(` make a `<bad-url-token>` (§4.3.14), which
/// swallows up to the `)`.
#[test]
fn unquoted_url_is_one_url_token() {
    let url = |s: &str| Token::Url(s.to_string());
    assert_eq!(toks("url(0001.png)"), vec![url("0001.png")]);
    assert_eq!(
        toks("URL(  a1.5e3.png  ) x"),
        vec![url("a1.5e3.png"), Token::Ident("x".into())]
    );
    assert_eq!(toks(r"url(a\)b)"), vec![url("a)b")]);
    assert_eq!(toks("url(a"), vec![url("a")]);
    assert_eq!(
        toks("url( \"a.png\")"),
        vec![
            Token::Function("url".into()),
            Token::String("a.png".into()),
            Token::RParen
        ]
    );
    assert_eq!(
        toks("url(a b) red"),
        vec![Token::BadUrl, Token::Ident("red".into())]
    );
    assert_eq!(
        toks("url(a\"b) red"),
        vec![Token::BadUrl, Token::Ident("red".into())]
    );
    assert_eq!(
        toks("url(a(b) red"),
        vec![Token::BadUrl, Token::Ident("red".into())]
    );
    // The serializer escapes what a url token cannot hold raw.
    for text in ["a b", "a)b", "q\"'", "x\\y"] {
        let rendered = crate::parse::values::render_value(&[url(text)]);
        assert_eq!(toks(&rendered), vec![url(text)], "{rendered}");
    }
}

fn toks(src: &str) -> Vec<Token> {
    tokenize(src).unwrap()
}

/// CSS Syntax 3 §4.3.12: a numeric literal is consumed whole. An
/// integer stays `Number`; anything with a fraction or exponent is
/// a `Float` — never `Number Delim('.') Number`, which loses the
/// leading zeros of the fraction (`0.05` came out as 0.5).
#[test]
fn decimals_are_single_float_tokens() {
    assert_eq!(toks("0.05"), vec![Token::Float(0.05)]);
    assert_eq!(toks("1.5"), vec![Token::Float(1.5)]);
    assert_eq!(toks(".5"), vec![Token::Float(0.5)]);
    assert_eq!(toks("12"), vec![Token::Number(12)]);
}

#[test]
fn exponents_are_part_of_the_number() {
    assert_eq!(toks("1e3"), vec![Token::Float(1000.0)]);
    assert_eq!(toks("2.5E-1"), vec![Token::Float(0.25)]);
    // `e` not followed by a digit starts a unit, not an exponent.
    assert_eq!(toks("1em"), vec![dim(1.0, true, "em")]);
}

#[test]
fn percentages_carry_fractions() {
    assert_eq!(toks("50%"), vec![Token::Percentage(50.0)]);
    assert_eq!(toks("12.5%"), vec![Token::Percentage(12.5)]);
    // Whitespace breaks the promotion.
    assert_eq!(toks("50 %"), vec![Token::Number(50), Token::Delim('%')]);
}

/// C4G-NUMBER-RANGE / C5G-INT-CLAMP-SITE — CSS Syntax 3 §4.3.12 /
/// §4.3.13: a literal of digits alone has the *integer* type flag
/// whatever its size, and keeps its value: CSS Values 4 §5.1's clamp to
/// the implementation's range belongs to the property that consumes the
/// `<integer>`, not to the token — a custom property passes the literal
/// on unchanged. Past `i64` the token saturates. A dimension's number
/// part keeps its value too; a percentage has no type flag.
#[test]
fn oversized_integer_keeps_its_value_and_stays_integer() {
    assert_eq!(toks("99999999999"), vec![Token::Number(99_999_999_999)]);
    assert_eq!(
        toks("99999999999999999999999"),
        vec![Token::Number(i64::MAX)]
    );
    assert_eq!(
        toks("99999999999px"),
        vec![Token::Dimension {
            value: 99_999_999_999.0,
            integer: true,
            unit: "px".to_string(),
        }]
    );
    assert_eq!(
        toks("99999999999%"),
        vec![Token::Percentage(99_999_999_999.0)]
    );
    assert_eq!(toks("99999999999.5"), vec![Token::Float(99_999_999_999.5)]);
}

/// CSS Values 4 §7: numbers that overflow the representable range
/// clamp to the largest finite value — no `inf` or `NaN` token ever
/// reaches a value parser.
#[test]
fn overflowing_literals_clamp_to_finite() {
    for src in ["1e400", "1e400%", ".1e400"] {
        for tok in toks(src) {
            match tok {
                Token::Float(f) | Token::Percentage(f) => {
                    assert!(f.is_finite(), "{src} → {f}");
                    assert_eq!(f, f64::MAX, "{src}");
                }
                other => panic!("{src} → {other:?}"),
            }
        }
    }
}

#[test]
fn more_number_edge_cases() {
    assert_eq!(toks("1e"), vec![dim(1.0, true, "e")]);
    assert_eq!(toks("1e+"), vec![dim(1.0, true, "e"), Token::Delim('+')]);
    assert_eq!(toks("-.5"), vec![Token::Delim('-'), Token::Float(0.5)]);
    assert_eq!(toks("1.5.5"), vec![Token::Float(1.5), Token::Float(0.5)]);
    assert_eq!(
        toks("50%%"),
        vec![Token::Percentage(50.0), Token::Delim('%')]
    );
    assert_eq!(toks("1E3"), vec![Token::Float(1000.0)]);
}

/// A trailing `.` with no digit after it is not part of the number.
#[test]
fn dot_without_following_digit_is_a_delim() {
    assert_eq!(toks("1."), vec![Token::Number(1), Token::Delim('.')]);
    assert_eq!(
        toks("a.b"),
        vec![
            Token::Ident("a".to_string()),
            Token::Delim('.'),
            Token::Ident("b".to_string())
        ]
    );
}

/// CSS Syntax 3 §4.3.7 "consume an escaped code point": `\` + 1..6
/// hex digits is a code point (one following whitespace is eaten);
/// `\` + any other char is that char; an escaped newline inside a
/// string is dropped (§4.3.5).
#[test]
fn string_escapes_decode_hex_and_literal_forms() {
    assert_eq!(
        toks(r#""\201C""#),
        vec![Token::String("\u{201C}".to_string())]
    );
    assert_eq!(
        toks(r#""\201c quote""#),
        vec![Token::String("\u{201C}quote".to_string())]
    );
    assert_eq!(toks(r#""a\"b""#), vec![Token::String("a\"b".to_string())]);
    assert_eq!(toks(r#""a\\b""#), vec![Token::String("a\\b".to_string())]);
    assert_eq!(toks("\"a\\\nb\""), vec![Token::String("ab".to_string())]);
    // Out-of-range / surrogate code points become U+FFFD.
    assert_eq!(
        toks(r#""\110000""#),
        vec![Token::String("\u{FFFD}".to_string())]
    );
}

/// Identifiers may contain non-ASCII code points (§4.3.9): `größe`
/// and `--größe` are single idents, not ident + garbage.
#[test]
fn non_ascii_identifiers_are_single_tokens() {
    assert_eq!(toks("größe"), vec![Token::Ident("größe".to_string())]);
    assert_eq!(toks("--größe"), vec![Token::Ident("--größe".to_string())]);
    assert_eq!(toks("日本語"), vec![Token::Ident("日本語".to_string())]);
}

/// CSS Syntax 3 §4.3.7 / §4.3.11: an identifier may contain (and
/// start with) escapes, decoded in the token; `\` + newline is not
/// a valid escape (§4.3.8), so it ends the identifier and is a
/// `Delim('\\')`.
#[test]
fn identifier_escapes_decode() {
    let ident = |s: &str| Token::Ident(s.to_string());
    assert_eq!(toks(r"col\6f r"), vec![ident("color")]);
    assert_eq!(toks(r"\31 0"), vec![ident("10")]);
    assert_eq!(toks(r"-\31 x"), vec![ident("-1x")]);
    assert_eq!(toks(r"a\:b"), vec![ident("a:b")]);
    assert_eq!(toks(r"r\67 b("), vec![Token::Function("rgb".to_string())]);
    assert_eq!(
        toks("a\\\nb"),
        vec![ident("a"), Token::Delim('\\'), ident("b")]
    );
}

#[test]
fn times_are_dimensions() {
    assert_eq!(toks("1.05s"), vec![dim(1.05, false, "s")]);
    assert_eq!(toks("200ms"), vec![dim(200.0, true, "ms")]);
}

/// CSS Syntax 3 §4.3.3: a number immediately followed by an ident
/// is one `<dimension-token>`; with whitespace between, a number and
/// an ident (`C1G-VAR-TOKENS`).
#[test]
fn a_number_and_a_following_ident_form_a_dimension_only_when_adjacent() {
    assert_eq!(toks("1fr"), vec![dim(1.0, true, "fr")]);
    assert_eq!(toks("1.5s"), vec![dim(1.5, false, "s")]);
    assert_eq!(
        toks("1 fr"),
        vec![Token::Number(1), Token::Ident("fr".to_string())]
    );
    assert_eq!(toks(r"2\66 r"), vec![dim(2.0, true, "fr")]);
}

fn dim(value: f64, integer: bool, unit: &str) -> Token {
    Token::Dimension {
        value,
        integer,
        unit: unit.to_string(),
    }
}
