//! The showcase shell — header, sidebar, main view.
//!
//! Built entirely from native HTML elements + CSS. No opinionated
//! components, no widgets, no framework affordances — what the
//! browser would render given the same markup is what the
//! terminal renders.
//!
//! Structure:
//!
//! ```text
//! <div class="app">                 ← flex column
//!   <header class="app-header">
//!     <h1>rdom showcase</h1>
//!   </header>
//!   <div class="app-body">          ← flex row, takes remaining height
//!     <aside class="sidebar">       ← fixed width, non-interactive in M2
//!       <nav>
//!         <ul>
//!           <li>Hello World</li>    ← one <li> per registered demo
//!           ...
//!         </ul>
//!       </nav>
//!     </aside>
//!     <main class="main">           ← demo mounts here
//!       (active demo's subtree)
//!     </main>
//!   </div>
//! </div>
//! ```
//!
//! M2 mounts the demo at `DEMOS[0]` into `<main>` statically. M3
//! makes the sidebar interactive (click/keyboard to switch demos)
//! and wires per-demo stylesheet push/pop via M1's multi-slot
//! stylesheet API.

use rdom_tui::{NodeId, Stylesheet, TuiDom};

use crate::DEMOS;

/// References to load-bearing nodes the App needs to interact with
/// after the shell is built — e.g., M3 will use `main` to swap
/// demo subtrees on nav clicks.
#[derive(Copy, Clone, Debug)]
pub struct ShellHandles {
    /// The root `<div class="app">` — append to `dom.root()` (the
    /// shell does this internally, exposed for tests that want to
    /// re-query the tree).
    pub app_root: NodeId,
    /// The `<main>` container that hosts the active demo. Caller
    /// appends the demo's `build()` result to this node.
    pub main: NodeId,
    /// The sidebar `<aside>` — M3 attaches click listeners here.
    pub sidebar: NodeId,
}

/// Build the showcase shell under `dom.root()`. Does NOT mount any
/// demo — the caller picks one from [`crate::DEMOS`] and appends
/// its `build()` result to [`ShellHandles::main`].
///
/// Returns the handles to load-bearing nodes; the shell itself is
/// already attached to `dom.root()` when this returns.
pub fn build_shell(dom: &mut TuiDom) -> ShellHandles {
    // <div class="app">
    let app = dom.create_element("div");
    dom.set_attribute(app, "class", "app").unwrap();
    dom.append_child(dom.root(), app).unwrap();

    // <header class="app-header"><h1>rdom showcase</h1></header>
    let header = dom.create_element("header");
    dom.set_attribute(header, "class", "app-header").unwrap();
    let h1 = dom.create_element("h1");
    let h1_text = dom.create_text_node("rdom showcase");
    dom.append_child(h1, h1_text).unwrap();
    dom.append_child(header, h1).unwrap();
    dom.append_child(app, header).unwrap();

    // <div class="app-body"> (flex row container)
    let body = dom.create_element("div");
    dom.set_attribute(body, "class", "app-body").unwrap();
    dom.append_child(app, body).unwrap();

    // <aside class="sidebar">
    //   <h2>Demos</h2>
    //   <nav><ul>… one <li> per demo …</ul></nav>
    // </aside>
    let sidebar = dom.create_element("aside");
    dom.set_attribute(sidebar, "class", "sidebar").unwrap();
    let h2 = dom.create_element("h2");
    let h2_text = dom.create_text_node("Demos");
    dom.append_child(h2, h2_text).unwrap();
    dom.append_child(sidebar, h2).unwrap();
    let nav = dom.create_element("nav");
    let ul = dom.create_element("ul");
    for demo in DEMOS {
        let li = dom.create_element("li");
        let title = dom.create_text_node(demo.title());
        dom.append_child(li, title).unwrap();
        dom.append_child(ul, li).unwrap();
    }
    dom.append_child(nav, ul).unwrap();
    dom.append_child(sidebar, nav).unwrap();
    dom.append_child(body, sidebar).unwrap();

    // <main class="main"> — demo mount point.
    let main = dom.create_element("main");
    dom.set_attribute(main, "class", "main").unwrap();
    dom.append_child(body, main).unwrap();

    ShellHandles {
        app_root: app,
        main,
        sidebar,
    }
}

