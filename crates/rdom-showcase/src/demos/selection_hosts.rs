//! Selection hosts — `user-select: contain` and `user-select: all`.
//!
//! A drag that starts inside the `contain` box stays inside it however
//! far the pointer goes (CSS UI 4 §6.1: the selection may not extend
//! past the element that contains its start). A click anywhere in the
//! `all` line selects the whole line as one unit. Outside both, text
//! selects normally; Ctrl-C copies what is selected.

use std::io;

use rdom_parser::parse_into;
use rdom_tui::{App, NodeId, Stylesheet, TuiDom};

use crate::{Category, Demo, Source};

pub const MARKUP: &str = r#"<div class="sel-hosts">
  <p class="lead">Free text: a drag from here selects across the boxes below.</p>
  <div class="contain"><p>user-select: contain. Start a drag in this box and drag out of it: the selection stays inside.</p></div>
  <p class="all">user-select: all. One click selects this whole line.</p>
  <p class="tail">More free text after the hosts.</p>
</div>"#;

pub const CSS: &str = r#"
.sel-hosts {
  display: flex;
  flex-direction: column;
  padding: 1 2;
  gap: 1;
  width: 60;
}
.sel-hosts .contain {
  user-select: contain;
  border: rounded;
  border-color: rgb(120, 200, 160);
  padding: 0 1;
}
.sel-hosts .all {
  user-select: all;
  color: rgb(255, 210, 120);
}
"#;

pub fn build(dom: &mut TuiDom) -> NodeId {
    let host = dom.create_element("div");
    parse_into(dom, MARKUP, host).expect("template parses");
    host
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

pub struct SelectionHosts;

impl Demo for SelectionHosts {
    fn slug(&self) -> &'static str {
        "selection/selection-hosts"
    }

    fn title(&self) -> &'static str {
        "Selection hosts"
    }

    fn category(&self) -> Category {
        Category::Selection
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
