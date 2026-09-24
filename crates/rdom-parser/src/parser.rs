//! Recursive-descent HTML-ish parser.
//!
//! Consumes a template string, emits `Dom<Ext>` tree under a mount
//! NodeId. Supports:
//!
//! - Start tags (`<tag>`), end tags (`</tag>`), self-closing (`<br/>`)
//! - Void elements (`<br>`, `<hr>`, `<img>`, …) auto-close without `/>`
//! - Attributes: `name="value"` / `name='value'` / `name=value` / `name`
//! - Text with entity decoding: the common named references
//!   (`&amp; &lt; &copy; &mdash; …`) plus `&#NNN;` / `&#xHH;`; U+0000,
//!   surrogates and out-of-range code points decode to U+FFFD
//! - A `<` not followed by an ASCII letter, `/`, `!`, or `?` is text
//!   (HTML §13.2.5.6 tag-open state), so `a < b` needs no escaping
//! - `<style>` / `<script>` bodies are RAWTEXT (no tags, no entities)
//!   and `<textarea>` / `<title>` bodies are RCDATA (entities only),
//!   each ending at its own case-insensitive end tag (HTML §13.2.5.3–6)
//! - Comments: `<!-- … -->` preserved as Comment nodes
//! - `<!DOCTYPE …>` is consumed and produces no node; any other `<!…>`
//!   or `<?…>` is a bogus comment kept as a Comment node (§13.2.5.41)
//! - A newline right after `<textarea>` is dropped (§13.2.6.4.7)
//! - Case-insensitive tag names (tags are normalized to lowercase)
//!
//! Out of scope: CDATA, namespace prefixes, processing instructions,
//! tree-construction error recovery (a mismatched or missing end tag is
//! a hard error — this is a template parser, not a browser).

use rdom_core::{Dom, NodeId, is_void_element};

use crate::entities::{LONGEST_LEGACY_NAME, LONGEST_NAME, NAMED_REFERENCES};

use crate::error::{ParseError, Result};

