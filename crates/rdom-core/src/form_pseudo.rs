//! What the input pseudo-classes match (Selectors 4 §14, with HTML
//! §4.16.3's definitions): the predicates the substrate answers from
//! names, attributes and tree shape. The validity verdicts behind
//! `:valid` / `:invalid` live in `constraint`.

use crate::dom::Dom;
use crate::node_id::NodeId;

impl<Ext> Dom<Ext> {
    /// Whether `id` matches `:read-write` (HTML §4.16.3):
    ///
    /// - an `<input>` that the `readonly` attribute applies to (the
    ///   text, date / time and number states) and that is *mutable* —
    ///   no `readonly` attribute, not [actually
    ///   disabled](Self::is_actually_disabled);
    /// - a `<textarea>` without `readonly` that is not actually disabled;
    /// - any other element that is an [editing host or
    ///   editable](Self::is_editable_or_editing_host).
    ///
    /// `:read-only` matches every other element.
    pub fn is_read_write(&self, id: NodeId) -> bool {
        match self.get_node(id).and_then(|n| n.tag_name()) {
            None => false,
            Some("input") => {
                self.input_type_state(id)
                    .is_some_and(crate::constraint::readonly_applies)
                    && self.is_mutable(id)
            }
            Some("textarea") => self.is_mutable(id),
            Some(_) => self.is_editable_or_editing_host(id),
        }
    }

    /// No `readonly` attribute and not actually disabled.
    fn is_mutable(&self, id: NodeId) -> bool {
        !self.has_attribute(id, "readonly") && !self.is_actually_disabled(id)
    }
}
