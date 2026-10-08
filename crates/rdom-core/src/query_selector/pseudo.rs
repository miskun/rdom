//! The pseudo-classes without an argument (Selectors 4 §8–§14).

use super::matcher::Cx;
use crate::dom::Dom;
use crate::node::NodeData;
use crate::node_id::NodeId;
use crate::selectors::PseudoClass;

impl<Ext> Dom<Ext> {
    pub(super) fn match_pseudo(&self, id: NodeId, p: PseudoClass, cx: &mut Cx<'_>) -> bool {
        let Some(node) = self.get_node(id) else {
            return false;
        };
        match p {
            // Selectors 4 §13.3.3–§13.3.5: Level 4 drops the parent requirement — an
            // element without a parent is its first and last child.
            PseudoClass::FirstChild => self.prev_element_sibling_id(id).is_none(),
            PseudoClass::LastChild => self.next_element_sibling_id(id).is_none(),
            PseudoClass::OnlyChild => {
                self.prev_element_sibling_id(id).is_none()
                    && self.next_element_sibling_id(id).is_none()
            }
            // HTML §4.16.3: `a` / `area` with `href`; never visited.
            PseudoClass::AnyLink | PseudoClass::Link => match &node.data {
                NodeData::Element { tag, attrs, .. } => {
                    matches!(tag.as_str(), "a" | "area") && attrs.contains_key("href")
                }
                _ => false,
            },
            PseudoClass::Visited => false,
            PseudoClass::Dir(dir) => {
                dir.is_some_and(|d| self.directionality_in(id, &mut cx.caches.dir) == d)
            }
            PseudoClass::FirstOfType => self.type_position(id, cx).is_some_and(|(s, _)| s == 1),
            PseudoClass::LastOfType => self.type_position(id, cx).is_some_and(|(_, e)| e == 1),
            PseudoClass::OnlyOfType => self.type_position(id, cx) == Some((1, 1)),
            PseudoClass::Empty => {
                // Selectors 4 §14.2: no element children and no text
                // children with non-empty data — comments and zero-length
                // text nodes do not count; whitespace-only text does
                // (Level 3, and browsers today).
                let mut c = node.first_child;
                while let Some(cid) = c {
                    let Some(cn) = self.get_node(cid) else {
                        return false;
                    };
                    match &cn.data {
                        NodeData::Element { .. } => return false,
                        NodeData::Text { data } if !data.is_empty() => return false,
                        _ => {}
                    }
                    c = cn.next_sibling;
                }
                true
            }
            PseudoClass::Root => id == self.root(),
            // Selectors 4 §14.3: the scoping root; with none, `:root`.
            PseudoClass::Scope => id == cx.scope.unwrap_or_else(|| self.root()),
            // Selectors 4 §9.2 / §9.4 / §13.3: `:hover`, `:active` and
            // `:focus-within` match the element holding the state and
            // every ancestor of it (the flat tree is the node tree:
            // rdom has no shadow roots).
            PseudoClass::Hover => self.hovered().is_some_and(|h| self.is_ancestor(id, h)),
            PseudoClass::Active => self.active().is_some_and(|h| self.is_ancestor(id, h)),
            PseudoClass::Focus => self.focused() == Some(id),
            PseudoClass::FocusVisible => self.focus_visible && self.focused() == Some(id),
            PseudoClass::FocusWithin => self.focused().is_some_and(|h| self.is_ancestor(id, h)),
            PseudoClass::Checked => self
                .get_node(id)
                .map(|n| match &n.data {
                    NodeData::Element { attrs, .. } => attrs.contains_key("checked"),
                    _ => false,
                })
                .unwrap_or(false),
            PseudoClass::PlaceholderShown => self.is_placeholder_shown(id),
            PseudoClass::Indeterminate => self.indeterminate_with(id, cx.caches),
            PseudoClass::Open => self
                .get_node(id)
                .map(|n| match &n.data {
                    NodeData::Element { attrs, .. } => attrs.contains_key("open"),
                    _ => false,
                })
                .unwrap_or(false),
            PseudoClass::Disabled => self.is_actually_disabled(id),
            PseudoClass::Enabled => self.is_enabled_control(id),
            PseudoClass::Valid => self.constraint_validity(id) == Some(true),
            PseudoClass::Invalid => self.constraint_validity(id) == Some(false),
            PseudoClass::Required => self.is_required_control(id),
            PseudoClass::Optional => self.is_optional_control(id),
            PseudoClass::ReadWrite => self.is_read_write(id),
            PseudoClass::ReadOnly => node.tag_name().is_some() && !self.is_read_write(id),
        }
    }
}
