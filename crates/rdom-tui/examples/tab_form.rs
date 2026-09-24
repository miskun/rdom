//! Tab-navigable form built from the native `<form>`, `<input>`,
//! `<textarea>` and `<button>` built-ins.
//!
//! No keydown listeners: the runtime supplies caret, character
//! insertion, Backspace, OS clipboard, the `:focus` cascade and form
//! submission from the built-ins alone. The only app-level code is
//! the stylesheet and a `submit` handler that reads the field values
//! with `runtime::builtins::form::collect`.
//!
//! Run: `cargo run -p rdom-tui --example tab_form`
//!
//! Self-contained on purpose: this file is the whole program. The
//! browsable version lives in `rdom-showcase` ("Forms → Tab form").

use std::io;

use rdom_parser::parse_into;
use rdom_tui::runtime::builtins::form;
use rdom_tui::{App, ListenerOptions, TuiDom};

const MARKUP: &str = r#"<div class="tab-form-demo">
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

const CSS: &str = r#"
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
.tab-form-demo .hint { height: 1; }
.tab-form-demo .row {
  display: flex;
  flex-direction: row;
  gap: 1;
}
.tab-form-demo .row label { width: 9; }
.tab-form-demo .row input { flex: 1; }
.tab-form-demo .row textarea { flex: 1; height: 3; }
.tab-form-demo .status { height: 1; }
"#;

fn main() -> io::Result<()> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    parse_into(&mut dom, MARKUP, root).expect("template parses");

    let form_el = dom.query_selector("form").expect("form").id();
    let status_text = dom
        .query_selector(".status")
        .expect("status")
        .first_child()
        .expect("status text")
        .id();

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

    App::new(dom, rdom_css::from_css(CSS))?.run()
}
