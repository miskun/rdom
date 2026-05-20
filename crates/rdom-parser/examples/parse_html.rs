//! Parse an HTML-ish template and walk the resulting tree.
//!
//! Run with: `cargo run --example parse_html -p rdom-parser`

use rdom_core::{Dom, NodeId, NodeType};
use rdom_parser::parse;

fn main() {
    let template = r#"
        <div class="card" id="hero">
          <h1>Welcome &amp; hello</h1>
          <p>This is <strong>rdom-parser</strong>.</p>
          <!-- TODO: add icon -->
          <ul>
            <li>Zero external deps</li>
            <li>Full UTF-8: 中文, 🦀, 👨‍👩‍👧</li>
            <li>Error reporting with line/col + hints</li>
          </ul>
          <button disabled>OK</button>
        </div>
    "#;

    let (dom, _ids): (Dom<()>, _) = parse(template).unwrap();
    let root = dom.root();

    println!("── Parsed tree ─────────────────────────────────");
    walk(&dom, root, 0);

    println!("\n── Selector queries ────────────────────────────");
    let lis = dom.query_selector_all_in(root, "li").unwrap();
    println!("Found {} <li>:", lis.len());
    for li in lis {
        println!("  • {}", dom.text_content(li));
    }

    let card = dom.query_selector_in(root, ".card").unwrap().unwrap();
    println!(
        "\n.card's id attribute: {:?}",
        dom.get_attribute(card, "id")
    );

    let button = dom.query_selector_in(root, "button[disabled]").unwrap();
    if let Some(b) = button {
        println!("Disabled button: {}", dom.text_content(b));
    }

    println!("\n── Text content (flattened) ────────────────────");
    println!(
        "{}",
        dom.text_content(card)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    );
}

/// Pretty-print a subtree.
fn walk(dom: &Dom<()>, id: NodeId, depth: usize) {
    let indent = "  ".repeat(depth);
    let node = dom.node(id);
    match node.node_type() {
        NodeType::Element => {
            let tag = node.tag_name().unwrap_or("?");
            let attrs: Vec<String> = dom
                .attributes(id)
                .map(|(k, v)| format!(r#" {k}="{v}""#))
                .collect();
            let classes: Vec<&str> = dom.class_list(id).collect();
            let class_attr = if classes.is_empty() {
                String::new()
            } else {
                format!(r#" class="{}""#, classes.join(" "))
            };
            println!("{indent}<{tag}{class_attr}{}>", attrs.join(""));
            for child in node.child_nodes() {
                walk(dom, child.id(), depth + 1);
            }
            // Only print close tag if there are children or a bare
            // self-close doesn't render well.
            if node.has_child_nodes() {
                println!("{indent}</{tag}>");
            }
        }
        NodeType::Text => {
            let s = node.node_value().unwrap_or("").trim();
            if !s.is_empty() {
                println!("{indent}{s:?}");
            }
        }
        NodeType::Comment => {
            println!("{indent}<!--{}-->", node.node_value().unwrap_or(""));
        }
        NodeType::Fragment => {
            for child in node.child_nodes() {
                walk(dom, child.id(), depth);
            }
        }
    }
}
