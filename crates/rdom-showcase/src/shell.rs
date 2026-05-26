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
//! and wires per-demo stylesheet push/pop via M1's multi-slot
//! stylesheet API.

use rdom_tui::{NodeId, Stylesheet, TuiDom};

use crate::{Category, DEMOS};

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
    /// 1b will populate it with keyboard-shortcut hints; today the
    /// scroll listener writes scroll-position info here when a
    /// descendant of the view-content mount scrolls.
    pub status_bar: NodeId,
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
            // The first `<li>` in the registry gets `autofocus` so
            // the showcase boots with a keyboard-navigable element
            // already focused. Without it, the app starts with
            // nothing focused (web-faithful) and the user has to
            // press Tab once before arrow keys do anything — easy
            // to mistake for "the keyboard nav is broken." rdom's
            // runtime/autofocus module picks the first eligible
            // `[autofocus]` element on mount.
            if demo.slug() == DEMOS[0].slug() {
                dom.set_attribute(li, "autofocus", "").unwrap();
            }
            let title = dom.create_text_node(demo.title());
            dom.append_child(li, title).unwrap();
            dom.append_child(ul, li).unwrap();
        }
        dom.append_child(details, ul).unwrap();
        dom.append_child(nav, details).unwrap();
    }
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

    // <footer class="status-bar"></footer> — sibling of `.app`,
    // outside the bordered panel. Seeded with the global keyboard
    // hints; `status_bar::wire_focus_hints` refreshes them on
    // focus change; the scroll listener takes over transiently
    // during active scrolling.
    let status_bar = dom.create_element("footer");
    dom.set_attribute(status_bar, "class", "status-bar")
        .unwrap();
    dom.append_child(app_shell, status_bar).unwrap();
    crate::status_bar::seed_default_hints(dom, status_bar);

    ShellHandles {
        app_root: app,
        main: view_content,
        sidebar,
        source_disclosure,
        status_bar,
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
///
/// **Fitting-pane chain.** `.app`, `.app-body`, and `.main` each
/// declare `min-width: 0` / `min-height: 0`. This is a web-faithful
/// app-shell pattern, not a workaround: by default flex items
/// can't shrink below their intrinsic content size (CSS Flexbox
/// §4.5), so a content-heavy demo would force its ancestor chain
/// to grow past the viewport. The `min-*: 0` opt-in says "I am a
/// fitting pane — clip me to the viewport, don't grow me to my
/// children's content." Real CSS authors use the same pattern in
/// browser app shells; the substrate fix (`M5-MIN-CONTENT-1`)
/// adopted the contract, the chrome opts into the override for
/// every container in the fitting chain. If a future structural
/// change adds another container between `.app` and `.main`, that
/// container needs the same opt-in or content overflow will
/// reappear.
const BASE_CSS: &str = r#"
/* `.app-shell` is the outer flex column that holds the bordered
 * `.app` panel and the status bar below it. `.app` flexes to fill
 * remaining viewport height; the status bar holds its intrinsic
 * 1-row footprint. `min-*: 0` keeps the panel shrinkable past its
 * children's intrinsic content size (same fitting-pane pattern
 * used inside `.app`).
 */
.app-shell {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  min-height: 0;
}

.app {
  /* BFC-1 Phase 4.3: with the block/flex dispatch wired,
   * containers that want flex distribution (flex-grow, flex-
   * direction) MUST declare `display: flex`. Default `<div>` is
   * Block flow per UA + CSS3 Display Module. */
  display: flex;
  flex: 1;
  flex-direction: column;
  border: solid;
  border-color: rgb(45, 47, 49);
  border-collapse: collapse;
  /* App shell fits the viewport — opt out of intrinsic-min on
   * both axes so the children's content can scroll rather than
   * the shell ballooning to fit them. Mirrors `.app-body` and
   * `.main`. */
  min-width: 0;
  min-height: 0;
}

.app-header {
  height: 3;
  border: solid;
  border-color: rgb(45, 47, 49);
  padding: 0 1;
}
.app-header h1 {
  color: rgb(200, 220, 255);
  font-weight: bold;
}

.app-body {
  display: flex;
  flex: 1;
  flex-direction: row;
  /* Body is an app-shell pane — it should track the available
   * height, not balloon to the sum of its children's intrinsic
   * heights. Web-faithful: real CSS authors use `min-height: 0`
   * on flex items they want shrinkable past their content. */
  min-height: 0;
}

.sidebar {
  width: 28;
  height: 100%;
  border: solid;
  border-color: rgb(45, 47, 49);
  padding: 1;
  /* The nav is taller than the viewport on small terminals;
   * scroll instead of clipping. The substrate floors each item
   * at its intrinsic content height (CSS Flexbox §4.5 min-*:
   * auto), so nothing squishes regardless. */
  overflow-y: auto;
}
.sidebar h2 {
  color: rgb(150, 170, 200);
  font-weight: bold;
}

.main {
  display: flex;
  flex: 1;
  flex-direction: column;
  border: solid;
  border-color: rgb(45, 47, 49);
  /* Opt into responsive shrink past intrinsic content size — the
   * source disclosure can hold lines wider than the terminal, but
   * `<main>` should still fit the row alongside the sidebar.
   * `min-height: 0` lets `<main>` fit the available height of
   * `.app-body` so the demo can fill the viewport vertically.
   * Web-faithful: real CSS authors use `min-width: 0` /
   * `min-height: 0` on flex items they want shrinkable past
   * their content. */
  min-width: 0;
  min-height: 0;
}

.main .view-content {
  flex: 1;
}

/* Source disclosure. Two states:
 *
 * - CLOSED (default) — intrinsic height = 1 row summary +
 *   1 row `border-top: solid` = 2 outer rows. The demo gets all
 *   the remaining vertical space inside `<main>`.
 * - OPEN — fixed 12 outer rows = 1 border-top + 11-row content
 *   area, with `overflow: auto` so long source scrolls inside the
 *   panel rather than pushing chrome around. Predictable split
 *   regardless of demo source length.
 *
 * `[open]` attribute selector targets only the open state, leaving
 * the closed state at its intrinsic. The substrate's UA rule
 * `details:not([open]) > *:not(summary) { display: none }` hides
 * the body when closed, so there's nothing to scroll when closed.
 */
/* Horizontal padding lives on the container, NOT on each child.
 * Web-idiom: container provides the inset, every block child
 * (<summary>, <h3>, <pre>, plus anything added later) inherits the
 * same content-box left edge automatically. Per-child padding on
 * <summary> + <pre> with nothing on <h3> previously left the
 * "Markup" / "CSS" labels flush against the panel border. */
.main .source-disclosure {
  border-top: solid;
  border-color: rgb(45, 47, 49);
  padding: 0 1;
}
.main .source-disclosure[open] {
  height: 12;
  /* `overflow: auto` reserves a scrollbar gutter only when the
   * scrollbar actually shows (CSS-default `scrollbar-gutter: auto`
   * post-`CHROME-SCROLL-GUTTER-DEFAULT-1`), so short source doesn't
   * lose a content column. */
  overflow: auto;
}
.main .source-disclosure summary {
  color: rgb(180, 200, 230);
  font-weight: bold;
}
.main .source-disclosure h3 {
  color: rgb(180, 200, 230);
}
.main .source-disclosure pre {
  color: rgb(200, 210, 230);
}

/* Status bar — sibling of `.app` under `.app-shell`. Lives outside
 * the bordered panel so its row doesn't fight `.app`'s
 * `border-collapse`. Empty by default; the scroll listener writes
 * scroll-position info here when a descendant of view-content
 * scrolls. Phase 1b will seed default keyboard-shortcut hints.
 *
 * `min-height: 1` keeps the bar visible when empty — same reason
 * as the prior `.scroll-indicator` rule (substrate's auto-min
 * floor would otherwise let it collapse to zero under tight
 * viewport pressure).
 */
.status-bar {
  height: 1;
  min-height: 1;
  padding: 0 1;
  color: rgb(150, 170, 200);
}
/* Keyboard-hint spans inside the status bar. Two-tone styling
 * (key vs label) mirrors the conventional terminal-status-line
 * pattern: the *key* is what the user has to press, so it's bright
 * + bold; the *label* describes what pressing it does, so it's
 * muted to read as supporting prose.
 */
.status-bar .key {
  color: rgb(220, 230, 255);
  font-weight: bold;
}
.status-bar .label {
  color: rgb(140, 160, 190);
}
.status-bar .sep {
  color: rgb(80, 90, 110);
}
"#;

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
