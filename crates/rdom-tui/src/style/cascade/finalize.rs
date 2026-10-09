//! The computed values the cascade derives once an element's
//! declarations are applied: whether it establishes a block formatting
//! context, `justify-items: legacy`'s inheritance, and `display:
//! contents` on an element whose children are not its rendering.

use crate::layout::Display;
use crate::style::ComputedStyle;

/// Compute `establishes_new_bfc` from the working style + parent
/// context. Runs after the cascade ladder so all source properties
/// are at their final values. Per CSS 2.1 §9.4.1 + Flexbox §3:
///
/// An element establishes a new block formatting context when:
/// - It's a flex or grid container (`flow: Flex` / `Grid`) — they form
///   independent formatting contexts for their items (CSS Flexbox §3,
///   CSS Grid 2 §5.1).
/// - It's an inline-block — establishes a new BFC for its content
///   (which then lays out as block).
/// - It is a scroll container (`overflow` `hidden` / `scroll` / `auto`
///   on an axis). `overflow: clip` is not one and forms no BFC (CSS
///   Overflow 3 §3.1).
/// - It's absolutely or fixed positioned — out-of-flow boxes form
///   their own BFCs.
/// - (Root element is also a BFC — handled implicitly because
///   layout starts at root regardless.)
///
/// Margin collapsing checks this predicate: parent-child margin
/// collapse happens only when the parent does NOT establish a new
/// BFC.
pub(super) fn finalize_bfc_formation(working: &mut ComputedStyle) {
    use crate::layout::{Flow, Position};
    working.establishes_new_bfc = matches!(working.flow, Flow::Flex | Flow::Grid | Flow::FlowRoot | Flow::Table)
        || matches!(working.display, Display::InlineBlock)
        || working.is_scroll_container()
        || matches!(working.position, Position::Absolute | Position::Fixed)
        // CSS 2.1 §9.4.1: floats establish a new block formatting context.
        || working.float != crate::layout::Float::None
        // CSS Box Alignment 3 §5.1: a block container whose
        // `align-content` is not `normal` is an independent formatting
        // context.
        || (working.flow.is_block_flow()
            && working.align_content.keyword != crate::layout::Align::Normal)
        // CSS Containment 2 §3.2, §3.4: layout and paint containment make
        // an independent formatting context.
        || crate::style::containment::layout(working)
        || crate::style::containment::paint(working)
        // CSS Multi-column 1 §2: a multi-column container establishes a new
        // block formatting context.
        || working.is_multicol_container();
}

/// CSS Box Alignment 3 §6.2: `justify-items: legacy` (its initial value)
/// computes to the parent's value when that is `legacy` with a side
/// (`legacy center`, …), and to `normal` otherwise — so a `legacy` value
/// reaches the descendants that do not set `justify-items`.
pub(super) fn finalize_justify_items(working: &mut ComputedStyle, parent: &ComputedStyle) {
    use crate::layout::{Align, Alignment};
    if working.justify_items == Alignment::LEGACY {
        working.justify_items =
            if parent.justify_items.legacy && parent.justify_items.keyword != Align::Normal {
                parent.justify_items
            } else {
                Alignment::NORMAL
            };
    }
}

/// CSS Display 3 Appendix B: `display: contents` on a replaced element
/// or a form control — whose children are not its rendering — behaves
/// as `display: none`. rdom computes it so, so layout, paint, hit
/// testing and focus all see no box.
pub(super) fn finalize_unusual_contents(working: &mut ComputedStyle, tag: Option<&str>) {
    const NO_CONTENTS: &[&str] = &[
        "br", "wbr", "meter", "progress", "canvas", "embed", "object", "audio", "iframe", "img",
        "video", "frame", "frameset", "input", "textarea", "select",
    ];
    if working.display == Display::Contents && tag.is_some_and(|t| NO_CONTENTS.contains(&t)) {
        working.display = Display::None;
    }
}
