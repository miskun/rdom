//! UA rules: Lists, scrollbars, selection, document metadata.

use crate::TuiStyle;
use crate::color::named;
use crate::color::system::HIGHLIGHT;
use crate::counters::{CounterOp, CounterStyle};
use crate::layout::{Display, ListStyleType, Padding};

/// The UA rules of this group, in cascade order.
pub(super) fn rules() -> Vec<(&'static str, TuiStyle)> {
    vec![
        // ── Lists ──
        // HTML §15.3.8: `ul` / `ol` / `menu` have `padding-inline-start:
        // 40px`, the room an outside marker hangs in (CSS Lists 3 §3.5).
        // 40px is 2.5em at the 16px default font — about five digits of
        // it — so a browser's list at the page's edge shows "10. " and a
        // short roman numeral whole; four cells hold "10. " and "iv. ",
        // and a wider marker overflows the list's box into what is beside
        // it, as a browser's does (C10G-MARKER-CLIP).
        // Description lists: `dd` is indented from `dt`.
        ("ul", list(CounterStyle::named("disc"), "4")),
        ("ol", list(CounterStyle::named("decimal"), "4")),
        ("menu", list(CounterStyle::named("disc"), "4")),
        // HTML §15.3.8: `li { display: list-item }` — which increments
        // the `list-item` counter implicitly (CSS Lists 3 §4.6) — and
        // `ol[reversed] { counter-reset: reversed(list-item) }`. `start`
        // and `value` are presentational hints (`cascade::hints`).
        ("li", super::css(TuiStyle::new(), "display", "list-item")),
        // HTML §15.3.8: a bullet list inside a list shows `circle`, one
        // more level down `square`.
        (
            ":is(ul, ol, menu) ul, :is(ul, ol, menu) menu",
            TuiStyle::new().list_style_type(ListStyleType::Style(CounterStyle::named("circle"))),
        ),
        (
            ":is(ul, ol, menu) :is(ul, ol, menu) ul, :is(ul, ol, menu) :is(ul, ol, menu) menu",
            TuiStyle::new().list_style_type(ListStyleType::Style(CounterStyle::named("square"))),
        ),
        // CSS Lists 3 §3.2's UA sheet: `::marker { white-space: pre }`
        // (its `text-transform: none` is applied by the backend: no
        // author rule can set it on a marker).
        (
            "::marker",
            super::css(TuiStyle::new(), "white-space", "pre"),
        ),
        (
            "ol[reversed]",
            TuiStyle::new().counter_reset(vec![CounterOp::reversed("list-item", None)]),
        ),
        // The markers are each `li`'s `::marker`, from the list's
        // `list-style-type` (CSS Lists 3 §3); resetting `list-item` on
        // `ul` too keeps a nested bullet list from advancing the
        // enclosing numbering.
        ("dl", TuiStyle::new().display(Display::Block)),
        ("dt", TuiStyle::new().display(Display::Block).bold(true)),
        (
            "dd",
            TuiStyle::new()
                .display(Display::Block)
                .padding(Padding::new(0, 0, 0, 2)),
        ),
        // ── Scrollbars ──
        // `::scrollbar` and `::scrollbar-thumb` paint inside the
        // 1-cell gutter that `reserve_scrollbar_gutter` carved out.
        // The UA ships a light/heavy two-glyph look: track renders
        // `│` (U+2502 LIGHT VERTICAL) / `─` (U+2500 LIGHT HORIZONTAL),
        // thumb renders `┃` (U+2503 HEAVY VERTICAL) / `━` (U+2501
        // HEAVY HORIZONTAL), both in the same muted fg. No bg fill,
        // so underlying content shows through the gutter — closer to
        // the convention used by modern TUIs (helix, lazygit, gum).
        // Glyph weight, not color, distinguishes track from thumb.
        //
        // Both `content` properties are deliberately UNSET; paint
        // picks the axis-appropriate fallback glyph
        // (`paint_pass::scrollbar::FALLBACK_TRACK_V/H` and
        // `FALLBACK_THUMB_V/H`). Authors who set `content` get the
        // literal glyph on both axes — picking a glyph that reads
        // both ways (full block, half block, shaded blocks) is the
        // documented path until `::scrollbar:vertical` /
        // `:horizontal` pseudo-class targeting lands (tracked as
        // `UA-SB-1` in `TECH_DEBT.md`).
        //
        // Authors retheme by overriding either rule at any
        // specificity:
        //
        //   *::scrollbar       { bg: Black; fg: White }
        //   *::scrollbar-thumb { content: "█"; fg: LightBlue }
        //
        // The `*` universal host is required because the parser
        // rejects bare `::scrollbar` (host-required, same rule
        // as `::before` / `::after` / `::backdrop` / `::selection`).
        // Track and thumb are independent design tokens: the
        // platform's colors (`NATIVE_SCROLLBAR_*`), which a bar styled
        // by the standard properties under `scrollbar-color: auto` also
        // paints in (CSS Scrollbars 1 §2).
        (
            "*::scrollbar",
            TuiStyle::new().fg(crate::layout::NATIVE_SCROLLBAR_TRACK),
        ),
        (
            "*::scrollbar-thumb",
            TuiStyle::new().fg(crate::layout::NATIVE_SCROLLBAR_THUMB),
        ),
        // ── Selection ──
        // Distinct bg color for selected text so a 1-cell selection
        // is visually different from the caret cell next to it.
        // Without this rule, selection overlay paints nothing
        // visible, and Shift+arrow extending by one cell produces
        // no visible change. Authors override with their own
        // `::selection` rule at any specificity.
        (
            "*::selection",
            TuiStyle::new().bg(HIGHLIGHT).fg(named::WHITE),
        ),
        // ── Document metadata ──
        // `<style>` carries CSS source as text content; it
        // must not render. Matches HTML's display:none for
        // the tag.
        ("style", TuiStyle::new().display(Display::None)),
    ]
}

/// A list container's UA style (HTML §15.3.8): block, resetting the
/// `list-item` counter, its items' `list-style-type`, and
/// `padding-inline-start` (`cells`).
fn list(kind: CounterStyle, cells: &str) -> TuiStyle {
    super::css(
        TuiStyle::new()
            .counter_reset(vec![CounterOp::new("list-item", 0)])
            .list_style_type(ListStyleType::Style(kind))
            .display(Display::Block),
        "padding-inline-start",
        cells,
    )
}
