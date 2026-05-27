//! Inline formatting — a flowing `<p>` paragraph with embedded
//! `<strong>`, `<em>`, `<code>`, and `<mark>` showing IFC text
//! wrapping with mixed inline styles.
//!
//! Exercises the inline formatting context (`is_ifc_block`):
//! mixed text + inline-element children pack into lines via
//! `compute_inline_layout`, wrapping at the container's content
//! width. Each inline child carries its own cascaded style
//! (`<strong>` → bold, `<em>` → italic, `<code>` → gold-on-tinted-
//! bg, `<mark>` → black-on-yellow), and the IFC paint walks
//! fragments owned by each inline element so the styles apply
//! per-fragment.
//!
//! Authoring note: the paragraph's last child is an empty
//! `<span></span>`. This is the `SUB-2` workaround — IFC detection
//! requires at least one inline ELEMENT child (text-only `<p>`
//! falls back to block layout). The trailing span makes the `<p>`
//! a real IFC so the inline children + interleaved text pack
//! together with consistent line wrapping. The empty span has no
//! visible content; without it the paragraph would also work, but
//! the wrap semantics would differ slightly (block-layout
//! intrinsic-width pure-text-leaf branch vs. IFC wrap).

use std::io;

use rdom_tui::{App, NodeId, Stylesheet, TuiDom};

use crate::{Category, Demo, Source};

pub const MARKUP: &str = r#"<div class="inline-fmt">
  <h1>Inline formatting</h1>
  <p>Paragraphs can mix <strong>bold</strong>, <em>italic</em>, <code>code spans</code>, and <mark>highlighted text</mark> inside a single flowing line that wraps at the container's content width.<span></span></p>
  <p>A second paragraph keeps the same pattern: <strong>strong</strong> + <em>em</em> + <code>code</code> + <mark>mark</mark> all participating in the inline formatting context, each carrying its own cascaded style per fragment.<span></span></p>
</div>"#;

pub const CSS: &str = r#"
.inline-fmt {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 1 2;
  gap: 1;
}
.inline-fmt h1 {
  color: rgb(180, 220, 255);
  font-weight: bold;
}
"#;

pub fn build(dom: &mut TuiDom) -> NodeId {
    let root = dom.create_element("div");
    dom.set_attribute(root, "class", "inline-fmt").unwrap();

    let h1 = dom.create_element("h1");
    let h1_t = dom.create_text_node("Inline formatting");
    dom.append_child(h1, h1_t).unwrap();
    dom.append_child(root, h1).unwrap();

    let p1 = build_paragraph(
        dom,
        &[
            Run::Text("Paragraphs can mix "),
            Run::Tagged("strong", "bold"),
            Run::Text(", "),
            Run::Tagged("em", "italic"),
            Run::Text(", "),
            Run::Tagged("code", "code spans"),
            Run::Text(", and "),
            Run::Tagged("mark", "highlighted text"),
            Run::Text(" inside a single flowing line that wraps at the container's content width."),
        ],
    );
    dom.append_child(root, p1).unwrap();

    let p2 = build_paragraph(
        dom,
        &[
            Run::Text("A second paragraph keeps the same pattern: "),
            Run::Tagged("strong", "strong"),
            Run::Text(" + "),
            Run::Tagged("em", "em"),
            Run::Text(" + "),
            Run::Tagged("code", "code"),
            Run::Text(" + "),
            Run::Tagged("mark", "mark"),
            Run::Text(
                " all participating in the inline formatting context, each carrying its own cascaded style per fragment.",
            ),
        ],
    );
    dom.append_child(root, p2).unwrap();

    root
}

enum Run {
    Text(&'static str),
    Tagged(&'static str, &'static str),
}

fn build_paragraph(dom: &mut TuiDom, runs: &[Run]) -> NodeId {
    let p = dom.create_element("p");
    for run in runs {
        match run {
            Run::Text(s) => {
                let t = dom.create_text_node(s);
                dom.append_child(p, t).unwrap();
            }
            Run::Tagged(tag, body) => {
                let el = dom.create_element(tag);
                let t = dom.create_text_node(body);
                dom.append_child(el, t).unwrap();
                dom.append_child(p, el).unwrap();
            }
        }
    }
    // SUB-2 workaround: empty trailing <span> makes <p> an IFC.
    let tail = dom.create_element("span");
    dom.append_child(p, tail).unwrap();
    p
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

pub struct InlineFormatting;

impl Demo for InlineFormatting {
    fn slug(&self) -> &'static str {
        "text/inline-formatting"
    }

    fn title(&self) -> &'static str {
        "Inline formatting"
    }

    fn category(&self) -> Category {
        Category::Text
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
