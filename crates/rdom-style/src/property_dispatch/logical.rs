//! The flow-relative properties (CSS Logical 1 §2–§6): each one names a
//! physical property, or a start / end pair of them, through the
//! element's writing mode and direction.
//!
//! rdom lays out every box as `horizontal-tb` (DIVERGENCES), so the
//! block axis is always vertical: the block-axis properties and the
//! sizes map onto their physical twins when declared, sharing their
//! storage — a later declaration of either wins, as CSS Logical 1 §4
//! asks. The inline-axis properties and the corner radii depend on
//! `direction` (inline-start is the left edge under `ltr`, the right one
//! under `rtl`), which only the cascade knows: a block keeps them as
//! written, with every declaration after them, on its `pending` list,
//! and the cascade replays that list in order through [`set_mapped`]
//! with the element's direction.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::DispatchError;
use super::table::{Field, fields_of};
use crate::TuiStyle;
use crate::layout::TextDirection;
use crate::parse::token::Token;

/// What a flow-relative property writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mapping {
    /// One physical property, the value as written.
    One(&'static str),
    /// The start and end properties of an axis: one value for both, or
    /// two values in that order (`margin-inline: 1 2`).
    Pair(&'static str, &'static str),
    /// Two physical properties, the whole value to each (`border-block`).
    Both(&'static str, &'static str),
}

/// The physical form of a block-axis or size property (direction does
/// not matter in `horizontal-tb`), `None` for any other name.
fn block_axis(name: &str) -> Option<Mapping> {
    use Mapping::*;
    Some(match name {
        "inline-size" => One("width"),
        // CSS Overflow 3 §3.1: the block / inline axis's `overflow`.
        "overflow-block" => One("overflow-y"),
        "overflow-inline" => One("overflow-x"),
        // CSS Overscroll Behavior 1 §3: the block / inline axis's.
        "overscroll-behavior-block" => One("overscroll-behavior-y"),
        "overscroll-behavior-inline" => One("overscroll-behavior-x"),
        // CSS Scroll Snap 1 §4.1–§4.2.
        "scroll-padding-block-start" => One("scroll-padding-top"),
        "scroll-padding-block-end" => One("scroll-padding-bottom"),
        "scroll-padding-block" => Pair("scroll-padding-top", "scroll-padding-bottom"),
        "scroll-margin-block-start" => One("scroll-margin-top"),
        "scroll-margin-block-end" => One("scroll-margin-bottom"),
        "scroll-margin-block" => Pair("scroll-margin-top", "scroll-margin-bottom"),
        "block-size" => One("height"),
        "min-inline-size" => One("min-width"),
        "min-block-size" => One("min-height"),
        "max-inline-size" => One("max-width"),
        "max-block-size" => One("max-height"),
        "margin-block-start" => One("margin-top"),
        "margin-block-end" => One("margin-bottom"),
        "margin-block" => Pair("margin-top", "margin-bottom"),
        "padding-block-start" => One("padding-top"),
        "padding-block-end" => One("padding-bottom"),
        "padding-block" => Pair("padding-top", "padding-bottom"),
        "inset-block-start" => One("top"),
        "inset-block-end" => One("bottom"),
        "inset-block" => Pair("top", "bottom"),
        "border-block-start" => One("border-top"),
        "border-block-end" => One("border-bottom"),
        "border-block" => Both("border-top", "border-bottom"),
        "border-block-start-color" => One("border-top-color"),
        "border-block-start-style" => One("border-top-style"),
        "border-block-start-width" => One("border-top-width"),
        "border-block-end-color" => One("border-bottom-color"),
        "border-block-end-style" => One("border-bottom-style"),
        "border-block-end-width" => One("border-bottom-width"),
        "border-block-color" => Pair("border-top-color", "border-bottom-color"),
        "border-block-style" => Pair("border-top-style", "border-bottom-style"),
        "border-block-width" => Pair("border-top-width", "border-bottom-width"),
        // Both sides take the whole value: direction does not matter.
        "border-inline" => Both("border-left", "border-right"),
        _ => return None,
    })
}

