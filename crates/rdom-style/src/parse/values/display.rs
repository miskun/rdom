//! `display` (CSS Display 3 §2): the box keywords, the legacy single
//! keywords and the multi-keyword syntax, mapped onto rdom's outer
//! [`Display`] / inner [`Flow`] pair and the `list-item` flag (the
//! table is on [`Flow`]); and the shortest serialization back.

use crate::layout::{Display, Flow};
use crate::parse::token::Token;

/// The outer display type (§2.1).
#[derive(Clone, Copy, PartialEq)]
enum Outer {
    Block,
    Inline,
}

/// The inner display types rdom lays out (§2.2).
#[derive(Clone, Copy, PartialEq)]
enum Inner {
    Flow,
    FlowRoot,
    Flex,
}

/// Parse a `display` value: `[ <display-outside> || <display-inside> ]
/// | <display-listitem> | <display-box> | <display-legacy>` (§2),
/// ASCII case-insensitive, as `(outer, inner, list_item)`. An omitted
/// outer type is `block`, an omitted inner type `flow`; `list-item`
/// takes only `flow` / `flow-root`. `run-in`, `grid`, `table` and
/// `ruby` have no layout in rdom yet and are invalid.
pub fn parse_display(value: &[Token]) -> Option<(Display, Flow, bool)> {
    let words = value
        .iter()
        .map(|t| match t {
            Token::Ident(s) => Some(s.to_ascii_lowercase()),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    // §2.5 / §2.7: the box and legacy keywords stand alone.
    if let [word] = words.as_slice() {
        let single = match word.as_str() {
            "none" => Some((Display::None, Flow::Block, false)),
            "contents" => Some((Display::Contents, Flow::Block, false)),
            "inline-block" => Some((Display::InlineBlock, Flow::Block, false)),
            "inline-flex" => Some((Display::Inline, Flow::Flex, false)),
            _ => None,
        };
        if single.is_some() {
            return single;
        }
    }
    let (mut outer, mut inner, mut list_item) = (None, None, false);
    for word in &words {
        match word.as_str() {
            "block" if outer.is_none() => outer = Some(Outer::Block),
            "inline" if outer.is_none() => outer = Some(Outer::Inline),
            "flow" if inner.is_none() => inner = Some(Inner::Flow),
            "flow-root" if inner.is_none() => inner = Some(Inner::FlowRoot),
            "flex" if inner.is_none() => inner = Some(Inner::Flex),
            "list-item" if !list_item => list_item = true,
            _ => return None,
        }
    }
    if words.is_empty() || (list_item && inner == Some(Inner::Flex)) {
        return None;
    }
    let pair = match (outer.unwrap_or(Outer::Block), inner.unwrap_or(Inner::Flow)) {
        (Outer::Block, Inner::Flow) => (Display::Block, Flow::Block),
        (Outer::Block, Inner::FlowRoot) => (Display::Block, Flow::FlowRoot),
        (Outer::Inline, Inner::Flow) => (Display::Inline, Flow::Block),
        // An inline block is an inline-level flow-root box (§2.7).
        (Outer::Inline, Inner::FlowRoot) => (Display::InlineBlock, Flow::Block),
        (Outer::Block, Inner::Flex) => (Display::Block, Flow::Flex),
        (Outer::Inline, Inner::Flex) => (Display::Inline, Flow::Flex),
    };
    Some((pair.0, pair.1, list_item))
}

/// The shortest text of a `display` value (CSSOM §6.7.2): a legacy
/// keyword where one exists, else the keywords that are not the
/// default (`inline list-item`, `flow-root list-item`).
pub fn serialize_display(display: Display, flow: Flow, list_item: bool) -> String {
    let (outer, inner) = match (display, flow) {
        (Display::None, _) => return "none".to_string(),
        (Display::Contents, _) => return "contents".to_string(),
        (Display::InlineBlock, _) => ("inline", "flow-root"),
        (Display::Block, Flow::Block) => ("block", "flow"),
        (Display::Block, Flow::FlowRoot) => ("block", "flow-root"),
        (Display::Block, Flow::Flex) => ("block", "flex"),
        (Display::Inline, Flow::Block) => ("inline", "flow"),
        // `inline flow-root` is the inline block above.
        (Display::Inline, Flow::FlowRoot) => ("inline", "flow-root"),
        (Display::Inline, Flow::Flex) => ("inline", "flex"),
    };
    if list_item {
        let mut parts = Vec::with_capacity(3);
        if outer == "inline" {
            parts.push(outer);
        }
        if inner == "flow-root" {
            parts.push(inner);
        }
        parts.push("list-item");
        return parts.join(" ");
    }
    match (outer, inner) {
        ("block", "flow") => "block",
        ("block", "flow-root") => "flow-root",
        ("block", "flex") => "flex",
        ("inline", "flow") => "inline",
        ("inline", "flow-root") => "inline-block",
        _ => "inline-flex",
    }
    .to_string()
}
