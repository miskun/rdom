//! The `contenteditable` enumerated attribute — HTML §6.8.1. Attribute
//! semantics only; what editing does lives in the backend.
//!
//! Keywords match ASCII case-insensitively (§2.3.3): `true` and the
//! empty string are the *true* state, `false` the *false* state,
//! `plaintext-only` the *plaintext-only* state. A missing or invalid
//! value is the *inherit* state (both defaults are inherit), which
//! [`Dom::content_editable_state`] reports as `None`.

use crate::dom::Dom;
use crate::node_id::NodeId;

/// A `contenteditable` attribute's explicit state (HTML §6.8.1). The
/// inherit state is the absence of one (`None`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContentEditableState {
    /// `true` or `""`: the element is an editing host.
    True,
    /// `false`: the element is not editable, whatever its ancestors are.
    False,
    /// `plaintext-only`: an editing host whose edits are plain text.
    PlaintextOnly,
}

impl ContentEditableState {
    /// Parse an attribute value: the keyword, ASCII case-insensitively;
    /// `None` (inherit) for anything else.
    pub fn from_attribute(value: &str) -> Option<Self> {
        if value.is_empty() || value.eq_ignore_ascii_case("true") {
            Some(Self::True)
        } else if value.eq_ignore_ascii_case("false") {
            Some(Self::False)
        } else if value.eq_ignore_ascii_case("plaintext-only") {
            Some(Self::PlaintextOnly)
        } else {
            None
        }
    }

    /// The canonical keyword — what the `contentEditable` IDL getter
    /// returns for this state.
    pub fn keyword(self) -> &'static str {
        match self {
            Self::True => "true",
            Self::False => "false",
            Self::PlaintextOnly => "plaintext-only",
        }
    }

    /// `true` for the two states that make an element an editing host.
    pub fn is_editing_host(self) -> bool {
        matches!(self, Self::True | Self::PlaintextOnly)
    }
}

impl<Ext> Dom<Ext> {
    /// `id`'s explicit `contenteditable` state, or `None` for the
    /// inherit state (attribute missing or invalid) and for non-elements.
    pub fn content_editable_state(&self, id: NodeId) -> Option<ContentEditableState> {
        self.get_attribute(id, "contenteditable")
            .and_then(ContentEditableState::from_attribute)
    }
}