/// The physical form of an inline-axis property or a corner radius
/// under `direction`, `None` for any other name.
fn inline_axis(name: &str, direction: TextDirection) -> Option<Mapping> {
    use Mapping::*;
    let rtl = direction == TextDirection::Rtl;
    // (inline-start side, inline-end side) of each property family.
    let pick = |ltr: (&'static str, &'static str)| {
        if rtl { (ltr.1, ltr.0) } else { ltr }
    };
    let side = |family: (&'static str, &'static str), end: bool| {
        let (s, e) = pick(family);
        One(if end { e } else { s })
    };
    let pair = |family: (&'static str, &'static str)| {
        let (s, e) = pick(family);
        Pair(s, e)
    };
    const MARGIN: (&str, &str) = ("margin-left", "margin-right");
    const PADDING: (&str, &str) = ("padding-left", "padding-right");
    const INSET: (&str, &str) = ("left", "right");
    const BORDER: (&str, &str) = ("border-left", "border-right");
    const COLOR: (&str, &str) = ("border-left-color", "border-right-color");
    const STYLE: (&str, &str) = ("border-left-style", "border-right-style");
    const WIDTH: (&str, &str) = ("border-left-width", "border-right-width");
    const SCROLL_PADDING: (&str, &str) = ("scroll-padding-left", "scroll-padding-right");
    const SCROLL_MARGIN: (&str, &str) = ("scroll-margin-left", "scroll-margin-right");
    const TOP: (&str, &str) = ("border-top-left-radius", "border-top-right-radius");
    const BOTTOM: (&str, &str) = ("border-bottom-left-radius", "border-bottom-right-radius");
    Some(match name {
        "margin-inline-start" => side(MARGIN, false),
        "margin-inline-end" => side(MARGIN, true),
        "margin-inline" => pair(MARGIN),
        "padding-inline-start" => side(PADDING, false),
        "padding-inline-end" => side(PADDING, true),
        "padding-inline" => pair(PADDING),
        "inset-inline-start" => side(INSET, false),
        "inset-inline-end" => side(INSET, true),
        "inset-inline" => pair(INSET),
        "scroll-padding-inline-start" => side(SCROLL_PADDING, false),
        "scroll-padding-inline-end" => side(SCROLL_PADDING, true),
        "scroll-padding-inline" => pair(SCROLL_PADDING),
        "scroll-margin-inline-start" => side(SCROLL_MARGIN, false),
        "scroll-margin-inline-end" => side(SCROLL_MARGIN, true),
        "scroll-margin-inline" => pair(SCROLL_MARGIN),
        "border-inline-start" => side(BORDER, false),
        "border-inline-end" => side(BORDER, true),
        "border-inline-start-color" => side(COLOR, false),
        "border-inline-start-style" => side(STYLE, false),
        "border-inline-start-width" => side(WIDTH, false),
        "border-inline-end-color" => side(COLOR, true),
        "border-inline-end-style" => side(STYLE, true),
        "border-inline-end-width" => side(WIDTH, true),
        "border-inline-color" => pair(COLOR),
        "border-inline-style" => pair(STYLE),
        "border-inline-width" => pair(WIDTH),
        // CSS Logical 1 §6: `border-<block>-<inline>-radius`, the block
        // side first.
        "border-start-start-radius" => side(TOP, false),
        "border-start-end-radius" => side(TOP, true),
        "border-end-start-radius" => side(BOTTOM, false),
        "border-end-end-radius" => side(BOTTOM, true),
        _ => return None,
    })
}

