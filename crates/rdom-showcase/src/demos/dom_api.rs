//! DOM accessor walkthrough — exercises every M4 accessor family
//! and prints the result as a static report.
//!
//! Non-interactive. Visually a multi-line `<pre>` block per
//! theme; conceptually a living documentation reference + smoke
//! test that the substrate's accessor surface still works
//! end-to-end.
//!
//! Three themes:
//!
//! 1. **form-edit** — `<input>` / `<textarea>` / `<select>`
//!    reads + writes via the per-tag accessors.
//! 2. **tree-walk** — document-level hit-test accessors
//!    (`element_from_point`, …) against a laid-out tree.
//! 3. **cssom** — `el.style()` reads, `el.style_mut()` writes,
//!    `cssText` round-trips, camelCase aliases.

use std::fmt::Write;
use std::io;

use rdom_tui::runtime::builtins::input;
use rdom_tui::{NodeId, Stylesheet, TuiAccessors, TuiAccessorsMut, TuiDocAccessors, TuiDom};

use crate::{Category, Demo, Source};

pub const MARKUP: &str = r#"<div class="dom-api-demo">
  <h1>rdom DOM API demo</h1>
  <pre class="report">…three themed walk-throughs…</pre>
</div>"#;

pub const CSS: &str = r#"
.dom-api-demo {
  flex: 1;
  flex-direction: column;
  padding: 1 2;
  gap: 1;
}
.dom-api-demo h1 {
  height: 1;
  color: rgb(180, 220, 255);
  font-weight: bold;
}
.dom-api-demo .report {
  flex: 1;
  display: block;
}
"#;

pub fn build(dom: &mut TuiDom) -> NodeId {
    let root = dom.create_element("div");
    dom.set_attribute(root, "class", "dom-api-demo").unwrap();

    let h1 = dom.create_element("h1");
    let h1_text = dom.create_text_node("rdom DOM API demo");
    dom.append_child(h1, h1_text).unwrap();
    dom.append_child(root, h1).unwrap();

    let pre = dom.create_element("pre");
    dom.set_attribute(pre, "class", "report").unwrap();
    let report = run_walkthroughs();
    let pre_text = dom.create_text_node(&report);
    dom.append_child(pre, pre_text).unwrap();
    dom.append_child(root, pre).unwrap();

    root
}

pub fn stylesheet() -> Stylesheet {
    rdom_css::from_css(CSS)
}

pub fn run_standalone() -> io::Result<()> {
    // Standalone mode: print the report to stdout (matches the
    // original example's behavior). The TUI App is not used —
    // this example is informational, not interactive.
    println!("=== rdom DOM API demo ===\n");
    println!("{}", run_walkthroughs());
    Ok(())
}

/// Run the three themed walk-throughs and return the concatenated
/// report as a string. Used by both `build` (renders into a `<pre>`)
/// and `run_standalone` (prints to stdout).
fn run_walkthroughs() -> String {
    let mut out = String::new();
    write_form_edit(&mut out);
    out.push('\n');
    write_tree_walk(&mut out);
    out.push('\n');
    write_cssom(&mut out);
    out
}

fn write_form_edit(out: &mut String) {
    let _ = writeln!(out, "─── form-edit ──────────────────────────────────");
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();

    let form = dom.create_element("form");
    let input = dom.create_element("input");
    dom.set_attribute(input, "name", "email").unwrap();
    dom.set_attribute(input, "value", "a@b.c").unwrap();
    let textarea = dom.create_element("textarea");
    dom.set_attribute(textarea, "name", "bio").unwrap();
    let t = dom.create_text_node("Hello");
    dom.append_child(textarea, t).unwrap();
    let select = dom.create_element("select");
    dom.set_attribute(select, "name", "role").unwrap();
    let opt_admin = dom.create_element("option");
    dom.set_attribute(opt_admin, "value", "admin").unwrap();
    let opt_user = dom.create_element("option");
    dom.set_attribute(opt_user, "value", "user").unwrap();
    dom.set_attribute(opt_user, "selected", "").unwrap();
    dom.append_child(select, opt_admin).unwrap();
    dom.append_child(select, opt_user).unwrap();
    let button = dom.create_element("button");
    dom.set_attribute(button, "type", "submit").unwrap();
    dom.append_child(form, input).unwrap();
    dom.append_child(form, textarea).unwrap();
    dom.append_child(form, select).unwrap();
    dom.append_child(form, button).unwrap();
    dom.append_child(root, form).unwrap();

    input::seed_all(&mut dom);

    let _ = writeln!(
        out,
        "  smart input value     → {:?}",
        dom.node(input).value()
    );
    let _ = writeln!(
        out,
        "  narrow input_value    → {:?}",
        dom.node(input).input_value()
    );
    let _ = writeln!(
        out,
        "  input_name            → {:?}",
        dom.node(input).input_name()
    );
    let _ = writeln!(
        out,
        "  textarea_value        → {:?}",
        dom.node(textarea).textarea_value()
    );
    let _ = writeln!(
        out,
        "  select_value          → {:?}",
        dom.node(select).select_value()
    );
    let _ = writeln!(
        out,
        "  select_selected_index → {:?}",
        dom.node(select).select_selected_index()
    );
    let _ = writeln!(
        out,
        "  option_value (user)   → {:?}",
        dom.node(opt_user).option_value()
    );

    let elts = dom.node(form).form_elements().unwrap();
    let _ = writeln!(out, "  form_elements count   → {}", elts.len());
    let _ = writeln!(
        out,
        "  form_length           → {:?}",
        dom.node(form).form_length()
    );

    dom.node_mut(input).set_value("new@addr.test").unwrap();
    let _ = writeln!(
        out,
        "  after set_value       → {:?}",
        dom.node(input).input_value()
    );

    let prevented = dom
        .node_mut(form)
        .form_request_submit(Some(button))
        .unwrap();
    let _ = writeln!(
        out,
        "  form_request_submit   → prevented={prevented}, submitter={button:?}"
    );
}

