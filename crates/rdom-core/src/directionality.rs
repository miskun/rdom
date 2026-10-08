//! The directionality of an element (HTML §3.2.6.4), which `:dir()`
//! matches (Selectors 4 §7.1) — the document's semantics, not the CSS
//! `direction` property.
//!
//! An element's `dir` attribute decides (`ltr`, `rtl`, ASCII
//! case-insensitive); `auto` — and a `<bdi>` without a valid `dir` —
//! takes the direction of the first strong character of its text (or of
//! a `<textarea>`'s / text `<input>`'s value), `ltr` when there is none;
//! an `<input type=tel>` without one is `ltr`; any other element has its
//! parent element's directionality, the root `ltr`.
//!
//! "Strong" is the Unicode bidirectional class L, R or AL (UAX #9). rdom
//! carries no Bidi_Class table: a character is strong when it is
//! alphabetic (or LRM / RLM), and right-to-left when it lies in the
//! Hebrew, Arabic, Syriac, Thaana, NKo, Samaritan or Mandaic blocks or
//! their presentation forms and supplements (DIVERGENCES §2).

use std::collections::HashMap;

use crate::dom::Dom;
use crate::input_type::InputTypeState;
use crate::node::NodeData;
use crate::node_id::NodeId;

/// An element's directionality (HTML §3.2.6.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Directionality {
    /// Left to right.
    Ltr,
    /// Right to left.
    Rtl,
}

impl<Ext> Dom<Ext> {
    /// The directionality of the element `id` (HTML §3.2.6.4; module
    /// doc). A node that is not an element has its parent's.
    pub fn directionality(&self, id: NodeId) -> Directionality {
        self.directionality_in(id, &mut HashMap::new())
    }

    /// [`Self::directionality`], memoized in `memo` for `id` and every
    /// ancestor the answer was inherited through.
    pub(crate) fn directionality_in(
        &self,
        id: NodeId,
        memo: &mut HashMap<NodeId, Directionality>,
    ) -> Directionality {
        let mut inherit = Vec::new();
        let mut cur = Some(id);
        let found = loop {
            let Some(c) = cur else {
                break Directionality::Ltr;
            };
            if let Some(&known) = memo.get(&c) {
                break known;
            }
            if let Some(own) = self.own_directionality(c) {
                memo.insert(c, own);
                break own;
            }
            inherit.push(c);
            cur = self.get_node(c).and_then(|n| n.parent);
        };
        for c in inherit {
            memo.insert(c, found);
        }
        found
    }

    /// The directionality `id`'s own attributes decide, `None` when it
    /// inherits its parent's.
    fn own_directionality(&self, id: NodeId) -> Option<Directionality> {
        let NodeData::Element { tag, attrs, .. } = &self.get_node(id)?.data else {
            return None;
        };
        match valid_dir(attrs.get("dir").map(String::as_str)) {
            Some(Dir::Ltr) => Some(Directionality::Ltr),
            Some(Dir::Rtl) => Some(Directionality::Rtl),
            Some(Dir::Auto) => Some(self.auto_directionality(id)),
            None if tag == "bdi" => Some(self.auto_directionality(id)),
            None if self.input_type_state(id) == Some(InputTypeState::Tel) => {
                Some(Directionality::Ltr)
            }
            None => None,
        }
    }

    /// HTML's "auto directionality": a text control's value, else the
    /// element's contained text; `ltr` without a strong character.
    fn auto_directionality(&self, id: NodeId) -> Directionality {
        let text_value = match self.input_type_state(id) {
            Some(
                InputTypeState::Text
                | InputTypeState::Search
                | InputTypeState::Tel
                | InputTypeState::Url
                | InputTypeState::Email,
            ) => Some(self.get_attribute(id, "value").unwrap_or("").to_string()),
            _ if self.tag_is(id, "textarea") => Some(self.text_content(id)),
            _ => None,
        };
        let found = match text_value {
            Some(value) => first_strong(&value),
            None => self.contained_text_direction(id),
        };
        found.unwrap_or(Directionality::Ltr)
    }

    /// The first strong character of the text below `id` in tree order,
    /// skipping the subtrees of `<bdi>`, `<script>`, `<style>`,
    /// `<textarea>` and elements with a valid `dir`.
    fn contained_text_direction(&self, id: NodeId) -> Option<Directionality> {
        // Tree order: a stack of nodes to visit, children pushed last
        // first.
        let mut stack = Vec::new();
        self.push_children_reversed(id, &mut stack);
        while let Some(c) = stack.pop() {
            match &self.get_node(c)?.data {
                NodeData::Text { data } => {
                    if let Some(found) = first_strong(data) {
                        return Some(found);
                    }
                }
                NodeData::Element { tag, attrs, .. }
                    if !matches!(tag.as_str(), "bdi" | "script" | "style" | "textarea")
                        && valid_dir(attrs.get("dir").map(String::as_str)).is_none() =>
                {
                    self.push_children_reversed(c, &mut stack);
                }
                _ => {}
            }
        }
        None
    }

    fn push_children_reversed(&self, id: NodeId, stack: &mut Vec<NodeId>) {
        let at = stack.len();
        let mut child = self.get_node(id).and_then(|n| n.first_child);
        while let Some(c) = child {
            stack.push(c);
            child = self.get_node(c).and_then(|n| n.next_sibling);
        }
        stack[at..].reverse();
    }
}

/// The `dir` attribute's state (HTML §3.2.6.4: an enumerated attribute,
/// ASCII case-insensitive, an invalid value in no state).
enum Dir {
    Ltr,
    Rtl,
    Auto,
}

fn valid_dir(value: Option<&str>) -> Option<Dir> {
    let value = value?;
    [("ltr", Dir::Ltr), ("rtl", Dir::Rtl), ("auto", Dir::Auto)]
        .into_iter()
        .find(|(k, _)| value.eq_ignore_ascii_case(k))
        .map(|(_, d)| d)
}

/// The direction of the first strong character of `text` (module doc).
fn first_strong(text: &str) -> Option<Directionality> {
    text.chars().find_map(strong)
}

fn strong(c: char) -> Option<Directionality> {
    match c {
        '\u{200E}' => Some(Directionality::Ltr),
        '\u{200F}' => Some(Directionality::Rtl),
        c if !c.is_alphabetic() => None,
        '\u{0590}'..='\u{08FF}'
        | '\u{FB1D}'..='\u{FDFF}'
        | '\u{FE70}'..='\u{FEFF}'
        | '\u{10800}'..='\u{10FFF}'
        | '\u{1E800}'..='\u{1EFFF}' => Some(Directionality::Rtl),
        _ => Some(Directionality::Ltr),
    }
}
