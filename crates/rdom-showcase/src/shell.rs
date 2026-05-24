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

use crate::{Category, DEMOS};

/// References to load-bearing nodes the App needs to interact with
/// after the shell is built — e.g., M3 will use `main` to swap
/// demo subtrees on nav clicks.
#[derive(Copy, Clone, Debug)]
pub struct ShellHandles {
    /// The root `<div class="app">` — append to `dom.root()` (the
    /// shell does this internally, exposed for tests that want to
    /// re-query the tree).
    pub app_root: NodeId,
    /// The view-content container that hosts the active demo OR
    /// the active source view. Caller appends the demo's `build()`
    /// result (Demo mode) or a `<pre>` block carrying MARKUP + CSS
    /// strings (Source mode) here.
    pub main: NodeId,
    /// The sidebar `<aside>` — M3 attaches click listeners here.
    pub sidebar: NodeId,
    /// The `<nav class="view-tabs">` container holding the
    /// Demo / Source `<button>`s. M7 D1 attaches click listeners
    /// here to switch view mode.
    pub view_tabs: NodeId,
    /// The scroll-position indicator at the bottom of `<main>`.
    /// M7 D3 updates this element's text on every `scroll` event
    /// fired by a descendant of the view-content mount.
    pub scroll_indicator: NodeId,
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
    //   <nav>
    //     <details open><summary>Layout</summary>
    //       <ul>
    //         <li data-demo-slug="layout/hello-world" tabindex="0">Hello World</li>
    //         ...
    //       </ul>
    //     </details>
    //     <details open><summary>Cascade</summary>...</details>
    //     ...
    //   </nav>
    // </aside>
    //
    // Demos grouped by `Category` enum. Each category renders as
    // a `<details>` with its title in `<summary>` — UA gives us
    // the disclosure triangle for free. `<li>`s carry the demo's
    // slug in a `data-demo-slug` attribute so the click handler
    // (M3 D4) can identify which demo to mount. `tabindex="0"`
    // makes them keyboard-focusable (M3 D5).
    let sidebar = dom.create_element("aside");
    dom.set_attribute(sidebar, "class", "sidebar").unwrap();
    let nav = dom.create_element("nav");

    // Group demos by category. Iterates the registry in declaration
    // order, which is also the order categories appear in the
    // sidebar — first demo's category goes first, etc.
    let mut seen_categories: Vec<Category> = Vec::new();
    for demo in DEMOS {
        if !seen_categories.contains(&demo.category()) {
            seen_categories.push(demo.category());
        }
    }
    for cat in &seen_categories {
        let details = dom.create_element("details");
        dom.set_attribute(details, "open", "").unwrap();
        let summary = dom.create_element("summary");
        let summary_text = dom.create_text_node(cat.title());
        dom.append_child(summary, summary_text).unwrap();
        dom.append_child(details, summary).unwrap();

        let ul = dom.create_element("ul");
        for demo in DEMOS.iter().filter(|d| d.category() == *cat) {
            let li = dom.create_element("li");
            dom.set_attribute(li, "data-demo-slug", demo.slug())
                .unwrap();
            dom.set_attribute(li, "tabindex", "0").unwrap();
            let title = dom.create_text_node(demo.title());
            dom.append_child(li, title).unwrap();
            dom.append_child(ul, li).unwrap();
        }
        dom.append_child(details, ul).unwrap();
        dom.append_child(nav, details).unwrap();
    }
    dom.append_child(sidebar, nav).unwrap();
    dom.append_child(body, sidebar).unwrap();

    // <main class="main"> — wraps the view tabs + the content mount.
    //
    // ```
    // <main class="main">
    //   <nav class="view-tabs">
    //     <button class="view-tab active" data-view="demo">Demo</button>
    //     <button class="view-tab" data-view="source">Source</button>
    //   </nav>
    //   <div class="view-content"></div>  ← where demos mount
    // </main>
    // ```
    //
    // Tabs in M7 D1 toggle the main view between the live demo
    // and its source (markup + CSS strings). The `<button>` carries
    // `data-view` so the click handler can switch without per-button
    // listeners.
    let main = dom.create_element("main");
    dom.set_attribute(main, "class", "main").unwrap();

    let view_tabs = dom.create_element("nav");
    dom.set_attribute(view_tabs, "class", "view-tabs").unwrap();
    for (label, view) in [("Demo", "demo"), ("Source", "source")] {
        let tab = dom.create_element("button");
        dom.set_attribute(tab, "class", "view-tab").unwrap();
        dom.set_attribute(tab, "data-view", view).unwrap();
        let tab_text = dom.create_text_node(label);
        dom.append_child(tab, tab_text).unwrap();
        dom.append_child(view_tabs, tab).unwrap();
    }
    dom.append_child(main, view_tabs).unwrap();

    let view_content = dom.create_element("div");
    dom.set_attribute(view_content, "class", "view-content")
        .unwrap();
    dom.append_child(main, view_content).unwrap();

    // <div class="scroll-indicator"></div> — status row at the
    // bottom of <main>. Empty when the active demo has no
    // scrollable element; populated by the `scroll` listener
    // (M7 D3) with "Row N/M — P%" text.
    let scroll_indicator = dom.create_element("div");
    dom.set_attribute(scroll_indicator, "class", "scroll-indicator")
        .unwrap();
    dom.append_child(main, scroll_indicator).unwrap();
    dom.append_child(body, main).unwrap();

