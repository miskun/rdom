//! Hover — a button-like div that changes its border + text color
//! on `:hover`. Demonstrates rdom's `:hover` pseudo-class working
//! through the cascade — no JavaScript needed.
//!
//! Move your terminal mouse over the box to see it react.

use rdom_tui::{NodeId, Stylesheet, TuiDom};

use crate::{Category, Demo, Source};

const MARKUP: &str = r#"<div class="hover-demo">
  <div class="card">Hover me</div>
  <p class="hint">Move the mouse over the box. The border + text color come from a <code>:hover</code> rule.</p>
</div>"#;

const CSS: &str = r#"
.hover-demo {
  padding: 1;
  display: flex;
  flex-direction: column;
  gap: 1;
}
.hover-demo .card {
  height: 3;
  width: 20;
  border: solid;
  border-color: rgb(120, 130, 150);
  color: rgb(180, 190, 210);
  padding: 0 2;
}
.hover-demo .card:hover {
  border-color: rgb(220, 200, 120);
  color: rgb(255, 230, 150);
  font-weight: bold;
}
.hover-demo .hint {
  color: rgb(140, 150, 170);
}
.hover-demo .hint code {
  color: rgb(200, 200, 230);
}
"#;

pub struct Hover;

impl Demo for Hover {
    fn slug(&self) -> &'static str {
        "cascade/hover"
    }

    fn title(&self) -> &'static str {
        "Hover"
    }

    fn category(&self) -> Category {
        Category::Cascade
    }

    fn build(&self, dom: &mut TuiDom) -> NodeId {
        let root = dom.create_element("div");
        dom.set_attribute(root, "class", "hover-demo").unwrap();

        let card = dom.create_element("div");
        dom.set_attribute(card, "class", "card").unwrap();
        let card_text = dom.create_text_node("Hover me");
        dom.append_child(card, card_text).unwrap();
        dom.append_child(root, card).unwrap();

        let hint = dom.create_element("p");
        dom.set_attribute(hint, "class", "hint").unwrap();
        let hint_pre = dom
            .create_text_node("Move the mouse over the box. The border + text color come from a ");
        let code = dom.create_element("code");
        let code_text = dom.create_text_node(":hover");
        dom.append_child(code, code_text).unwrap();
        let hint_post = dom.create_text_node(" rule.");
        dom.append_child(hint, hint_pre).unwrap();
        dom.append_child(hint, code).unwrap();
        dom.append_child(hint, hint_post).unwrap();
        dom.append_child(root, hint).unwrap();

        root
    }

    fn stylesheet(&self) -> Stylesheet {
        rdom_css::from_css(CSS)
    }

    fn source(&self) -> Source {
        Source {
            markup: MARKUP,
            css: CSS,
        }
    }
}