/// The shell's base stylesheet — chrome layout (header height,
/// sidebar width, main-view flex), no demo styles. Demos push
/// their own sheets on top via M1's multi-slot stylesheet API.
pub fn base_stylesheet() -> Stylesheet {
    rdom_css::from_css(BASE_CSS)
}

/// Chrome stylesheet for the shell. Authored as CSS so it reads
/// like CSS — the showcase IS the dogfooding fixture, so when a
/// consumer browses the source they should see the same shape
/// they'd write themselves.
///
/// Borders use `border-collapse: collapse` on the outer `.app` so
/// adjacent inner borders share cells instead of stacking into
/// double rules at every junction. The four child boxes
/// (`.app-header`, `.sidebar`, `.main`, plus `.app-body` as a
/// row container with no border of its own) line up cleanly.
const BASE_CSS: &str = r#"
.app {
  flex-direction: column;
  width: 1fr;
  height: 1fr;
  border: solid;
  border-color: rgb(70, 80, 100);
  border-collapse: collapse;
}

.app-header {
  height: 3;
  border: solid;
  border-color: rgb(70, 80, 100);
  padding: 0 2;
}
.app-header h1 {
  color: rgb(200, 220, 255);
  font-weight: bold;
}

.app-body {
  flex-direction: row;
  width: 1fr;
  height: 1fr;
}

.sidebar {
  width: 28;
  border: solid;
  border-color: rgb(70, 80, 100);
  padding: 1;
}
.sidebar h2 {
  color: rgb(150, 170, 200);
  font-weight: bold;
}

.main {
  width: 1fr;
  border: solid;
  border-color: rgb(70, 80, 100);
  padding: 1;
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_shell_attaches_app_root_under_dom_root() {
        let mut dom: TuiDom = TuiDom::new();
        let handles = build_shell(&mut dom);
        let parent = dom
            .node(handles.app_root)
            .parent_node()
            .expect("app root has a parent");
        assert_eq!(parent.id(), dom.root());
    }

    #[test]
    fn shell_has_main_under_app_body_under_app_root() {
        let mut dom: TuiDom = TuiDom::new();
        let handles = build_shell(&mut dom);
        // main → app-body → app-root
        let body = dom
            .node(handles.main)
            .parent_node()
            .expect("main has a parent")
            .id();
        let app = dom
            .node(body)
            .parent_node()
            .expect("body has a parent")
            .id();
        assert_eq!(app, handles.app_root);
    }

    #[test]
    fn sidebar_contains_one_li_per_registered_demo() {
        let mut dom: TuiDom = TuiDom::new();
        let handles = build_shell(&mut dom);

        // Sidebar children: <h2>Demos</h2>, then <nav>. Skip the h2.
        let nav = dom
            .node(handles.sidebar)
            .child_nodes()
            .find(|n| n.tag_name() == Some("nav"))
            .expect("sidebar has a <nav>");
        let ul = nav.first_element_child().expect("nav has a <ul>");
        assert_eq!(ul.tag_name(), Some("ul"));
        let li_count = ul
            .child_nodes()
            .filter(|n| n.tag_name() == Some("li"))
            .count();
        assert_eq!(li_count, crate::DEMOS.len(), "one <li> per registered demo");
    }

    #[test]
    fn sidebar_has_demos_heading() {
        let mut dom: TuiDom = TuiDom::new();
        let handles = build_shell(&mut dom);

        let h2 = dom
            .node(handles.sidebar)
            .child_nodes()
            .find(|n| n.tag_name() == Some("h2"))
            .expect("sidebar has an <h2>");
        let text = h2
            .child_nodes()
            .next()
            .and_then(|n| n.node_value().map(|s| s.to_string()));
        assert_eq!(text.as_deref(), Some("Demos"));
    }
}
