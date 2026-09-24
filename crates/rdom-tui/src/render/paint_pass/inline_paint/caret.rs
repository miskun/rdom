//! The caret overlay: a REVERSED cell at the focused editable's
//! collapsed-selection caret, painted once per inline-flow container
//! after that container's fragments so it sits on top of them.
//!
//! Colors come from the cascade (`caret-color` / `caret-text-color`);
//! `Auto` swaps the focused element's cascaded `color` and
//! `background-color`, with a high-contrast fallback when either is
//! the terminal default. The caret's cell position comes from the
//! runtime's caret model (`runtime::editing::caret`), which is the
//! same mapping the editing pipeline uses — paint does not re-derive
//! it.

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::render::inline::inline_flow_container;
use crate::render::{Buffer, Rect, Style};
use crate::style::Modifier;

/// If the focused element is editable and its collapsed-selection
/// caret falls inside the inline-flow container `id`, paint a
/// REVERSED cell at the caret's `(cell_x, cell_y)`. No-op otherwise.
pub(in crate::render::paint_pass) fn paint_caret_if_editable(
    dom: &Dom<TuiExt>,
    buf: &mut Buffer,
    id: NodeId,
    clip: Rect,
) {
    use crate::layout::{CaretColor, CaretTextColor};

    // Must have a collapsed selection (caret), not a range.
    let sel = match dom.selection() {
        Some(s) if s.is_collapsed() => s,
        _ => return,
    };
    let Some(focused) = dom.focused() else {
        return;
    };
    if crate::node::nearest_editable_ancestor(dom, focused).is_none() {
        return;
    };
    let caret_ifc = inline_flow_container(dom, sel.focus.node);
    if caret_ifc != Some(id) {
        return;
    }
    // Resolve caret colors from cascade. `caret-color: transparent`
    // suppresses paint; `Auto` means "derive from the underlying
    // cell's existing fg/bg" (classic swap visual).
    let computed = match dom.node(focused).ext().and_then(|e| e.computed.as_ref()) {
        Some(c) => c,
        None => return,
    };
    if matches!(computed.caret_color, CaretColor::Transparent) {
        return;
    }
    let Some((x, y)) = crate::runtime::editing::caret::cell_of_position(dom, sel.focus) else {
        return;
    };
    if x < clip.x || x >= clip.right() || y < clip.y || y >= clip.bottom() {
        return;
    }

    // `Auto` caret colors derive from the focused element's
    // CASCADED `color` and `background-color` — NOT the cell's
    // currently-painted values. For an empty cell (no glyph painted
    // yet), the painted fg is `Color::Reset`, which would give an
    // invisible caret. Using cascade values gives the predictable
    // "swap text-color and bg-color" visual that authors expect.
    //
    // Fallback: if the cascade resolved to `Color::Reset` (no
    // explicit value, terminal-default), the caret would be
    // invisible (the cell paints as default-on-default). Substitute
    // a sensible high-contrast default — White for the bg-side,
    // Black for the fg-side — so an unstyled textarea/input still
    // shows a visible caret. Authors override via `caret-color` /
    // `caret-text-color`.
    let resolve_reset_fg = |c: crate::Color| match c {
        crate::Color::Reset => crate::Color::Rgb(0xFF, 0xFF, 0xFF),
        other => other,
    };
    let resolve_reset_bg = |c: crate::Color| match c {
        crate::Color::Reset => crate::Color::Rgb(0x00, 0x00, 0x00),
        other => other,
    };
    let cascaded_fg = resolve_reset_fg(computed.fg);
    let cascaded_bg = resolve_reset_bg(computed.bg);
    let under_mod = buf.cell(x, y).map(|c| c.modifier).unwrap_or_default();

    let caret_bg = match &computed.caret_color {
        CaretColor::Auto => cascaded_fg,
        CaretColor::Transparent => return, // already handled above
        CaretColor::Color(tc) => match tc {
            crate::TuiColor::Literal(c) => *c,
            crate::TuiColor::Var { .. } => cascaded_fg,
        },
    };
    let caret_fg = match &computed.caret_text_color {
        CaretTextColor::Auto => cascaded_bg,
        CaretTextColor::Color(tc) => match tc {
            crate::TuiColor::Literal(c) => *c,
            crate::TuiColor::Var { .. } => cascaded_bg,
        },
    };

    let mut new_style = Style::new().fg(caret_fg).bg(caret_bg);
    // Preserve non-color modifiers (bold/italic etc.) that were on
    // the underlying cell so the caret doesn't strip them. `Modifier`
    // is a bitflag — re-add each set bit.
    for m in [
        Modifier::BOLD,
        Modifier::ITALIC,
        Modifier::UNDERLINED,
        Modifier::SLOW_BLINK,
        Modifier::RAPID_BLINK,
        Modifier::HIDDEN,
        Modifier::CROSSED_OUT,
    ] {
        if under_mod.contains(m) {
            new_style = new_style.add_modifier(m);
        }
    }
    buf.set_style(x, y, new_style);
}
