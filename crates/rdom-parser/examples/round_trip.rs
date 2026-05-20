//! Parse + `outer_markup` round-trip. For a defined subset of input,
//! the parser and serializer are inverses: `parse(src).outer_markup()
//! == src`.
//!
//! Run with: `cargo run --example round_trip -p rdom-parser`

use rdom_core::Dom;
use rdom_parser::parse;

fn main() {
    let corpus = [
        "<div></div>",
        "<div><span>hi</span></div>",
        "<br/>",
        r#"<div class="a b" id="x"><p>Hi</p></div>"#,
        "<!-- note --><div></div>",
        "<p>&amp; &lt; &gt;</p>",
        "<ul><li>one</li><li>two</li><li>three</li></ul>",
    ];

    let mut all_match = true;
    for src in &corpus {
        match round_trip(src) {
            Ok(out) => {
                let ok = &out == src;
                if ok {
                    println!("✓ {src}");
                } else {
                    all_match = false;
                    println!("✗ {src}");
                    println!("    → {out}");
                }
            }
            Err(e) => {
                all_match = false;
                println!("✗ {src}");
                println!("    parse error: {e}");
            }
        }
    }

    println!();
    if all_match {
        println!("All round-trips match.");
    } else {
        println!("Some round-trips diverged (see above).");
        std::process::exit(1);
    }
}

fn round_trip(src: &str) -> Result<String, rdom_parser::ParseError> {
    let (dom, ids): (Dom<()>, _) = parse(src)?;
    // For single top-level nodes (most cases), serialize the node
    // itself; for multi-top-level (like "<!-- … --><div></div>"),
    // serialize the fragment root's inner markup.
    let out = if ids.len() == 1 {
        dom.outer_markup(ids[0])
    } else {
        dom.inner_markup(dom.root())
    };
    Ok(out)
}
