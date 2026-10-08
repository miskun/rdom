//! UA rules: Block interactive elements and the ARIA tree pattern.

use crate::color::named;
use crate::color::system::{ACCENT, BORDER_DEFAULT};
use crate::layout::{Border, Display, Padding};
use crate::{Color, Content, TuiStyle};

/// The UA rules of this group, in cascade order.
pub(super) fn rules() -> Vec<(&'static str, TuiStyle)> {
    vec![
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
        // The content slot (HTML §15.5.20): everything but the first
        // `<summary>` child is `::details-content`'s, which hides it
        // while `<details>` lacks `open` — the UA's `content-visibility:
        // hidden` there, which rdom's backend applies to the slot (rdom
        // has no `content-visibility` yet: C14-CONTAIN). The runtime
        // (`runtime::builtins::details`) toggles `open` on click /
        // Enter / Space.
        (
            "details::details-content",
            TuiStyle::new().display(Display::Block),
        ),
        // ── Tree (ARIA tree pattern) ──
        // rdom has no `<tree>` element — trees are built the
        // web-faithful way with `role`: `<ul role=tree>` holds
        // `<li role=treeitem>`s, and a branch nests its children in
        // a `<ul role=group>`. `runtime::builtins::tree` drives the
        // keyboard / pointer behavior; the guide-line paint pass
        // (`render::paint_pass::tree_guides`) draws BOTH the `│ ├ └`
        // connectors AND the `▼` / `▶` disclosure chevron into the
        // gutter these paddings reserve.
        //
        // Why the chevron is painted, not a `::before`: it lives in
        // the reserved padding gutter next to the connectors, not in
        // the label's content box, so painting both in one pass keeps
        // them aligned and visually unified regardless of the row's
        // own pseudo content (DIVERGENCES §ARIA tree).
        //
        // State is attribute-driven (presence of `aria-expanded` =
        // branch; `true`/`false` = open/closed; `aria-busy` =
        // loading, app-styleable; `aria-selected` = chosen;
        // `data-rdom-active` = keyboard cursor). See DIVERGENCES.md.
        // `padding: 0` overrides the `ul` default left padding so the
        // tree sits flush-left; indentation comes from the per-level
        // group/treeitem paddings below, not the list default.
        (
            "[role=tree]",
            TuiStyle::new()
                .display(Display::Block)
                .padding(Padding::new(0, 0, 0, 0)),
        ),
        // The group adds NO indent of its own — each nesting level's
        // 2-cell step comes from the treeitem's own `padding-left: 2`
        // (the arrow field). The guide paint draws the connector for
        // a child in the cell to the LEFT of the child's box, which
        // lands under the parent item's arrow column.
        // `padding: 0` overrides the `ul` default left padding so the
        // group adds no indent of its own.
        (
            "[role=group]",
            TuiStyle::new()
                .display(Display::Block)
                .padding(Padding::new(0, 0, 0, 0)),
        ),
        // The treeitem is the row. `padding-left: 2` reserves the
        // `▼ `/`▶ ` expand-arrow field before the label; the
        // connector + ancestor trunks are painted in the gutter to
        // the left. `border-color` is the GUIDE color — the guide
        // paint reads `computed.border_color` for the connector/arrow
        // glyphs even though the item paints no actual CSS border.
        // Authors retheme guides with
        // `[role=treeitem] { border-color: … }`. See DIVERGENCES.md.
        (
            "[role=treeitem]",
            TuiStyle::new()
                .display(Display::Block)
                .padding(Padding::new(0, 0, 0, 2))
                .border_fg(BORDER_DEFAULT),
        ),
        // Collapsed branch hides its child group — same disclosure
        // mechanic as `details:not([open]) > *`.
        (
            "[role=treeitem][aria-expanded=false] > [role=group]",
            TuiStyle::new().display(Display::None),
        ),
        // The tree uses the ARIA active-descendant model: the container holds
        // focus on behalf of the cursor row. No reset hack is needed anymore —
        // the focus tint is scoped to atomic controls, so a `[role=tree]`
        // container never gets a fill. Its visible focus indicator is the
        // active ROW below (and, when scrollable, the accent scrollbar thumb).
        // Keyboard cursor (active descendant) — highlighted only
        // while the tree itself holds focus, so the cursor dims when
        // focus leaves. The bg is consumed by the tree paint pass as
        // a single-row fill (not a full-subtree fill); see
        // `tree_guides`.
        (
            "[role=tree]:focus [data-rdom-active]",
            TuiStyle::new().bg(Color::Rgb(0x2d, 0x2f, 0x31)),
        ),
        // Selected row — accent fill, same treatment as
        // `option[selected]`. Also painted as a single-row fill.
        (
            "[role=treeitem][aria-selected=true]",
            TuiStyle::new().bg(ACCENT).fg(named::BLACK),
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
                .border(Border::single())
                .border_radius(crate::layout::BorderRadius::cells(1.0))
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
    ]
}
