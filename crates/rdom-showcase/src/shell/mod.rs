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
//! <div class="app-shell">             ← flex column, viewport
//!   <div class="app">                 ← bordered panel (flex: 1)
//!     <header class="app-header">
//!       <h1>rdom showcase</h1>
//!     </header>
//!     <div class="app-body">          ← flex row, takes remaining height
//!       <aside class="sidebar">       ← demo navigator
//!         <nav>
//!           <ul>
//!             <li>Hello World</li>    ← one <li> per registered demo
//!             ...
//!           </ul>
//!         </nav>
//!       </aside>
//!       <main class="main">           ← demo + source panel
//!         <div class="view-content">  ← demo mounts here (flex: 1)
//!         <details class="source-disclosure">…</details>
//!       </main>
//!     </div>
//!   </div>
//!   <footer class="status-bar">       ← 1-row status line (Phase 1a)
//! </div>
//! ```
//!
//! M2 mounts the demo at `DEMOS[0]` into `<main>` statically. M3
//! makes the sidebar interactive (click/keyboard to switch demos)
//! and swaps the mounted demo's stylesheet on every switch
//! (`crate::demo_sheet`).

use rdom_tui::{NodeId, Stylesheet, TuiDom};

use crate::{Category, DEMOS};

mod base_css;
use base_css::BASE_CSS;

/// References to load-bearing nodes the App needs to interact with
/// after the shell is built — e.g., M3 will use `main` to swap
/// demo subtrees on nav clicks.
#[derive(Copy, Clone, Debug)]
pub struct ShellHandles {
    /// The root `<div class="app">` — sits inside the outer
    /// `<div class="app-shell">` flex column that holds both the
    /// bordered panel AND the status bar.
    pub app_root: NodeId,
    /// The view-content container that hosts the active demo OR
    /// the active source view. Caller appends the demo's `build()`
    /// result (Demo mode) or a `<pre>` block carrying MARKUP + CSS
    /// strings (Source mode) here.
    pub main: NodeId,
    /// The sidebar `<aside>` — M3 attaches click listeners here.
    pub sidebar: NodeId,
    /// The `<details class="source-disclosure">` element below
    /// the view-content mount. Its `<summary>` is "Source"; the
    /// body is rebuilt by `mount_demo` to contain the active
    /// demo's MARKUP + CSS strings. UA's `<details>` chrome
    /// handles the toggle.
    pub source_disclosure: NodeId,
    /// The status bar — a `<footer class="status-bar">` SIBLING of
    /// `.app` (not a descendant). Lives outside the bordered panel
    /// so its row doesn't fight `.app`'s `border-collapse`. Phase
    /// 1b populates it with keyboard-shortcut hints; today the
    /// scroll listener writes scroll-position info here when a
    /// descendant of the view-content mount scrolls.
    ///
    /// **Internal structure:** a flex row with two slots —
    /// [`Self::status_bar_hints`] on the left and
    /// [`Self::status_bar_mouse_pos`] on the right. Writers should
    /// target the slot they own; replacing the whole footer would
    /// wipe the sibling slot.
    pub status_bar: NodeId,
    /// Left-aligned slot inside `.status-bar` that holds the
    /// keyboard-hint spans / scroll indicator text. Owned by
    /// `seed_default_hints`, `wire_focus_hints`, and
    /// `wire_scroll_indicator`.
    pub status_bar_hints: NodeId,
    /// Right-aligned slot inside `.status-bar` that holds the live
    /// mouse-position display ("X: 42 Y: 7"). Owned by
    /// `wire_mouse_position_indicator`. Empty until the first
    /// `mousemove` event fires.
    pub status_bar_mouse_pos: NodeId,
}

