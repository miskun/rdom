//! RAWTEXT (`<style>`, `<script>`) and RCDATA (`<textarea>`, `<title>`)
//! bodies: one text node up to the element's own end tag (HTML
//! §13.2.5.3–6). Split out of `mod.rs` (C16G-DEPTH-CAPS).

use rdom_core::{Dom, NodeId};

use super::Parser;
use crate::char_refs::decode_character_references;
use crate::error::Result;

impl Parser<'_> {
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
    pub(super) fn parse_special_text<Ext>(
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
}
