//! Tab-navigable form built from native `<form>` + `<input>` +
//! `<textarea>` + `<button>` builtins.
//!
//! No keydown listeners — the runtime supplies caret, character
//! insertion, Backspace, OS clipboard, `:focus` cascade, and form
//! submission entirely from the built-ins. The only app-level code
//! is the stylesheet + the `submit` handler that reads field
//! values via `runtime::builtins::form::collect`.

use std::io;

use rdom_tui::runtime::builtins::form;
use rdom_tui::{App, ListenerOptions, NodeId, Stylesheet, TuiDom};

use crate::{Category, Demo, Source};

pub const MARKUP: &str = r#"<div class="tab-form-demo">
  <h1>Tab-navigable form demo</h1>
  <p class="hint">Tab/Shift-Tab: focus  •  letters/digits: type  •  Backspace: delete  •  Enter: submit  •  Ctrl-C: quit</p>
  <form>
    <div class="row"><label>  Name: </label><input type="text" name="name"></div>
    <div class="row"><label> Email: </label><input type="email" name="email"></div>
    <div class="row"><label> Notes: </label><textarea name="notes"></textarea></div>
    <div class="row"><button>Submit</button></div>
  </form>
  <div class="status">(not submitted)</div>
</div>"#;

pub const CSS: &str = r#"
.tab-form-demo {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 1 2;
  gap: 1;
}
.tab-form-demo h1 {
  color: rgb(180, 220, 255);
  font-weight: bold;
}
.tab-form-demo .hint {
  height: 1;
}
.tab-form-demo .row {
  display: flex;
  flex-direction: row;
  gap: 1;
}
.tab-form-demo .row label {
  width: 9;
}
.tab-form-demo .row input {
  flex: 1;
}
.tab-form-demo .row textarea {
  flex: 1;
  height: 3;
}
.tab-form-demo .status {
  height: 1;
}
"#;

pub fn build(dom: &mut TuiDom) -> NodeId {
    let root = dom.create_element("div");
    dom.set_attribute(root, "class", "tab-form-demo").unwrap();

    let h1 = dom.create_element("h1");
    let h1_text = dom.create_text_node("Tab-navigable form demo");
    dom.append_child(h1, h1_text).unwrap();
    dom.append_child(root, h1).unwrap();

    let hint = dom.create_element("p");
    dom.set_attribute(hint, "class", "hint").unwrap();
    let hint_text = dom.create_text_node(
        "Tab/Shift-Tab: focus  •  letters/digits: type  •  Backspace: delete  •  Enter: submit  •  Ctrl-C: quit",
    );
    dom.append_child(hint, hint_text).unwrap();
    dom.append_child(root, hint).unwrap();

    let form_el = dom.create_element("form");

    for (label, name, ty) in [("Name", "name", "text"), ("Email", "email", "email")] {
        let row = dom.create_element("div");
        dom.set_attribute(row, "class", "row").unwrap();

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

    let notes_row = dom.create_element("div");
    dom.set_attribute(notes_row, "class", "row").unwrap();
    let notes_label = dom.create_element("label");
    let notes_label_t = dom.create_text_node(&format!("{:>6}: ", "Notes"));
    dom.append_child(notes_label, notes_label_t).unwrap();
    let notes_ta = dom.create_element("textarea");
    dom.set_attribute(notes_ta, "name", "notes").unwrap();
    dom.append_child(notes_row, notes_label).unwrap();
    dom.append_child(notes_row, notes_ta).unwrap();
    dom.append_child(form_el, notes_row).unwrap();

    let submit_row = dom.create_element("div");
    dom.set_attribute(submit_row, "class", "row").unwrap();
    let submit_btn = dom.create_element("button");
    let submit_label = dom.create_text_node("Submit");
    dom.append_child(submit_btn, submit_label).unwrap();
    dom.append_child(submit_row, submit_btn).unwrap();
    dom.append_child(form_el, submit_row).unwrap();
    dom.append_child(root, form_el).unwrap();

    let status = dom.create_element("div");
    dom.set_attribute(status, "class", "status").unwrap();
    let status_text = dom.create_text_node("(not submitted)");
    dom.append_child(status, status_text).unwrap();
    dom.append_child(root, status).unwrap();

    // Submit handler — reads field values via `form::collect` and
    // writes the result into the status line.
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

    root
}

pub fn stylesheet() -> Stylesheet {
    rdom_css::from_css(CSS)
}

pub fn run_standalone() -> io::Result<()> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let demo_root = build(&mut dom);
    dom.append_child(root, demo_root).unwrap();
    App::new(dom, stylesheet())?.run()
}

pub struct TabForm;

impl Demo for TabForm {
    fn slug(&self) -> &'static str {
        "forms/tab-form"
    }

    fn title(&self) -> &'static str {
        "Tab form"
    }

    fn category(&self) -> Category {
        Category::Forms
    }

    fn build(&self, dom: &mut TuiDom) -> NodeId {
        build(dom)
    }

    fn stylesheet(&self) -> Stylesheet {
        stylesheet()
    }

    fn source(&self) -> Source {
        Source {
            markup: MARKUP,
            css: CSS,
        }
    }
}
