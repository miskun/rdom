//! User-agent stylesheet — the default rules baked into every
//! `Stylesheet::new()`.
//!
//! Authors override any rule by writing their own at equal-or-higher
//! specificity — the standard browser model. The UA exposes no
//! theming API and no `--rdom-*` custom properties; consumers
//! re-theme by writing override rules in their own stylesheet.
//!
//! This is the single source of truth for the UA. The
//! `ua_total_rule_count` test below pins the count — any accidental
//! rule addition or deletion breaks the test and requires a
//! deliberate update.
//!
//! ## Organization
//!
//! Rules are grouped by element family for grep-ability:
//!
//! 1. Form / interaction state (`[disabled]`, `[hidden]`)
//! 2. Inline typography (emphasis, weight, semantic inline, code, edits, highlight, abbreviation, etc.)
//! 3. Links (`<a>` / `<a href>` / `:hover`)
//! 4. Block typography (paragraphs, headings, pre, blockquote, hr, figures)
//! 5. Block structural / sectioning
//! 6. Block interactive (`<details>`, `<summary>`, `<dialog>`, `<form>`, `<fieldset>`, `<legend>`)
//! 7. Form fields (`<input>`, `<textarea>`)
//! 8. Toggle widgets (`<input type=checkbox/radio>` + state glyphs)
//! 9. Select widget (`<select>`, `<option>`, `<optgroup>`)
//! 10. Canvas
//! 11. Tables
//! 12. Gauge widgets (`<progress>`, `<meter>`, `<input type=range>`)
//! 13. Lists
//! 14. Document metadata (`<style>`)

use crate::color::named;
use crate::layout::{
    Border, Direction, Display, Length, Overflow, Padding, Position, Size, TextDecoration,
    UserSelect, WhiteSpace,
};
use crate::{Color, Content, TuiStyle};

/// Field background tint — subtle dark warm gray. On dark
/// terminals it reads as a soft pillow under input/textarea text
/// (just visible enough to mark the field affordance). On light
/// terminals it's a dark rectangle.
const FIELD_BG: Color = Color::Rgb(0x1f, 0x21, 0x23);
/// Muted text — #7F868B (lens-k8s-tui `TextMuted`). Cool gray that
/// reads as supporting prose against both light and dark surfaces.
/// Used for `[disabled]`, placeholder, `<small>`, `<abbr>`,
/// blockquote text, scrollbar glyphs, helper text, etc.
const TEXT_MUTED: Color = Color::Rgb(0x7F, 0x86, 0x8B);
/// Default border — #3B4042 (lens-k8s-tui `BorderDefault`). Subtle
/// gray for box-drawing borders and `<hr>` rules — distinct from
/// `TEXT_MUTED` so a border next to muted text still reads as
/// chrome rather than as more text.
const BORDER_DEFAULT: Color = Color::Rgb(0x3B, 0x40, 0x42);
/// Accent — dodgerblue (#1E90FF). Vivid on both light and dark
/// terminals. Replaces every former `ACCENT` use:
/// link fg, kbd, button fg, dialog border, focus glyph, select
/// chevron, range/progress bar accent, selected-option bg.
const ACCENT: Color = named::DODGERBLUE;

