//! Generating a counter's representation (CSS Counter Styles 3 §3.1 —
//! "generate a counter representation"): range check, the system's
//! algorithm, padding, the negative sign, fallback — and, for a marker,
//! the prefix and suffix.

use super::rule::{CounterRange, CounterStyleRule, System};
use super::style::CounterStyleLookup;

/// The longest representation generated, in Unicode code points (§3.1:
/// UAs "must support representations at least 60 Unicode codepoints
/// long" and may use the fallback style past that). A longer one —
/// a `symbolic` or `additive` style's huge value, a large `pad` — is
/// not built: the fallback style writes the value instead, and `pad`
/// never pads past it. This bounds every allocation by the cap.
pub const MAX_REPRESENTATION_CHARS: usize = 60;

/// How many styles a fallback (§3.7) or `extends` (§3.1.7) chain visits
/// before it stops at `decimal`. A cycle stops at its first repeat.
pub const MAX_FALLBACK_DEPTH: usize = 8;

/// What is generated: `counter()`'s text, or a marker's (with the
/// style's prefix and suffix).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Representation {
    Counter,
    Marker,
}

/// A style's descriptors with `extends` resolved and every default
/// filled in.
struct Resolved<'a> {
    system: &'a System,
    symbols: &'a [String],
    additive: &'a [(u32, String)],
    negative: (&'a str, &'a str),
    prefix: &'a str,
    suffix: &'a str,
    range: Option<&'a CounterRange>,
    pad: Option<(u32, &'a str)>,
    fallback: &'a str,
}

/// `name`'s descriptors through `lookup`, its `extends` chain resolved
/// (§3.1.7: a missing base is `decimal`; a cycle extends `decimal`).
/// `None` when nothing defines `name`.
fn resolve<'a>(lookup: &'a impl CounterStyleLookup, name: &str) -> Option<Resolved<'a>> {
    let mut chain: Vec<(&str, &'a CounterStyleRule)> = Vec::new();
    let mut cur = name;
    let base = loop {
        let rule = match lookup.rule(cur) {
            Some(rule) => rule,
            // An undefined base (or an undefined `name`) is `decimal`.
            None if chain.is_empty() => return None,
            None => lookup.rule("decimal")?,
        };
        match rule.system.as_ref() {
            Some(System::Extends(next))
                if chain.len() < MAX_FALLBACK_DEPTH
                    && !chain.iter().any(|(n, _)| *n == &**next) =>
            {
                chain.push((cur, rule));
                cur = next;
            }
            // A cycle or an over-long chain: extends `decimal`.
            Some(System::Extends(_)) => {
                chain.push((cur, rule));
                break lookup.rule("decimal")?;
            }
            _ => break rule,
        }
    };
    // The base's descriptors, each overridden by the nearest rule of the
    // chain that declares it.
    let pick = |f: &dyn Fn(&'a CounterStyleRule) -> bool| {
        chain.iter().map(|(_, r)| *r).find(|r| f(r)).unwrap_or(base)
    };
    let system = base.system.as_ref().unwrap_or(&System::Symbolic);
    let negative = pick(&|r| r.negative.is_some()).negative.as_ref();
    Some(Resolved {
        system,
        symbols: base.symbols.as_deref().unwrap_or(&[]),
        additive: base.additive_symbols.as_deref().unwrap_or(&[]),
        negative: negative.map_or(("-", ""), |(a, b)| (a.as_str(), b.as_str())),
        prefix: pick(&|r| r.prefix.is_some())
            .prefix
            .as_deref()
            .unwrap_or(""),
        suffix: pick(&|r| r.suffix.is_some())
            .suffix
            .as_deref()
            .unwrap_or(". "),
        range: pick(&|r| r.range.is_some()).range.as_ref(),
        pad: pick(&|r| r.pad.is_some())
            .pad
            .as_ref()
            .map(|(n, s)| (*n, s.as_str())),
        fallback: pick(&|r| r.fallback.is_some())
            .fallback
            .as_deref()
            .unwrap_or("decimal"),
    })
}

/// `value` in the style `name` (resolved through `lookup`; undefined is
/// `decimal`, `none` nothing), as `what` asks: `counter()`'s text, or a
/// marker's with the prefix and suffix. `rtl`: the text's direction,
/// which `disclosure-closed` points along.
pub(super) fn generate(
    lookup: &impl CounterStyleLookup,
    name: &str,
    value: i32,
    rtl: bool,
    what: Representation,
) -> String {
    if name == "none" {
        return String::new();
    }
    let mut visited: Vec<&str> = Vec::new();
    let mut cur = name;
    loop {
        let Some(style) = resolve(lookup, cur) else {
            // An undefined style is `decimal` (§3: "If … it does not
            // name a defined counter style, use decimal").
            if cur == "decimal" {
                return decimal_fallback(value, what);
            }
            cur = "decimal";
            continue;
        };
        if let Some(text) = represent(&style, cur, value, rtl) {
            return match what {
                Representation::Counter => text,
                Representation::Marker => format!("{}{text}{}", style.prefix, style.suffix),
            };
        }
        // Out of range, or the algorithm failed: the fallback (§3.7), at
        // most `MAX_FALLBACK_DEPTH` styles deep, never twice.
        visited.push(cur);
        let next = style.fallback;
        cur = if visited.len() >= MAX_FALLBACK_DEPTH || visited.contains(&next) {
            "decimal"
        } else {
            next
        };
        if visited.contains(&cur) {
            return decimal_fallback(value, what);
        }
    }
}