/// Every flow-relative property name, block axis then inline axis.
pub(super) const NAMES: &[&str] = &[
    "inline-size",
    "block-size",
    "min-inline-size",
    "min-block-size",
    "max-inline-size",
    "max-block-size",
    "overflow-block",
    "overflow-inline",
    "overscroll-behavior-block",
    "overscroll-behavior-inline",
    "scroll-padding-block-start",
    "scroll-padding-block-end",
    "scroll-padding-block",
    "scroll-padding-inline-start",
    "scroll-padding-inline-end",
    "scroll-padding-inline",
    "scroll-margin-block-start",
    "scroll-margin-block-end",
    "scroll-margin-block",
    "scroll-margin-inline-start",
    "scroll-margin-inline-end",
    "scroll-margin-inline",
    "margin-block-start",
    "margin-block-end",
    "margin-block",
    "margin-inline-start",
    "margin-inline-end",
    "margin-inline",
    "padding-block-start",
    "padding-block-end",
    "padding-block",
    "padding-inline-start",
    "padding-inline-end",
    "padding-inline",
    "inset-block-start",
    "inset-block-end",
    "inset-block",
    "inset-inline-start",
    "inset-inline-end",
    "inset-inline",
    "border-block-start",
    "border-block-end",
    "border-block",
    "border-inline-start",
    "border-inline-end",
    "border-inline",
    "border-block-start-color",
    "border-block-start-style",
    "border-block-start-width",
    "border-block-end-color",
    "border-block-end-style",
    "border-block-end-width",
    "border-block-color",
    "border-block-style",
    "border-block-width",
    "border-inline-start-color",
    "border-inline-start-style",
    "border-inline-start-width",
    "border-inline-end-color",
    "border-inline-end-style",
    "border-inline-end-width",
    "border-inline-color",
    "border-inline-style",
    "border-inline-width",
    "border-start-start-radius",
    "border-start-end-radius",
    "border-end-start-radius",
    "border-end-end-radius",
];

/// Whether `name` reads and writes another property's storage — a
/// block-axis or size flow-relative property, its physical twin's (one
/// storage in `horizontal-tb`). A declaration block lists the physical
/// name for it (CSSOM `cssText`, `length`, `item()`), never both.
pub fn is_storage_alias(name: &str) -> bool {
    block_axis(name).is_some()
}

/// Whether `name` maps by `direction` — the cascade must know the
/// element's direction to apply it.
pub(crate) fn is_directional(name: &str) -> bool {
    inline_axis(name, TextDirection::Ltr).is_some()
}

/// The storage fields `name` may write: its physical property's, under
/// either direction for an inline-axis property (`!important` routing,
/// removal and the CSS-wide keywords fold over them). `None` for a name
/// that is not flow-relative.
pub(super) fn fields(name: &str) -> Option<&'static [Field]> {
    static FIELDS: OnceLock<HashMap<&'static str, Vec<Field>>> = OnceLock::new();
    let map = FIELDS.get_or_init(|| {
        NAMES
            .iter()
            .map(|&name| {
                let mut out: Vec<Field> = Vec::new();
                for direction in [TextDirection::Ltr, TextDirection::Rtl] {
                    for physical in targets(mapping(name, direction).expect("a NAMES entry maps")) {
                        for f in fields_of(physical).unwrap_or(&[]) {
                            if !out.contains(f) {
                                out.push(*f);
                            }
                        }
                    }
                }
                (name, out)
            })
            .collect()
    });
    map.get(name).map(Vec::as_slice)
}

fn mapping(name: &str, direction: TextDirection) -> Option<Mapping> {
    block_axis(name).or_else(|| inline_axis(name, direction))
}

fn targets(m: Mapping) -> Vec<&'static str> {
    match m {
        Mapping::One(a) => vec![a],
        Mapping::Pair(a, b) | Mapping::Both(a, b) => vec![a, b],
    }
}

/// Write the flow-relative property `name` onto `style` through its
/// physical property under `direction`. `None` when `name` is not
/// flow-relative. The value is checked for every target before any is
/// written, so an invalid one leaves `style` as it was.
pub(super) fn set_mapped(
    name: &str,
    value: &[Token],
    style: &mut TuiStyle,
    direction: TextDirection,
) -> Option<Result<(), DispatchError>> {
    let writes = match mapping(name, direction)? {
        Mapping::One(a) => vec![(a, value)],
        Mapping::Both(a, b) => vec![(a, value), (b, value)],
        Mapping::Pair(a, b) => match crate::parse::values::components(value).as_deref() {
            Some([one]) => vec![(a, *one), (b, *one)],
            Some([start, end]) => vec![(a, *start), (b, *end)],
            _ => return Some(Err(DispatchError::InvalidValue)),
        },
    };
    for (physical, v) in &writes {
        if let Err(e) = super::set::set_parsed(physical, v, &mut TuiStyle::new()) {
            return Some(Err(e));
        }
    }
    for (physical, v) in writes {
        if let Err(e) = super::set::set_parsed(physical, v, style) {
            return Some(Err(e));
        }
    }
    Some(Ok(()))
}

