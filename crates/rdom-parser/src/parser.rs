//! Recursive-descent HTML-ish parser.
//!
//! Consumes a template string, emits `Dom<Ext>` tree under a mount
//! NodeId. Supports:
//!
//! - Start tags (`<tag>`), end tags (`</tag>`), self-closing (`<br/>`)
//! - Void elements (`<br>`, `<hr>`, `<img>`, …) auto-close without `/>`
//! - Attributes: `name="value"` / `name='value'` / `name=value` / `name`
//! - Text with entity decoding: `&amp; &lt; &gt; &quot; &apos; &#NNN; &#xHH;`
//! - Comments: `<!-- … -->` preserved as Comment nodes
//! - Case-insensitive tag names (tags are normalized to lowercase)
//!
//! Out of scope: `<!DOCTYPE>`, CDATA, namespace prefixes, processing
//! instructions, `<script>` / `<style>` raw-text mode.

use rdom_core::{Dom, NodeId};

use crate::error::{ParseError, Result};

/// HTML5 void tag set — never have children, always self-close.
/// Mirror of `rdom_core::markup::VOID_TAGS` (we don't depend on that
/// private constant, so redeclare here).
const VOID_TAGS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr", "vr",
];

fn is_void_tag(tag: &str) -> bool {
    VOID_TAGS.contains(&tag)
}

/// Parse `template` into a fresh `Dom<Ext>` with a Fragment root. The
/// returned ids are the top-level children of the fragment.
pub fn parse<Ext>(template: &str) -> Result<(Dom<Ext>, Vec<NodeId>)>
where
    Ext: Default + 'static,
{
    let mut dom = Dom::new();
    let root = dom.root();
    let ids = parse_into(&mut dom, template, root)?;
    Ok((dom, ids))
}

/// Parse `template` and append the parsed tree under `mount`. Returns
/// the ids of the top-level parsed nodes (direct children appended to
/// `mount`). Does not alter existing children of `mount`.
pub fn parse_into<Ext>(dom: &mut Dom<Ext>, template: &str, mount: NodeId) -> Result<Vec<NodeId>>
where
    Ext: Default + 'static,
{
    let mut p = Parser::new(template);
    let ids = p.parse_nodes(dom, mount)?;
    Ok(ids)
}

// ─── Internal parser ────────────────────────────────────────────────