/// `decimal` with no lookup: the last resort when even `decimal` failed
/// (a lookup that does not define it).
fn decimal_fallback(value: i32, what: Representation) -> String {
    match what {
        Representation::Counter => value.to_string(),
        Representation::Marker => format!("{value}. "),
    }
}

/// The representation of `value` in `style` without prefix and suffix,
/// or `None` when it is out of range, the algorithm fails, or the text
/// would pass [`MAX_REPRESENTATION_CHARS`] (§3.1).
fn represent(style: &Resolved<'_>, name: &str, value: i32, rtl: bool) -> Option<String> {
    let v = i64::from(value);
    if !in_range(style, v) {
        return None;
    }
    let needs_symbols = !matches!(style.system, System::Additive);
    if needs_symbols && style.symbols.is_empty() {
        return None;
    }
    let negative = v < 0 && style.system.uses_negative();
    let magnitude = if negative { -v } else { v };
    let mut text = match style.system {
        System::Cyclic => {
            let i = (magnitude - 1).rem_euclid(style.symbols.len() as i64) as usize;
            // `disclosure-closed` points to the inline end (§6.3).
            if rtl && name == "disclosure-closed" {
                "\u{25c2}".to_string()
            } else {
                style.symbols[i].clone()
            }
        }
        System::Fixed(first) => {
            let i = usize::try_from(magnitude - i64::from(*first)).ok()?;
            style.symbols.get(i)?.clone()
        }
        System::Symbolic => symbolic(style.symbols, magnitude)?,
        System::Alphabetic => alphabetic(style.symbols, magnitude)?,
        System::Numeric => numeric(style.symbols, magnitude),
        System::Additive => additive(style.additive, magnitude)?,
        // `resolve` replaced it with its base's system.
        System::Extends(_) => return None,
    };
    // `pad` (§3.6): to its length in graphemes — code points here — the
    // negative sign counting.
    if let Some((len, symbol)) = style.pad {
        let sign = if negative {
            style.negative.0.chars().count() + style.negative.1.chars().count()
        } else {
            0
        };
        let have = text.chars().count() + sign;
        let want = (len as usize).min(MAX_REPRESENTATION_CHARS);
        if have < want && !symbol.is_empty() {
            let pads = (want - have).div_ceil(symbol.chars().count().max(1));
            text.insert_str(0, &symbol.repeat(pads));
        }
    }
    if negative {
        text = format!("{}{text}{}", style.negative.0, style.negative.1);
    }
    (text.chars().count() <= MAX_REPRESENTATION_CHARS).then_some(text)
}

fn in_range(style: &Resolved<'_>, v: i64) -> bool {
    match style.range {
        Some(CounterRange::Ranges(ranges)) => ranges.iter().any(|&(lo, hi)| lo <= v && v <= hi),
        _ => {
            let (lo, hi) = style.system.auto_range();
            lo <= v && v <= hi
        }
    }
}

/// §3.1.3: the symbol `(n - 1) mod len`, written `ceil(n / len)` times.
fn symbolic(symbols: &[String], n: i64) -> Option<String> {
    if n < 1 {
        return None;
    }
    let len = symbols.len() as i64;
    let symbol = &symbols[((n - 1) % len) as usize];
    let times = usize::try_from((n + len - 1) / len).ok()?;
    if times.saturating_mul(symbol.chars().count()) > MAX_REPRESENTATION_CHARS {
        return None;
    }
    Some(symbol.repeat(times))
}

/// §3.1.5: bijective base-`len` (`a`, …, `z`, `aa`, …).
fn alphabetic(symbols: &[String], n: i64) -> Option<String> {
    if n < 1 || symbols.len() < 2 {
        return None;
    }
    let len = symbols.len() as i64;
    let mut digits = Vec::new();
    let mut k = n;
    while k > 0 {
        k -= 1;
        digits.push(&symbols[(k % len) as usize]);
        k /= len;
    }
    Some(digits.into_iter().rev().map(String::as_str).collect())
}

/// §3.1.4: positional base-`len`, the first symbol the zero digit.
fn numeric(symbols: &[String], n: i64) -> String {
    let len = symbols.len().max(2) as i64;
    if n == 0 {
        return symbols[0].clone();
    }
    let mut digits = Vec::new();
    let mut k = n;
    while k > 0 {
        digits.push(&symbols[(k % len) as usize]);
        k /= len;
    }
    digits.into_iter().rev().map(String::as_str).collect()
}

/// §3.1.6: the weighted symbols, greedily from the heaviest; `None`
/// when they cannot sum to `n`, or the text would pass the cap (checked
/// before anything is built).
fn additive(tuples: &[(u32, String)], n: i64) -> Option<String> {
    if n == 0 {
        return tuples.iter().find(|(w, _)| *w == 0).map(|(_, s)| s.clone());
    }
    let mut parts: Vec<(&str, usize)> = Vec::new();
    let mut chars = 0usize;
    let mut rest = n;
    for (weight, symbol) in tuples {
        let w = i64::from(*weight);
        if w == 0 || rest < w {
            continue;
        }
        let reps = rest / w;
        chars = chars.saturating_add(
            usize::try_from(reps)
                .unwrap_or(usize::MAX)
                .saturating_mul(symbol.chars().count()),
        );
        if chars > MAX_REPRESENTATION_CHARS {
            return None;
        }
        parts.push((symbol, reps as usize));
        rest -= reps * w;
        if rest == 0 {
            return Some(parts.iter().map(|(s, r)| s.repeat(*r)).collect());
        }
    }
    None
}
