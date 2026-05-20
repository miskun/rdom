//! Tab-navigable form built from native `<form>` + `<input>` +
//! `<button>` builtins. Tab cycles focus through the inputs and the
//! submit button; typing edits the focused input; Enter on the
//! submit button (or click) dispatches `submit` on the form, and the
//! handler reads the field values via `runtime::builtins::form::collect`.
//!
//! The runtime supplies all editing behavior (caret, character
//! insertion, Backspace, OS clipboard, `:focus` cascade) — there are
//! no keydown listeners or synthetic events in this demo. The only
//! app-level code is the stylesheet and the `submit` handler.
//!
//! Controls:
//!   Tab / Shift-Tab   — cycle focus through the inputs and submit button.
//!   Letters / digits  — type into the focused input.
//!   Backspace         — delete the previous character.
//!   Enter             — activate the submit button (when focused).
//!   Ctrl-C            — quit.
//!
//! Run: `cargo run --example tab_form -p rdom-tui`

use std::io;

use rdom_tui::prelude::*;
use rdom_tui::runtime::builtins::form;

fn main() -> io::Result<()> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();

    let screen = dom.create_element("screen");

    let title = dom.create_element("title");
    let title_text = dom.create_text_node("Tab-navigable form demo");
    dom.append_child(title, title_text).unwrap();

    let hint = dom.create_element("hint");
    let hint_text = dom.create_text_node(
        "Tab/Shift-Tab: focus  •  letters/digits: type  •  Backspace: delete  •  Enter: submit  •  Ctrl-C: quit",
    );
    dom.append_child(hint, hint_text).unwrap();

    // Two `<input>` fields + a `<textarea>` wrapped in a `<form>`.
    // `name` attributes feed `form::collect`. `<input>`, `<textarea>`,
    // and `<button>` are implicitly focusable, so no explicit
    // `tabindex` is needed.
    let form_el = dom.create_element("form");

    let inputs: [(&str, &str, &str); 2] = [("Name", "name", "text"), ("Email", "email", "email")];
    for (label, name, ty) in inputs {
        let row = dom.create_element("row");

        let label_el = dom.create_element("label");
        let label_t = dom.create_text_node(&format!("{label:>6}: "));
        dom.append_child(label_el, label_t).unwrap();

        let input = dom.create_element("input");
        dom.set_attribute(input, "type", ty).unwrap();
        dom.set_attribute(input, "name", name).unwrap();

        dom.append_child(row, label_el).unwrap();
        dom.append_child(row, input).unwrap();
        dom.append_child(form_el, row).unwrap();
    }

    // Notes — multi-line `<textarea>`. Same row structure as inputs,
    // just a taller widget. The author CSS below sets row height to
    // `Auto` so single-line rows stay 1-tall and this row grows to
    // the textarea's height.
    let notes_row = dom.create_element("row");
    let notes_label = dom.create_element("label");
    let notes_label_t = dom.create_text_node(&format!("{:>6}: ", "Notes"));
    dom.append_child(notes_label, notes_label_t).unwrap();
    let notes_ta = dom.create_element("textarea");
    dom.set_attribute(notes_ta, "name", "notes").unwrap();
    dom.append_child(notes_row, notes_label).unwrap();
    dom.append_child(notes_row, notes_ta).unwrap();
    dom.append_child(form_el, notes_row).unwrap();

    // Submit button — `<button>` defaults to `type="submit"` inside
    // a form, so click / Enter / Space all dispatch `submit`.
    let submit_row = dom.create_element("row");
    let submit_btn = dom.create_element("button");
    let submit_label = dom.create_text_node("Submit");
    dom.append_child(submit_btn, submit_label).unwrap();
    dom.append_child(submit_row, submit_btn).unwrap();
    dom.append_child(form_el, submit_row).unwrap();

    let status = dom.create_element("status");
    let status_text = dom.create_text_node("(not submitted)");
    dom.append_child(status, status_text).unwrap();

    dom.append_child(screen, title).unwrap();
    dom.append_child(screen, hint).unwrap();
    dom.append_child(screen, form_el).unwrap();
    dom.append_child(screen, status).unwrap();
    dom.append_child(root, screen).unwrap();

    // Author CSS is layout-only — `<screen>`, `<title>`, `<hint>`,
    // `<row>`, `<label>`, `<status>` are custom tags rdom doesn't
    // ship UA defaults for, so they need a minimum of structural
    // styling to render at all. Native elements (`<form>`, `<input>`,
    // `<button>`) intentionally have NO author rules: this example
    // exists to show what tab-navigable forms look like out-of-the-
    // box. The only `<input>` rule sets `width: Flex(1)` so inputs
    // grow in their row (UA default is `Fixed(20)`).
    let sheet = Stylesheet::new()
        .rule(
            "screen",
            TuiStyle::new()
                .direction(Direction::Column)
                .width(Size::Flex(1))
                .height(Size::Flex(1))
                .padding(Padding::symmetric(2, 1))
                .gap(1),
        )
        .unwrap()
        .rule("title", TuiStyle::new().height(Size::Fixed(1)))
        .unwrap()
        .rule("hint", TuiStyle::new().height(Size::Fixed(1)))
        .unwrap()
        .rule(
            "row",
            TuiStyle::new()
                .direction(Direction::Row)
                .gap(1)
                .height(Size::Auto),
        )
        .unwrap()
        .rule("label", TuiStyle::new().width(Size::Fixed(9)))
        .unwrap()
        .rule("input", TuiStyle::new().width(Size::Flex(1)))
        .unwrap()
        .rule(
            "textarea",
            TuiStyle::new().width(Size::Flex(1)).height(Size::Fixed(3)),
        )
        .unwrap()
        .rule("status", TuiStyle::new().height(Size::Fixed(1)))
        .unwrap();

    // Submit handler — reads field values via `form::collect`
    // and writes the result into the status line.
    dom.add_event_listener(form_el, "submit", ListenerOptions::default(), move |ctx| {
        let values = form::collect(ctx.dom, form_el);
        let mut msg = String::from("submitted: ");
        for (i, (name, value)) in values.iter().enumerate() {
            if i > 0 {
                msg.push_str(", ");
            }
            msg.push_str(&format!("{name}={value:?}"));
        }
        let _ = ctx.dom.node_mut(status_text).set_node_value(&msg);
    })
    .unwrap();

    App::new(dom, sheet)?.run()
}