struct Parser<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
    line: u32,
    col: u32,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src,
            bytes: src.as_bytes(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    // ── Cursor ────────────────────────────────────────────────────

    fn eof(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.bytes.get(self.pos + offset).copied()
    }

    fn starts_with(&self, needle: &str) -> bool {
        self.src[self.pos..].starts_with(needle)
    }

    fn advance(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        if b == b'\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(b)
    }

    fn advance_n(&mut self, n: usize) {
        for _ in 0..n {
            if self.advance().is_none() {
                break;
            }
        }
    }

    fn skip_ws(&mut self) {
        while let Some(b) = self.peek() {
            if b.is_ascii_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn err(&self, msg: impl Into<String>) -> ParseError {
        ParseError::new(msg, self.line, self.col, self.pos)
    }

    // ── Top-level: parse children of `parent` ─────────────────────

    fn parse_nodes<Ext>(&mut self, dom: &mut Dom<Ext>, parent: NodeId) -> Result<Vec<NodeId>>
    where
        Ext: Default + 'static,
    {
        let mut out = Vec::new();
        loop {
            if self.eof() {
                break;
            }
            if self.starts_with("</") {
                // Bubble up to the containing element parser.
                break;
            }
            if self.starts_with("<!--") {
                let id = self.parse_comment(dom, parent)?;
                out.push(id);
                continue;
            }
            if self.peek() == Some(b'<') {
                let id = self.parse_element(dom, parent)?;
                out.push(id);
                continue;
            }
            // Plain text until next '<'.
            let id = self.parse_text(dom, parent)?;
            if let Some(id) = id {
                out.push(id);
            }
        }
        Ok(out)
    }

    // ── Comment ────────────────────────────────────────────────────

    fn parse_comment<Ext>(&mut self, dom: &mut Dom<Ext>, parent: NodeId) -> Result<NodeId>
    where
        Ext: Default + 'static,
    {
        // Consume `<!--`.
        self.advance_n(4);
        let start = self.pos;
        loop {
            if self.eof() {
                return Err(self
                    .err("unterminated comment")
                    .with_hint("missing `-->` closing"));
            }
            if self.starts_with("-->") {
                let data = &self.src[start..self.pos];
                self.advance_n(3);
                let id = dom.create_comment(data);
                dom.append_child(parent, id)
                    .map_err(|e| self.err(format!("failed to append comment: {:?}", e)))?;
                return Ok(id);
            }
            self.advance();
        }
    }

    // ── Text ───────────────────────────────────────────────────────

    /// Consume chars until next `<`, decode entities, emit a Text node.
    /// Returns `None` when the captured text is empty (no Text node
    /// created). UTF-8-safe — we slice by byte but the boundaries are
    /// always on valid char boundaries because we advance byte-at-a-time
    /// only via `self.advance()` which respects the source encoding.
    fn parse_text<Ext>(&mut self, dom: &mut Dom<Ext>, parent: NodeId) -> Result<Option<NodeId>>
    where
        Ext: Default + 'static,
    {
        let mut out = String::new();
        loop {
            // Collect consecutive raw (non-'<', non-'&') bytes as a
            // UTF-8 slice from the source.
            let slice_start = self.pos;
            while let Some(b) = self.peek() {
                if b == b'<' || b == b'&' {
                    break;
                }
                self.advance();
            }
            if slice_start < self.pos {
                out.push_str(&self.src[slice_start..self.pos]);
            }
            match self.peek() {
                None | Some(b'<') => break,
                Some(b'&') => {
                    out.push_str(&self.parse_entity()?);
                }
                _ => unreachable!(),
            }
        }
        if out.is_empty() {
            return Ok(None);
        }
        let id = dom.create_text_node(&out);
        dom.append_child(parent, id)
            .map_err(|e| self.err(format!("failed to append text: {:?}", e)))?;
        Ok(Some(id))
    }

    // ── Entity ─────────────────────────────────────────────────────

    fn parse_entity(&mut self) -> Result<String> {
        // We've seen '&'. Try to match a known entity; fall back to
        // preserving as-is on malformed input (lenient mode, matches
        // browser tolerance).
        debug_assert_eq!(self.peek(), Some(b'&'));
        let save = self.snapshot();

        self.advance(); // consume '&'

        // Find the end of the entity — next ';' or 16 chars max.
        let start = self.pos;
        let mut end = None;
        for i in 0..16 {
            match self.peek_at(i) {
                Some(b';') => {
                    end = Some(self.pos + i);
                    break;
                }
                Some(b) if b.is_ascii_alphanumeric() || b == b'#' || b == b'x' || b == b'X' => {
                    continue;
                }
                _ => break,
            }
        }

        let Some(end) = end else {
            // No terminator found — restore cursor and emit '&' literally.
            self.restore(save);
            self.advance(); // consume the '&'
            return Ok("&".to_string());
        };

        let body = &self.src[start..end];
        let decoded = decode_entity_body(body);
        if let Some(d) = decoded {
            // Skip past the ';'.
            let consume = end - self.pos + 1;
            self.advance_n(consume);
            Ok(d)
        } else {
            // Unknown entity — leave the '&' literal and continue;
            // later chars will be consumed as text.
            self.restore(save);
            self.advance();
            Ok("&".to_string())
        }
    }

    // ── Element ────────────────────────────────────────────────────

    fn parse_element<Ext>(&mut self, dom: &mut Dom<Ext>, parent: NodeId) -> Result<NodeId>
    where
        Ext: Default + 'static,
    {
        debug_assert_eq!(self.peek(), Some(b'<'));
        self.advance(); // '<'

        let tag = self.parse_tag_name()?;
        let tag_lc = tag.to_ascii_lowercase();

        let element = dom.create_element(&tag_lc);

        // Parse attributes until '>' or '/>'.
        loop {
            self.skip_ws();
            match self.peek() {
                None => {
                    return Err(self
                        .err(format!("unexpected EOF inside <{}>", tag_lc))
                        .with_hint("missing closing `>`"));
                }
                Some(b'>') => {
                    self.advance();
                    break;
                }
                Some(b'/') => {
                    // Self-closing.
                    self.advance();
                    self.skip_ws();
                    if self.peek() != Some(b'>') {
                        return Err(self
                            .err(format!("expected `>` after `/` in <{}/>", tag_lc))
                            .with_hint("self-closing syntax is `/>`"));
                    }
                    self.advance();
                    dom.append_child(parent, element)
                        .map_err(|e| self.err(format!("failed to append <{}>: {:?}", tag_lc, e)))?;
                    return Ok(element);
                }
                Some(_) => {
                    self.parse_attribute(dom, element)?;
                }
            }
        }

        // Void tag? Done.
        if is_void_tag(&tag_lc) {
            dom.append_child(parent, element)
                .map_err(|e| self.err(format!("failed to append <{}>: {:?}", tag_lc, e)))?;
            return Ok(element);
        }

        // Parse children, then expect </tag>.
        self.parse_nodes(dom, element)?;

        if !self.starts_with("</") {
            return Err(self
                .err(format!("missing closing tag for <{}>", tag_lc))
                .with_hint(format!("add </{}> to close", tag_lc)));
        }
        self.advance_n(2); // '</'

        let close_tag = self.parse_tag_name()?;
        if close_tag.to_ascii_lowercase() != tag_lc {
            return Err(self
                .err(format!(
                    "mismatched closing tag: found </{}>, expected </{}>",
                    close_tag, tag_lc
                ))
                .with_hint("tags must be properly nested"));
        }
        self.skip_ws();
        if self.peek() != Some(b'>') {
            return Err(self
                .err(format!("expected `>` in </{}>", tag_lc))
                .with_hint("no attributes on closing tags"));
        }
        self.advance();

        dom.append_child(parent, element)
            .map_err(|e| self.err(format!("failed to append <{}>: {:?}", tag_lc, e)))?;
        Ok(element)
    }

    fn parse_tag_name(&mut self) -> Result<String> {
        let start = self.pos;
        while let Some(b) = self.peek() {
            if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' {
                self.advance();
            } else {
                break;
            }
        }
        if start == self.pos {
            return Err(self
                .err("expected tag name")
                .with_hint("tag names start with a letter"));
        }
        Ok(self.src[start..self.pos].to_string())
    }

    fn parse_attribute<Ext>(&mut self, dom: &mut Dom<Ext>, element: NodeId) -> Result<()>
    where
        Ext: Default + 'static,
    {
        let name = self.parse_attr_name()?;
        self.skip_ws();

        let value = if self.peek() == Some(b'=') {
            self.advance();
            self.skip_ws();
            Some(self.parse_attr_value()?)
        } else {
            None
        };

        match value {
            Some(v) => {
                // Classes are normalized into the classList; other
                // attrs go into the attribute map.
                if name.eq_ignore_ascii_case("class") {
                    for token in v.split_ascii_whitespace() {
                        dom.add_class(element, token)
                            .map_err(|e| self.err(format!("failed to add class: {:?}", e)))?;
                    }
                } else {
                    dom.set_attribute(element, &name, &v)
                        .map_err(|e| self.err(format!("failed to set attribute: {:?}", e)))?;
                }
            }
            None => {
                // Boolean attribute.
                dom.set_attribute(element, &name, "")
                    .map_err(|e| self.err(format!("failed to set attribute: {:?}", e)))?;
            }
        }
        Ok(())
    }

    fn parse_attr_name(&mut self) -> Result<String> {
        let start = self.pos;
        while let Some(b) = self.peek() {
            if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b':' {
                self.advance();
            } else {
                break;
            }
        }
        if start == self.pos {
            return Err(self.err("expected attribute name"));
        }
        Ok(self.src[start..self.pos].to_string())
    }

    fn parse_attr_value(&mut self) -> Result<String> {
        let first = self.peek();
        match first {
            Some(b'"') => self.parse_quoted(b'"'),
            Some(b'\'') => self.parse_quoted(b'\''),
            Some(_) => self.parse_unquoted(),
            None => Err(self.err("unexpected EOF in attribute value")),
        }
    }

    fn parse_quoted(&mut self, quote: u8) -> Result<String> {
        self.advance(); // opening quote
        let mut out = String::new();
        loop {
            // Collect consecutive raw bytes up to the next quote or &.
            let slice_start = self.pos;
            while let Some(b) = self.peek() {
                if b == quote || b == b'&' {
                    break;
                }
                self.advance();
            }
            if slice_start < self.pos {
                out.push_str(&self.src[slice_start..self.pos]);
            }
            match self.peek() {
                None => {
                    return Err(self
                        .err(format!(
                            "unterminated attribute value (expected `{}`)",
                            quote as char
                        ))
                        .with_hint("missing closing quote"));
                }
                Some(b) if b == quote => {
                    self.advance();
                    return Ok(out);
                }
                Some(b'&') => {
                    out.push_str(&self.parse_entity()?);
                }
                _ => unreachable!(),
            }
        }
    }

    fn parse_unquoted(&mut self) -> Result<String> {
        let mut out = String::new();
        loop {
            let slice_start = self.pos;
            while let Some(b) = self.peek() {
                if b.is_ascii_whitespace() || b == b'>' || b == b'/' || b == b'&' {
                    break;
                }
                self.advance();
            }
            if slice_start < self.pos {
                out.push_str(&self.src[slice_start..self.pos]);
            }
            match self.peek() {
                Some(b'&') => out.push_str(&self.parse_entity()?),
                _ => break,
            }
        }
        if out.is_empty() {
            return Err(self
                .err("empty unquoted attribute value")
                .with_hint("use \"\" or '' for empty value"));
        }
        Ok(out)
    }

    // ── Snapshots (for entity recovery) ─────────────────────────

    fn snapshot(&self) -> (usize, u32, u32) {
        (self.pos, self.line, self.col)
    }

    fn restore(&mut self, (pos, line, col): (usize, u32, u32)) {
        self.pos = pos;
        self.line = line;
        self.col = col;
    }
}

// ─── Entity decoding ────────────────────────────────────────────────

/// Decode an entity body (chars between `&` and `;`). Returns `None`
/// for unrecognized input — caller emits the `&` literal.
fn decode_entity_body(body: &str) -> Option<String> {
    match body {
        "amp" => Some("&".to_string()),
        "lt" => Some("<".to_string()),
        "gt" => Some(">".to_string()),
        "quot" => Some("\"".to_string()),
        "apos" => Some("'".to_string()),
        "nbsp" => Some("\u{00A0}".to_string()),
        _ => {
            if let Some(rest) = body.strip_prefix('#') {
                if let Some(hex) = rest.strip_prefix('x').or_else(|| rest.strip_prefix('X')) {
                    let n = u32::from_str_radix(hex, 16).ok()?;
                    let c = char::from_u32(n)?;
                    Some(c.to_string())
                } else {
                    let n: u32 = rest.parse().ok()?;
                    let c = char::from_u32(n)?;
                    Some(c.to_string())
                }
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(s: &str) -> (Dom<()>, Vec<NodeId>) {
        parse(s).unwrap()
    }

    // ── Basic elements ───────────────────────────────────────────────

    #[test]
    fn empty_element() {
        let (dom, ids) = parse_str("<div></div>");
        assert_eq!(ids.len(), 1);
        let n = dom.node(ids[0]);
        assert_eq!(n.tag_name(), Some("div"));
        assert_eq!(n.child_nodes().count(), 0);
    }

    #[test]
    fn self_closing_element() {
        let (dom, ids) = parse_str("<br/>");
        assert_eq!(ids.len(), 1);
        assert_eq!(dom.node(ids[0]).tag_name(), Some("br"));
    }

    #[test]
    fn self_closing_with_space() {
        let (dom, ids) = parse_str("<br />");
        assert_eq!(dom.node(ids[0]).tag_name(), Some("br"));
    }

    #[test]
    fn void_element_auto_closes() {
        // `<br>` without `/>` still treated as void.
        let (dom, ids) = parse_str("<br>");
        assert_eq!(ids.len(), 1);
        assert_eq!(dom.node(ids[0]).tag_name(), Some("br"));
    }

    #[test]
    fn multiple_void_elements() {
        let (dom, ids) = parse_str("<br><hr><img>");
        assert_eq!(ids.len(), 3);
        assert_eq!(dom.node(ids[0]).tag_name(), Some("br"));
        assert_eq!(dom.node(ids[1]).tag_name(), Some("hr"));
        assert_eq!(dom.node(ids[2]).tag_name(), Some("img"));
    }

    #[test]
    fn case_insensitive_tag_names() {
        let (dom, ids) = parse_str("<DIV></div>");
        assert_eq!(dom.node(ids[0]).tag_name(), Some("div"));
    }

    // ── Nested elements ──────────────────────────────────────────────

    #[test]
    fn nested_elements() {
        let (dom, ids) = parse_str("<div><span></span></div>");
        let outer = ids[0];
        assert_eq!(dom.node(outer).child_nodes().count(), 1);
        let inner = dom.node(outer).first_child().unwrap().id();
        assert_eq!(dom.node(inner).tag_name(), Some("span"));
    }

    #[test]
    fn deeply_nested() {
        let (dom, ids) = parse_str("<a><b><c><d></d></c></b></a>");
        let mut cur = ids[0];
        for tag in &["a", "b", "c", "d"] {
            assert_eq!(dom.node(cur).tag_name(), Some(*tag));
            cur = dom.node(cur).first_child().map(|n| n.id()).unwrap_or(cur);
        }
    }

    // ── Text content ─────────────────────────────────────────────────

    #[test]
    fn text_node() {
        let (dom, ids) = parse_str("<div>hello</div>");
        let child = dom.node(ids[0]).first_child().unwrap();
        assert_eq!(child.node_value(), Some("hello"));
    }

    #[test]
    fn mixed_content() {
        let (dom, ids) = parse_str("<div>before <b>mid</b> after</div>");
        let div = ids[0];
        let children: Vec<_> = dom.node(div).child_nodes().collect();
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].node_value(), Some("before "));
        assert_eq!(children[1].tag_name(), Some("b"));
        assert_eq!(children[2].node_value(), Some(" after"));
    }

    #[test]
    fn text_at_top_level() {
        let (dom, ids) = parse_str("hello <span>world</span>");
        assert_eq!(ids.len(), 2);
        let root = dom.root();
        let first = dom.node(root).first_child().unwrap();
        assert_eq!(first.node_value(), Some("hello "));
    }

    // ── Attributes ──────────────────────────────────────────────────

    #[test]
    fn double_quoted_attr() {
        let (dom, ids) = parse_str(r#"<div id="main"></div>"#);
        assert_eq!(dom.node(ids[0]).get_attribute("id"), Some("main"));
    }

    #[test]
    fn single_quoted_attr() {
        let (dom, ids) = parse_str("<div id='main'></div>");
        assert_eq!(dom.node(ids[0]).get_attribute("id"), Some("main"));
    }

    #[test]
    fn unquoted_attr() {
        let (dom, ids) = parse_str("<div id=main></div>");
        assert_eq!(dom.node(ids[0]).get_attribute("id"), Some("main"));
    }

    #[test]
    fn boolean_attr() {
        let (dom, ids) = parse_str("<input disabled>");
        assert_eq!(dom.node(ids[0]).get_attribute("disabled"), Some(""));
        assert!(dom.node(ids[0]).has_attribute("disabled"));
    }

    #[test]
    fn multiple_attrs() {
        let (dom, ids) = parse_str(r#"<div id="x" role="banner" data-n="5"></div>"#);
        let n = dom.node(ids[0]);
        assert_eq!(n.get_attribute("id"), Some("x"));
        assert_eq!(n.get_attribute("role"), Some("banner"));
        assert_eq!(n.get_attribute("data-n"), Some("5"));
    }

    #[test]
    fn class_attr_populates_classlist() {
        let (dom, ids) = parse_str(r#"<div class="a b c"></div>"#);
        let n = dom.node(ids[0]);
        assert!(n.has_class("a"));
        assert!(n.has_class("b"));
        assert!(n.has_class("c"));
    }

    #[test]
    fn attr_name_case_preserved() {
        // Unlike tag names, we preserve attribute name case.
        let (dom, ids) = parse_str(r#"<div dataFoo="bar"></div>"#);
        assert_eq!(dom.node(ids[0]).get_attribute("dataFoo"), Some("bar"));
    }

    #[test]
    fn whitespace_around_attrs() {
        let (dom, ids) = parse_str("<div  id=main  role=banner  ></div>");
        assert_eq!(dom.node(ids[0]).get_attribute("id"), Some("main"));
        assert_eq!(dom.node(ids[0]).get_attribute("role"), Some("banner"));
    }

    #[test]
    fn attr_name_with_hyphens_and_colons() {
        let (dom, ids) = parse_str(r#"<div data-x="1" aria:label="y"></div>"#);
        assert_eq!(dom.node(ids[0]).get_attribute("data-x"), Some("1"));
        assert_eq!(dom.node(ids[0]).get_attribute("aria:label"), Some("y"));
    }

    // ── Entities ─────────────────────────────────────────────────────

    #[test]
    fn entity_amp() {
        let (dom, ids) = parse_str("<div>a &amp; b</div>");
        let child = dom.node(ids[0]).first_child().unwrap();
        assert_eq!(child.node_value(), Some("a & b"));
    }

    #[test]
    fn entity_lt_gt_quot_apos() {
        let (dom, ids) = parse_str("<div>&lt;tag&gt; &quot;q&quot; &apos;a&apos;</div>");
        let child = dom.node(ids[0]).first_child().unwrap();
        assert_eq!(child.node_value(), Some("<tag> \"q\" 'a'"));
    }

    #[test]
    fn entity_decimal_numeric() {
        let (dom, ids) = parse_str("<div>&#65;&#66;</div>");
        let child = dom.node(ids[0]).first_child().unwrap();
        assert_eq!(child.node_value(), Some("AB"));
    }

    #[test]
    fn entity_hex_numeric() {
        let (dom, ids) = parse_str("<div>&#x41;&#X42;</div>");
        let child = dom.node(ids[0]).first_child().unwrap();
        assert_eq!(child.node_value(), Some("AB"));
    }

    #[test]
    fn entity_in_attr_value() {
        let (dom, ids) = parse_str(r#"<div title="a &amp; b"></div>"#);
        assert_eq!(dom.node(ids[0]).get_attribute("title"), Some("a & b"));
    }

    #[test]
    fn unknown_entity_preserved_as_literal_amp() {
        // `&unknown;` → '&' literal + "unknown;" as text
        let (dom, ids) = parse_str("<div>&xyz;</div>");
        let child = dom.node(ids[0]).first_child().unwrap();
        // We emit '&' and leave the rest to parse as text.
        assert_eq!(child.node_value(), Some("&xyz;"));
    }

    #[test]
    fn entity_nbsp() {
        let (dom, ids) = parse_str("<div>a&nbsp;b</div>");
        let child = dom.node(ids[0]).first_child().unwrap();
        assert_eq!(child.node_value(), Some("a\u{A0}b"));
    }

    // ── Comments ─────────────────────────────────────────────────────

    #[test]
    fn comment_preserved() {
        let (dom, ids) = parse_str("<!-- hello -->");
        assert_eq!(ids.len(), 1);
        let c = dom.node(ids[0]);
        assert_eq!(c.node_type(), rdom_core::NodeType::Comment);
        assert_eq!(c.data(), Some(" hello "));
    }

    #[test]
    fn comment_inside_element() {
        let (dom, ids) = parse_str("<div><!-- note -->body</div>");
        let div = ids[0];
        let children: Vec<_> = dom.node(div).child_nodes().collect();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].node_type(), rdom_core::NodeType::Comment);
        assert_eq!(children[1].node_value(), Some("body"));
    }

    // ── Errors ───────────────────────────────────────────────────────

    #[test]
    fn error_mismatched_tags() {
        let err = parse::<()>("<div></span>").unwrap_err();
        assert!(err.msg.contains("mismatched"));
    }

    #[test]
    fn error_missing_close() {
        let err = parse::<()>("<div>").unwrap_err();
        assert!(err.msg.contains("missing closing"));
    }

    #[test]
    fn error_unterminated_comment() {
        let err = parse::<()>("<!-- never ends").unwrap_err();
        assert!(err.msg.contains("unterminated"));
    }

    #[test]
    fn error_unterminated_attr_value() {
        let err = parse::<()>(r#"<div id="abc>"#).unwrap_err();
        assert!(err.msg.contains("unterminated"));
    }

    #[test]
    fn error_position_reported() {
        let err = parse::<()>("<div>\n<span></p>\n</div>").unwrap_err();
        // Mismatched </p> is on line 2.
        assert_eq!(err.line, 2);
    }

    #[test]
    fn error_has_hint() {
        let err = parse::<()>("<div>").unwrap_err();
        assert!(err.hint.is_some());
    }

    // ── parse_into API ───────────────────────────────────────────────

    #[test]
    fn parse_into_appends_to_mount() {
        let mut dom: Dom<()> = Dom::new();
        let mount = dom.create_element("body");
        let root = dom.root();
        dom.append_child(root, mount).unwrap();

        let ids = parse_into(&mut dom, "<h1>Title</h1><p>Body</p>", mount).unwrap();
        assert_eq!(ids.len(), 2);
        assert_eq!(dom.node(mount).child_nodes().count(), 2);
    }

    // ── Complex templates ────────────────────────────────────────────

    #[test]
    fn realistic_template() {
        let t = r#"
            <div class="card" id="hero">
              <h1>Welcome</h1>
              <p>Hello &amp; welcome to <strong>rdom</strong>.</p>
              <br/>
              <!-- TODO: add icon -->
              <button disabled>OK</button>
            </div>
        "#;
        let (dom, ids) = parse::<()>(t).unwrap();
        // Top-level: the outer div (plus potentially whitespace-only
        // text around it — we preserve all whitespace).
        let div_id = ids
            .iter()
            .find(|&&id| dom.node(id).tag_name() == Some("div"))
            .copied()
            .unwrap();
        let div = dom.node(div_id);
        assert!(div.has_class("card"));
        assert_eq!(div.get_attribute("id"), Some("hero"));

        // Find <h1> inside.
        let h1 = div
            .child_nodes()
            .find(|c| c.tag_name() == Some("h1"))
            .unwrap();
        assert_eq!(
            dom.node(h1.id()).first_child().unwrap().node_value(),
            Some("Welcome")
        );

        // The <button disabled> element.
        let btn = div
            .child_nodes()
            .find(|c| c.tag_name() == Some("button"))
            .unwrap();
        assert!(dom.node(btn.id()).has_attribute("disabled"));
    }

    // ── Round-trip ───────────────────────────────────────────────────

    #[test]
    fn round_trip_simple() {
        let src = "<div><span>hi</span></div>";
        let (dom, ids) = parse::<()>(src).unwrap();
        let out = dom.outer_markup(ids[0]);
        assert_eq!(out, src);
    }

    #[test]
    fn round_trip_with_attrs() {
        let src = r#"<div data-x="1" id="main"><p></p></div>"#;
        let (dom, ids) = parse::<()>(src).unwrap();
        let out = dom.outer_markup(ids[0]);
        // Attributes sort alphabetically in outer_markup, matching input order.
        assert_eq!(out, src);
    }

    #[test]
    fn round_trip_void_element() {
        let src = "<hr/>";
        let (dom, ids) = parse::<()>(src).unwrap();
        let out = dom.outer_markup(ids[0]);
        assert_eq!(out, "<hr/>");
    }

    #[test]
    fn round_trip_entities_escaped() {
        let src = "<div>a &amp; b &lt;c&gt;</div>";
        let (dom, ids) = parse::<()>(src).unwrap();
        let out = dom.outer_markup(ids[0]);
        assert_eq!(out, src);
    }

    // ── Whitespace preservation ──────────────────────────────────────

    #[test]
    fn whitespace_preserved_in_text() {
        let (dom, ids) = parse_str("<p>  hello   world  </p>");
        let child = dom.node(ids[0]).first_child().unwrap();
        assert_eq!(child.node_value(), Some("  hello   world  "));
    }

    #[test]
    fn newlines_preserved() {
        let (dom, ids) = parse_str("<pre>line1\nline2</pre>");
        let child = dom.node(ids[0]).first_child().unwrap();
        assert_eq!(child.node_value(), Some("line1\nline2"));
    }

    // ── Many children ────────────────────────────────────────────────

    #[test]
    fn many_children() {
        let src: String = (0..50).map(|_| "<li>x</li>").collect();
        let (dom, ids) = parse::<()>(&format!("<ul>{}</ul>", src)).unwrap();
        let ul = ids[0];
        assert_eq!(dom.node(ul).child_element_count(), 50);
    }

    // ── Empty template ───────────────────────────────────────────────

    #[test]
    fn empty_template() {
        let (_, ids) = parse_str("");
        assert!(ids.is_empty());
    }

    #[test]
    fn whitespace_only_template() {
        let (dom, ids) = parse_str("   \n  ");
        // A single text node containing the whitespace.
        assert_eq!(ids.len(), 1);
        let c = dom.node(ids[0]);
        assert_eq!(c.node_type(), rdom_core::NodeType::Text);
    }

    // ── Tag name chars ───────────────────────────────────────────────

    #[test]
    fn hyphenated_tag() {
        let (dom, ids) = parse_str("<tree-item></tree-item>");
        assert_eq!(dom.node(ids[0]).tag_name(), Some("tree-item"));
    }

    #[test]
    fn underscore_tag() {
        let (dom, ids) = parse_str("<my_element></my_element>");
        assert_eq!(dom.node(ids[0]).tag_name(), Some("my_element"));
    }

    // ── Siblings + lack of whitespace ────────────────────────────────

    #[test]
    fn adjacent_elements() {
        let (dom, ids) = parse_str("<a></a><b></b>");
        assert_eq!(ids.len(), 2);
        assert_eq!(dom.node(ids[0]).tag_name(), Some("a"));
        assert_eq!(dom.node(ids[1]).tag_name(), Some("b"));
    }
}
