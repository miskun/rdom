//! Markup serialization — `outer_markup()` / `inner_markup()`.
//!
//! Walks the tree and writes HTML-ish text. Attributes and text are
//! entity-encoded. Void elements (no children + known self-closing tags)
//! emit `<hr/>` style. Round-trip-compatible with the DOMParser in
//! Phase 10 — `Parser::parse(x.outer_markup()).outer_markup() == x.outer_markup()`.

use crate::dom::Dom;
use crate::node::NodeData;
use crate::node_id::NodeId;

/// HTML5 void elements — never have children, always self-close.
/// Extended slightly to include terminal-oriented tags like `hr` / `vr`.
const VOID_TAGS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr", "vr",
];

fn is_void_tag(tag: &str) -> bool {
    VOID_TAGS.contains(&tag)
}

/// Entity-encode a string for use inside an attribute value or text node.
/// Matches the minimal set browsers require: `& < > " '`.
fn escape_for_attr(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
}

/// Text nodes don't need to escape quotes but must escape the three html-critical chars.
fn escape_for_text(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c => out.push(c),
        }
    }
}

impl<Ext> Dom<Ext> {
    /// Serialize `id` and its subtree to HTML-ish markup.
    ///
    /// - Elements: `<tag attr="v" class="c d">children</tag>`
    /// - Void elements: `<hr/>`
    /// - Text: entity-encoded content
    /// - Comments: `<!-- data -->`
    /// - Fragments: concatenated children (no wrapper tag)
    pub fn outer_markup(&self, id: NodeId) -> String {
        let mut out = String::new();
        self.write_node(id, &mut out);
        out
    }

    /// Children serialized without `id`'s own wrapper. For an Element
    /// this is the classic `innerHTML`; for a Fragment it's identical to
    /// `outer_markup`; for Text/Comment it returns an empty string.
    pub fn inner_markup(&self, id: NodeId) -> String {
        let mut out = String::new();
        let Some(node) = self.get_node(id) else {
            return out;
        };
        match &node.data {
            NodeData::Element { .. } | NodeData::Fragment => {
                let mut child = node.first_child;
                while let Some(c) = child {
                    self.write_node(c, &mut out);
                    child = self.get_node(c).and_then(|n| n.next_sibling);
                }
            }
            _ => {}
        }
        out
    }

