//! The line-clamp container (CSS Overflow 4 §4), decided at the end of
//! an element's or pseudo-element's cascade.

use crate::layout::Display;
use crate::style::ComputedStyle;

/// CSS Overflow 4 §4: whether the box is a line-clamp container — a
/// block container with `max-lines` and `continue: collapse` (or
/// `discard`, rdom's same clamp) — and the legacy form: `continue:
/// -webkit-legacy` takes effect on a `display: -webkit-box` /
/// `-webkit-inline-box` (parsed as `flex` / `inline-flex`, Compat
/// Standard) whose `-webkit-box-orient` is vertical, which then lays out
/// as a block container, as every engine lays it out (a `flex` box with
/// those properties is taken for one, DIVERGENCES). Runs before the BFC
/// rule, which reads the `flow` it may change.
pub(super) fn finalize_line_clamp(working: &mut ComputedStyle) {
    use crate::layout::{Continue, Flow};
    let legacy = working.max_lines.is_some()
        && working.continue_ == Continue::WebkitLegacy
        && working.flow == Flow::Flex
        && working.webkit_box_orient.is_vertical();
    if legacy {
        match working.display {
            Display::Inline => {
                working.display = Display::InlineBlock;
                working.flow = Flow::Block;
            }
            _ => working.flow = Flow::FlowRoot,
        }
    }
    working.line_clamp_container = working.max_lines.is_some()
        && working.flow.is_block_flow()
        && (legacy || matches!(working.continue_, Continue::Collapse | Continue::Discard));
}
