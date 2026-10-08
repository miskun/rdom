//! UA rules: Select, canvas, tables, gauges, range sliders.

use super::css;
use crate::color::named;
use crate::color::system::{ACCENT, FIELD_BG, TEXT_MUTED};
use crate::layout::{Direction, Display, Length, Overflow, Padding, Position, Size};
use crate::{Color, Content, TuiStyle};

/// The UA rules of this group, in cascade order.
pub(super) fn rules() -> Vec<(&'static str, TuiStyle)> {
    vec![
        // ── Select widget ──
        // `<select>` is a block container: its options stack vertically.
        // `<option>` is a Block row displaying its text label.
        // `<optgroup>` renders its label as a bold separator.
        // Selection highlight is bg LightBlue / fg Black; the
        // navigation-focused option gets a subtler SecondaryBg.
        // `<select>` defaults to single-select dropdown:
        // 1-cell chrome row, overflow:hidden so the option
        // list is clipped until the dropdown opens. Listbox
        // selects (`multiple` or `size`) and explicitly-
        // opened dropdowns (`data-rdom-open`) override to
        // Auto height + Visible overflow so every option
        // is visible.
        (
            "select",
            TuiStyle::new()
                .display(Display::Block)
                .position(Position::Relative)
                .width(Size::Fixed(20))
                .height(Size::Fixed(1))
                .overflow(Overflow::Hidden)
                .padding(Padding::new(0, 1, 0, 1))
                .bg(FIELD_BG)
                .fg(named::WHITE),
        ),
        (
            "select[multiple]",
            TuiStyle::new()
                .height(Size::Auto)
                .overflow(Overflow::Visible),
        ),
        (
            // HTML §4.10.7: a display size of 1 is still the drop-down
            // box; only `size > 1` is the list box. The runtime's
            // `is_dropdown` parses the number; the sheet approximates it
            // by excluding the spellings of "1" (`size="abc"` renders as
            // a list box — a documented edge case).
            "select[size]:not([size=\"1\"]):not([size=\"0\"]):not([size=\"\"])",
            TuiStyle::new()
                .height(Size::Auto)
                .overflow(Overflow::Visible),
        ),
        // An open drop-down keeps its one row in flow; its option list
        // overflows below it, rendered in the top layer (the runtime puts
        // the select there as a picker, HTML's `::picker(select)`), so it
        // overlays the page — on the field background, which `:where()`
        // keeps below the selected / highlighted options' own.
        (
            "select[data-rdom-open]",
            TuiStyle::new().overflow(Overflow::Visible),
        ),
        (
            ":where(select[data-rdom-open]) > option",
            TuiStyle::new().bg(FIELD_BG),
        ),
        (
            "option",
            TuiStyle::new()
                .display(Display::Block)
                .height(Size::Fixed(1))
                .padding(Padding::new(0, 1, 0, 1)),
        ),
        // Options inside a closed single-select dropdown are
        // invisible — the chrome row is the only visible cell.
        // Listbox selects (`multiple` / `size`) and open
        // dropdowns (`data-rdom-open`) win via specificity and
        // restore the default `display: Block`.
        (
            "select:not([multiple]):not([size]):not([data-rdom-open]) > option",
            TuiStyle::new().display(Display::None),
        ),
        // Dropdown affordance — `▾` chevron pinned to the right
        // edge of closed single-select dropdowns. Absolute
        // positioning so the chevron sits at a fixed offset
        // regardless of selected option's length, with
        // `right: Cells(1)` placing it inside the 1-cell right
        // padding. Listbox modes (`[multiple]`, `[size]`) and
        // open dropdowns (`[data-rdom-open]`) get no chevron —
        // they show the full option list, so the
        // "click to open" affordance is moot.
        (
            "select:not([multiple]):not([size]):not([data-rdom-open])::after",
            TuiStyle::new()
                .content(Content::Str("▾".into()))
                .position(Position::Absolute)
                .right(Length::Cells(1))
                .top(Length::Cells(0))
                .fg(ACCENT),
        ),
        (
            "option[selected]",
            TuiStyle::new().bg(ACCENT).fg(named::BLACK),
        ),
        (
            // Option-highlight bg is inlined rather than borrowing
            // `BORDER_DEFAULT` — "highlighted option" is a selection
            // token, not a border token, even when the value happens
            // to coincide today. Independent tokens shouldn't share
            // a constant.
            "option[data-rdom-highlight]:not([selected])",
            TuiStyle::new().bg(Color::Rgb(0x3B, 0x40, 0x42)),
        ),
        ("option[disabled]", TuiStyle::new().fg(TEXT_MUTED)),
        // `<optgroup>` — semantic grouping container. The
        // `label` attribute renders as a bold separator line
        // via a `::before` pseudo-element that uses
        // `content: attr(label)`. Authors restyle with a more-
        // specific rule on `optgroup::before`.
        (
            "optgroup",
            TuiStyle::new()
                .display(Display::Block)
                .padding(Padding::new(0, 1, 0, 1)),
        ),
        (
            "optgroup::before",
            css(
                TuiStyle::new().bold(true).fg(TEXT_MUTED),
                "content",
                "attr(label)",
            ),
        ),
        // ── Canvas ──
        // `<canvas>` is a raw-buffer escape hatch. When a
        // paint callback is registered via
        // `runtime::builtins::canvas::set_paint`, the paint
        // pass calls it with a bounded `RenderContext`. No
        // callback → children paint normally (HTML fallback).
        // Default 40×10 matches the HTML default (300×150 at
        // 7.5x5 font scale); apps override with author CSS.
        (
            "canvas",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(40))
                .height(Size::Fixed(10)),
        ),
        // No `canvas:focus` reset hack is needed anymore: the focus tint is
        // scoped to atomic controls, so a `<canvas>` (a replaced/content
        // element the app paints) never gets a background fill on focus —
        // matching the web, where focusing a canvas never touches its pixels.
        // ── Tables ──
        // `<table>` is the layout primitive for tabular data.
        // Structure: optional `<caption>`, optional `<thead>` /
        // `<tbody>` / `<tfoot>` row groups (or bare `<tr>`s),
        // and `<td>` / `<th>` cells inside rows.
        //
        // V1 uses plain block and flex layout: the table and row
        // groups are block containers, their rows stacking
        // vertically; each `<tr>` is a row flex container whose children
        // are the cells. Column widths sync across rows via a
        // pre-pass that computes max content width per column
        // index and writes Fixed widths to each cell.
        //
        // Borders are author CSS — no default borders (matches
        // HTML5 default). Authors style with `td { border: 1 }`
        // etc. No colspan/rowspan in v1.
        ("table", TuiStyle::new().display(Display::Block)),
        (
            "caption",
            TuiStyle::new()
                .display(Display::Block)
                .italic(true)
                .fg(TEXT_MUTED),
        ),
        ("thead", TuiStyle::new().display(Display::Block)),
        ("tbody", TuiStyle::new().display(Display::Block)),
        ("tfoot", TuiStyle::new().display(Display::Block)),
        (
            "tr",
            // `<tr>` lays its `<td>`/`<th>` cells out horizontally.
            // Pre-BFC-1 this worked implicitly because every container
            // ran flex; post-BFC-1 the UA must explicitly opt the row
            // into flex flow (CSS3 Display Module: `display: flex` =
            // outer `block` + inner `flex`).
            TuiStyle::new()
                .display(Display::Block)
                .flow(crate::layout::Flow::Flex)
                .direction(Direction::Row)
                .height(Size::Fixed(1)),
        ),
        (
            "td",
            TuiStyle::new()
                .display(Display::Block)
                .padding(Padding::new(0, 1, 0, 1)),
        ),
        (
            "th",
            TuiStyle::new()
                .display(Display::Block)
                .padding(Padding::new(0, 1, 0, 1))
                .bold(true),
        ),
        // `<colgroup>` and `<col>` carry column metadata. Not
        // rendered — hidden via `display: none` so apps that
        // target them via CSS for other reasons still have the
        // element available in the tree.
        ("colgroup", TuiStyle::new().display(Display::None)),
        ("col", TuiStyle::new().display(Display::None)),
        // ── Gauge widgets ──
        // `<progress>` and `<meter>` paint a horizontal block-
        // character bar. Display:Block + fixed width so the
        // bar has a known cell budget. `<progress>` defaults
        // to LightBlue (accent palette); `<meter>` keeps
        // LightGreen as its semantic "optimum" default and the
        // paint layer overrides fg for suboptimal / out-of-range
        // zones at runtime.
        (
            "progress",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(20))
                .height(Size::Fixed(1))
                .fg(ACCENT),
        ),
        (
            "meter",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(20))
                .height(Size::Fixed(1))
                .fg(named::LIMEGREEN),
        ),
        // ── Range slider (native HTML `<input type="range">`) ──
        // Same shape as `<progress>` / `<meter>`: 20×1 block-
        // sized container; the runtime's range builtin attaches
        // a canvas paint callback that draws the track + thumb
        // glyphs within this rect. Accent color is LightBlue
        // (matching `<progress>` and the rest of the accent
        // family).
        (
            "input[type=range]",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(20))
                .height(Size::Fixed(1))
                .fg(ACCENT),
        ),
    ]
}
