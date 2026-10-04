//! UA rules: Lists, scrollbars, selection, document metadata.

use crate::color::named;
use crate::color::system::HIGHLIGHT;
use crate::counters::{CounterOp, CounterStyle};
use crate::layout::{Display, Padding};
use crate::{Color, Content, TuiStyle};

/// The UA rules of this group, in cascade order.
pub(super) fn rules() -> Vec<(&'static str, TuiStyle)> {
    vec![
        // ── Lists ──
        // `ul` / `ol` / `menu` use left padding so nested list
        // markers (added by authors via `li::before`) have room.
        // Description lists: `dd` is indented from `dt`.
        (
            "ul",
            TuiStyle::new()
                .counter_reset(vec![CounterOp {
                    name: "list-item".into(),
                    value: 0,
                }])
                .display(Display::Block)
                .padding(Padding::new(0, 0, 0, 2)),
        ),
        (
            "ol",
            TuiStyle::new()
                .counter_reset(vec![CounterOp {
                    name: "list-item".into(),
                    value: 0,
                }])
                .display(Display::Block)
                .padding(Padding::new(0, 0, 0, 2)),
        ),
        (
            "menu",
            TuiStyle::new()
                .counter_reset(vec![CounterOp {
                    name: "list-item".into(),
                    value: 0,
                }])
                .display(Display::Block)
                .padding(Padding::new(0, 0, 0, 2)),
        ),
        (
            "li",
            TuiStyle::new()
                .display(Display::Block)
                .counter_increment(vec![CounterOp {
                    name: "list-item".into(),
                    value: 1,
                }]),
        ),
        // List item markers — bullet glyph + space before the
        // `<li>` content. Child-combinator scoping: only direct
        // `<li>` children get the marker, matching CSS
        // `list-style-type: disc`. Nested lists pick up the same
        // marker from their own parent.
        //
        // `<ol>` counts: the UA resets the `list-item` counter on every
        // list container (`ul`, `ol`, `menu` — HTML §15.3.8), increments
        // it on every `<li>`, and renders it in the `<ol>` marker (CSS
        // Lists 3 §3, via explicit UA rules because rdom has no
        // `display: list-item`). Resetting on `<ul>` too is what keeps a
        // nested bullet list from advancing the enclosing numbering.
        (
            "ul > li::before",
            TuiStyle::new().content(Content::Str("• ".into())),
        ),
        (
            "ol > li::before",
            TuiStyle::new().content(Content::Concat(vec![
                Content::Counter {
                    name: "list-item".into(),
                    style: CounterStyle::Decimal,
                },
                Content::Str(". ".into()),
            ])),
        ),
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
        // Scrollbar fg is inlined rather than borrowing a shared
        // constant — track and thumb are independent design tokens
        // even when their values relate to other UA colors. A future
        // tweak to one must not silently move the other.
        (
            "*::scrollbar",
            TuiStyle::new().fg(Color::Rgb(0x2D, 0x2F, 0x31)),
        ),
        (
            "*::scrollbar-thumb",
            TuiStyle::new().fg(Color::Rgb(0x41, 0x43, 0x45)),
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