/// Build the showcase shell under `dom.root()`. Does NOT mount any
/// demo — the caller picks one from [`crate::DEMOS`] and appends
/// its `build()` result to [`ShellHandles::main`].
///
/// Returns the handles to load-bearing nodes; the shell itself is
/// already attached to `dom.root()` when this returns.
pub fn build_shell(dom: &mut TuiDom) -> ShellHandles {
    // <div class="app-shell">                  ← flex column, viewport
    //   <div class="app">…</div>               ← bordered panel (flex: 1)
    //   <footer class="status-bar">…</footer>  ← 1-row status line
    // </div>
    //
    // Wrapping `.app` and the status bar in an outer flex column
    // lets the panel grab the viewport's remaining height while the
    // status bar holds its 1-row strip — and keeps the status bar
    // OUTSIDE `.app`'s border so it doesn't collide with
    // `border-collapse` on the panel's bottom edge.
    let app_shell = dom.create_element("div");
    dom.set_attribute(app_shell, "class", "app-shell").unwrap();
    dom.append_child(dom.root(), app_shell).unwrap();

    // <div class="app">
    let app = dom.create_element("div");
    dom.set_attribute(app, "class", "app").unwrap();
    dom.append_child(app_shell, app).unwrap();

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
    //     <ul role="tree" class="sidebar-tree" autofocus>
    //       <li role="treeitem" aria-expanded="true">Layout
    //         <ul role="group">
    //           <li role="treeitem" data-demo-slug="layout/hello-world">Hello World</li>
    //           ...
    //         </ul>
    //       </li>
    //       <li role="treeitem" aria-expanded="true">Cascade …</li>
    //       ...
    //     </ul>
    //   </nav>
    // </aside>
    //
    // TREE-2: the sidebar IS the native ARIA tree built-in
    // (`<ul role=tree>` / `role=treeitem` / `role=group`). Each
    // distinct `Category` becomes a branch treeitem (a collapsible
    // row — `aria-expanded` presence is what makes the built-in
    // treat it as a branch), and each demo becomes a leaf treeitem
    // carrying its slug in `data-demo-slug`. `runtime::builtins::tree`
    // supplies arrows / Home / End / Right / Left / Enter / Space,
    // the active-descendant cursor, the `│ ├ └` guides, and the
    // `▾`/`▸` chevrons — so the showcase navigates itself with the
    // very primitive it ships. No per-item `tabindex`: the tree
    // container is the single tab stop (implicitly focusable via
    // `role=tree`), and it carries `autofocus` so arrow keys work on
    // the first paint without a Tab press. Activation (pointer or
    // Enter/Space) dispatches a bubbling `click` on the active
    // treeitem, which `wire_sidebar_click` turns into a demo mount.
    let sidebar = dom.create_element("aside");
    dom.set_attribute(sidebar, "class", "sidebar").unwrap();
    let nav = dom.create_element("nav");

    let tree = dom.create_element("ul");
    dom.set_attribute(tree, "role", "tree").unwrap();
    // Chrome-specific class — NOT `nav-tree`. The mounted demo's
    // stylesheet applies to the whole document, so reusing a demo's
    // class name bleeds that demo's rules onto the chrome while it is
    // mounted: the `tree_nav` demo's `.nav-tree { padding: 1 2 }` would
    // inset the whole sidebar nav.
    // The chrome owns the `sidebar-tree` namespace; demos never use it.
    dom.set_attribute(tree, "class", "sidebar-tree").unwrap();
    dom.set_attribute(tree, "autofocus", "").unwrap();

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
        // Branch row: a treeitem with a text label + a child group.
        let branch = dom.create_element("li");
        dom.set_attribute(branch, "role", "treeitem").unwrap();
        dom.set_attribute(branch, "aria-expanded", "true").unwrap();
        let branch_label = dom.create_text_node(cat.title());
        dom.append_child(branch, branch_label).unwrap();

        let group = dom.create_element("ul");
        dom.set_attribute(group, "role", "group").unwrap();
        for demo in DEMOS.iter().filter(|d| d.category() == *cat) {
            let li = dom.create_element("li");
            dom.set_attribute(li, "role", "treeitem").unwrap();
            dom.set_attribute(li, "data-demo-slug", demo.slug())
                .unwrap();
            let title = dom.create_text_node(demo.title());
            dom.append_child(li, title).unwrap();
            dom.append_child(group, li).unwrap();
        }
        dom.append_child(branch, group).unwrap();
        dom.append_child(tree, branch).unwrap();
    }
    dom.append_child(nav, tree).unwrap();
    dom.append_child(sidebar, nav).unwrap();
    dom.append_child(body, sidebar).unwrap();

    // <main class="main">
    //   <div class="view-content"></div>             ← demo mounts here
    //   <details class="source-disclosure">
    //     <summary>Source</summary>
    //     <pre class="source-markup">…</pre>
    //     <pre class="source-css">…</pre>
    //   </details>
    //   <div class="scroll-indicator"></div>
    // </main>
    //
    // Source revealed via native `<details>` disclosure — the
    // browser-faithful pattern for "additional content the
    // reader can opt into." UA handles the toggle (click summary
    // or Enter/Space on focus). No custom tab UI, no view-mode
    // state, no `.active` class flipping.
    let main = dom.create_element("main");
    dom.set_attribute(main, "class", "main").unwrap();

    let view_content = dom.create_element("div");
    dom.set_attribute(view_content, "class", "view-content")
        .unwrap();
    dom.append_child(main, view_content).unwrap();

    // Source disclosure. Body is empty until the first
    // `mount_demo` populates it with the active demo's MARKUP +
    // CSS strings. Closed by default — demo gets the screen real
    // estate unless the author opens it.
    let source_disclosure = dom.create_element("details");
    dom.set_attribute(source_disclosure, "class", "source-disclosure")
        .unwrap();
    let summary = dom.create_element("summary");
    let summary_text = dom.create_text_node("Source");
    dom.append_child(summary, summary_text).unwrap();
    dom.append_child(source_disclosure, summary).unwrap();
    dom.append_child(main, source_disclosure).unwrap();

    dom.append_child(body, main).unwrap();

    // <footer class="status-bar">
    //   <span class="status-bar-hints">…</span>      ← left slot
    //   <span class="status-bar-mouse-pos">…</span>  ← right slot
    // </footer>
    //
    // Sibling of `.app`, outside the bordered panel. Two-slot
    // layout (hints left, mouse position right) so the
    // mouse-position indicator (high-frequency updates, ~60Hz)
    // doesn't clobber the keyboard-hint content owned by the
    // focus / scroll listeners. CSS makes `.status-bar` a flex
    // row; the mouse-pos span has `margin-left: auto` to push it
    // to the right edge.
    let status_bar = dom.create_element("footer");
    dom.set_attribute(status_bar, "class", "status-bar")
        .unwrap();
    dom.append_child(app_shell, status_bar).unwrap();

    // Both slots are `<div>` (block by default) — NOT `<span>`.
    // A `<span>` slot is `display: inline`; its inline-content
    // children (hint spans + space text nodes) try to participate
    // in the PARENT's IFC, but the parent is `.status-bar` with
    // `display: flex` (not an IFC). Result: text nodes between
    // hint spans get dropped and the mouse-pos slot has nowhere
    // to render its text. `<div>` slots are block-level → they
    // establish their own IFC for their inline children, and the
    // flex container positions them as block-level flex items.
    let status_bar_hints = dom.create_element("div");
    dom.set_attribute(status_bar_hints, "class", "status-bar-hints")
        .unwrap();
    dom.append_child(status_bar, status_bar_hints).unwrap();

    let status_bar_mouse_pos = dom.create_element("div");
    dom.set_attribute(status_bar_mouse_pos, "class", "status-bar-mouse-pos")
        .unwrap();
    dom.append_child(status_bar, status_bar_mouse_pos).unwrap();

    crate::status_bar::seed_default_hints(dom, status_bar_hints);

    ShellHandles {
        app_root: app,
        main: view_content,
        sidebar,
        source_disclosure,
        status_bar,
        status_bar_hints,
        status_bar_mouse_pos,
    }
}

