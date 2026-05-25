//! Selectable text — prose + code + CJK, plus a `user-select: none`
//! chrome row that's deliberately unselectable.
//!
//! Exercises drag-to-select, double-click word, triple-click line,
//! `::selection` overlay, Ctrl-C / Cmd-C copy to clipboard, and
//! `user-select: none` opt-out on UI chrome.

use std::io;

use rdom_tui::{App, NodeId, Stylesheet, TuiDom};

use crate::{Category, Demo, Source};

const PROSE: &str = concat!(
    "Terminal UIs should let you select text. That's table stakes ",
    "for any interactive tool that shows text — error messages, log ",
    "lines, config values, you name it. rdom-tui's runtime ships ",
    "with drag, double-click, triple-click, Shift+arrow, and Ctrl-A ",
    "support out of the box."
);

const CODE: &str = concat!(
    "let sel = Selection::new(",
    "Position::new(t, 0), ",
    "Position::new(t, 5));"
);

const CJK: &str = "中文字符也可以选择。CJK graphemes snap to full width.";

pub const MARKUP: &str = r#"<div class="selectable-text-demo">
  <h1>Selectable text demo</h1>
  <div class="chrome">Drag over prose, code, or CJK. This bar is user-select:none.</div>
  <p class="prose">…prose…</p>
  <pre class="code">…code…</pre>
  <p class="cjk">…CJK…</p>
</div>"#;

pub const CSS: &str = r#"
.selectable-text-demo {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 1 3;
  gap: 1;
}
.selectable-text-demo h1 {
  height: 1;
  color: rgb(180, 220, 255);
  font-weight: bold;
}
.selectable-text-demo .chrome {
  height: 1;
  user-select: none;
}
.selectable-text-demo .code {
  display: block;
}
"#;

pub fn build(dom: &mut TuiDom) -> NodeId {
    let root = dom.create_element("div");
    dom.set_attribute(root, "class", "selectable-text-demo")
        .unwrap();

    let h1 = dom.create_element("h1");
    let h1_text = dom.create_text_node("Selectable text demo");
    dom.append_child(h1, h1_text).unwrap();
    dom.append_child(root, h1).unwrap();

    let chrome = dom.create_element("div");
    dom.set_attribute(chrome, "class", "chrome").unwrap();
    let chrome_text =
        dom.create_text_node("Drag over prose, code, or CJK. This bar is user-select:none.");
    dom.append_child(chrome, chrome_text).unwrap();
    dom.append_child(root, chrome).unwrap();

    // Prose paragraph — must contain at least one inline element
    // child for `is_ifc_block` to register it as an IFC (substrate
    // workaround `SUB-2` in TECH_DEBT.md). A trailing empty
    // <span> does the trick.
    let prose = dom.create_element("p");
    dom.set_attribute(prose, "class", "prose").unwrap();
    let prose_text = dom.create_text_node(PROSE);
    dom.append_child(prose, prose_text).unwrap();
    let prose_tail = dom.create_element("span");
    dom.append_child(prose, prose_tail).unwrap();
    dom.append_child(root, prose).unwrap();

    // Code block — same IFC workaround.
    let code = dom.create_element("pre");
    dom.set_attribute(code, "class", "code").unwrap();
    let code_text = dom.create_text_node(CODE);
    dom.append_child(code, code_text).unwrap();
    let code_tail = dom.create_element("span");
    dom.append_child(code, code_tail).unwrap();
    dom.append_child(root, code).unwrap();

    let cjk = dom.create_element("p");
    dom.set_attribute(cjk, "class", "cjk").unwrap();
    let cjk_text = dom.create_text_node(CJK);
    dom.append_child(cjk, cjk_text).unwrap();
    let cjk_tail = dom.create_element("span");
    dom.append_child(cjk, cjk_tail).unwrap();
    dom.append_child(root, cjk).unwrap();

    root
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

pub struct SelectableText;

impl Demo for SelectableText {
    fn slug(&self) -> &'static str {
        "selection/selectable-text"
    }

    fn title(&self) -> &'static str {
        "Selectable text"
    }

    fn category(&self) -> Category {
        Category::Selection
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
