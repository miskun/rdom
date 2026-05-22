//! `dom_api_demo` — exercises every M4 accessor family in
//! three themed walk-throughs:
//!
//! 1. **form-edit** — `<input>` / `<textarea>` / `<select>`
//!    reads + writes via the per-tag accessors, plus
//!    `form_elements` and `form_request_submit`.
//! 2. **tree-walk** — document-level hit-test accessors
//!    (`element_from_point`, `elements_from_point`,
//!    `caret_position_from_point`) against a laid-out tree.
//! 3. **cssom** — `el.style()` reads, `el.style_mut()` writes,
//!    `cssText` round-trips, and the build-script-generated
//!    camelCase aliases (`el.style().color()`,
//!    `el.style().background_color()`, …).
//!
//! Non-interactive — just prints accessor output. Useful as a
//! living documentation reference + smoke test.
//!
//! Run with: `cargo run -p rdom-tui --example dom_api_demo`.

use rdom_tui::prelude::*;
use rdom_tui::runtime::builtins::input;

fn main() {
    println!("=== rdom DOM API demo ===\n");
    demo_form_edit();
    println!();
    demo_tree_walk();
    println!();
    demo_cssom();
}

// ── Theme 1: form-edit ──────────────────────────────────────────

fn demo_form_edit() {
    println!("─── form-edit ──────────────────────────────────");
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();

    // Build: <form><input name="email" value="a@b.c"><textarea name="bio">Hello</textarea><select name="role"><option value="admin">Admin</option><option value="user" selected>User</option></select><button type="submit">Send</button></form>
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

    // Seed the input's text-child so `input::value` reads the
    // attribute (this is what `App::build` does at startup).
    input::seed_all(&mut dom);

    // Smart + narrow reads.
    println!("  smart input value     → {:?}", dom.node(input).value());
    println!(
        "  narrow input_value    → {:?}",
        dom.node(input).input_value()
    );
    println!(
        "  input_name            → {:?}",
        dom.node(input).input_name()
    );
    println!(
        "  textarea_value        → {:?}",
        dom.node(textarea).textarea_value()
    );
    println!(
        "  select_value          → {:?}",
        dom.node(select).select_value()
    );
    println!(
        "  select_selected_index → {:?}",
        dom.node(select).select_selected_index()
    );
    println!(
        "  option_value (user)   → {:?}",
        dom.node(opt_user).option_value()
    );

    // Form-level walks.
    let elts = dom.node(form).form_elements().unwrap();
    println!("  form_elements count   → {}", elts.len());
    println!(
        "  form_length           → {:?}",
        dom.node(form).form_length()
    );

    // Programmatic write: change the email.
    dom.node_mut(input).set_value("new@addr.test").unwrap();
    println!(
        "  after set_value       → {:?}",
        dom.node(input).input_value()
    );

    // Programmatic submit.
    let prevented = dom
        .node_mut(form)
        .form_request_submit(Some(button))
        .unwrap();
    println!("  form_request_submit   → prevented={prevented}, submitter={button:?}");
}

// ── Theme 2: tree-walk ──────────────────────────────────────────

fn demo_tree_walk() {
    println!("─── tree-walk ──────────────────────────────────");
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

    // Stylesheet sized so the divs have positions to hit-test
    // against.
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .width(Size::Fixed(40))
            .height(Size::Fixed(5)),
    );
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 80, 24));

    println!(
        "  element_from_point(2,1)   → {:?}",
        dom.element_from_point(2, 1).map(|n| n.id())
    );
    let path: Vec<rdom_tui::NodeId> = dom
        .elements_from_point(2, 1)
        .iter()
        .map(|n| n.id())
        .collect();
    println!("  elements_from_point depth → {}", path.len());
    println!(
        "  element_from_point(200,200) → {:?} (off-screen)",
        dom.element_from_point(200, 200).map(|n| n.id())
    );

    // Per-element navigation (substrate accessors, M4a step 14).
    let p_node = dom.node(inner);
    println!("  inner.is_connected      → {}", p_node.is_connected());
    println!(
        "  closest(\"div\")          → {:?}",
        p_node.closest("div").map(|n| n.id())
    );
    println!("  matches(\"p.para\")       → {}", p_node.matches("p.para"));
    println!(
        "  outer.query_selector(\"p\") → {:?}",
        dom.node(outer).query_selector("p").map(|n| n.id())
    );
}

// ── Theme 3: cssom ──────────────────────────────────────────────

fn demo_cssom() {
    println!("─── cssom ──────────────────────────────────────");
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();

    // Write via `el.style_mut()` — updates both inline_style
    // AND the `style="…"` attribute (§8.5 lock).
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
    println!(
        "  get_property_value(color) → {:?}",
        style.get_property_value("color")
    );
    println!(
        "  get_property_priority(padding) → {:?}",
        style.get_property_priority("padding")
    );
    println!("  cssText → {:?}", style.css_text());
    println!("  length  → {}", style.length());
    for i in 0..style.length() {
        println!("    item({i}) = {:?}", style.item(i));
    }

    // Build-script-generated camelCase aliases.
    println!("  .color()          → {:?}", style.color());
    println!("  .background_color() → {:?}", style.background_color());

    // External `style="…"` mutation → observer refreshes.
    // (Without `App::build` the observer isn't installed, so
    // demonstrate the manual install + write path.)
    let observer_id = rdom_tui::cssom::install_inline_style_observer(&mut dom);
    dom.set_attribute(div, "style", "color: blue; gap: 5")
        .unwrap();
    println!(
        "  after external write → color = {:?}",
        dom.node(div).style().unwrap().get_property_value("color")
    );
    dom.remove_mutation_observer(observer_id);

    // Document-level CSSOM-shaped accessors.
    println!(
        "  caret_position_from_point(0, 0) → {:?}",
        dom.caret_position_from_point(0, 0)
    );
}
