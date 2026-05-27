//! M5.6 — TUI app-shell demo. Showcases `border-collapse: collapse`
//! in a headline grid layout: outer shell + header + 3-column body +
//! footer, every internal border shared with the outer shell's frame.
//!
//! ```text
//! ┌────────────────────────────────────┐
//! ├─────┬───────────────────────┬──────┤
//! │     │                       │      │
//! │     ├───────────────────────┤      │
//! │     │                       │      │
//! ├─────┴───────────────────────┴──────┤
//! │ status                             │
//! └────────────────────────────────────┘
//! ```
//!
//! Closes the `M5-COLLAPSE-2` limitation noted in the original M5
//! review gate: coincident-corner cells (shell's top-left + header's
//! top-left at (0,0)) render correctly via per-cell border masks
//! that OR directional bits in additively.

use std::io;

use rdom_tui::{App, NodeId, Stylesheet, TuiDom};

use crate::{Category, Demo, Source};

pub const MARKUP: &str = r#"<div class="app-shell-demo">
  <div class="header"></div>
  <div class="body">
    <div class="sidebar"></div>
    <div class="middle"></div>
    <div class="right-panel"></div>
  </div>
  <div class="footer"></div>
</div>"#;

pub const CSS: &str = r#"
.app-shell-demo {
  flex: 1;
  display: flex;
  flex-direction: column;
  border: solid;
  border-collapse: collapse;
}
.app-shell-demo .header {
  height: 3;
  border: solid;
}
.app-shell-demo .body {
  flex: 1;
  display: flex;
  flex-direction: row;
  border: solid;
  /* BORDER-MODEL-1: collapse is non-inheriting; the body declares
   * it so its direct children (sidebar/middle/right-panel) share
   * borders with each other and with the body's own ring. */
  border-collapse: collapse;
}
.app-shell-demo .sidebar {
  width: 20;
  min-width: 12;
  border: solid;
}
.app-shell-demo .middle {
  flex: 1;
  border: solid;
}
.app-shell-demo .right-panel {
  width: 24;
  max-width: 32;
  border: solid;
}
.app-shell-demo .footer {
  height: 3;
  border: solid;
}
"#;

pub fn build(dom: &mut TuiDom) -> NodeId {
    let shell = dom.create_element("div");
    dom.set_attribute(shell, "class", "app-shell-demo").unwrap();

    let header = dom.create_element("div");
    dom.set_attribute(header, "class", "header").unwrap();
    dom.append_child(shell, header).unwrap();

    let body = dom.create_element("div");
    dom.set_attribute(body, "class", "body").unwrap();
    let sidebar = dom.create_element("div");
    dom.set_attribute(sidebar, "class", "sidebar").unwrap();
    let middle = dom.create_element("div");
    dom.set_attribute(middle, "class", "middle").unwrap();
    let right_panel = dom.create_element("div");
    dom.set_attribute(right_panel, "class", "right-panel")
        .unwrap();
    dom.append_child(body, sidebar).unwrap();
    dom.append_child(body, middle).unwrap();
    dom.append_child(body, right_panel).unwrap();
    dom.append_child(shell, body).unwrap();

    let footer = dom.create_element("div");
    dom.set_attribute(footer, "class", "footer").unwrap();
    dom.append_child(shell, footer).unwrap();

    shell
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

pub struct AppShell;

impl Demo for AppShell {
    fn slug(&self) -> &'static str {
        "layout/app-shell"
    }

    fn title(&self) -> &'static str {
        "App shell"
    }

    fn category(&self) -> Category {
        Category::Layout
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
