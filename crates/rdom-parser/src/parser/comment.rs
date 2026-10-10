//! Comments and declarations: `<!-- … -->` kept as a Comment node, a
//! bogus comment (`<!…>`, `<?…>`, HTML §13.2.5.41) too, and `<!DOCTYPE
//! …>` consumed with no node. Split out of `mod.rs` (C16G-DEPTH-CAPS).

use rdom_core::{Dom, NodeId};

use super::Parser;
use crate::error::Result;

impl Parser<'_> {
    pub(super) fn parse_comment<Ext>(
        &mut self,
        dom: &mut Dom<Ext>,
        parent: NodeId,
    ) -> Result<NodeId>
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

    /// HTML §13.2.5.41 bogus comment state: everything from just after
    /// the `<` up to the next `>` becomes a Comment node's data
    /// (`<?xml version="1.0"?>` → `?xml version="1.0"?`).
    pub(super) fn parse_bogus_comment<Ext>(
        &mut self,
        dom: &mut Dom<Ext>,
        parent: NodeId,
    ) -> Result<NodeId>
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
    pub(super) fn skip_declaration(&mut self) {
        while let Some(b) = self.advance() {
            if b == b'>' {
                return;
            }
        }
    }
}