/// Where a character reference sits. HTML §13.2.5.73 leaves a legacy
/// no-semicolon reference literal inside an attribute value when `=` or
/// an alphanumeric follows it (`?a=1&copy=2`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum RefContext {
    Text,
    Attribute,
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
    if !p.eof() {
        // `parse_nodes` stops at `</…`; at the top level nothing is open,
        // so a stray end tag is an error rather than silent truncation.
        return Err(p
            .err("unexpected closing tag at top level")
            .with_hint("nothing is open here — remove the end tag or open its element"));
    }
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
            if self.starts_with("<!") {
                if self.src[self.pos + 2..]
                    .get(..7)
                    .is_some_and(|k| k.eq_ignore_ascii_case("DOCTYPE"))
                {
                    // `<!DOCTYPE html>`: consumed, no node (rdom has no
                    // DocumentType node — see DIVERGENCES §HTML parsing).
                    self.skip_declaration();
                } else {
                    // Any other `<!…>` is a bogus comment (§13.2.5.42).
                    let id = self.parse_bogus_comment(dom, parent)?;
                    out.push(id);
                }
                continue;
            }
            if self.starts_with("<?") {
                // `<?…>` is a bogus comment (§13.2.5.6 "?" branch).
                let id = self.parse_bogus_comment(dom, parent)?;
                out.push(id);
                continue;
            }
            if self.peek() == Some(b'<') && self.peek_at(1).is_some_and(|b| b.is_ascii_alphabetic())
            {
                let id = self.parse_element(dom, parent)?;
                out.push(id);
                continue;
            }
            // Plain text until the next tag open. A `<` not followed by
            // an ASCII letter, `/`, `!`, or `?` is text (§13.2.5.6).
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
                if b == b'&' || (b == b'<' && self.at_tag_open()) {
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
                    out.push_str(&self.parse_entity(RefContext::Text)?);
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

    /// Is the `<` at the cursor a real tag open (HTML §13.2.5.6)? Only
    /// when followed by an ASCII letter, `/`, `!`, or `?`.
    fn at_tag_open(&self) -> bool {
        self.peek() == Some(b'<')
            && self
                .peek_at(1)
                .is_some_and(|b| b.is_ascii_alphabetic() || matches!(b, b'/' | b'!' | b'?'))
    }

    /// HTML §13.2.5.41 bogus comment state: everything from just after
    /// the `<` up to the next `>` becomes a Comment node's data
    /// (`<?xml version="1.0"?>` → `?xml version="1.0"?`).
    fn parse_bogus_comment<Ext>(&mut self, dom: &mut Dom<Ext>, parent: NodeId) -> Result<NodeId>
    where
        Ext: Default + 'static,
    {
        self.advance(); // '<'
        let start = self.pos;
        while let Some(b) = self.peek() {
            if b == b'>' {
                break;
            }
            self.advance();
        }
        let data = self.src[start..self.pos].to_string();
        if self.peek() == Some(b'>') {
            self.advance();
        }
        let id = dom.create_comment(&data);
        dom.append_child(parent, id)
            .map_err(|e| self.err(format!("failed to append comment: {:?}", e)))?;
        Ok(id)
    }

    /// Consume a `<!DOCTYPE …>` declaration through its closing `>`, or
    /// to EOF.
    fn skip_declaration(&mut self) {
        while let Some(b) = self.advance() {
            if b == b'>' {
                return;
            }
        }
    }

    /// Byte offset of the `</tag` (ASCII-case-insensitive, followed by
    /// whitespace, `/`, or `>`) that ends a RAWTEXT / RCDATA element,
    /// searching from the cursor. `None` at EOF.
    fn find_end_tag(&self, tag_lc: &str) -> Option<usize> {
        let hay = &self.bytes[self.pos..];
        let needle_len = 2 + tag_lc.len();
        let mut i = 0;
        while i + needle_len <= hay.len() {
            if hay[i] == b'<' && hay[i + 1] == b'/' {
                let name = &hay[i + 2..i + needle_len];
                if name.eq_ignore_ascii_case(tag_lc.as_bytes()) {
                    let after = hay.get(i + needle_len).copied();
                    if after.is_none_or(|b| b.is_ascii_whitespace() || b == b'>' || b == b'/') {
                        return Some(self.pos + i);
                    }
                }
            }
            i += 1;
        }
        None
    }

    /// Parse the body of a RAWTEXT (`<style>`, `<script>`) or RCDATA
    /// (`<textarea>`, `<title>`) element as a single text node, then
    /// consume its end tag. RCDATA decodes character references; RAWTEXT
    /// takes the bytes verbatim (HTML §13.2.5.3–6).
    fn parse_special_text<Ext>(
        &mut self,
        dom: &mut Dom<Ext>,
        element: NodeId,
        tag_lc: &str,
        decode_entities: bool,
    ) -> Result<()>
    where
        Ext: Default + 'static,
    {
        let Some(end) = self.find_end_tag(tag_lc) else {
            return Err(self
                .err(format!("missing closing tag for <{}>", tag_lc))
                .with_hint(format!("add </{}> to close", tag_lc)));
        };
        let mut raw = &self.src[self.pos..end];
        // HTML §13.2.6.4.7: a newline immediately after `<textarea>` is
        // ignored (the same rule HTML applies to `<pre>` / `<listing>`).
        if tag_lc == "textarea" {
            raw = raw
                .strip_prefix("\r\n")
                .or_else(|| raw.strip_prefix('\n'))
                .unwrap_or(raw);
        }
        let text = if decode_entities {
            decode_character_references(raw)
        } else {
            raw.to_string()
        };
        if !text.is_empty() {
            let id = dom.create_text_node(&text);
            dom.append_child(element, id)
                .map_err(|e| self.err(format!("failed to append text: {:?}", e)))?;
        }
        // Walk (not jump) so line / column stay right for later errors.
        self.advance_n(end - self.pos);
        self.advance_n(2 + tag_lc.len()); // `</tag`
        self.skip_ws();
        if self.peek() != Some(b'>') {
            return Err(self
                .err(format!("expected `>` in </{}>", tag_lc))
                .with_hint("no attributes on closing tags"));
        }
        self.advance();
        Ok(())
    }

    // ── Entity ─────────────────────────────────────────────────────

    fn parse_entity(&mut self, context: RefContext) -> Result<String> {
        // We've seen '&'. One scanner for the text path, the RCDATA path
        // and attribute values (`scan_reference`), so all agree on what a
        // reference looks like; unknown or malformed input keeps the `&`
        // literal (as browsers flush the raw characters).
        debug_assert_eq!(self.peek(), Some(b'&'));
        match scan_reference(&self.src[self.pos + 1..], context) {
            Some((decoded, consumed)) => {
                self.advance_n(1 + consumed);
                Ok(decoded)
            }
            None => {
                self.advance(); // consume the '&'
                Ok("&".to_string())
            }
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
        if is_void_element(&tag_lc) {
            dom.append_child(parent, element)
                .map_err(|e| self.err(format!("failed to append <{}>: {:?}", tag_lc, e)))?;
            return Ok(element);
        }

        // RAWTEXT / RCDATA elements take their body as one text node
        // up to their own end tag.
        match tag_lc.as_str() {
            "style" | "script" => {
                self.parse_special_text(dom, element, &tag_lc, false)?;
                dom.append_child(parent, element)
                    .map_err(|e| self.err(format!("failed to append <{}>: {:?}", tag_lc, e)))?;
                return Ok(element);
            }
            "textarea" | "title" => {
                self.parse_special_text(dom, element, &tag_lc, true)?;
                dom.append_child(parent, element)
                    .map_err(|e| self.err(format!("failed to append <{}>: {:?}", tag_lc, e)))?;
                return Ok(element);
            }
            _ => {}
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
                    out.push_str(&self.parse_entity(RefContext::Attribute)?);
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
                Some(b'&') => out.push_str(&self.parse_entity(RefContext::Attribute)?),
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
}

// ─── Entity decoding ────────────────────────────────────────────────

/// HTML §13.2.5.80: the C1 control range 0x80–0x9F is remapped to the
/// Windows-1252 code points (`&#146;` is `’`, `&#150;` is `–`); the five
/// unmapped positions (0x81, 0x8D, 0x8F, 0x90, 0x9D) pass through.
const C1_REMAP: [(u32, char); 27] = [
    (0x80, '\u{20AC}'),
    (0x82, '\u{201A}'),
    (0x83, '\u{0192}'),
    (0x84, '\u{201E}'),
    (0x85, '\u{2026}'),
    (0x86, '\u{2020}'),
    (0x87, '\u{2021}'),
    (0x88, '\u{02C6}'),
    (0x89, '\u{2030}'),
    (0x8A, '\u{0160}'),
    (0x8B, '\u{2039}'),
    (0x8C, '\u{0152}'),
    (0x8E, '\u{017D}'),
    (0x91, '\u{2018}'),
    (0x92, '\u{2019}'),
    (0x93, '\u{201C}'),
    (0x94, '\u{201D}'),
    (0x95, '\u{2022}'),
    (0x96, '\u{2013}'),
    (0x97, '\u{2014}'),
    (0x98, '\u{02DC}'),
    (0x99, '\u{2122}'),
    (0x9A, '\u{0161}'),
    (0x9B, '\u{203A}'),
    (0x9C, '\u{0153}'),
    (0x9E, '\u{017E}'),
    (0x9F, '\u{0178}'),
];

/// Decode the digits of a numeric reference per HTML §13.2.5.80: U+0000,
/// surrogates and values above U+10FFFF (including anything that
/// overflows `u32`) yield U+FFFD; the C1 range is remapped.
fn decode_numeric(digits: &str, radix: u32) -> char {
    let n = u32::from_str_radix(digits, radix).unwrap_or(u32::MAX);
    match n {
        0 | 0xD800..=0xDFFF => '\u{FFFD}',
        0x80..=0x9F => C1_REMAP
            .iter()
            .find(|(from, _)| *from == n)
            .map_or_else(|| char::from_u32(n).unwrap_or('\u{FFFD}'), |(_, to)| *to),
        _ => char::from_u32(n).unwrap_or('\u{FFFD}'),
    }
}

/// Look `name` (without `&`, with or without `;`) up in the WHATWG table.
fn named_reference(name: &str) -> Option<&'static str> {
    NAMED_REFERENCES
        .binary_search_by(|(n, _)| (*n).cmp(name))
        .ok()
        .map(|i| NAMED_REFERENCES[i].1)
}

/// Scan a character reference whose `&` has just been consumed
/// (`after_amp` starts right after it). Returns the decoded text and the
/// number of bytes to consume, or `None` when the caller should keep the
/// `&` literal. HTML §13.2.5.72–80:
///
/// - numeric: `#` + digits (or `#x` + hex digits), an optional `;`;
/// - named: the *longest* table prefix wins, so `&notit;` is `¬it;`
///   (the legacy `not`) and `&notin;` is `∉`;
/// - a legacy (no-`;`) match inside an attribute value is left literal
///   when the next character is `=` or alphanumeric, so query strings
///   like `?a=1&copy=2` survive.
fn scan_reference(after_amp: &str, context: RefContext) -> Option<(String, usize)> {
    let bytes = after_amp.as_bytes();
    if bytes.first() == Some(&b'#') {
        // Decimal / hexadecimal character reference states consume digits
        // of their radix only; whatever follows (`;` or not) is left for
        // the caller (`&#65abc;` → `A` + `abc;`).
        let (start, radix): (usize, u32) = match bytes.get(1) {
            Some(b'x' | b'X') => (2, 16),
            _ => (1, 10),
        };
        let mut n = start;
        while n < bytes.len() && (bytes[n] as char).is_digit(radix) {
            n += 1;
        }
        if n == start {
            return None;
        }
        let c = decode_numeric(&after_amp[start..n], radix);
        let consumed = if bytes.get(n) == Some(&b';') {
            n + 1
        } else {
            n
        };
        return Some((c.to_string(), consumed));
    }
    let mut n = 0;
    while n < bytes.len() && n < LONGEST_NAME && bytes[n].is_ascii_alphanumeric() {
        n += 1;
    }
    if n == 0 {
        return None;
    }
    if bytes.get(n) == Some(&b';')
        && let Some(v) = named_reference(&after_amp[..=n])
    {
        return Some((v.to_string(), n + 1));
    }
    // Only legacy names can match without `;`, and they are short.
    for len in (1..=n.min(LONGEST_LEGACY_NAME)).rev() {
        let Some(v) = named_reference(&after_amp[..len]) else {
            continue;
        };
        if context == RefContext::Attribute
            && let Some(&next) = bytes.get(len)
            && (next == b'=' || next.is_ascii_alphanumeric())
        {
            return None;
        }
        return Some((v.to_string(), len));
    }
    None
}

/// Decode every character reference in `text` (RCDATA bodies) with the
/// same scanner the text path uses.
fn decode_character_references(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let after = &rest[amp + 1..];
        match scan_reference(after, RefContext::Text) {
            Some((decoded, consumed)) => {
                out.push_str(&decoded);
                rest = &after[consumed..];
            }
            None => {
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(s: &str) -> (Dom<()>, Vec<NodeId>) {
        parse(s).unwrap()
    }

    /// `named_reference` binary-searches the generated table, so it must
    /// stay byte-sorted and free of duplicates, and it must be the whole
    /// WHATWG list (2 231 entries, 106 legacy names without `;`).
    /// Numeric edge cases from HTML §13.2.5.75–80: no digits → literal;
    /// a missing `;` still decodes; the scan stops at the first non-digit.
    #[test]
    fn numeric_reference_scanner_edges() {
        let text = |s: &str| scan_reference(s, RefContext::Text);
        assert_eq!(text("#;x"), None);
        assert_eq!(text("#x;"), None);
        assert_eq!(text("#"), None);
        assert_eq!(text("#65 rest"), Some(("A".to_string(), 3)));
        assert_eq!(text("#65;"), Some(("A".to_string(), 4)));
        assert_eq!(text("#65abc;"), Some(("A".to_string(), 3)));
        assert_eq!(text("#x41g"), Some(("A".to_string(), 4)));
        assert_eq!(text("#X41;"), Some(("A".to_string(), 5)));
        assert_eq!(text("#150;"), Some(("\u{2013}".to_string(), 5)));
        assert_eq!(text("#129;"), Some(("\u{81}".to_string(), 5)));
    }

    /// Named edge cases: longest prefix, case sensitivity, the attribute
    /// caveat only for legacy names, and the legacy-length bound.
    #[test]
    fn named_reference_scanner_edges() {
        let text = |s: &str| scan_reference(s, RefContext::Text);
        let attr = |s: &str| scan_reference(s, RefContext::Attribute);
        assert_eq!(text("notit;"), Some(("\u{AC}".to_string(), 3)));
        assert_eq!(text("notin;"), Some(("\u{2209}".to_string(), 6)));
        assert_eq!(text("Amp;"), None, "names are case-sensitive");
        assert_eq!(text("AMP;"), Some(("&".to_string(), 4)));
        assert_eq!(attr("copy=2"), None);
        assert_eq!(attr("copyx"), None);
        assert_eq!(attr("copy 2"), Some(("\u{A9}".to_string(), 4)));
        assert_eq!(attr("copy;=2"), Some(("\u{A9}".to_string(), 5)));
        assert_eq!(text("ThisIsNotAReferenceAtAllButLong;"), None);
    }

    #[test]
    fn named_reference_table_is_the_full_sorted_whatwg_list() {
        assert_eq!(NAMED_REFERENCES.len(), 2231);
        let legacy = NAMED_REFERENCES
            .iter()
            .filter(|(n, _)| !n.ends_with(';'))
            .count();
        assert_eq!(legacy, 106);
        for w in NAMED_REFERENCES.windows(2) {
            assert!(
                w[0].0 < w[1].0,
                "{:?} must sort before {:?}",
                w[0].0,
                w[1].0
            );
        }
        assert!(
            NAMED_REFERENCES
                .iter()
                .all(|(n, _)| n.len() <= LONGEST_NAME)
        );
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
