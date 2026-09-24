//! Markup serialization — `outer_markup()` / `inner_markup()`.
//!
//! Walks the tree and writes HTML-ish text. Attributes and text are
//! entity-encoded. Void elements (no children + known self-closing tags)
//! emit `<hr/>` style. Round-trip-compatible with the DOMParser in
//! Phase 10 — `Parser::parse(x.outer_markup()).outer_markup() == x.outer_markup()`.

use crate::dom::Dom;
use crate::node::NodeData;
use crate::node_id::NodeId;

/// The void elements of HTML §13.3 (serialization) — elements that
/// never have children and are serialized without an end tag. Shared
/// with `rdom-parser`, which treats a start tag of one of these as the
/// whole element; keeping one list means what the parser accepts is
/// exactly what the serializer emits.
pub const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "basefont", "bgsound", "br", "col", "embed", "frame", "hr", "img", "input",
    "keygen", "link", "meta", "param", "source", "track", "wbr",
];

/// Is `tag` (lowercase) one of [`VOID_ELEMENTS`]?
pub fn is_void_element(tag: &str) -> bool {
    VOID_ELEMENTS.contains(&tag)
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

/// HTML §13.3 step 2 "text node" branch: a text child of one of these
/// elements is serialized verbatim, because their content model is raw
/// text — escaping `<` / `&` inside `<style>` would corrupt the CSS on
/// round-trip. `textarea` and `title` are *escapable* raw text and are
/// deliberately not listed.
fn serializes_children_raw(tag: &str) -> bool {
    matches!(
        tag,
        "style" | "script" | "xmp" | "iframe" | "noembed" | "noframes" | "plaintext"
    )
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
        self.write_node(id, &mut out, false);
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
            NodeData::Element { tag, .. } => {
                self.write_children(id, &mut out, serializes_children_raw(tag));
            }
            NodeData::Fragment => self.write_children(id, &mut out, false),
            _ => {}
        }
        out
    }

    fn write_children(&self, id: NodeId, out: &mut String, raw_text: bool) {
        let mut child = self.get_node(id).and_then(|n| n.first_child);
        while let Some(c) = child {
            self.write_node(c, out, raw_text);
            child = self.get_node(c).and_then(|n| n.next_sibling);
        }
    }

    /// `raw_text` is true when `id`'s parent serializes its text
    /// children verbatim (see [`serializes_children_raw`]).
    fn write_node(&self, id: NodeId, out: &mut String, raw_text: bool) {
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

                if is_void_element(tag) && node.first_child.is_none() {
                    out.push_str("/>");
                    return;
                }
                out.push('>');
                self.write_children(id, out, serializes_children_raw(tag));
                out.push_str("</");
                out.push_str(tag);
                out.push('>');
            }
            NodeData::Text { data } => {
                if raw_text {
                    out.push_str(data);
                } else {
                    escape_for_text(data, out);
                }
            }
            NodeData::Comment { data } => {
                out.push_str("<!--");
                out.push_str(data); // comments pass through unescaped in HTML
                out.push_str("-->");
            }
            NodeData::Fragment => self.write_children(id, out, false),
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

    /// HTML §13.3 serialization: text under `style` / `script` (and the
    /// other raw-text elements) is emitted as-is; `textarea` / `title`
    /// are escapable raw text and keep the escaping.
    #[test]
    fn raw_text_element_children_are_not_escaped() {
        let mut dom: Dom = Dom::new();
        let style = dom.create_element("style");
        let css = dom.create_text_node("a > b { content: \"<&\"; }");
        dom.append_child(style, css).unwrap();
        assert_eq!(
            dom.outer_markup(style),
            "<style>a > b { content: \"<&\"; }</style>"
        );
        assert_eq!(dom.inner_markup(style), "a > b { content: \"<&\"; }");

        let ta = dom.create_element("textarea");
        let t = dom.create_text_node("<b>&");
        dom.append_child(ta, t).unwrap();
        assert_eq!(dom.outer_markup(ta), "<textarea>&lt;b&gt;&amp;</textarea>");
    }

    /// `PARSER-VOID-TAGS-1`: one exported list, exactly HTML §13.3's
    /// void elements. `vr` was never an HTML element.
    #[test]
    fn void_elements_are_the_html_serialization_set() {
        let expected = [
            "area", "base", "basefont", "bgsound", "br", "col", "embed", "frame", "hr", "img",
            "input", "keygen", "link", "meta", "param", "source", "track", "wbr",
        ];
        assert_eq!(crate::VOID_ELEMENTS, &expected);
        assert!(crate::is_void_element("br"));
        assert!(!crate::is_void_element("vr"));
        assert!(!crate::is_void_element("div"));
        let mut dom: Dom = Dom::new();
        let vr = dom.create_element("vr");
        assert_eq!(dom.outer_markup(vr), "<vr></vr>");
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