fn write_tree_walk(out: &mut String) {
    use rdom_tui::prelude::*;
    let _ = writeln!(out, "─── tree-walk ──────────────────────────────────");
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let outer = dom.create_element("div");
    dom.set_attribute(outer, "id", "outer").unwrap();
    let inner = dom.create_element("p");
    dom.add_class(inner, "para").unwrap();
    let text = dom.create_text_node("Hello, world");
    dom.append_child(inner, text).unwrap();
    dom.append_child(outer, inner).unwrap();
    dom.append_child(root, outer).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .width(Size::Fixed(40))
            .height(Size::Fixed(5)),
    );
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 24));

    let _ = writeln!(
        out,
        "  element_from_point(2,1)   → {:?}",
        dom.element_from_point(2, 1).map(|n| n.id())
    );
    let path: Vec<rdom_tui::NodeId> = dom
        .elements_from_point(2, 1)
        .iter()
        .map(|n| n.id())
        .collect();
    let _ = writeln!(out, "  elements_from_point depth → {}", path.len());
    let _ = writeln!(
        out,
        "  element_from_point(200,200) → {:?} (off-screen)",
        dom.element_from_point(200, 200).map(|n| n.id())
    );

    let p_node = dom.node(inner);
    let _ = writeln!(out, "  inner.is_connected      → {}", p_node.is_connected());
    let _ = writeln!(
        out,
        "  closest(\"div\")          → {:?}",
        p_node.closest("div").map(|n| n.id())
    );
    let _ = writeln!(
        out,
        "  matches(\"p.para\")       → {}",
        p_node.matches("p.para")
    );
    let _ = writeln!(
        out,
        "  outer.query_selector(\"p\") → {:?}",
        dom.node(outer).query_selector("p").map(|n| n.id())
    );
}

fn write_cssom(out: &mut String) {
    let _ = writeln!(out, "─── cssom ──────────────────────────────────────");
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();

    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .set_property("color", "red")
        .unwrap();
    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .set_property("gap", "2")
        .unwrap();
    dom.node_mut(div)
        .style_mut()
        .unwrap()
        .set_property_important("padding", "1 2 3 4")
        .unwrap();

    let style = dom.node(div).style().unwrap();
    let _ = writeln!(
        out,
        "  get_property_value(color) → {:?}",
        style.get_property_value("color")
    );
    let _ = writeln!(
        out,
        "  get_property_priority(padding) → {:?}",
        style.get_property_priority("padding")
    );
    let _ = writeln!(out, "  cssText → {:?}", style.css_text());
    let _ = writeln!(out, "  length  → {}", style.length());
    for i in 0..style.length() {
        let _ = writeln!(out, "    item({i}) = {:?}", style.item(i));
    }

    let _ = writeln!(out, "  .color()          → {:?}", style.color());
    let _ = writeln!(
        out,
        "  .background_color() → {:?}",
        style.background_color()
    );

    let observer_id = rdom_tui::cssom::install_inline_style_observer(&mut dom);
    dom.set_attribute(div, "style", "color: blue; gap: 5")
        .unwrap();
    let _ = writeln!(
        out,
        "  after external write → color = {:?}",
        dom.node(div).style().unwrap().get_property_value("color")
    );
    dom.remove_mutation_observer(observer_id);

    let _ = writeln!(
        out,
        "  caret_position_from_point(0, 0) → {:?}",
        dom.caret_position_from_point(0, 0)
    );
}

pub struct DomApi;

impl Demo for DomApi {
    fn slug(&self) -> &'static str {
        "builtins/dom-api"
    }

    fn title(&self) -> &'static str {
        "DOM API walkthrough"
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
