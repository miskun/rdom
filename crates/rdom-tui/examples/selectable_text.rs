//! Selectable text — prose + code + CJK, all selectable with the
//! usual mouse gestures. Demonstrates `user-select: none` as an
//! opt-out for UI chrome.
//!
//! The `.chrome` paragraph has `user-select: none` — drag over it
//! and nothing happens, which is what you want for UI chrome that
//! shouldn't leak into the clipboard. Every other paragraph is
//! selectable.
//!
//! ## What this demo covers
//!
//! - **Drag to select** — mouse down on text, drag, mouse up.
//! - **Double-click** — selects a word.
//! - **Triple-click** — selects a line (IFC-wrapped lines, so
//!   wrapped paragraphs split cleanly).
//! - **`::selection` overlay** — selected cells paint as reversed
//!   fg/bg by default; author CSS overrides at `::selection`.
//! - **Ctrl-C / Cmd-C** — copies the serialized selection to the
//!   system clipboard (via `arboard`). Ctrl-V fires a `paste`
//!   event; Ctrl-X fires `cut`. Ctrl-C with no selection quits.
//!
//! ## What this demo does NOT cover (deliberately)
//!
//! - **Focus-based keyboard extension** (`Shift+arrow`,
//!   `Shift+Ctrl+arrow`, `Ctrl-A`) — those require a focusable
//!   container (e.g. `<p tabindex="0">`). Scoped out here to keep
//!   the demo focused on the mouse-driven path; a companion demo
//!   can cover keyboard-extend.
//!
//! Run: `cargo run --example selectable_text -p rdom-tui`

use std::io;

use rdom_tui::prelude::*;

const PROSE: &str = concat!(
    "Terminal UIs should let you select text. That's table stakes ",
    "for any interactive tool that shows text — error messages, log ",
    "lines, config values, you name it. rdom-tui's runtime ships ",
    "with drag, double-click, triple-click, Shift+arrow, and Ctrl-A ",
    "support out of the box."
);

const CODE: &str = concat!(
    "let sel = Selection::new(",
    "Position::new(t, 0), ",
    "Position::new(t, 5));"
);

const CJK: &str = "中文字符也可以选择。CJK graphemes snap to full width.";

fn main() -> io::Result<()> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();

    let screen = dom.create_element("screen");

    let title = dom.create_element("title");
    let title_t = dom.create_text_node("Selectable text demo");
    dom.append_child(title, title_t).unwrap();

    // UI chrome — user-select:none, so dragging over this row
    // doesn't start a selection.
    let chrome = dom.create_element("chrome");
    let chrome_t =
        dom.create_text_node("Drag over prose, code, or CJK. This bar is user-select:none.");
    dom.append_child(chrome, chrome_t).unwrap();

    // Prose paragraph — must contain at least one inline element
    // child for `is_ifc_block` to register it as an IFC. A trailing
    // empty <span> does the trick without adding visible content.
    // Not focusable (no tabindex) since this demo scopes itself to
    // the mouse-driven selection path.
    let prose = dom.create_element("p");
    let prose_t = dom.create_text_node(PROSE);
    dom.append_child(prose, prose_t).unwrap();
    let prose_tail = dom.create_element("span");
    dom.append_child(prose, prose_tail).unwrap();

    // Code block — also non-focusable, for the same reason.
    let code = dom.create_element("code-block");
    let code_t = dom.create_text_node(CODE);
    dom.append_child(code, code_t).unwrap();
    let code_tail = dom.create_element("span");
    dom.append_child(code, code_tail).unwrap();

    // CJK sample — non-focusable.
    let cjk = dom.create_element("p");
    let cjk_t = dom.create_text_node(CJK);
    dom.append_child(cjk, cjk_t).unwrap();
    let cjk_tail = dom.create_element("span");
    dom.append_child(cjk, cjk_tail).unwrap();

    dom.append_child(screen, title).unwrap();
    dom.append_child(screen, chrome).unwrap();
    dom.append_child(screen, prose).unwrap();
    dom.append_child(screen, code).unwrap();
    dom.append_child(screen, cjk).unwrap();
    dom.append_child(root, screen).unwrap();

    // Author CSS is structural-only. `<screen>`, `<title>`, `<chrome>`,
    // `<code-block>` are custom tags; `<p>` and `<span>` are HTML.
    // Only layout rules + the functional `user_select: None` on
    // `<chrome>` (the demo's whole point is that chrome doesn't get
    // selected). No colors / borders / focus styling — this example
    // exists to show what selection + clipboard look like with bare
    // UA defaults.
    let sheet = Stylesheet::new()
        .rule(
            "screen",
            TuiStyle::new()
                .direction(Direction::Column)
                .width(Size::Flex(1))
                .height(Size::Flex(1))
                .padding(Padding::symmetric(3, 1))
                .gap(1),
        )
        .unwrap()
        .rule("title", TuiStyle::new().height(Size::Fixed(1)))
        .unwrap()
        .rule(
            "chrome",
            TuiStyle::new()
                .user_select(UserSelect::None)
                .height(Size::Fixed(1)),
        )
        .unwrap()
        .rule("p", TuiStyle::new().width(Size::Flex(1)))
        .unwrap()
        .rule(
            "code-block",
            TuiStyle::new().display(Display::Block).width(Size::Flex(1)),
        )
        .unwrap();

    App::new(dom, sheet)?.run()
}
