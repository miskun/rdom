//! Error types for arena operations.
//!
//! DOM spec defines hierarchy errors as throwable; in Rust we return
//! `Result<_, DomError>` from any operation that can fail. Tests exercise
//! every error variant.

use crate::NodeId;
use crate::node::NodeType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomError {
    /// NodeId doesn't exist in this arena (may have been freed).
    InvalidNode(NodeId),

    /// Would create a cycle: attempted to insert a node into its own
    /// subtree (spec `HierarchyRequestError`).
    HierarchyRequest,

    /// `insert_before` / `replace_child` reference child does not belong
    /// to the target parent (spec `NotFoundError`).
    NotFound,

    /// Operation requires a specific node type — e.g. `set_node_value`
    /// only works on Text and Comment nodes, not Element / Fragment.
    WrongNodeType {
        expected: &'static str,
        got: NodeType,
    },

    /// A byte offset supplied to a text-editing operation either
    /// overshoots the node's data length or falls inside a multi-byte
    /// UTF-8 codepoint (not on a `char` boundary). Callers should
    /// derive offsets from `Position` or grapheme walks to avoid this.
    InvalidOffset { node: NodeId, offset: usize },
}

impl std::fmt::Display for DomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DomError::InvalidNode(id) => write!(f, "invalid node id: {id:?}"),
            DomError::HierarchyRequest => write!(f, "hierarchy request error: would create cycle"),
            DomError::NotFound => write!(f, "reference node not found in target"),
            DomError::WrongNodeType { expected, got } => {
                write!(f, "wrong node type: expected {expected}, got {got:?}")
            }
            DomError::InvalidOffset { node, offset } => {
                write!(
                    f,
                    "invalid byte offset {offset} for node {node:?} (out of range or mid-codepoint)"
                )
            }
        }
    }
}

impl std::error::Error for DomError {}

pub type Result<T> = std::result::Result<T, DomError>;
