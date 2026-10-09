//! The mask properties (CSS Masking 1 §6–§7): parsed by their grammars and
//! kept as CSS text — a cell has no alpha to mask by, so they draw nothing
//! (DIVERGENCES §1). The layer grammars reuse the background ones (§6.2–
//! §6.6 define them as `background-*`'s).

use super::background::{image_text, join_components, leading_repeat, position_text, size_text};
use super::border::paint_length;
use super::numeric::{Range, components, number, split_commas};
use crate::parse::token::Token;

fn ident(c: &[Token]) -> Option<String> {
    match c {
        [Token::Ident(k)] => Some(k.to_ascii_lowercase()),
        _ => None,
    }
}

/// Each comma-separated layer through `one`, joined back.
fn layers(value: &[Token], one: impl Fn(&[Token]) -> Option<String>) -> Option<String> {
    let texts = split_commas(value)?
        .into_iter()
        .map(one)
        .collect::<Option<Vec<_>>>()?;
    Some(texts.join(", "))
}

/// One of `words`, as written lowercased.
fn one_of(c: &[Token], words: &[&str]) -> Option<String> {
    ident(c).filter(|k| words.contains(&k.as_str()))
}

const MODES: &[&str] = &["alpha", "luminance", "match-source"];
const BOXES: &[&str] = &[
    "content-box",
    "padding-box",
    "border-box",
    "fill-box",
    "stroke-box",
    "view-box",
];
const COMPOSITES: &[&str] = &["add", "subtract", "intersect", "exclude"];

/// The longhand `name`'s value as CSS text, `None` when invalid or not a
/// mask longhand.
pub fn parse_mask_longhand(name: &str, value: &[Token]) -> Option<String> {
    let whole =
        |value: &[Token], check: &dyn Fn(&[&[Token]]) -> Option<String>| check(&components(value)?);
    match name {
        "mask-image" => layers(value, |l| match components(l)?.as_slice() {
            [one] => image_text(one),
            _ => None,
        }),
        "mask-mode" => layers(value, |l| one_of(l, MODES)),
        "mask-repeat" => layers(value, |l| {
            let parts = components(l)?;
            let (_, used) = leading_repeat(&parts)?;
            (used == parts.len()).then(|| join_components(&parts))
        }),
        "mask-position" => layers(value, |l| position_text(&components(l)?)),
        "mask-size" => layers(value, |l| size_text(&components(l)?)),
        "mask-clip" => layers(value, |l| {
            one_of(l, BOXES).or_else(|| one_of(l, &["no-clip"]))
        }),
        "mask-origin" => layers(value, |l| one_of(l, BOXES)),
        "mask-composite" => layers(value, |l| one_of(l, COMPOSITES)),
        "mask-type" => one_of(value, &["luminance", "alpha"]),
        "mask-border-source" => match components(value)?.as_slice() {
            [one] => image_text(one),
            _ => None,
        },
        "mask-border-slice" => whole(value, &slice_text),
        "mask-border-width" => whole(value, &width_text),
        "mask-border-outset" => whole(value, &outset_text),
        "mask-border-repeat" => whole(value, &|parts: &[&[Token]]| {
            let words = ["stretch", "repeat", "round", "space"];
            ((1..=2).contains(&parts.len()) && parts.iter().all(|p| one_of(p, &words).is_some()))
                .then(|| join_components(parts))
        }),
        "mask-border-mode" => one_of(value, &["luminance", "alpha"]),
        _ => None,
    }
}

/// `<number> | <percentage>` (non-negative).
fn number_or_percent(c: &[Token]) -> bool {
    matches!(c, [Token::Percentage(p)] if *p >= 0.0) || number(c, Range::NonNegative).is_some()
}

/// `mask-border-slice`: `[<number> | <percentage>]{1,4} fill?` (`fill` at
/// either end).
fn slice_text(parts: &[&[Token]]) -> Option<String> {
    let fill = |c: &[Token]| ident(c).as_deref() == Some("fill");
    let core: &[&[Token]] = match parts {
        [f, rest @ ..] if fill(f) => rest,
        [rest @ .., f] if fill(f) => rest,
        all => all,
    };
    ((1..=4).contains(&core.len()) && core.iter().all(|c| number_or_percent(c)))
        .then(|| join_components(parts))
}

/// `mask-border-width`: `[<length-percentage> | <number> | auto]{1,4}`.
fn width_text(parts: &[&[Token]]) -> Option<String> {
    let one = |c: &[Token]| {
        ident(c).as_deref() == Some("auto") || paint_length(c, true, Range::NonNegative).is_some()
    };
    ((1..=4).contains(&parts.len()) && parts.iter().all(|c| one(c))).then(|| join_components(parts))
}

/// `mask-border-outset`: `[<length> | <number>]{1,4}`.
fn outset_text(parts: &[&[Token]]) -> Option<String> {
    let one = |c: &[Token]| paint_length(c, false, Range::NonNegative).is_some();
    ((1..=4).contains(&parts.len()) && parts.iter().all(|c| one(c))).then(|| join_components(parts))
}

