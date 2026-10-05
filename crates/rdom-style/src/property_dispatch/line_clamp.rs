//! `line-clamp`, `max-lines`, `block-ellipsis`, `continue` (CSS Overflow
//! 4 §4) and the legacy `-webkit-line-clamp` / `-webkit-box-orient`:
//! their `set` and `serialize` arms.

use super::value_serializers::{serialize_css_string, specified};
use crate::layout::{BlockEllipsis, Continue};
use crate::parse::token::Token;
use crate::parse::values::{
    parse_block_ellipsis, parse_box_orient, parse_continue, parse_line_clamp, parse_max_lines,
    parse_webkit_line_clamp,
};
use crate::{TuiStyle, Value};

/// Parse and write one of the names. `None` when `name` is not one;
/// `Some(None)` when its value is invalid.
pub(super) fn set(name: &str, value: &[Token], style: &mut TuiStyle) -> Option<Option<()>> {
    let mut three = |(lines, ellipsis, continue_): (Option<u32>, BlockEllipsis, Continue)| {
        style.max_lines = Some(Value::Specified(lines));
        style.block_ellipsis = Some(Value::Specified(ellipsis));
        style.continue_ = Some(Value::Specified(continue_));
    };
    Some(match name {
        "line-clamp" => parse_line_clamp(value).map(&mut three),
        "-webkit-line-clamp" => parse_webkit_line_clamp(value).map(&mut three),
        "max-lines" => parse_max_lines(value).map(|n| {
            style.max_lines = Some(Value::Specified(n));
        }),
        "block-ellipsis" => parse_block_ellipsis(value).map(|e| {
            style.block_ellipsis = Some(Value::Specified(e));
        }),
        "continue" => parse_continue(value).map(|c| {
            style.continue_ = Some(Value::Specified(c));
        }),
        "-webkit-box-orient" => parse_box_orient(value).map(|o| {
            style.webkit_box_orient = Some(Value::Specified(o));
        }),
        _ => return None,
    })
}

/// Serialize one of the names. `None` when `name` is not one.
pub(super) fn serialize(name: &str, style: &TuiStyle) -> Option<Option<String>> {
    let lines = style.max_lines.as_ref().and_then(specified);
    let ellipsis = style.block_ellipsis.as_ref().and_then(specified);
    let continue_ = style.continue_.as_ref().and_then(specified);
    Some(match name {
        "max-lines" => lines.map(|n| lines_text(*n)),
        "block-ellipsis" => ellipsis.map(ellipsis_text),
        "continue" => continue_.map(|c| c.keyword().to_string()),
        "-webkit-box-orient" => style
            .webkit_box_orient
            .as_ref()
            .and_then(specified)
            .map(|o| o.keyword().to_string()),
        // The shorthands serialize when their longhands are one of the
        // shorthand's forms.
        "line-clamp" => match (lines, ellipsis, continue_) {
            (Some(None), Some(BlockEllipsis::NoEllipsis), Some(Continue::Auto)) => {
                Some("none".into())
            }
            (Some(Some(n)), Some(e), Some(c @ (Continue::Collapse | Continue::WebkitLegacy))) => {
                let mut out = n.to_string();
                if *e != BlockEllipsis::Auto {
                    out = format!("{out} {}", ellipsis_text(e));
                }
                if *c == Continue::WebkitLegacy {
                    out.push_str(" -webkit-legacy");
                }
                Some(out)
            }
            _ => None,
        },
        "-webkit-line-clamp" => match (lines, ellipsis, continue_) {
            (Some(None), Some(BlockEllipsis::Auto), Some(Continue::Auto)) => Some("none".into()),
            (Some(Some(n)), Some(BlockEllipsis::Auto), Some(Continue::WebkitLegacy)) => {
                Some(n.to_string())
            }
            _ => None,
        },
        _ => return None,
    })
}

fn lines_text(n: Option<u32>) -> String {
    n.map_or_else(|| "none".to_string(), |n| n.to_string())
}

fn ellipsis_text(e: &BlockEllipsis) -> String {
    match e {
        BlockEllipsis::NoEllipsis => "no-ellipsis".to_string(),
        BlockEllipsis::Auto => "auto".to_string(),
        BlockEllipsis::Str(s) => serialize_css_string(s),
    }
}