    fn write_node(&self, id: NodeId, out: &mut String) {
        let Some(node) = self.get_node(id) else {
            return;
        };
        match &node.data {
            NodeData::Element {
                tag,
                attrs,
                classes,
                ..
            } => {
                out.push('<');
                out.push_str(tag);
                // class attribute (if any).
                if !classes.is_empty() {
                    out.push_str(" class=\"");
                    let mut first = true;
                    for c in classes {
                        if !first {
                            out.push(' ');
                        }
                        first = false;
                        escape_for_attr(c, out);
                    }
                    out.push('"');
                }
                // Other attributes, alphabetically via BTreeMap — skip
                // "class" because we rendered it from the classList above.
                for (k, v) in attrs.iter().filter(|(k, _)| k.as_str() != "class") {
                    out.push(' ');
                    out.push_str(k);
                    if !v.is_empty() {
                        out.push_str("=\"");
                        escape_for_attr(v, out);
                        out.push('"');
                    } else {
                        // Boolean attribute — `disabled`, `hidden`, etc.
                        // Empty string value is the canonical "present"
                        // form in HTML5.
                    }
                }

                if is_void_tag(tag) && node.first_child.is_none() {
                    out.push_str("/>");
                    return;
                }
                out.push('>');

                let mut child = node.first_child;
                while let Some(c) = child {
                    self.write_node(c, out);
                    child = self.get_node(c).and_then(|n| n.next_sibling);
                }

                out.push_str("</");
                out.push_str(tag);
                out.push('>');
            }
            NodeData::Text { data } => {
                escape_for_text(data, out);
            }
            NodeData::Comment { data } => {
                out.push_str("<!--");
                out.push_str(data); // comments pass through unescaped in HTML
                out.push_str("-->");
            }
            NodeData::Fragment => {
                let mut child = node.first_child;
                while let Some(c) = child {
                    self.write_node(c, out);
                    child = self.get_node(c).and_then(|n| n.next_sibling);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Dom;

    #[test]
    fn element_without_attrs() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        assert_eq!(dom.outer_markup(el), "<div></div>");
    }

    #[test]
    fn element_with_attrs() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.set_attribute(el, "role", "banner").unwrap();
        dom.set_attribute(el, "data-x", "1").unwrap();
        // Attributes in alphabetic order.
        assert_eq!(
            dom.outer_markup(el),
            r#"<div data-x="1" role="banner"></div>"#
        );
    }

    #[test]
    fn element_with_classes() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.add_class(el, "two").unwrap();
        dom.add_class(el, "one").unwrap();
        // class attribute ordered by BTreeSet (alphabetic).
        assert_eq!(dom.outer_markup(el), r#"<div class="one two"></div>"#);
    }

    #[test]
    fn void_tag_self_closes() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("hr");
        assert_eq!(dom.outer_markup(el), "<hr/>");
    }

    #[test]
    fn nested_elements() {
        let mut dom: Dom = Dom::new();
        let div = dom.create_element("div");
        let span = dom.create_element("span");
        let text = dom.create_text_node("hi");
        dom.append_child(span, text).unwrap();
        dom.append_child(div, span).unwrap();
        assert_eq!(dom.outer_markup(div), "<div><span>hi</span></div>");
    }

    #[test]
    fn text_content_is_escaped() {
        let mut dom: Dom = Dom::new();
        let div = dom.create_element("div");
        let t = dom.create_text_node("a & b <c>");
        dom.append_child(div, t).unwrap();
        assert_eq!(dom.outer_markup(div), "<div>a &amp; b &lt;c&gt;</div>");
    }

    #[test]
    fn attribute_values_are_escaped() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.set_attribute(el, "title", r#"she said "hi""#).unwrap();
        assert_eq!(
            dom.outer_markup(el),
            r#"<div title="she said &quot;hi&quot;"></div>"#
        );
    }

    #[test]
    fn comment_node_serializes() {
        let mut dom: Dom = Dom::new();
        let c = dom.create_comment(" note ");
        assert_eq!(dom.outer_markup(c), "<!-- note -->");
    }

    #[test]
    fn fragment_is_childrens_concat() {
        let mut dom: Dom = Dom::new();
        let frag = dom.create_document_fragment();
        let a = dom.create_element("a");
        let b = dom.create_element("b");
        dom.append_child(frag, a).unwrap();
        dom.append_child(frag, b).unwrap();
        assert_eq!(dom.outer_markup(frag), "<a></a><b></b>");
    }

    #[test]
    fn inner_markup_omits_wrapper() {
        let mut dom: Dom = Dom::new();
        let div = dom.create_element("div");
        let span = dom.create_element("span");
        let text = dom.create_text_node("inner");
        dom.append_child(span, text).unwrap();
        dom.append_child(div, span).unwrap();
        assert_eq!(dom.inner_markup(div), "<span>inner</span>");
    }

    #[test]
    fn boolean_attribute_emits_bare_name() {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("input");
        dom.toggle_attribute(el, "disabled").unwrap(); // becomes empty-valued
        // Attribute value is "" so we omit the `="..."` part.
        assert_eq!(dom.outer_markup(el), "<input disabled/>");
    }

    #[test]
    fn class_attribute_goes_through_class_list() {
        // Setting `class` directly via set_attribute stores it separately
        // from classList. outer_markup renders from classList (the
        // source-of-truth) and ignores raw `class` attr to avoid doubling.
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        dom.add_class(el, "a").unwrap();
        dom.add_class(el, "b").unwrap();
        // classList has a, b. Never set via set_attribute("class", …).
        assert_eq!(dom.outer_markup(el), r#"<div class="a b"></div>"#);
    }

    #[test]
    fn deep_tree() {
        let mut dom: Dom = Dom::new();
        let html = dom.create_element("html");
        let body = dom.create_element("body");
        let h1 = dom.create_element("h1");
        let t = dom.create_text_node("Welcome");
        dom.append_child(h1, t).unwrap();
        dom.append_child(body, h1).unwrap();
        dom.append_child(html, body).unwrap();
        assert_eq!(
            dom.outer_markup(html),
            "<html><body><h1>Welcome</h1></body></html>"
        );
    }
}
