//! Attributes of a start tag (HTML §13.2.5.32–§13.2.5.39): names, the
//! three value forms (double-quoted, single-quoted, unquoted) and their
//! character references, added to the element as they are read. Split
//! out of `parser.rs` (C7G-SIZES).

use rdom_core::{Dom, NodeId};

use super::Parser;
use crate::char_refs::RefContext;
use crate::error::Result;

impl Parser<'_> {
    pub(super) fn parse_attribute<Ext>(&mut self, dom: &mut Dom<Ext>, element: NodeId) -> Result<()>
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