/// The shell's base stylesheet — chrome layout (header height,
/// sidebar width, main-view flex), no demo styles. The mounted
/// demo's sheet sits on top of it (`crate::demo_sheet`).
pub fn base_stylesheet() -> Stylesheet {
    rdom_css::from_css(BASE_CSS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_shell_attaches_app_root_under_app_shell_under_dom_root() {
        // Phase 1a: `.app` no longer sits directly under `dom.root()`
        // — it's wrapped in `<div class="app-shell">` so the status
        // bar can be a sibling. Chain: app_root → .app-shell → root.
        let mut dom: TuiDom = TuiDom::new();
        let handles = build_shell(&mut dom);
        let app_shell = dom
            .node(handles.app_root)
            .parent_node()
            .expect("app root has a parent");
        assert_eq!(
            app_shell.get_attribute("class"),
            Some("app-shell"),
            "app_root's parent should be the `.app-shell` wrapper"
        );
        let grand = app_shell.parent_node().expect("app-shell has a parent");
        assert_eq!(grand.id(), dom.root());
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
    fn shell_exposes_source_disclosure() {
        // M7 D1 (refactored): the shell produces a <details
        // class="source-disclosure"> with a <summary>"Source"
        // child. `mount_demo` rebuilds the rest of the body.
        let mut dom: TuiDom = TuiDom::new();
        let handles = build_shell(&mut dom);

        let disclosure = dom.node(handles.source_disclosure);
        assert_eq!(disclosure.tag_name(), Some("details"));
        let summary = disclosure
            .child_nodes()
            .find(|n| n.tag_name() == Some("summary"))
            .expect("<summary> child");
        let summary_text: String = summary
            .child_nodes()
            .filter_map(|c| c.node_value().map(str::to_string))
            .collect();
        assert_eq!(summary_text, "Source");
    }

    /// The `<ul role="tree">` navigator inside the sidebar, if any.
    fn find_nav_tree(dom: &TuiDom, sidebar: NodeId) -> Option<NodeId> {
        fn walk(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
            let n = dom.node(id);
            if n.tag_name() == Some("ul") && n.get_attribute("role") == Some("tree") {
                return Some(id);
            }
            for c in n.child_nodes() {
                if let Some(found) = walk(dom, c.id()) {
                    return Some(found);
                }
            }
            None
        }
        walk(dom, sidebar)
    }

    /// Every `<li role="treeitem" data-demo-slug>` (the demo leaves)
    /// under `root`, in document order.
    fn collect_demo_leaves(dom: &TuiDom, root: NodeId) -> Vec<NodeId> {
        fn walk(dom: &TuiDom, id: NodeId, out: &mut Vec<NodeId>) {
            let n = dom.node(id);
            if n.tag_name() == Some("li")
                && n.get_attribute("role") == Some("treeitem")
                && n.has_attribute("data-demo-slug")
            {
                out.push(id);
            }
            for c in n.child_nodes() {
                walk(dom, c.id(), out);
            }
        }
        let mut out = Vec::new();
        walk(dom, root, &mut out);
        out
    }

    #[test]
    fn sidebar_navigator_is_a_role_tree_with_autofocus() {
        // TREE-2: the sidebar is now the native ARIA tree built-in.
        // The `[role=tree]` container is the single tab stop (it's
        // implicitly focusable) and carries `autofocus` so the
        // keyboard navigation is live on first paint — no Tab
        // required. The bespoke `wire_sidebar_keys` roving-focus
        // handler is gone; `runtime::builtins::tree` supplies the
        // arrows / Home / End / Enter / Space + active-descendant
        // cursor.
        let mut dom: TuiDom = TuiDom::new();
        let handles = build_shell(&mut dom);

        let tree = find_nav_tree(&dom, handles.sidebar).expect("sidebar has a <ul role=tree>");
        let node = dom.node(tree);
        assert_eq!(node.tag_name(), Some("ul"));
        assert_eq!(node.get_attribute("role"), Some("tree"));
        assert!(
            node.has_attribute("autofocus"),
            "the tree container carries autofocus so arrow keys work on boot"
        );
    }

    #[test]
    fn sidebar_has_one_treeitem_leaf_per_registered_demo() {
        let mut dom: TuiDom = TuiDom::new();
        let handles = build_shell(&mut dom);
        let tree = find_nav_tree(&dom, handles.sidebar).expect("sidebar has a <ul role=tree>");

        let leaves = collect_demo_leaves(&dom, tree);
        assert_eq!(
            leaves.len(),
            crate::DEMOS.len(),
            "one <li role=treeitem data-demo-slug> per registered demo"
        );
    }

    #[test]
    fn each_demo_leaf_carries_role_treeitem_and_data_demo_slug() {
        // Click handler reads `data-demo-slug` to identify which
        // demo to mount; the tree built-in keys behavior off
        // `role=treeitem`. Both must be present on every leaf, and
        // the set of slugs must exactly cover the registry.
        let mut dom: TuiDom = TuiDom::new();
        let handles = build_shell(&mut dom);
        let tree = find_nav_tree(&dom, handles.sidebar).unwrap();

        let mut slugs: Vec<String> = Vec::new();
        for leaf in collect_demo_leaves(&dom, tree) {
            let node = dom.node(leaf);
            assert_eq!(node.get_attribute("role"), Some("treeitem"));
            slugs.push(node.get_attribute("data-demo-slug").unwrap().to_string());
        }

        let expected: Vec<&'static str> = crate::DEMOS.iter().map(|d| d.slug()).collect();
        assert_eq!(slugs.len(), expected.len());
        for slug in expected {
            assert!(
                slugs.iter().any(|s| s == slug),
                "demo slug {slug:?} missing from sidebar tree"
            );
        }
    }

    #[test]
    fn categories_are_branch_treeitems_holding_a_role_group() {
        // Each distinct category becomes a branch: a
        // `<li role=treeitem aria-expanded=true>` whose label text
        // is the category title, containing a `<ul role=group>` of
        // that category's demo leaves. `aria-expanded` presence (not
        // child count) is what makes the tree built-in treat the row
        // as a collapsible branch.
        let mut dom: TuiDom = TuiDom::new();
        let handles = build_shell(&mut dom);
        let tree = find_nav_tree(&dom, handles.sidebar).unwrap();

        let mut distinct: Vec<Category> = Vec::new();
        for d in crate::DEMOS {
            if !distinct.contains(&d.category()) {
                distinct.push(d.category());
            }
        }

        let branches: Vec<_> = dom
            .node(tree)
            .child_nodes()
            .filter(|n| n.tag_name() == Some("li") && n.get_attribute("role") == Some("treeitem"))
            .collect();
        assert_eq!(
            branches.len(),
            distinct.len(),
            "one top-level branch treeitem per distinct category"
        );

        for (branch, cat) in branches.iter().zip(distinct.iter()) {
            assert_eq!(
                branch.get_attribute("aria-expanded"),
                Some("true"),
                "category branch starts expanded"
            );
            assert!(
                branch.get_attribute("data-demo-slug").is_none(),
                "category branch carries no demo slug — Enter/click on it only toggles"
            );
            let group = branch
                .child_nodes()
                .find(|n| n.get_attribute("role") == Some("group"))
                .expect("branch holds a <ul role=group>");
            assert_eq!(group.tag_name(), Some("ul"));
            // Every child of the group is a demo leaf in this category.
            for leaf in group.child_nodes().filter(|n| n.tag_name() == Some("li")) {
                let slug = leaf.get_attribute("data-demo-slug").expect("leaf has slug");
                let demo = crate::DEMOS
                    .iter()
                    .find(|d| d.slug() == slug)
                    .expect("slug resolves to a demo");
                assert_eq!(demo.category(), *cat, "leaf grouped under its own category");
            }
        }
    }
}
