//! UA chrome showcase — what naked HTML built-ins look like with
//! rdom's UA stylesheet only.
//!
//! Demonstrates:
//! - `<button>` bracket chrome + `:focus` indicator
//! - `<details>` / `<summary>` disclosure triangle (▶ collapsed, ▼ open)
//! - `<ul>` / `<li>` bullet markers (• via `ul > li::before`)
//! - `<dialog open>` modal chrome (border + padding)
//!
//! Author CSS is structural only — the only rule is on the demo's
//! root wrapper. Every native HTML element below renders with UA
//! defaults alone.

use std::io;

use rdom_tui::{App, NodeId, Stylesheet, TuiDom};

use crate::{Category, Demo, Source};

pub const MARKUP: &str = r#"<div class="ua-chrome-demo">
  <h1>rdom UA chrome — pure defaults, no author CSS</h1>

  <section>
    <h3>Buttons</h3>
    <button>Save</button>
    <button>Cancel</button>
    <button>Continue</button>
  </section>

  <section>
    <h3>Lists</h3>
    <ul>
      <li>unordered first</li>
      <li>unordered second</li>
    </ul>
    <ol>
      <li>ordered first</li>
      <li>ordered second</li>
    </ol>
  </section>

  <section>
    <h3>Disclosure</h3>
    <details>
      <summary>Click or press Enter to toggle</summary>
      <p>Hidden until the disclosure is opened.</p>
    </details>
  </section>

  <section>
    <h3>Dialog (always open)</h3>
    <dialog open>Native dialog chrome: border + padding.</dialog>
  </section>
</div>"#;

pub const CSS: &str = r#"
.ua-chrome-demo {
  flex: 1;
  flex-direction: column;
  padding: 1 2;
  gap: 1;
}
"#;

pub fn build(dom: &mut TuiDom) -> NodeId {
    let root = dom.create_element("div");
    dom.set_attribute(root, "class", "ua-chrome-demo").unwrap();

    let h1 = dom.create_element("h1");
    let h1_text = dom.create_text_node("rdom UA chrome — pure defaults, no author CSS");
    dom.append_child(h1, h1_text).unwrap();
    dom.append_child(root, h1).unwrap();

    section(dom, root, "Buttons", |dom, sec| {
        for label in ["Save", "Cancel", "Continue"] {
            let btn = dom.create_element("button");
            let t = dom.create_text_node(label);
            dom.append_child(btn, t).unwrap();
            dom.append_child(sec, btn).unwrap();
        }
    });

    section(dom, root, "Lists", |dom, sec| {
        let ul = dom.create_element("ul");
        for item in ["unordered first", "unordered second"] {
            let li = dom.create_element("li");
            let t = dom.create_text_node(item);
            dom.append_child(li, t).unwrap();
            dom.append_child(ul, li).unwrap();
        }
        dom.append_child(sec, ul).unwrap();

        let ol = dom.create_element("ol");
        for item in ["ordered first", "ordered second"] {
            let li = dom.create_element("li");
            let t = dom.create_text_node(item);
            dom.append_child(li, t).unwrap();
            dom.append_child(ol, li).unwrap();
        }
        dom.append_child(sec, ol).unwrap();
    });

    section(dom, root, "Disclosure", |dom, sec| {
        let details = dom.create_element("details");
        let summary = dom.create_element("summary");
        let s_t = dom.create_text_node("Click or press Enter to toggle");
        dom.append_child(summary, s_t).unwrap();
        dom.append_child(details, summary).unwrap();
        let p = dom.create_element("p");
        let p_t = dom.create_text_node("Hidden until the disclosure is opened.");
        dom.append_child(p, p_t).unwrap();
        dom.append_child(details, p).unwrap();
        dom.append_child(sec, details).unwrap();
    });

    section(dom, root, "Dialog (always open)", |dom, sec| {
        let dialog = dom.create_element("dialog");
        dom.set_attribute(dialog, "open", "").unwrap();
        let d_t = dom.create_text_node("Native dialog chrome: border + padding.");
        dom.append_child(dialog, d_t).unwrap();
        dom.append_child(sec, dialog).unwrap();
    });

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

fn section(
    dom: &mut TuiDom,
    parent: NodeId,
    title: &str,
    build: impl FnOnce(&mut TuiDom, NodeId),
) {
    let sec = dom.create_element("section");
    let label = dom.create_element("h3");
    let t = dom.create_text_node(title);
    dom.append_child(label, t).unwrap();
    dom.append_child(sec, label).unwrap();
    build(dom, sec);
    dom.append_child(parent, sec).unwrap();
}

pub struct UaChrome;

impl Demo for UaChrome {
    fn slug(&self) -> &'static str {
        "builtins/ua-chrome"
    }

    fn title(&self) -> &'static str {
        "UA chrome"
    }

    fn category(&self) -> Category {
        Category::BuiltIns
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
