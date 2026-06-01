//! ARIA tree navigation — a `<ul role=tree>` with guide lines,
//! keyboard navigation, and a **fake lazy-loaded branch**.
//!
//! Built entirely from the web-faithful ARIA tree pattern
//! (`role="tree"` / `"treeitem"` / `"group"` + `aria-expanded`).
//! `rdom_tui`'s `runtime::builtins::tree` supplies the keyboard
//! (Arrows / Home / End / Enter / Space), the active-descendant
//! cursor, the `│ ├ └` guide paint, and the `▾`/`▸` chevrons.
//!
//! The "Nodes" branch starts collapsed with no children. Expanding
//! it (Right arrow or clicking the twisty) sets `aria-busy`, shows a
//! "Loading…" placeholder, and schedules a 2-second `set_timeout`
//! that swaps in the real children and clears `aria-busy`. This is
//! the canonical async-children pattern: the substrate fires
//! `toggle` on expand; loading is pure app-level DOM mutation.

use std::cell::Cell;
use std::io;
use std::rc::Rc;

use rdom_tui::runtime::timers::{TimerCtx, TuiTimers};
use rdom_tui::{App, ListenerOptions, NodeId, Stylesheet, TuiDom};

use crate::{Category, Demo, Source};

pub const MARKUP: &str = r#"<ul role="tree" class="nav-tree">
  <li role="treeitem" aria-expanded="true">Cluster
    <ul role="group">
      <li role="treeitem">Pods</li>
      <li role="treeitem">Services</li>
      <li role="treeitem" aria-expanded="false">Nodes</li> <!-- lazy: children load after 2s -->
    </ul>
  </li>
  <li role="treeitem">Settings</li>
</ul>"#;

pub const CSS: &str = r#"
.nav-tree {
  flex: 1;
  padding: 1 2;
}
/* Retheme the guide lines via the treeitem border-color. */
.nav-tree[role=tree] [role=treeitem] {
  border-color: rgb(90, 100, 110);
}
/* Loading rows read as muted while aria-busy is set on the parent. */
.nav-tree [role=treeitem][aria-busy=true] {
  color: rgb(150, 160, 170);
}
"#;

/// Create a `<li role=treeitem>` with `label`, optional
/// `aria-expanded`, appended to `parent`.
fn item(dom: &mut TuiDom, parent: NodeId, label: &str, expanded: Option<bool>) -> NodeId {
    let li = dom.create_element("li");
    dom.set_attribute(li, "role", "treeitem").unwrap();
    if let Some(open) = expanded {
        dom.set_attribute(li, "aria-expanded", if open { "true" } else { "false" })
            .unwrap();
    }
    let t = dom.create_text_node(label);
    dom.append_child(li, t).unwrap();
    dom.append_child(parent, li).unwrap();
    li
}

fn group(dom: &mut TuiDom, parent: NodeId) -> NodeId {
    let g = dom.create_element("ul");
    dom.set_attribute(g, "role", "group").unwrap();
    dom.append_child(parent, g).unwrap();
    g
}

pub fn build(dom: &mut TuiDom) -> NodeId {
    let tree = dom.create_element("ul");
    dom.set_attribute(tree, "role", "tree").unwrap();
    dom.set_attribute(tree, "class", "nav-tree").unwrap();

    let cluster = item(dom, tree, "Cluster", Some(true));
    let cluster_group = group(dom, cluster);
    item(dom, cluster_group, "Pods", None);
    item(dom, cluster_group, "Services", None);

    // The lazy branch: a collapsed branch with no children yet.
    let nodes = item(dom, cluster_group, "Nodes", Some(false));

    item(dom, tree, "Settings", None);

    // Wire the lazy load. `toggle` fires on expand/collapse (non-
    // bubbling, so we listen on the branch itself). On the first
    // open we fake a 2s fetch.
    let loaded = Rc::new(Cell::new(false));
    dom.add_event_listener(nodes, "toggle", ListenerOptions::default(), move |ctx| {
        // Only act on expand, and only once.
        if ctx.dom.node(nodes).get_attribute("aria-expanded") != Some("true") {
            return;
        }
        if loaded.get() {
            return;
        }
        loaded.set(true);

        // Show the loading affordance: aria-busy + a placeholder row.
        let _ = ctx.dom.set_attribute(nodes, "aria-busy", "true");
        let lazy_group = {
            let g = ctx.dom.create_element("ul");
            let _ = ctx.dom.set_attribute(g, "role", "group");
            let _ = ctx.dom.append_child(nodes, g);
            g
        };
        let placeholder = ctx.dom.create_element("li");
        let _ = ctx.dom.set_attribute(placeholder, "role", "treeitem");
        let pt = ctx.dom.create_text_node("Loading…");
        let _ = ctx.dom.append_child(placeholder, pt);
        let _ = ctx.dom.append_child(lazy_group, placeholder);

        // Fake fetch — swap in the real children after 2 seconds.
        ctx.set_timeout(
            move |tick: &mut TimerCtx<'_>| {
                let _ = tick.dom.clear_children(lazy_group);
                for name in ["node-1", "node-2", "node-3"] {
                    let li = tick.dom.create_element("li");
                    let _ = tick.dom.set_attribute(li, "role", "treeitem");
                    let t = tick.dom.create_text_node(name);
                    let _ = tick.dom.append_child(li, t);
                    let _ = tick.dom.append_child(lazy_group, li);
                }
                let _ = tick.dom.remove_attribute(nodes, "aria-busy");
            },
            2000,
        );
    })
    .unwrap();

    tree
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

pub struct TreeNav;

impl Demo for TreeNav {
    fn slug(&self) -> &'static str {
        "built-ins/tree-nav"
    }

    fn title(&self) -> &'static str {
        "ARIA tree (lazy load)"
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
