//! Form lifecycle — constraint validation, `<fieldset disabled>`, reset
//! and a `formnovalidate` submitter, all native.
//!
//! `required` and `pattern` make the fields invalid until they are
//! filled in; the author's `:invalid` / `:valid` rules color them. Save
//! runs constraint validation, so while a field is invalid the form
//! fires `invalid` at each bad control instead of `submit`. Draft
//! carries `formnovalidate` and submits anyway. Reset restores every
//! field's default value. The control inside `<fieldset disabled>` is
//! neither focusable, validated nor submitted. The status line shows
//! what the `submit` event's detail reports: the submitter, whether
//! validation was skipped, and the method.

use std::io;

use rdom_parser::parse_into;
use rdom_tui::runtime::builtins::form;
use rdom_tui::{App, FormMethod, ListenerOptions, NodeId, Stylesheet, TuiDom};

use crate::{Category, Demo, Source};

pub const MARKUP: &str = r#"<div class="form-life">
  <h1>Form lifecycle</h1>
  <p class="hint">Tab: focus  •  type  •  Enter: activate  •  Save validates, Draft skips it</p>
  <form class="signup" method="post">
    <div class="row"><label>Name*</label><input name="name" required></div>
    <div class="row"><label>Code</label><input name="code" pattern="[A-Z]{3}" value="abc"></div>
    <fieldset disabled>
      <legend>Disabled fieldset</legend>
      <div class="row"><label>Note</label><input name="note" value="not sent"></div>
    </fieldset>
    <div class="row buttons"><button>Save</button><button formnovalidate>Draft</button><button type="reset">Reset</button></div>
  </form>
  <p class="status">(not submitted)</p>
</div>"#;

pub const CSS: &str = r#"
.form-life {
  display: flex;
  flex-direction: column;
  padding: 1 2;
  gap: 1;
}
.form-life h1 {
  color: rgb(180, 220, 255);
  font-weight: bold;
}
.form-life .row {
  display: flex;
  flex-direction: row;
  gap: 1;
}
.form-life .row label {
  width: 7;
}
.form-life .row input {
  width: 20;
}
.form-life input:invalid {
  color: rgb(255, 120, 120);
}
.form-life input:valid {
  color: rgb(140, 220, 140);
}
.form-life .status {
  height: 1;
}
"#;

/// The demo's elements a driver (or a test) needs.
#[derive(Debug, Clone, Copy)]
pub struct Parts {
    pub root: NodeId,
    pub form: NodeId,
    pub name: NodeId,
    pub code: NodeId,
    pub save: NodeId,
    pub draft: NodeId,
    pub reset: NodeId,
    pub status: NodeId,
}

pub fn build(dom: &mut TuiDom) -> NodeId {
    build_parts(dom).root
}

pub fn build_parts(dom: &mut TuiDom) -> Parts {
    let host = dom.create_element("div");
    parse_into(dom, MARKUP, host).expect("template parses");
    let root = dom.node(host).first_element_child().unwrap().id();
    let one = |dom: &TuiDom, sel: &str| {
        dom.node(root)
            .query_selector(sel)
            .unwrap_or_else(|| panic!("{sel} is in the markup"))
            .id()
    };
    let buttons: Vec<NodeId> = dom
        .node(root)
        .query_selector_all(".buttons button")
        .iter()
        .map(|b| b.id())
        .collect();
    let parts = Parts {
        root: host,
        form: one(dom, "form"),
        name: one(dom, "input[name=name]"),
        code: one(dom, "input[name=code]"),
        save: buttons[0],
        draft: buttons[1],
        reset: buttons[2],
        status: one(dom, ".status"),
    };
    wire(dom, parts);
    parts
}

fn wire(dom: &mut TuiDom, parts: Parts) {
    let Parts { form, status, .. } = parts;
    dom.add_event_listener(form, "submit", ListenerOptions::default(), move |ctx| {
        let (submitter, no_validate, method) = match ctx.event.detail.as_submit() {
            Some(d) => (d.submitter, d.no_validate, d.method),
            None => (None, false, FormMethod::Get),
        };
        let by = submitter.map_or_else(
            || "the form".to_string(),
            |b| ctx.dom.node(b).text_content(),
        );
        let checked = if no_validate {
            "not validated"
        } else {
            "validated"
        };
        let method = method.as_str();
        let values: Vec<String> = form::collect(ctx.dom, form)
            .iter()
            .map(|(k, v)| format!("{k}={v:?}"))
            .collect();
        set_status(
            ctx.dom,
            status,
            &format!("submit by {by}, {checked}, {method}: {}", values.join(" ")),
        );
    })
    .unwrap();
    // `invalid` does not bubble: listen in the capture phase.
    let capture = ListenerOptions::capture();
    dom.add_event_listener(form, "invalid", capture, move |ctx| {
        set_status(ctx.dom, status, "blocked: fix the fields in red");
    })
    .unwrap();
    dom.add_event_listener(form, "reset", ListenerOptions::default(), move |ctx| {
        set_status(ctx.dom, status, "reset: defaults restored");
    })
    .unwrap();
}

fn set_status(dom: &mut TuiDom, status: NodeId, text: &str) {
    dom.node_mut(status)
        .set_text_content(text)
        .expect("the status line takes text");
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

pub struct FormLifecycle;

impl Demo for FormLifecycle {
    fn slug(&self) -> &'static str {
        "forms/form-lifecycle"
    }

    fn title(&self) -> &'static str {
        "Form lifecycle"
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
