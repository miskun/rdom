//! Headless DOM — build a tree programmatically, query with CSS
//! selectors, mutate, serialize. No terminal involved.
//!
//! Run with: `cargo run --example tree_builder -p rdom-core`

use rdom_core::Dom;

fn main() {
    // Build this tree:
    //
    //   <div id="app">
    //     <h1>Hello, rdom!</h1>
    //     <ul class="items">
    //       <li class="active">one</li>
    //       <li>two</li>
    //       <li>three</li>
    //     </ul>
    //   </div>
    let mut dom: Dom = Dom::new();
    let root = dom.root();

    let app = dom.create_element("div");
    dom.set_attribute(app, "id", "app").unwrap();

    let h1 = dom.create_element("h1");
    let h1_text = dom.create_text_node("Hello, rdom!");
    dom.append_child(h1, h1_text).unwrap();
    dom.append_child(app, h1).unwrap();

    let ul = dom.create_element("ul");
    dom.add_class(ul, "items").unwrap();
    for (i, label) in ["one", "two", "three"].iter().enumerate() {
        let li = dom.create_element("li");
        if i == 0 {
            dom.add_class(li, "active").unwrap();
        }
        let text = dom.create_text_node(label);
        dom.append_child(li, text).unwrap();
        dom.append_child(ul, li).unwrap();
    }
    dom.append_child(app, ul).unwrap();
    dom.append_child(root, app).unwrap();

    println!("── Tree built ──────────────────────────────────");
    println!("{}", dom.outer_markup(app));
    println!();

    // CSS selector queries.
    println!("── Queries ─────────────────────────────────────");
    let items = dom.query_selector_all_in(root, "li").unwrap();
    println!("Found {} <li> elements:", items.len());
    for li in &items {
        println!("  - {}", dom.text_content(*li));
    }

    let active = dom.query_selector_in(root, "li.active").unwrap().unwrap();
    println!("\nActive item: {}", dom.text_content(active));

    // Closest: walk up ancestors.
    let list = dom.closest(active, "ul").unwrap().unwrap();
    println!(
        "Its containing <ul> has class `items`: {}",
        dom.has_class(list, "items")
    );

    // Matches: does this element match a selector?
    println!(
        "\nDoes `#app` match `div[id=app]`? {}",
        dom.matches(app, "div[id=app]").unwrap()
    );

    // Mutate: add a fourth item, remove first.
    println!("\n── Mutating ────────────────────────────────────");
    let li4 = dom.create_element("li");
    let t4 = dom.create_text_node("four (added later)");
    dom.append_child(li4, t4).unwrap();
    dom.append_child(ul, li4).unwrap();

    let first_li = items[0];
    dom.drop_subtree(first_li).unwrap();

    println!("After mutation:");
    for li in dom.query_selector_all_in(root, "li").unwrap() {
        println!("  - {}", dom.text_content(li));
    }

    // Serialize the final tree.
    println!("\n── Final markup ────────────────────────────────");
    println!("{}", dom.outer_markup(app));
}
