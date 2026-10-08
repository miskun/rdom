//! Building a [`Range`] from two boundary points, checked (DOM §5.5).

use crate::dom::Dom;
use crate::error::DomError;
use crate::node::NodeData;
use crate::node_id::NodeId;
use crate::selection::{Position, Range};

impl<Ext> Dom<Ext> {
    /// A node's length (DOM §4.2): a text or comment node's data length
    /// (bytes, as `Position` offsets are), any other node's child count.
    /// `None` for a freed node.
    pub fn node_length(&self, id: NodeId) -> Option<usize> {
        let node = self.get_node(id)?;
        Some(match &node.data {
            NodeData::Text { data } | NodeData::Comment { data } => data.len(),
            _ => {
                let mut count = 0;
                let mut child = node.first_child;
                while let Some(c) = child {
                    count += 1;
                    child = self.get_node(c).and_then(|n| n.next_sibling);
                }
                count
            }
        })
    }

    /// The range between the boundary points `a` and `b`, in document
    /// order whichever comes first (DOM §5.2) — `document.createRange()`
    /// with `setStart` / `setEnd`, checked as they check each point (DOM
    /// §5.5 "set the start or end"): an offset past the node's
    /// [length](Self::node_length) — or, in a text node, inside a UTF-8
    /// character — is [`DomError::InvalidOffset`] (the web's
    /// `IndexSizeError`), a freed node [`DomError::InvalidNode`]. Where the
    /// web collapses a range whose points are in different trees, two
    /// such points are [`DomError::InvalidState`]: no range holds them.
    pub fn range_between(&self, a: Position, b: Position) -> Result<Range, DomError> {
        self.check_boundary_point(a)?;
        self.check_boundary_point(b)?;
        match self.compare_boundary_points(a, b) {
            Some(std::cmp::Ordering::Greater) => Ok(Range::ordered_unchecked(b, a)),
            Some(_) => Ok(Range::ordered_unchecked(a, b)),
            None => Err(DomError::InvalidState(
                "the boundary points are in different trees",
            )),
        }
    }

    /// DOM §5.5 "set the start or end" step 2: `p`'s offset is within its
    /// node's length (and on a character boundary of a text node's data).
    fn check_boundary_point(&self, p: Position) -> Result<(), DomError> {
        let node = self.get_node(p.node).ok_or(DomError::InvalidNode(p.node))?;
        let fits = match &node.data {
            NodeData::Text { data } | NodeData::Comment { data } => data.is_char_boundary(p.offset),
            _ => self.node_length(p.node).is_some_and(|len| p.offset <= len),
        };
        if fits {
            Ok(())
        } else {
            Err(DomError::InvalidOffset {
                node: p.node,
                offset: p.offset,
            })
        }
    }
}