    ShellHandles {
        app_root: app,
        main: view_content,
        sidebar,
        view_tabs,
        scroll_indicator,
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
///
/// **`.main` has NO padding by design.** The chrome's job is to
/// define the panel container — its borders, position, and
/// stacking relative to the sidebar. Choosing the content's
/// visual inset belongs to the demo, not the chrome. This lets
/// canvas-style or full-bleed demos paint to every cell of the
/// panel without fighting an injected panel margin. Text demos
/// that want content padding set their own `padding: 1` (or
/// other value) on their content root.
///
/// `.sidebar` keeps its `padding: 1` because the sidebar IS the
/// showcase's own nav UI — not a demo — and the chrome owns its
/// look.
const BASE_CSS: &str = r#"
.app {
  flex: 1;
  flex-direction: column;
  border: solid;
  border-color: rgb(70, 80, 100);
  border-collapse: collapse;
}

.app-header {
  height: 3;
  border: solid;
  border-color: rgb(70, 80, 100);
  padding: 0 1;
}
.app-header h1 {
  color: rgb(200, 220, 255);
  font-weight: bold;
}

.app-body {
  flex: 1;
  flex-direction: row;
}

.sidebar {
  width: 28;
  height: 100%;
  border: solid;
  border-color: rgb(70, 80, 100);
  padding: 1;
}
.sidebar h2 {
  color: rgb(150, 170, 200);
  font-weight: bold;
}

.main {
  flex: 1;
  flex-direction: column;
  border: solid;
  border-color: rgb(70, 80, 100);
}

/* Tab strip across the top of the main view. Two buttons:
 * "Demo" (active by default) and "Source". The .active class is
 * toggled by the view-tab click handler at runtime.
 */
.main .view-tabs {
  flex-direction: row;
  height: 1;
  flex-shrink: 0;
  border-bottom: solid;
  border-color: rgb(70, 80, 100);
}
.main .view-tabs .view-tab {
  flex-shrink: 0;
  padding: 0 2;
  color: rgb(150, 170, 200);
}
.main .view-tabs .view-tab.active {
  color: rgb(220, 230, 255);
  font-weight: bold;
}

.main .view-content {
  flex: 1;
}

/* Scroll-position indicator at the bottom of <main>. Empty by
 * default — the listener writes "Row N/M — P%" text when a
 * descendant scrolls. The 1-cell height + flex-shrink: 0 keeps
 * the indicator visible without pushing demo content around.
 */
.main .scroll-indicator {
  height: 1;
  flex-shrink: 0;
  border-top: solid;
  border-color: rgb(70, 80, 100);
  padding: 0 1;
  color: rgb(150, 170, 200);
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
        // view-content (handles.main) → <main> → app-body → app-root.
        // The extra hop is from M7 D1: <main> now holds the view-tabs +
        // the view-content; handles.main points at the inner mount.
        let main_el = dom
            .node(handles.main)
            .parent_node()
            .expect("view-content has a parent")
            .id();
        let body = dom
            .node(main_el)
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
    fn shell_exposes_view_tabs() {
        // M7 D1: the shell now produces a <nav class="view-tabs">
        // with two <button data-view> children.
        let mut dom: TuiDom = TuiDom::new();
        let handles = build_shell(&mut dom);

        let tabs_node = dom.node(handles.view_tabs);
        assert_eq!(tabs_node.tag_name(), Some("nav"));
        let buttons: Vec<_> = tabs_node
            .child_nodes()
            .filter(|n| n.tag_name() == Some("button"))
            .collect();
        assert_eq!(buttons.len(), 2, "two tabs: Demo + Source");
        let views: Vec<&str> = buttons
            .iter()
            .filter_map(|b| b.get_attribute("data-view"))
            .collect();
        assert_eq!(views, vec!["demo", "source"]);
    }

    #[test]
    fn sidebar_contains_one_li_per_registered_demo() {
        let mut dom: TuiDom = TuiDom::new();
        let handles = build_shell(&mut dom);

        // Sidebar → <nav> → <details>* → <ul> → <li>*. Count
        // every <li> across every category.
        let nav = dom
            .node(handles.sidebar)
            .child_nodes()
            .find(|n| n.tag_name() == Some("nav"))
            .expect("sidebar has a <nav>");

        let mut li_count = 0usize;
        for details in nav
            .child_nodes()
            .filter(|n| n.tag_name() == Some("details"))
        {
            let ul = details
                .child_nodes()
                .find(|n| n.tag_name() == Some("ul"))
                .expect("each <details> has a <ul>");
            li_count += ul
                .child_nodes()
                .filter(|n| n.tag_name() == Some("li"))
                .count();
        }
        assert_eq!(
            li_count,
            crate::DEMOS.len(),
            "one <li> per registered demo across all category <details>"
        );
    }

    #[test]
    fn each_sidebar_li_carries_data_demo_slug() {
        // Click handler (M3 D4) reads this attribute to identify
        // which demo to mount. Pinning the contract.
        let mut dom: TuiDom = TuiDom::new();
        let handles = build_shell(&mut dom);
        let nav = dom
            .node(handles.sidebar)
            .child_nodes()
            .find(|n| n.tag_name() == Some("nav"))
            .unwrap();

        let mut slugs: Vec<String> = Vec::new();
        for details in nav
            .child_nodes()
            .filter(|n| n.tag_name() == Some("details"))
        {
            let ul = details
                .child_nodes()
                .find(|n| n.tag_name() == Some("ul"))
                .unwrap();
            for li in ul.child_nodes().filter(|n| n.tag_name() == Some("li")) {
                let slug = li
                    .get_attribute("data-demo-slug")
                    .map(str::to_string)
                    .expect("<li> has data-demo-slug");
                slugs.push(slug);
            }
        }

        let expected: Vec<&'static str> = crate::DEMOS.iter().map(|d| d.slug()).collect();
        assert_eq!(slugs.len(), expected.len());
        for slug in expected {
            assert!(
                slugs.iter().any(|s| s == slug),
                "demo slug {slug:?} missing from sidebar"
            );
        }
    }
}
