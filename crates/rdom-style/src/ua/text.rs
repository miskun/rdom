//! UA rules: Form state, inline and block typography, sectioning.

use crate::TuiStyle;
use crate::color::named;
use crate::color::system::{ACCENT, BORDER_DEFAULT, FIELD_BG, TEXT_MUTED};
use crate::layout::{
    Border, Display, LineHeight, Padding, Size, TextDecoration, TextDirection, UserSelect,
    VerticalAlign, WhiteSpace,
};

/// The UA rules of this group, in cascade order.
pub(super) fn rules() -> Vec<(&'static str, TuiStyle)> {
    vec![
        // ── Form / interaction state ──
        // Browser convention is "form control looks disabled" — usually
        // muted text + reduced opacity. We pick fg: gray (a real CSS
        // property, naturally inheriting through `<button disabled>`'s
        // text node) over the legacy SGR-2 `dim` modifier which had no
        // CSS analog and rendered however the terminal felt like it.
        //
        // `user-select: none` blocks mouse drag-selection inside any
        // disabled control. Browsers vary on this (Chromium permits
        // text selection inside `<input disabled>`, Firefox blocks
        // it); rdom matches Firefox + the broader HTML "must not be
        // interacted with" intent — and it falls out of the existing
        // user-select gate in hit_test::position_at without any
        // disabled-specific plumbing.
        //
        // `:disabled`, not `[disabled]` (P7-FIELDSET-DISABLED-1): it
        // matches only actually disabled controls (HTML §4.16.3) —
        // controls inside a `<fieldset disabled>` included, a
        // `<div disabled>` excluded.
        (
            ":disabled",
            TuiStyle::new().fg(TEXT_MUTED).user_select(UserSelect::None),
        ),
        // Global `hidden` attribute — HTML treats it as a boolean,
        // so the selector is `[hidden]` (not `[hidden=""]`). Any
        // value, including "false" or "until-found", still hides
        // the element from layout and paint.
        ("[hidden]", TuiStyle::new().display(Display::None)),
        // ── Inline typography ──
        // Emphasis + weight.
        ("b", TuiStyle::new().display(Display::Inline).bold(true)),
        (
            "strong",
            TuiStyle::new().display(Display::Inline).bold(true),
        ),
        ("em", TuiStyle::new().display(Display::Inline).italic(true)),
        ("i", TuiStyle::new().display(Display::Inline).italic(true)),
        (
            "u",
            TuiStyle::new()
                .display(Display::Inline)
                .text_decoration(TextDecoration::Underline),
        ),
        // Semantic inline.
        (
            "cite",
            TuiStyle::new().display(Display::Inline).italic(true),
        ),
        ("dfn", TuiStyle::new().display(Display::Inline).italic(true)),
        ("var", TuiStyle::new().display(Display::Inline).italic(true)),
        (
            "address",
            TuiStyle::new().display(Display::Inline).italic(true),
        ),
        // Code / samp / kbd.
        (
            "code",
            TuiStyle::new()
                .display(Display::Inline)
                .fg(named::GOLD)
                .bg(FIELD_BG),
        ),
        (
            "samp",
            TuiStyle::new()
                .display(Display::Inline)
                .fg(named::GOLD)
                .bg(FIELD_BG),
        ),
        (
            "kbd",
            TuiStyle::new()
                .display(Display::Inline)
                .fg(ACCENT)
                .bold(true),
        ),
        // Edits — `<del>` / `<s>` get proper strikethrough via
        // `<del>` / `<s>` render with `text-decoration: line-through`
        // (SGR-9). `<ins>` gets underline (same treatment as `<u>`).
        (
            "del",
            TuiStyle::new()
                .display(Display::Inline)
                .text_decoration(TextDecoration::LineThrough),
        ),
        (
            "s",
            TuiStyle::new()
                .display(Display::Inline)
                .text_decoration(TextDecoration::LineThrough),
        ),
        (
            "ins",
            TuiStyle::new()
                .display(Display::Inline)
                .text_decoration(TextDecoration::Underline),
        ),
        // Highlight.
        (
            "mark",
            TuiStyle::new()
                .display(Display::Inline)
                .bg(named::YELLOW)
                .fg(named::BLACK),
        ),
        // Abbreviation — muted fg (rdom's) so it reads as secondary
        // text; HTML §15.3.4's `abbr[title] { text-decoration: dotted
        // underline }` marks one with an expansion (SGR `4:4`, a solid
        // underline on a terminal without styled underlines). Browsers
        // show the title on hover; rdom has no tooltip.
        (
            "abbr",
            TuiStyle::new().display(Display::Inline).fg(TEXT_MUTED),
        ),
        (
            "abbr[title]",
            TuiStyle::new()
                .text_decoration_line(crate::layout::TextDecorationLine::UNDERLINE)
                .text_decoration_style(crate::layout::TextDecorationStyle::Dotted),
        ),
        // Small text — browser renders at reduced font size; the TUI
        // doesn't shrink, so we substitute muted fg.
        (
            "small",
            TuiStyle::new().display(Display::Inline).fg(TEXT_MUTED),
        ),
        // HTML §15.3.4: `sub { vertical-align: sub } sup { vertical-align:
        // super } sub, sup { line-height: normal }` — a row down or up.
        (
            "sub",
            TuiStyle::new()
                .display(Display::Inline)
                .vertical_align(VerticalAlign::Sub)
                .line_height(LineHeight::Normal),
        ),
        (
            "sup",
            TuiStyle::new()
                .display(Display::Inline)
                .vertical_align(VerticalAlign::Super)
                .line_height(LineHeight::Normal),
        ),
        // Pure-inline (no specific style, just the Display hint).
        ("q", TuiStyle::new().display(Display::Inline)),
        // HTML §15.3.6: `q::before { content: open-quote }
        // q::after { content: close-quote }` — the marks come from
        // `quotes` (CSS Generated Content 3 §2.2).
        (
            "q::before",
            TuiStyle::new().content(crate::Content::Quote(crate::QuoteKind::Open)),
        ),
        (
            "q::after",
            TuiStyle::new().content(crate::Content::Quote(crate::QuoteKind::Close)),
        ),
        ("output", TuiStyle::new().display(Display::Inline)),
        ("time", TuiStyle::new().display(Display::Inline)),
        ("data", TuiStyle::new().display(Display::Inline)),
        ("bdi", TuiStyle::new().display(Display::Inline)),
        ("bdo", TuiStyle::new().display(Display::Inline)),
        ("wbr", TuiStyle::new().display(Display::Inline)),
        ("span", TuiStyle::new().display(Display::Inline)),
        ("br", TuiStyle::new().display(Display::Inline)),
        // `<a>` is always inline; link styling only kicks in when
        // the anchor actually has `href`. Matches browser behavior
        // where a bare `<a>` is a named anchor / placeholder, not
        // a hyperlink. `a[href]:hover` emboldens for clickable
        // feedback.
        ("a", TuiStyle::new().display(Display::Inline)),
        (
            "a[href]",
            TuiStyle::new()
                .fg(ACCENT)
                .text_decoration(TextDecoration::Underline),
        ),
        ("a[href]:hover", TuiStyle::new().bold(true)),
        // ── Block typography ──
        // Paragraph + headings.
        ("p", TuiStyle::new().display(Display::Block)),
        // `<h1>` is bold (not bold + underline as legacy browsers
        // render). At TUI density an underline rule merges with
        // the text on narrow viewports and competes with the
        // underline UA on `<a href>` below — modern TUI headers
        // (helix, lazygit, gh) use bold-accent alone.
        ("h1", TuiStyle::new().display(Display::Block).bold(true)),
        ("h2", TuiStyle::new().display(Display::Block).bold(true)),
        ("h3", TuiStyle::new().display(Display::Block).bold(true)),
        ("h4", TuiStyle::new().display(Display::Block).bold(true)),
        ("h5", TuiStyle::new().display(Display::Block).bold(true)),
        ("h6", TuiStyle::new().display(Display::Block).bold(true)),
        ("hgroup", TuiStyle::new().display(Display::Block)),
        // Pre + blockquote.
        (
            "pre",
            TuiStyle::new()
                .display(Display::Block)
                .white_space(WhiteSpace::Pre)
                .bg(FIELD_BG),
        ),
        (
            "blockquote",
            TuiStyle::new()
                .display(Display::Block)
                .padding(Padding::new(0, 0, 0, 1))
                .border(Border::left())
                .border_fg(BORDER_DEFAULT)
                .fg(TEXT_MUTED),
        ),
        // Thematic break — `─` rule across the available width
        // via a `Border::top()` on an empty block. `Border::top()`
        // paints the box-drawing `─` on every cell of the top
        // edge; the content box is empty (`height: 0`, as the HTML
        // rendering section's `hr` has no height), so the border is
        // the box's one row under either `box-sizing`. Dim fg so it
        // recedes.
        (
            "hr",
            TuiStyle::new()
                .display(Display::Block)
                .height(Size::Fixed(0))
                .border(Border::top())
                .border_fg(BORDER_DEFAULT),
        ),
        // The HTML rendering section's "Bidirectional text" rules (§15.3.5)
        // map an element's directionality (HTML §3.2.6.4: its `dir`
        // attribute, `auto`'s first strong character, inheritance) onto
        // `direction` through `:dir()`.
        (
            "[dir]:dir(ltr), bdi:dir(ltr), input[type=tel i]:dir(ltr)",
            TuiStyle::new().text_direction(TextDirection::Ltr),
        ),
        (
            "[dir]:dir(rtl), bdi:dir(rtl)",
            TuiStyle::new().text_direction(TextDirection::Rtl),
        ),
        // Figures.
        ("figure", TuiStyle::new().display(Display::Block)),
        (
            "figcaption",
            TuiStyle::new()
                .display(Display::Block)
                .italic(true)
                .fg(TEXT_MUTED),
        ),
        // ── Block structural (sectioning) ──
        // These are pure semantic containers — default display
        // already Block, so listing them is redundant for layout
        // but makes the UA sheet a complete reference for authors
        // inspecting "what does the engine think of this tag?"
        ("article", TuiStyle::new().display(Display::Block)),
        ("section", TuiStyle::new().display(Display::Block)),
        ("aside", TuiStyle::new().display(Display::Block)),
        ("nav", TuiStyle::new().display(Display::Block)),
        ("header", TuiStyle::new().display(Display::Block)),
        ("footer", TuiStyle::new().display(Display::Block)),
        ("main", TuiStyle::new().display(Display::Block)),
        ("search", TuiStyle::new().display(Display::Block)),
    ]
}
