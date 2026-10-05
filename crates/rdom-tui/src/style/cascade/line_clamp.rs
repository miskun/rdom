//! The line-clamp container (CSS Overflow 4 §4) and the legacy
//! `-webkit-box` (Compat Standard §5), decided at the end of an
//! element's or pseudo-element's cascade.

use crate::layout::Display;
use crate::style::ComputedStyle;

/// CSS Overflow 4 §4: whether the box is a line-clamp container — a
/// block container with `max-lines` and `continue: collapse` (or
/// `discard`, rdom's same clamp) — and the legacy form: a `display:
/// -webkit-box` / `-webkit-inline-box` whose `-webkit-box-orient` is
/// vertical, with `max-lines` (from `-webkit-line-clamp` or
/// `line-clamp`), is one whatever its `continue` — the standard
/// `line-clamp: N` after the prefixed declarations, as autoprefixers
/// emit, resets `continue` to `collapse`, and every engine still clamps
/// — and lays out as a block container, as every engine lays the legacy
/// clamp out. A `-webkit-box` without a clamp is a flex container along
/// its `-webkit-box-orient`: a column when vertical, else a row (the
/// legacy flexbox ignores `flex-direction`). Runs before the BFC rule,
/// which reads the `flow` it may change.
pub(super) fn finalize_line_clamp(working: &mut ComputedStyle) {
    use crate::layout::{Continue, Direction, Flow};
    let webkit_box = working.webkit_box && working.flow == Flow::Flex;
    let vertical = working.webkit_box_orient.is_vertical();
    let legacy = webkit_box && vertical && working.max_lines.is_some();
    if legacy {
        match working.display {
            Display::Inline => {
                working.display = Display::InlineBlock;
                working.flow = Flow::Block;
            }
            _ => working.flow = Flow::FlowRoot,
        }
    } else if webkit_box {
        working.direction = if vertical {
            Direction::Column
        } else {
            Direction::Row
        };
        working.flex_reverse = false;
    }
    working.line_clamp_container = working.max_lines.is_some()
        && working.flow.is_block_flow()
        && (legacy || matches!(working.continue_, Continue::Collapse | Continue::Discard));
}
