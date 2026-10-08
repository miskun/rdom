//! UA rules: Select, canvas, tables, gauges, range sliders.

use super::css;
use crate::color::named;
use crate::color::system::{ACCENT, FIELD_BG, TEXT_MUTED};
use crate::layout::{
    Display, Length, Overflow, Padding, Position, Size, TablePart, TextAlign, VerticalAlign,
};
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
        // HTML §15.3.8: the table elements take the CSS table model's
        // `display` values, laid out by rdom-tui's table formatting
        // context (CSS 2.1 §17), and the section's other rules: `table {
        // box-sizing: border-box; text-indent: initial }` (a `width: 100%`
        // table fits its container border and all), `thead, tbody,
        // tfoot, table > tr { vertical-align: middle }` with `tr, td, th`
        // inheriting it, `thead, tbody, tfoot, tr { border-color: inherit
        // }`. Borders are author CSS — none by default, as in HTML. Cells
        // keep rdom's one-cell inline padding (a browser's is 1px);
        // `border-spacing` stays its initial 0 (HTML's 2px is no whole
        // cell, DIVERGENCES §2). HTML's quirks-mode `table` resets do not
        // apply: rdom has no quirks mode.
        (
            "table",
            css(
                css(
                    TuiStyle::new()
                        .display(Display::Block)
                        .flow(crate::layout::Flow::Table),
                    "box-sizing",
                    "border-box",
                ),
                "text-indent",
                "initial",
            ),
        ),
        // HTML §15.3.8: `caption { text-align: center }`, no font or
        // colour of its own.
        (
            "caption",
            TuiStyle::new()
                .display(Display::TablePart(TablePart::Caption))
                .text_align(TextAlign::Center),
        ),
        (
            "colgroup",
            TuiStyle::new().display(Display::TablePart(TablePart::ColumnGroup)),
        ),
        (
            "col",
            TuiStyle::new().display(Display::TablePart(TablePart::Column)),
        ),
        ("thead", row_part(TablePart::HeaderGroup, true)),
        ("tbody", row_part(TablePart::RowGroup, true)),
        ("tfoot", row_part(TablePart::FooterGroup, true)),
        ("tr", row_part(TablePart::Row, false)),
        (
            "table > tr",
            TuiStyle::new().vertical_align(VerticalAlign::Middle),
        ),
        // HTML §15.3.8: cells inherit `vertical-align`, so a cell is
        // centred in a taller row by default.
        ("td", cell(false)),
        ("th", cell(true)),
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

/// The UA style of a row group (`middle` when `group`) or a `<tr>`
/// (inheriting `vertical-align`), each inheriting `border-color` (HTML
/// §15.3.8).
fn row_part(part: TablePart, group: bool) -> TuiStyle {
    let mut s = css(
        TuiStyle::new().display(Display::TablePart(part)),
        "border-color",
        "inherit",
    );
    if group {
        s = s.vertical_align(VerticalAlign::Middle);
    } else {
        s.vertical_align = Some(crate::Value::Inherit);
    }
    s
}

/// The UA style of `<td>` (`<th>` when `header`): a table cell with
/// rdom's one-cell inline padding, inheriting `vertical-align` (HTML
/// §15.3.8), a header bold and centred unless its parent aligns its text
/// (§15.3.8's conditional `th` rule, as `TextAlign::InternalCenter`).
fn cell(header: bool) -> TuiStyle {
    let mut s = TuiStyle::new()
        .display(Display::TablePart(TablePart::Cell))
        .padding(Padding::new(0, 1, 0, 1));
    if header {
        s = s.bold(true).text_align_all(TextAlign::InternalCenter);
    }
    s.vertical_align = Some(crate::Value::Inherit);
    s
}