/// `name`'s value read back through its physical property when it is a
/// block-axis or size property (one storage). `None` when `name` is not
/// one; `Some(None)` when nothing is set. A `Pair` reads one value when
/// the sides agree, two otherwise; `Both` only when they agree.
pub(super) fn serialize_block_axis(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    let read = |n: &str| super::serialize(n, style);
    Some(match block_axis(name)? {
        Mapping::One(a) => read(a),
        Mapping::Both(a, b) => read(a).filter(|v| Some(v) == read(b).as_ref()),
        Mapping::Pair(a, b) => match (read(a), read(b)) {
            (Some(s), Some(e)) if s == e => Some(s),
            (Some(s), Some(e)) => Some(format!("{s} {e}")),
            _ => None,
        },
    })
}

/// The inline-axis longhands `name` sets — itself for a longhand, its
/// components for a shorthand (`margin-inline` is
/// `margin-inline-start` + `margin-inline-end`, `border-inline-start`
/// its width, style and color, CSS Logical 1 §4–§5). Empty for a name
/// that is not inline-axis.
fn inline_longhands(name: &str) -> &'static [&'static str] {
    macro_rules! pair {
        ($p:literal) => {
            &[concat!($p, "-inline-start"), concat!($p, "-inline-end")]
        };
    }
    macro_rules! ring {
        ($side:literal) => {
            &[
                concat!("border-inline-", $side, "-width"),
                concat!("border-inline-", $side, "-style"),
                concat!("border-inline-", $side, "-color"),
            ]
        };
    }
    macro_rules! sides {
        ($part:literal) => {
            &[
                concat!("border-inline-start-", $part),
                concat!("border-inline-end-", $part),
            ]
        };
    }
    match name {
        "margin-inline" => pair!("margin"),
        "padding-inline" => pair!("padding"),
        "inset-inline" => pair!("inset"),
        "border-inline-start" => ring!("start"),
        "border-inline-end" => ring!("end"),
        "border-inline-color" => sides!("color"),
        "border-inline-style" => sides!("style"),
        "border-inline-width" => sides!("width"),
        _ => match NAMES.iter().find(|&&n| n == name) {
            Some(n) if is_directional(n) => std::slice::from_ref(n),
            _ => &[],
        },
    }
}

/// Whether the kept declaration `d` sets the inline-axis longhand `l`.
fn sets_longhand(d: &crate::var::PendingDeclaration, l: &str) -> bool {
    inline_longhands(&d.name).contains(&l) && !d.restriction.drops(l)
}

/// For each inline-axis longhand of `name`, the index in `style.pending`
/// of the last declaration that sets it (CSSOM §6.6: a longhand's value
/// is its last declaration's). `None` when one of them has none.
fn last_declarations(name: &str, style: &TuiStyle) -> Option<Vec<usize>> {
    let longhands = inline_longhands(name);
    if longhands.is_empty() {
        return None;
    }
    longhands
        .iter()
        .map(|l| {
            style
                .pending
                .iter()
                .rposition(|d| d.directional && sets_longhand(d, l))
        })
        .collect()
}

/// `name`'s value when it is an inline-axis property (CSSOM §6.6
/// `getPropertyValue`): each longhand from the last declaration that
/// sets it — its own or a shorthand's component — and a shorthand only
/// when every longhand is set. Read through the physical properties of
/// an `ltr` element, so the text is what those serialize; the
/// direction does not change the logical value. `None` when `name` is
/// not inline-axis or no declaration sets any of its longhands (a
/// CSS-wide keyword from `all` may still hold its fields); `Some(None)`
/// when it is not fully set.
pub(super) fn serialize_inline_axis(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    if !is_directional(name) {
        return None;
    }
    let declared = |l: &&str| {
        style
            .pending
            .iter()
            .any(|d| d.directional && sets_longhand(d, l))
    };
    if !inline_longhands(name).iter().any(declared) {
        return None;
    }
    let Some(mut last) = last_declarations(name, style) else {
        return Some(None);
    };
    // Replaying those declarations in source order leaves each longhand
    // with its last declaration's value.
    last.sort_unstable();
    last.dedup();
    let mut scratch = TuiStyle::new();
    for i in last {
        let d = &style.pending[i];
        if set_mapped(&d.name, &d.value, &mut scratch, TextDirection::Ltr)
            .is_none_or(|r| r.is_err())
        {
            return Some(None);
        }
    }
    let read = |n: &str| super::serialize(n, &scratch);
    Some(match mapping(name, TextDirection::Ltr)? {
        Mapping::One(a) => read(a),
        Mapping::Both(a, b) => read(a).filter(|v| Some(v) == read(b).as_ref()),
        Mapping::Pair(a, b) => match (read(a), read(b)) {
            (Some(s), Some(e)) if s == e => Some(s),
            (Some(s), Some(e)) => Some(format!("{s} {e}")),
            _ => None,
        },
    })
}