/// The `mask` shorthand (§6.11): per layer, a `<mask-reference>`, a
/// `<position> [/ <bg-size>]`, a `<repeat-style>`, one or two
/// `<geometry-box>`es (origin, then clip; `no-clip` the clip), a
/// `<compositing-operator>` and a `<masking-mode>`, in any order. The
/// longhands' texts, by name, each layer's value or its initial one.
pub fn parse_mask_shorthand(value: &[Token]) -> Option<Vec<(&'static str, String)>> {
    let mut columns: [Vec<String>; 8] = Default::default();
    for layer in split_commas(value)? {
        let parts = components(layer)?;
        let mut image = None;
        let mut position = None;
        let mut size = None;
        let mut repeat = None;
        let mut boxes: Vec<String> = Vec::new();
        let mut composite = None;
        let mut mode = None;
        let mut i = 0;
        while i < parts.len() {
            let rest = &parts[i..];
            if let Some((_, used)) = leading_repeat(rest).filter(|_| repeat.is_none()) {
                repeat = Some(join_components(&rest[..used]));
                i += used;
            } else if let Some(m) = one_of(rest[0], MODES).filter(|_| mode.is_none()) {
                mode = Some(m);
                i += 1;
            } else if let Some(c) = one_of(rest[0], COMPOSITES).filter(|_| composite.is_none()) {
                composite = Some(c);
                i += 1;
            } else if let Some(b) = one_of(rest[0], BOXES)
                .or_else(|| one_of(rest[0], &["no-clip"]).filter(|_| boxes.len() == 1))
                .filter(|_| boxes.len() < 2)
            {
                boxes.push(b);
                i += 1;
            } else if let Some(t) = image_text(rest[0]).filter(|_| image.is_none()) {
                image = Some(t);
                i += 1;
            } else if position.is_none() {
                // The longest `<position>`, then an optional `/ <size>`.
                let n = (1..=rest.len().min(4))
                    .rev()
                    .find(|&n| position_text(&rest[..n]).is_some())?;
                position = position_text(&rest[..n]);
                i += n;
                if let Some([Token::Delim('/')]) = parts.get(i) {
                    let after = &parts[i + 1..];
                    let m = (1..=after.len().min(2))
                        .rev()
                        .find(|&m| size_text(&after[..m]).is_some())?;
                    size = size_text(&after[..m]);
                    i += 1 + m;
                }
            } else {
                return None;
            }
        }
        let origin = boxes
            .first()
            .cloned()
            .unwrap_or_else(|| "border-box".into());
        let clip = boxes.get(1).cloned().unwrap_or_else(|| origin.clone());
        let values = [
            image.unwrap_or_else(|| "none".into()),
            mode.unwrap_or_else(|| "match-source".into()),
            repeat.unwrap_or_else(|| "repeat".into()),
            position.unwrap_or_else(|| "0% 0%".into()),
            clip,
            origin,
            size.unwrap_or_else(|| "auto".into()),
            composite.unwrap_or_else(|| "add".into()),
        ];
        for (column, v) in columns.iter_mut().zip(values) {
            column.push(v);
        }
    }
    let names = [
        "mask-image",
        "mask-mode",
        "mask-repeat",
        "mask-position",
        "mask-clip",
        "mask-origin",
        "mask-size",
        "mask-composite",
    ];
    Some(
        names
            .into_iter()
            .zip(columns)
            .map(|(n, c)| (n, c.join(", ")))
            .collect(),
    )
}

/// The `mask-border` shorthand (§7.8): `<'mask-border-source'> ||
/// <'mask-border-slice'> [/ <'mask-border-width'>? [/
/// <'mask-border-outset'>]?]? || <'mask-border-repeat'> ||
/// <'mask-border-mode'>`. The longhands' texts, the omitted ones initial.
pub fn parse_mask_border_shorthand(value: &[Token]) -> Option<Vec<(&'static str, String)>> {
    let parts = components(value)?;
    let mut source = None;
    let mut repeat: Vec<&[Token]> = Vec::new();
    let mut mode = None;
    let mut run: Vec<&[Token]> = Vec::new();
    let repeat_words = ["stretch", "repeat", "round", "space"];
    for p in &parts {
        if let Some(m) = one_of(p, &["luminance", "alpha"]).filter(|_| mode.is_none()) {
            mode = Some(m);
        } else if one_of(p, &repeat_words).is_some() && repeat.len() < 2 {
            repeat.push(p);
        } else if let Some(t) = image_text(p).filter(|_| source.is_none()) {
            source = Some(t);
        } else {
            run.push(p);
        }
    }
    let mut slices = run.split(|p| matches!(p, [Token::Delim('/')]));
    let slice = match slices.next() {
        Some([]) | None => "0".to_string(),
        Some(s) => slice_text(s)?,
    };
    let width = match slices.next() {
        None | Some([]) => "auto".to_string(),
        Some(w) => width_text(w)?,
    };
    let outset = match slices.next() {
        None => "0".to_string(),
        Some(o) => outset_text(o)?,
    };
    if slices.next().is_some() {
        return None;
    }
    Some(vec![
        (
            "mask-border-source",
            source.unwrap_or_else(|| "none".into()),
        ),
        ("mask-border-slice", slice),
        ("mask-border-width", width),
        ("mask-border-outset", outset),
        (
            "mask-border-repeat",
            if repeat.is_empty() {
                "stretch".into()
            } else {
                join_components(&repeat)
            },
        ),
        ("mask-border-mode", mode.unwrap_or_else(|| "alpha".into())),
    ])
}
