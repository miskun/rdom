//! Where each line's content starts: at the inline-start edge — the
//! left one, or the right one under `direction: rtl` (CSS Writing
//! Modes 4 §2.1). rdom has no `text-align` yet (C9-TEXT-ALIGN), so every
//! line is aligned as its initial value `start` (CSS Text 3 §7.1) says.
//!
//! The content keeps its logical order left to right: rdom does not
//! reorder bidirectional text (DIVERGENCES), which for left-to-right
//! text in an `rtl` paragraph is what a browser shows too.

use rdom_core::{Dom, NodeId};

use super::LineBox;
use crate::ext::TuiExt;
use crate::layout::TextDirection;

/// Move each of `lines` to start at `container`'s inline-start edge of
/// a `content_width`-wide line: under `rtl`, flush right.
pub(super) fn start_lines_at_inline_start(
    dom: &Dom<TuiExt>,
    container: NodeId,
    lines: &mut [LineBox],
    content_width: u16,
) {
    let rtl = dom
        .node(container)
        .ext()
        .and_then(|e| e.computed.as_ref())
        .is_some_and(|c| c.text_direction == TextDirection::Rtl);
    if !rtl {
        return;
    }
    for line in lines {
        let shift = content_width.saturating_sub(line.width);
        if shift == 0 {
            continue;
        }
        for f in &mut line.fragments {
            f.x = f.x.saturating_add(shift);
        }
        for g in &mut line.generated {
            g.x = g.x.saturating_add(shift);
        }
    }
}