/// Whether the inline-axis property `name` is `!important` (CSSOM §6.6
/// `getPropertyPriority`): the last declaration of each of its longhands
/// is, by its own `!important`. `None` when `name` is not inline-axis.
pub(super) fn inline_axis_important(name: &str, style: &TuiStyle) -> Option<bool> {
    if !is_directional(name) {
        return None;
    }
    Some(
        last_declarations(name, style)
            .is_some_and(|last| last.iter().all(|&i| style.pending[i].important)),
    )
}

/// The `!important` bits of the fields a declaration of `name` writes
/// for an element of `direction`: a flow-relative property's physical
/// targets, any other property's own fields. `None` for an unknown name.
pub(crate) fn mapped_mask(name: &str, direction: TextDirection) -> Option<crate::ImportantMask> {
    let name = &*super::table::canonical_property_name(name);
    let Some(m) = mapping(name, direction) else {
        return super::table::property_mask(name);
    };
    Some(
        targets(m)
            .into_iter()
            .flat_map(|t| fields_of(t).unwrap_or(&[]))
            .fold(crate::ImportantMask::empty(), |acc, f| acc | f.mask()),
    )
}

/// CSSOM `removeProperty` of the inline-axis property `name` (CSSOM
/// §6.6): remove the kept declarations of its longhands. A declaration
/// that sets other longhands too (`margin-inline` when removing
/// `margin-inline-start`) is replaced by declarations of those, with its
/// values and priority. The physical properties are separate
/// declarations and stay. Returns whether anything was removed.
pub(super) fn remove_inline_axis(name: &str, style: &mut TuiStyle) -> bool {
    let gone = inline_longhands(name);
    let mut removed = false;
    let mut kept = Vec::with_capacity(style.pending.len());
    for d in std::mem::take(&mut style.pending) {
        let longhands = inline_longhands(&d.name);
        if !d.directional || !longhands.iter().any(|l| gone.contains(l)) {
            kept.push(d);
            continue;
        }
        removed = true;
        // A value waiting for substitution cannot be split before the
        // cascade: the declaration stays, without the removed longhands.
        if d.has_substitution {
            let mut d = d;
            let dropped: Vec<String> = gone.iter().map(|l| l.to_string()).collect();
            match &mut d.restriction {
                crate::var::Restriction::Without(names) => names.extend(dropped),
                other => *other = crate::var::Restriction::Without(dropped),
            }
            kept.push(d);
            continue;
        }
        let mut alone = TuiStyle::new();
        alone.pending.push(d.clone());
        for l in longhands.iter().filter(|l| !gone.contains(l)) {
            let Some(Some(text)) = serialize_inline_axis(l, &alone) else {
                continue;
            };
            let Ok(tokens) = crate::parse::tokenize(&text) else {
                continue;
            };
            let mut rest = crate::var::PendingDeclaration::new(l, &tokens, false);
            rest.important = d.important;
            kept.push(rest);
        }
    }
    style.pending = kept;
    removed
}

/// The physical properties the flow-relative `name` maps to under
/// `direction` (CSS Logical 1 §4); `None` for a name that is not
/// flow-relative. What a `transition-property` naming it animates.
pub(crate) fn physical_names(name: &str, direction: TextDirection) -> Option<Vec<&'static str>> {
    mapping(name, direction).map(targets)
}
