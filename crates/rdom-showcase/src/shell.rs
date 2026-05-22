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

use rdom_core::NodeId;
use rdom_tui::{Stylesheet, TuiDom};

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

    // <aside class="sidebar"><nav><ul>… one <li> per demo …</ul></nav></aside>
    let sidebar = dom.create_element("aside");
    dom.set_attribute(sidebar, "class", "sidebar").unwrap();
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
    // CSS-as-strings would let us reuse rdom-css here; for M2 we
    // build the sheet programmatically to avoid pulling rdom-css
    // into the showcase's dependency closure for a single shell
    // sheet. M3 will likely refactor this to a `<style>` tag inside
    // the shell markup once the showcase grows enough chrome to
    // warrant it.
    use rdom_style::Color;
    use rdom_tui::layout::{Border, Direction, Padding, Size};
    use rdom_tui::{Stylesheet, TuiStyle};

    Stylesheet::bare()
        // Outer shell — fills viewport, flex column. `Flex(1)` on both
        // axes is the rdom idiom for "fill available space" (see
        // crates/rdom-tui/examples/app_shell.rs).
        .rule_unchecked(
            ".app",
            TuiStyle::new()
                .direction(Direction::Column)
                .width(Size::Flex(1))
                .height(Size::Flex(1)),
        )
        // Header: 3-row strip with a border, just enough to host the title.
        .rule_unchecked(
            ".app-header",
            TuiStyle::new()
                .height(Size::Fixed(3))
                .border(Border::Single)
                .border_fg(Color::Rgb(70, 80, 100))
                .padding(Padding::symmetric(2, 0)),
        )
        .rule_unchecked(
            ".app-header h1",
            TuiStyle::new().fg(Color::Rgb(200, 220, 255)).bold(true),
        )
        // Body row: sidebar + main, takes the rest of the vertical space.
        .rule_unchecked(
            ".app-body",
            TuiStyle::new()
                .direction(Direction::Row)
                .height(Size::Flex(1))
                .width(Size::Flex(1)),
        )
        // Sidebar: fixed-width vertical strip with its own border.
        .rule_unchecked(
            ".sidebar",
            TuiStyle::new()
                .width(Size::Fixed(28))
                .height(Size::Flex(1))
                .border(Border::Single)
                .border_fg(Color::Rgb(70, 80, 100))
                .padding(Padding::all(1)),
        )
        // Main view: takes the rest of the horizontal space.
        .rule_unchecked(
            ".main",
            TuiStyle::new()
                .width(Size::Flex(1))
                .height(Size::Flex(1))
                .border(Border::Single)
                .border_fg(Color::Rgb(70, 80, 100))
                .padding(Padding::all(1)),
        )
}

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

        // Walk sidebar → nav → ul → li...
        let nav = dom
            .node(handles.sidebar)
            .first_element_child()
            .expect("sidebar has a <nav>");
        assert_eq!(nav.tag_name(), Some("nav"));
        let ul = nav.first_element_child().expect("nav has a <ul>");
        assert_eq!(ul.tag_name(), Some("ul"));
        let li_count = ul
            .child_nodes()
            .filter(|n| n.tag_name() == Some("li"))
            .count();
        assert_eq!(li_count, crate::DEMOS.len(), "one <li> per registered demo");
    }
}
