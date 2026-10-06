//! UA rules: Form fields, buttons, the focus indicator, toggle widgets.

use super::css;
use crate::color::system::{ACCENT, FIELD_BG, TEXT_MUTED};
use crate::layout::{
    BoxSizing, Display, FontStyle, FontWeight, LineHeight, Overflow, Padding, Size, Spacing,
    TextAlign, TextIndent, TextTransform, UserSelect, WhiteSpace,
};
use crate::{Color, Content, TuiStyle};

/// The UA rules of this group, in cascade order.
pub(super) fn rules() -> Vec<(&'static str, TuiStyle)> {
    vec![
        // ── Form controls: the inherited text properties reset ──
        // Chromium's `input, textarea, select, button { … line-height:
        // normal; text-transform: none; text-indent: 0; text-align:
        // start; letter-spacing: normal; word-spacing: normal; … }` and
        // its `font: -webkit-small-control` (Gecko's
        // `input { line-height: normal }`, HTML §15.5): a control lays its
        // own text out, so a page's `line-height: 1.5`, `text-indent`,
        // `uppercase` or `text-align: center` does not reach into it. The
        // font reset keeps the weight and style (the button rules below
        // set their own); the font's size and family are inert in a
        // terminal.
        (
            "input, textarea, select, button",
            TuiStyle::new()
                .line_height(LineHeight::Normal)
                .text_transform(TextTransform::NONE)
                .text_indent(TextIndent::default())
                .text_align(TextAlign::Start)
                .font_weight(FontWeight::Normal)
                .font_style(FontStyle::Normal)
                .letter_spacing(Spacing::Normal)
                .word_spacing(Spacing::Normal),
        ),
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
            "input:not([type=button]):not([type=submit]):not([type=image]):not([type=reset]):not([type=checkbox]):not([type=radio]):not([type=range]):not([type=hidden])",
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
        //
        // `user-select: none`: a button's label is a click affordance,
        // not prose, so drag-selecting it would be a distraction.
        // Browsers get the same effect mostly from form-control
        // internals rather than a declared UA rule (DIVERGENCES
        // §Selection & editing).
        // The `:disabled` rule already covers disabled buttons via
        // its own `user-select: none`; these rules cover the
        // enabled case.
        (
            "button",
            TuiStyle::new()
                .display(Display::InlineBlock)
                .fg(ACCENT)
                .bold(true)
                .user_select(UserSelect::None),
        ),
        // `width: auto` undoes the text-field `input { width: 20 }`
        // above, as for the toggles: the box hugs `[ label ]`.
        (
            "input[type=button], input[type=submit], input[type=image], input[type=reset]",
            TuiStyle::new()
                .display(Display::InlineBlock)
                .width(Size::Auto)
                .fg(ACCENT)
                .bold(true)
                .user_select(UserSelect::None),
        ),
        // `box-sizing: border-box` for the controls the HTML rendering
        // section's UA sheet lists (§15.5, "Form controls":
        // `input:is([type=radio], [type=checkbox], [type=reset],
        // [type=button], [type=submit], [type=color], [type=search]),
        // select, button`), plus `meter` and `progress`, which Chromium's
        // `html.css` sizes the same way. A text `<input>` and
        // `<textarea>` keep `content-box`: their width is the content
        // box, the padding sits outside it.
        (
            "input[type=radio], input[type=checkbox], input[type=reset], input[type=button], input[type=submit], input[type=color], input[type=search], select, button, meter, progress",
            TuiStyle::new().box_sizing(BoxSizing::BorderBox),
        ),
        (
            "button::before",
            TuiStyle::new().content(Content::Str("[ ".into())),
        ),
        // The button-family `<input>` label (HTML §4.10.5.1.19–21):
        // the `value` attribute, or — when it is absent — "Submit" /
        // "Reset" for those two types and nothing for `type=button`.
        // A `<button>` labels itself with its children; an `<input>`
        // has none, so its label is generated content alongside the
        // opening bracket, where layout, paint and hit-testing already
        // see it (`P6G-INPUT-BUTTON-LABEL-1`). An author `::before`
        // rule on these inputs replaces bracket and label together
        // (DIVERGENCES §Selection & editing).
        (
            "input[type=button]::before, input[type=submit]::before, input[type=reset]::before",
            css(TuiStyle::new(), "content", "\"[ \" attr(value)"),
        ),
        (
            "input[type=submit]:not([value])::before",
            TuiStyle::new().content(Content::Str("[ Submit".into())),
        ),
        (
            "input[type=reset]:not([value])::before",
            TuiStyle::new().content(Content::Str("[ Reset".into())),
        ),
        // An image button (HTML §4.10.5.1.20) has no image to show in a
        // terminal; browsers show a missing image's `alt` text, so it is
        // a button labelled with `alt` — "Submit" without one, the
        // UA-defined label HTML allows (`P7G-INPUT-IMAGE-1`).
        (
            "input[type=image]::before",
            css(TuiStyle::new(), "content", "\"[ \" attr(alt)"),
        ),
        (
            "input[type=image]:not([alt])::before",
            TuiStyle::new().content(Content::Str("[ Submit".into())),
        ),
        (
            "button::after, input[type=button]::after, input[type=submit]::after, input[type=image]::after, input[type=reset]::after",
            TuiStyle::new().content(Content::Str(" ]".into())),
        ),
        // ── Focus indicator: background tint, scoped to atomic controls ──
        // The web shows focus with an OUTLINE on every focusable element; a
        // TUI can't draw a no-reflow ring, so rdom substitutes a background
        // tint. A fill only reads as a focus affordance on small atomic
        // controls (the box IS the control). On a container — table, div,
        // scroll region, canvas — it floods the interior, nothing like the
        // web's outline, so the tint is *scoped to the control set*.
        // Containers express focus another way: scroll containers via the
        // `:focus::scrollbar-thumb` accent (below), grid/tree via their
        // internal cursor, otherwise the consumer's own CSS. This replaces the
        // old generic `:focus` tint + its per-element opt-out hacks
        // (`canvas:focus`, `[role=tree]:focus`). Non-important so authors
        // override freely. See DIVERGENCES.md "Focus affordances".
        // Keyed on `:focus-visible` (Selectors 4 §13.2), as browsers key
        // their focus ring: a button, toggle or select focused by a mouse
        // click shows no tint; keyboard focus and text fields do
        // (`runtime::focus::visible` in rdom-tui decides).
        (
            "button:focus-visible, summary:focus-visible, a:focus-visible, area:focus-visible",
            TuiStyle::new().bg(Color::Rgb(0x2d, 0x2f, 0x31)),
        ),
        // The text-field background chain
        // (`input:not([type=button])…`, specificity 0,7,1) would
        // otherwise hide the focus tint on text inputs. `!important`
        // scoped to just these controls — small widgets where a forced
        // tint is the right affordance, not app-painted surfaces.
        (
            "input:focus-visible, textarea:focus-visible, select:focus-visible",
            TuiStyle::new().bg_important(Color::Rgb(0x2d, 0x2f, 0x31)),
        ),
        // Focus indicator for SCROLL CONTAINERS: the scroll region that
        // CONTAINS the focus (`:focus-within` — the focused element itself
        // when it's a focusable scroll container like a tree, or the scroll
        // pane around a focused `<input>`) shows an accent thumb. This is the
        // same region the keyboard scroll keys act on, so the blue handle
        // marks "what your keyboard scrolls". The runtime keeps
        // `data-rdom-scroll-focus` on exactly that container — the nearest
        // *overflowing* scroll ancestor of the focus, which no selector can
        // express (`:focus-within` would light every overflowing ancestor).
        // The thumb GLYPH (`┃`/`━`) turns accent (DodgerBlue) via foreground
        // — a thin colored handle, not a filled block; unfocused → the paint
        // pass's gray fallback. The container analog of the web's focus
        // outline, via `::scrollbar-thumb`. See DIVERGENCES.md.
        (
            "[data-rdom-scroll-focus]::scrollbar-thumb",
            TuiStyle::new().fg(ACCENT),
        ),
        // Placeholder rendering via `:placeholder-shown` +
        // `attr()` content. When the input / textarea has a
        // non-empty `placeholder` attribute and is empty, the
        // `::before` pseudo-element injects the placeholder text.
        // That box is the `::placeholder` pseudo-element: its muted
        // color is a `::placeholder` rule, which authors override
        // with their own `::placeholder` rules (CSS Pseudo-Elements 4
        // §4.3; the backend layers them over `::before`).
        (
            "input:placeholder-shown::before",
            css(TuiStyle::new(), "content", "attr(placeholder)"),
        ),
        (
            "textarea:placeholder-shown::before",
            css(TuiStyle::new(), "content", "attr(placeholder)"),
        ),
        (
            "input::placeholder, textarea::placeholder",
            TuiStyle::new().fg(TEXT_MUTED),
        ),
        // ── Toggle widgets ──
        // `<input type="checkbox">` and `<input type="radio">`
        // render their state via UA `::before` content. The
        // `:checked` pseudo-class flips the glyph between
        // unchecked and checked variants. Authors override by
        // writing a more specific rule (e.g.,
        // `[type=checkbox]::before { content: "☐" }`).
        //
        // `inline-block`, like `<button>`: form controls are
        // inline-level in HTML, so `<label><input type=checkbox> Name</label>`
        // flows on one line. Width auto-grows from the glyph
        // content; height is one cell. (`UA-CHECKBOX-INLINE-1`)
        // `width: auto` undoes the text-field `input { width: 20 }`
        // above: the box hugs its `::before` glyph (`[ ] ` / `( ) `),
        // so a toggle in a flex row takes 4 cells, not 20
        // (`FLEX-BLOCK-MAIN-INTRINSIC-1` — the width came from the UA,
        // not from flex-basis resolution).
        //
        // No `user-select`, as in browsers' UA sheets: a click on a
        // toggle beside prose reaches it because the text-selection
        // drag takes no pointer capture (P6G-SELECTION-CAPTURE-1).
        (
            "input[type=checkbox]",
            TuiStyle::new()
                .display(Display::InlineBlock)
                .width(Size::Auto)
                .height(Size::Fixed(1)),
        ),
        (
            "input[type=radio]",
            TuiStyle::new()
                .display(Display::InlineBlock)
                .width(Size::Auto)
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
    ]
}
