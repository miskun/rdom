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