/// Return the slice of `(selector, style)` pairs that `Stylesheet::new()`
/// installs as UA defaults. Single source of truth for the user-agent
/// stylesheet.
///
/// Consumed by `Stylesheet::new()` in `stylesheet.rs`; not exposed in
/// `lib.rs` because it's a build detail of the public `Stylesheet::new()`
/// API rather than a separate user-facing entry point.
pub(crate) fn user_agent_defaults() -> Vec<(&'static str, TuiStyle)> {
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
        (
            "[disabled]",
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
        // Abbreviation — browser default is dotted underline + tooltip
        // on hover. TUI: solid underline (closest fidelity) + muted fg
        // so the abbreviation reads as secondary text.
        (
            "abbr",
            TuiStyle::new()
                .display(Display::Inline)
                .text_decoration(TextDecoration::Underline)
                .fg(TEXT_MUTED),
        ),
        // Small text — browser renders at reduced font size; the TUI
        // doesn't shrink, so we substitute muted fg.
        (
            "small",
            TuiStyle::new().display(Display::Inline).fg(TEXT_MUTED),
        ),
        // Pure-inline (no specific style, just the Display hint).
        ("sub", TuiStyle::new().display(Display::Inline)),
        ("sup", TuiStyle::new().display(Display::Inline)),
        ("q", TuiStyle::new().display(Display::Inline)),
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
        // via a `Border::top()` on a 1-row block. `Border::top()`
        // paints the box-drawing `─` on every cell of the top
        // edge, which on a 1-tall block is the only row → the
        // hr's single row IS the rule. Dim fg so it recedes.
        (
            "hr",
            TuiStyle::new()
                .display(Display::Block)
                .height(Size::Fixed(1))
                .border(Border::top())
                .border_fg(BORDER_DEFAULT),
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
        // ── Block interactive ──
        ("details", TuiStyle::new().display(Display::Block)),
        (
            "summary",
            TuiStyle::new().display(Display::Block).bold(true),
        ),
        // Disclosure triangle — `▸` (collapsed, U+25B8) / `▾`
        // (open, U+25BE). The base rule lands the right-pointing
        // small triangle; the more specific `details:open >
        // summary::before` overrides to the down-pointing small
        // triangle when the parent `<details>` is expanded.
        //
        // Small triangles (U+25B8 / U+25BE) sit at x-height and
        // read as punctuation alongside the summary text — modern
        // TUI convention (helix file tree, lazygit stash, gh
        // `<details>` rendering). The full-cell variants
        // (U+25B6 / U+25BC) read as media-player controls and
        // crowd the line.
        (
            "summary::before",
            TuiStyle::new().content(Content::Str("▸ ".into())),
        ),
        (
            "details:open > summary::before",
            TuiStyle::new().content(Content::Str("▾ ".into())),
        ),
        // Closed-details body suppression — when `<details>` lacks
        // the `open` attribute, only the `<summary>` child renders.
        // Every other direct child collapses out of layout per the
        // HTML spec's disclosure widget semantics. The runtime
        // (`runtime::builtins::details`) toggles `open` on click /
        // Enter / Space; this cascade rule is what makes the
        // body actually vanish when closed.
        (
            "details:not([open]) > *:not(summary)",
            TuiStyle::new().display(Display::None),
        ),
        // `<dialog>`: block when open, hidden via `display: none`
        // when the `open` attribute is absent. Author rules with
        // greater specificity than `dialog:not([open])` override
        // the hide.
        //
        // Open dialogs get structural chrome — rounded
        // `╭ ╮ ╰ ╯` border in `LightBlue` for a modal-feeling
        // frame, plus `padding: 1 2` for content breathing room.
        // Rounded corners are the modern-TUI convention (gum,
        // lipgloss, ratatui defaults, lazygit popups, helix
        // popups); square corners read 1990s `dialog(1)`.
        //
        // No bg fill on purpose — forcing `bg: Black` would create
        // a black hole on light terminal themes (Solarized Light,
        // Apple Terminal Basic). Dialogs separate from the
        // underlying document via their accent-colored border;
        // modal dialogs additionally get a `::backdrop` overlay
        // that recedes the background, so the combination already
        // makes the dialog pop without forcing any specific bg.
        //
        // Width / height stay `Auto` so the dialog shrinks to its
        // content; authors who want a fixed centered dialog set
        // their own size + position.
        (
            "dialog",
            TuiStyle::new()
                .display(Display::Block)
                .border(Border::rounded())
                .border_fg(ACCENT)
                .padding(Padding::new(1, 2, 1, 2)),
        ),
        ("dialog:not([open])", TuiStyle::new().display(Display::None)),
        ("form", TuiStyle::new().display(Display::Block)),
        (
            "fieldset",
            TuiStyle::new()
                .display(Display::Block)
                .padding(Padding::all(1)),
        ),
        ("legend", TuiStyle::new().display(Display::Block).bold(true)),
        // ── Form fields ──
        // `<input>` is a single-line text-family editor. White-
        // space `Pre` keeps spaces verbatim; overflow-x `Hidden`
        // clips long text past the right edge (no scrollbar).
        // 20-cell width matches HTML's default `size` attribute;
        // authors override with author CSS or `size`-based
        // sizing in a future polish pass.
        //
        // The bare `input` rule sets only structural defaults so
        // it applies cleanly to every input type. Text-family
        // chrome (bg tint, padding) lives in the more-specific
        // `:not(...)` rule below; button-family chrome lives in
        // the button rules further down.
        (
            "input",
            TuiStyle::new()
                .display(Display::Block)
                .white_space(WhiteSpace::Pre)
                .overflow_x(Overflow::Hidden)
                .width(Size::Fixed(20))
                .height(Size::Fixed(1)),
        ),
        // Text-family input chrome — bg tint as the field
        // affordance + padding for caret breathing room. The
        // `:not(...)` chain excludes types with their own UA
        // chrome (button family, checkbox/radio, range, hidden).
        // Unknown types (color, date, etc.) fall through to this
        // rule so they render with the same text affordance,
        // matching browsers' "unknown type → text" behavior.
        (
            "input:not([type=button]):not([type=submit]):not([type=reset]):not([type=checkbox]):not([type=radio]):not([type=range]):not([type=hidden])",
            TuiStyle::new()
                .padding(Padding::new(0, 1, 0, 1))
                .bg(FIELD_BG),
        ),
        // `<textarea>` is a multi-line editor. White-space `Pre`
        // preserves newlines as hard breaks; long lines overflow
        // horizontally with `Auto` (scrollbar appears when the
        // content exceeds the box). `rows`/`cols` attributes map
        // to a fixed cell box; default height bumped from HTML's
        // 2 → 4 because 2-row textareas are barely usable in a TUI.
        (
            "textarea",
            TuiStyle::new()
                .display(Display::Block)
                .white_space(WhiteSpace::PreWrap)
                .overflow_y(Overflow::Auto)
                .width(Size::Fixed(20))
                .height(Size::Fixed(4))
                .padding(Padding::new(0, 1, 0, 1))
                .bg(FIELD_BG),
        ),
        // ── Buttons ──
        // `<button>` and `<input type=button|submit|reset>` use the
        // bracketed-glyph TUI button idiom: `[ Label ]` via
        // `::before` / `::after`, accent-fg + bold, no bg fill.
        // `display: inline-block` sizes the button to its intrinsic
        // content on both axes (not full-row-stretch like a plain
        // Block child of a column parent would).
        //
        // Accent-on-document-bg (no bg fill) matches the modern
        // TUI button idiom — gh, lazygit, helix all render buttons
        // as accent-fg-bold without a filled rectangle (filled
        // rectangles are reserved for `selected` row-highlight
        // states). The earlier `bg: DarkGray` fill collided with
        // `<code>` / form-field tints and broke on light terminal
        // themes; dropping it lets the user's terminal bg show
        // through cleanly.
        //
        // `padding: 0 1` gives the bracketed label 1 cell of
        // breathing room on each side without adding a visible
        // outline.
        //
        // Focus is signaled via the unified two-signal indicator
        // (`reversed: true` + leading `▸ ` glyph) further down in
        // this rule list, not via base-style changes.
        //
        // Intrinsic-width sizing of these pseudo-elements is
        // handled by `render/layout_pass/intrinsic.rs::
        // pseudo_content_width`, which reads `computed_before()` /
        // `computed_after()` content alongside DOM children when
        // measuring along `Direction::Row`.
        (
            "button",
            TuiStyle::new()
                .display(Display::InlineBlock)
                .fg(ACCENT)
                .bold(true),
        ),
        (
            "input[type=button], input[type=submit], input[type=reset]",
            TuiStyle::new()
                .display(Display::InlineBlock)
                .fg(ACCENT)
                .bold(true),
        ),
        (
            "button::before, input[type=button]::before, input[type=submit]::before, input[type=reset]::before",
            TuiStyle::new().content(Content::Str("[ ".into())),
        ),
        (
            "button::after, input[type=button]::after, input[type=submit]::after, input[type=reset]::after",
            TuiStyle::new().content(Content::Str(" ]".into())),
        ),
        // ── Focus indicator ──
        // Single rule: subtle background tint. No content shift, no
        // reverse video. Marked `!important` so the indicator can't
        // be silently defeated by higher-specificity UA rules like
        // the `input:not(...):not(...)` text-field chain (specificity
        // 0,7,1). Authors who want to override the focus indicator
        // can still do so with their own `!important` rule, or with
        // a higher-origin selector.
        (
            ":focus",
            TuiStyle::new().bg_important(Color::Rgb(0x2d, 0x2f, 0x31)),
        ),
        // Placeholder rendering via `:placeholder-shown` +
        // `attr()` content. When the input / textarea has a
        // non-empty `placeholder` attribute and is empty, the
        // `::before` pseudo-element injects the placeholder text
        // at DarkGray+dim. Authors override with more-specific
        // rules.
        (
            "input:placeholder-shown::before",
            TuiStyle::new()
                .content(Content::Attr("placeholder".into()))
                .fg(TEXT_MUTED),
        ),
        (
            "textarea:placeholder-shown::before",
            TuiStyle::new()
                .content(Content::Attr("placeholder".into()))
                .fg(TEXT_MUTED),
        ),
        // ── Toggle widgets ──
        // `<input type="checkbox">` and `<input type="radio">`
        // render their state via UA `::before` content. The
        // `:checked` pseudo-class flips the glyph between
        // unchecked and checked variants. Authors override by
        // writing a more specific rule (e.g.,
        // `[type=checkbox]::before { content: "☐" }`).
        //
        // Display `Block` so the `::before` paint path
        // (`paint_inline_content`, the non-IFC path) handles
        // the glyph. Width auto-grows from glyph content;
        // height is one cell. Authors who want widgets inline
        // with text can override with `display: inline` plus
        // their own glyph painting strategy.
        (
            "input[type=checkbox]",
            TuiStyle::new()
                .display(Display::Block)
                .height(Size::Fixed(1)),
        ),
        (
            "input[type=radio]",
            TuiStyle::new()
                .display(Display::Block)
                .height(Size::Fixed(1)),
        ),
        (
            "input[type=checkbox]::before",
            TuiStyle::new().content(Content::Str("[ ] ".into())),
        ),
        (
            "input[type=checkbox]:checked::before",
            TuiStyle::new().content(Content::Str("[x] ".into())),
        ),
        (
            "input[type=radio]::before",
            TuiStyle::new().content(Content::Str("( ) ".into())),
        ),
        (
            "input[type=radio]:checked::before",
            TuiStyle::new().content(Content::Str("(\u{2022}) ".into())),
        ),
        // ── Select widget ──
        // `<select>` is a Block flex column (default direction).
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
            "select[size]",
            TuiStyle::new()
                .height(Size::Auto)
                .overflow(Overflow::Visible),
        ),
        (
            "select[data-rdom-open]",
            TuiStyle::new()
                .height(Size::Auto)
                .overflow(Overflow::Visible),
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
            TuiStyle::new()
                .content(Content::Attr("label".into()))
                .bold(true)
                .fg(TEXT_MUTED),
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
        // ── Tables ──
        // `<table>` is the layout primitive for tabular data.
        // Structure: optional `<caption>`, optional `<thead>` /
        // `<tbody>` / `<tfoot>` row groups (or bare `<tr>`s),
        // and `<td>` / `<th>` cells inside rows.
        //
        // V1 uses plain flex layout: the table and row groups
        // flow vertically (default Column direction); each
        // `<tr>` is a horizontal flex container whose children
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
        // ── Lists ──
        // `ul` / `ol` / `menu` use left padding so nested list
        // markers (added by authors via `li::before`) have room.
        // Description lists: `dd` is indented from `dt`.
        (
            "ul",
            TuiStyle::new()
                .display(Display::Block)
                .padding(Padding::new(0, 0, 0, 2)),
        ),
        (
            "ol",
            TuiStyle::new()
                .display(Display::Block)
                .padding(Padding::new(0, 0, 0, 2)),
        ),
        (
            "menu",
            TuiStyle::new()
                .display(Display::Block)
                .padding(Padding::new(0, 0, 0, 2)),
        ),
        ("li", TuiStyle::new().display(Display::Block)),
        // List item markers — bullet glyph + space before the
        // `<li>` content. Child-combinator scoping: only direct
        // `<li>` children get the marker, matching CSS
        // `list-style-type: disc`. Nested lists pick up the same
        // marker from their own parent.
        //
        // `<ol>` gets the same `• ` marker as `<ul>` in 0.1.0.
        // Real browsers render incrementing counters (1., 2., …)
        // there, but rdom 0.1.0 doesn't ship CSS counters and a
        // static `"1. "` glyph repeated on every item would *lie*
        // about ordering. A bullet on `<ol>` is a documented
        // honest fallback: visually identifies the rows as a
        // list without claiming positions. Tracked in
        // `TECH_DEBT.md` as `UA-OL-1` — upgrade to real counters
        // when CSS counters land.
        //
        // Authors who want numbered output either inline numbers
        // in `<li>` text directly or override with their own
        // `ol > li::before` rule.
        (
            "ul > li::before",
            TuiStyle::new().content(Content::Str("• ".into())),
        ),
        (
            "ol > li::before",
            TuiStyle::new().content(Content::Str("• ".into())),
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
        // Scrollbar fg is inlined (#7F868B) rather than borrowing a
        // shared constant — even though the value coincides with
        // `TEXT_MUTED` today, "scrollbar glyph color" and "muted
        // text" are independent design tokens. A future tweak to one
        // must not silently move the other.
        (
            "*::scrollbar",
            TuiStyle::new().fg(Color::Rgb(0x7F, 0x86, 0x8B)),
        ),
        (
            "*::scrollbar-thumb",
            TuiStyle::new().fg(Color::Rgb(0x7F, 0x86, 0x8B)),
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
            TuiStyle::new()
                .bg(Color::Rgb(0x39, 0x4B, 0x7E))
                .fg(Color::Rgb(0xFF, 0xFF, 0xFF)),
        ),
        // ── Document metadata ──
        // `<style>` carries CSS source as text content; it
        // must not render. Matches HTML's display:none for
        // the tag. Placed last so existing UA-rule tests that
        // index by source order keep their positions.
        ("style", TuiStyle::new().display(Display::None)),
    ]
}

#[cfg(test)]
mod tests {
    use super::TEXT_MUTED;
    use crate::color::named;
    use crate::stylesheet::{Rule, RuleOrigin, Stylesheet};
    use crate::{TuiColor, Value};

    /// Pin the UA rule count. Any accidental addition or deletion
    /// breaks this test and requires a deliberate update.
    #[test]
    fn ua_total_rule_count() {
        let s = Stylesheet::new();
        let ua: Vec<_> = s
            .rules()
            .iter()
            .filter(|r| r.origin == RuleOrigin::UserAgent)
            .collect();
        // Tripwire — any accidental rule addition or deletion breaks
        // this test and requires a deliberate update. Comma-list
        // selectors expand to one Rule per selector at insertion, so
        // the count can exceed the number of tuples in `ua_defaults`.
        assert_eq!(ua.len(), 128);
        let disabled = ua
            .iter()
            .find(|r| r.source_text == "[disabled]")
            .expect("[disabled] rule must exist");
        assert_eq!(
            disabled.style.fg,
            Some(Value::Specified(TuiColor::Literal(TEXT_MUTED))),
        );
    }

    /// Spot-check a handful of the Tier 1 UA rules to catch
    /// accidental regressions (e.g. if a future refactor drops
    /// entries from the defaults vec).
    #[test]
    fn ua_rules_cover_tier_1_semantics() {
        use crate::layout::Display;

        let s = Stylesheet::new();
        let ua: std::collections::HashMap<String, &Rule> = s
            .rules()
            .iter()
            .filter(|r| r.origin == RuleOrigin::UserAgent)
            .map(|r| (r.source_text.clone(), r))
            .collect();

        // A few coverage checks across each category.
        for tag in [
            "section",
            "article",
            "aside",
            "header",
            "footer",
            "main",
            "nav",
            "blockquote",
            "figure",
            "figcaption",
            "hr",
            "ul",
            "ol",
            "li",
            "dl",
            "dt",
            "dd",
            "details",
            "summary",
            "dialog",
            "form",
            "abbr",
            "cite",
            "mark",
            "kbd",
            "samp",
            "var",
            "dfn",
            "del",
            "ins",
            "sub",
            "sup",
            "small",
            "s",
            "q",
            "h4",
            "h5",
            "h6",
        ] {
            assert!(ua.contains_key(tag), "missing UA rule for <{tag}>");
        }

        // mark: yellow bg + black fg.
        let mark = ua["mark"];
        assert_eq!(
            mark.style.bg,
            Some(Value::Specified(TuiColor::Literal(named::YELLOW)))
        );
        assert_eq!(
            mark.style.fg,
            Some(Value::Specified(TuiColor::Literal(named::BLACK)))
        );

        // Headings h1-h6 all bold + block.
        for h in ["h1", "h2", "h3", "h4", "h5", "h6"] {
            let r = ua[h];
            assert_eq!(r.style.bold, Some(Value::Specified(true)));
            assert_eq!(r.style.display, Some(Value::Specified(Display::Block)));
        }

        // Inline + italic for semantic emphasis tags.
        for t in ["em", "i", "cite", "dfn", "var"] {
            let r = ua[t];
            assert_eq!(
                r.style.display,
                Some(Value::Specified(Display::Inline)),
                "<{t}> must be inline"
            );
            assert_eq!(
                r.style.italic,
                Some(Value::Specified(true)),
                "<{t}> must be italic"
            );
        }

        // Muted text: small / abbr render with the UA muted-text
        // color (`TEXT_MUTED` — lens `TextMuted`, #7F868B).
        for t in ["small", "abbr"] {
            let r = ua[t];
            assert_eq!(
                r.style.fg,
                Some(Value::Specified(TuiColor::Literal(TEXT_MUTED))),
                "<{t}> must use TEXT_MUTED fg"
            );
        }
        // <del> and <s> render with line-through, not dim.
        for t in ["del", "s"] {
            let r = ua[t];
            assert_eq!(
                r.style.text_decoration,
                Some(Value::Specified(crate::layout::TextDecoration::LineThrough)),
                "<{t}> must use text-decoration: line-through"
            );
        }
    }
}
