//! Matcher + public query_selector API.
//!
//! `Dom::query_selector_in(root, selector)` finds the first descendant
//! matching `selector`. `query_selector_all_in` returns every match.
//! `matches` tests a single node. `closest` walks ancestors finding the
//! first match. The DOM-shaped one-arg shortcuts
//! [`Dom::query_selector`](crate::Dom::query_selector) /
//! [`Dom::query_selector_all`](crate::Dom::query_selector_all) live on
//! `Dom` directly and pass `self.root()` as the root_id.
//!
//! The matcher evaluates each `ComplexSelector` right-to-left starting from
//! the candidate element (subject), then walks ancestors/siblings per
//! combinator. Good enough for small-to-medium subtrees; Phase 4 will add
//! indexes and selector "bloom" fast-rejection.

use crate::dom::Dom;
use crate::node::NodeData;
use crate::node_id::NodeId;
use crate::selectors::{
    self, AttrOp, Combinator, CompoundSelector, ParseError, PseudoClass, SelectorList,
    SimpleSelector,
};

impl<Ext> Dom<Ext> {
    /// Find the first descendant of `root_id` matching `selector`, in
    /// document order. Returns `None` if none matches. Errors if the
    /// selector is malformed.
    ///
    /// The DOM-shaped one-arg form is [`Dom::query_selector`]; this
    /// `_in` form is the explicit-root variant (M4b step 18 rename).
    pub fn query_selector_in(
        &self,
        root_id: NodeId,
        selector: &str,
    ) -> Result<Option<NodeId>, ParseError> {
        let list = selectors::parse(selector)?;
        let mut found = None;
        self.walk_descendants(root_id, &mut |id, data| {
            if found.is_some() {
                return;
            }
            if let NodeData::Element { .. } = data
                && self.matches_list(id, &list)
            {
                found = Some(id);
            }
        });
        Ok(found)
    }

    /// All descendants of `root_id` matching `selector`, in document
    /// order. The DOM-shaped one-arg form is
    /// [`Dom::query_selector_all`]; this `_in` form is the
    /// explicit-root variant (M4b step 18 rename).
    pub fn query_selector_all_in(
        &self,
        root_id: NodeId,
        selector: &str,
    ) -> Result<Vec<NodeId>, ParseError> {
        let list = selectors::parse(selector)?;
        let mut out = Vec::new();
        self.walk_descendants(root_id, &mut |id, data| {
            if matches!(data, NodeData::Element { .. }) && self.matches_list(id, &list) {
                out.push(id);
            }
        });
        Ok(out)
    }

    /// Does `id` match `selector`? Errors on malformed selector.
    pub fn matches(&self, id: NodeId, selector: &str) -> Result<bool, ParseError> {
        let list = selectors::parse(selector)?;
        Ok(self.matches_list(id, &list))
    }

    /// Walk from `id` (inclusive) up the tree and return the first ancestor
    /// that matches `selector`. `None` if none does.
    pub fn closest(&self, id: NodeId, selector: &str) -> Result<Option<NodeId>, ParseError> {
        let list = selectors::parse(selector)?;
        let mut cur = Some(id);
        while let Some(c) = cur {
            if matches!(
                self.get_node(c).map(|n| &n.data),
                Some(NodeData::Element { .. })
            ) && self.matches_list(c, &list)
            {
                return Ok(Some(c));
            }
            cur = self.get_node(c).and_then(|n| n.parent);
        }
        Ok(None)
    }

    // ─── Matcher ─────────────────────────────────────────────────────

    /// Does `id` match any selector in `list`?
    /// Does `id` match any selector in the pre-parsed `list`? Public so
    /// downstream crates (rdom-tui's cascade) can drive rule matching
    /// without re-parsing selector strings on every call.
    pub fn matches_list(&self, id: NodeId, list: &SelectorList) -> bool {
        list.0
            .iter()
            .any(|complex| self.matches_complex(id, complex))
    }

    fn matches_complex(&self, id: NodeId, complex: &selectors::ComplexSelector) -> bool {
        // Subject must match.
        if !self.matches_compound(id, &complex.subject) {
            return false;
        }
        // Walk ancestors/siblings per combinator. Each step's "candidate
        // pointer" represents the node we're trying to match against the
        // next compound on the outward path.
        let mut cur = id;
        for (comb, compound) in &complex.ancestors {
            match comb {
                Combinator::Descendant => {
                    let mut anc = self.get_node(cur).and_then(|n| n.parent);
                    let mut matched = None;
                    while let Some(a) = anc {
                        if self.matches_compound(a, compound) {
                            matched = Some(a);
                            break;
                        }
                        anc = self.get_node(a).and_then(|n| n.parent);
                    }
                    match matched {
                        Some(a) => cur = a,
                        None => return false,
                    }
                }
                Combinator::Child => {
                    let Some(parent) = self.get_node(cur).and_then(|n| n.parent) else {
                        return false;
                    };
                    if !self.matches_compound(parent, compound) {
                        return false;
                    }
                    cur = parent;
                }
                Combinator::AdjacentSibling => {
                    let Some(prev) = self.get_node(cur).and_then(|n| n.prev_sibling) else {
                        return false;
                    };
                    if !self.matches_compound(prev, compound) {
                        return false;
                    }
                    cur = prev;
                }
                Combinator::GeneralSibling => {
                    let mut sib = self.get_node(cur).and_then(|n| n.prev_sibling);
                    let mut matched = None;
                    while let Some(s) = sib {
                        if self.matches_compound(s, compound) {
                            matched = Some(s);
                            break;
                        }
                        sib = self.get_node(s).and_then(|n| n.prev_sibling);
                    }
                    match matched {
                        Some(s) => cur = s,
                        None => return false,
                    }
                }
            }
        }
        true
    }

    fn matches_compound(&self, id: NodeId, compound: &CompoundSelector) -> bool {
        let Some(node) = self.get_node(id) else {
            return false;
        };
        let NodeData::Element {
            tag,
            attrs,
            classes,
            ..
        } = &node.data
        else {
            return false;
        };
        for s in &compound.simples {
            match s {
                SimpleSelector::Universal => {}
                SimpleSelector::Type(t) => {
                    if tag != t {
                        return false;
                    }
                }
                SimpleSelector::Id(v) => {
                    if attrs.get("id").map(String::as_str) != Some(v.as_str()) {
                        return false;
                    }
                }
                SimpleSelector::Class(c) => {
                    if !classes.contains(c) {
                        return false;
                    }
                }
                SimpleSelector::Attribute { name, op, value } => {
                    if !match_attribute(attrs, name, *op, value.as_deref()) {
                        return false;
                    }
                }
                SimpleSelector::Not(inner) => {
                    if self.matches_list(id, inner) {
                        return false;
                    }
                }
                SimpleSelector::Pseudo(p) => {
                    if !self.match_pseudo(id, *p) {
                        return false;
                    }
                }
            }
        }
        true
    }

    fn match_pseudo(&self, id: NodeId, p: PseudoClass) -> bool {
        let Some(node) = self.get_node(id) else {
            return false;
        };
        match p {
            PseudoClass::FirstChild => {
                // No previous *element* sibling.
                self.prev_element_sibling_id(id).is_none() && node.parent.is_some()
            }
            PseudoClass::LastChild => {
                self.next_element_sibling_id(id).is_none() && node.parent.is_some()
            }
            PseudoClass::OnlyChild => {
                node.parent.is_some()
                    && self.prev_element_sibling_id(id).is_none()
                    && self.next_element_sibling_id(id).is_none()
            }
            PseudoClass::Empty => {
                // No child elements or text nodes (comments are allowed).
                let mut c = node.first_child;
                while let Some(cid) = c {
                    let Some(cn) = self.get_node(cid) else {
                        return false;
                    };
                    if matches!(cn.data, NodeData::Element { .. } | NodeData::Text { .. }) {
                        return false;
                    }
                    c = cn.next_sibling;
                }
                true
            }
            PseudoClass::Root => id == self.root(),
            PseudoClass::Hover => self.hovered() == Some(id),
            PseudoClass::Focus => self.focused() == Some(id),
            PseudoClass::Checked => self
                .get_node(id)
                .map(|n| match &n.data {
                    NodeData::Element { attrs, .. } => attrs.contains_key("checked"),
                    _ => false,
                })
                .unwrap_or(false),
            PseudoClass::PlaceholderShown => {
                // Must have a non-empty `placeholder` attribute AND
                // empty text content. Matches form controls showing
                // their placeholder hint.
                let has_placeholder = self
                    .get_node(id)
                    .map(|n| match &n.data {
                        NodeData::Element { attrs, .. } => attrs
                            .get("placeholder")
                            .map(|v| !v.is_empty())
                            .unwrap_or(false),
                        _ => false,
                    })
                    .unwrap_or(false);
                if !has_placeholder {
                    return false;
                }
                self.text_content(id).is_empty()
            }
            PseudoClass::Indeterminate => self
                .get_node(id)
                .map(|n| match &n.data {
                    NodeData::Element { tag, attrs, .. } => {
                        // v1: only `<progress>` without `value`
                        // attribute. Checkbox `indeterminate` IDL
                        // property + orphan radios deferred to
                        // polish.
                        tag == "progress" && !attrs.contains_key("value")
                    }
                    _ => false,
                })
                .unwrap_or(false),
            PseudoClass::Open => self
                .get_node(id)
                .map(|n| match &n.data {
                    NodeData::Element { attrs, .. } => attrs.contains_key("open"),
                    _ => false,
                })
                .unwrap_or(false),
        }
    }

    fn prev_element_sibling_id(&self, id: NodeId) -> Option<NodeId> {
        let mut cur = self.get_node(id).and_then(|n| n.prev_sibling);
        while let Some(c) = cur {
            let n = self.get_node(c)?;
            if matches!(n.data, NodeData::Element { .. }) {
                return Some(c);
            }
            cur = n.prev_sibling;
        }
        None
    }

    fn next_element_sibling_id(&self, id: NodeId) -> Option<NodeId> {
        let mut cur = self.get_node(id).and_then(|n| n.next_sibling);
        while let Some(c) = cur {
            let n = self.get_node(c)?;
            if matches!(n.data, NodeData::Element { .. }) {
                return Some(c);
            }
            cur = n.next_sibling;
        }
        None
    }
}

fn match_attribute(
    attrs: &std::collections::BTreeMap<String, String>,
    name: &str,
    op: Option<AttrOp>,
    want: Option<&str>,
) -> bool {
    let Some(have) = attrs.get(name) else {
        return false;
    };
    let Some(op) = op else { return true }; // `[name]` — presence only.
    let want = want.unwrap_or("");
    match op {
        AttrOp::Exact => have == want,
        AttrOp::Includes => have.split_ascii_whitespace().any(|tok| tok == want),
        AttrOp::DashMatch => have == want || have.starts_with(&format!("{want}-")),
        AttrOp::Prefix => !want.is_empty() && have.starts_with(want),
        AttrOp::Suffix => !want.is_empty() && have.ends_with(want),
        AttrOp::Substring => !want.is_empty() && have.contains(want),
    }
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::{Dom, NodeId};

    // Build:
    //   root
    //     div#a.outer
    //       span.first
    //       span.mid lang="en"
    //       p.last
    //         em "leaf"
    fn build() -> (Dom, [NodeId; 5]) {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let div = dom.create_element("div");
        dom.set_attribute(div, "id", "a").unwrap();
        dom.add_class(div, "outer").unwrap();

        let s1 = dom.create_element("span");
        dom.add_class(s1, "first").unwrap();

        let s2 = dom.create_element("span");
        dom.add_class(s2, "mid").unwrap();
        dom.set_attribute(s2, "lang", "en-US").unwrap();

        let p = dom.create_element("p");
        dom.add_class(p, "last").unwrap();

        let em = dom.create_element("em");
        let t = dom.create_text_node("leaf");
        dom.append_child(em, t).unwrap();
        dom.append_child(p, em).unwrap();

        dom.append_child(div, s1).unwrap();
        dom.append_child(div, s2).unwrap();
        dom.append_child(div, p).unwrap();
        dom.append_child(root, div).unwrap();

        (dom, [div, s1, s2, p, em])
    }

    #[test]
    fn matches_type() {
        let (dom, [div, ..]) = build();
        assert!(dom.matches(div, "div").unwrap());
        assert!(!dom.matches(div, "span").unwrap());
    }

    #[test]
    fn matches_id() {
        let (dom, [div, s1, ..]) = build();
        assert!(dom.matches(div, "#a").unwrap());
        assert!(!dom.matches(s1, "#a").unwrap());
    }

    #[test]
    fn matches_class() {
        let (dom, [_, s1, ..]) = build();
        assert!(dom.matches(s1, ".first").unwrap());
        assert!(!dom.matches(s1, ".missing").unwrap());
    }

    #[test]
    fn matches_attribute_variants() {
        let (dom, [_, _, s2, ..]) = build();
        assert!(dom.matches(s2, "[lang]").unwrap());
        assert!(dom.matches(s2, "[lang=en-US]").unwrap());
        assert!(dom.matches(s2, "[lang|=en]").unwrap());
        assert!(dom.matches(s2, "[lang^=en]").unwrap());
        assert!(dom.matches(s2, "[lang$=US]").unwrap());
        assert!(dom.matches(s2, "[lang*=n-U]").unwrap());
        assert!(!dom.matches(s2, "[lang=fr]").unwrap());
    }

    #[test]
    fn matches_compound() {
        let (dom, [div, ..]) = build();
        assert!(dom.matches(div, "div#a.outer").unwrap());
        assert!(!dom.matches(div, "div#b.outer").unwrap());
    }

    #[test]
    fn query_selector_descendant() {
        let (dom, [_, s1, ..]) = build();
        let root = dom.root();
        assert_eq!(dom.query_selector_in(root, "div .first").unwrap(), Some(s1));
    }

    #[test]
    fn query_selector_child_combinator() {
        let (dom, [_, _, _, _, em]) = build();
        let root = dom.root();
        // em is inside p which is inside div — only matches as descendant, not child of div.
        assert!(dom.query_selector_in(root, "div > em").unwrap().is_none());
        assert_eq!(dom.query_selector_in(root, "p > em").unwrap(), Some(em));
    }

    #[test]
    fn query_selector_adjacent_sibling() {
        let (dom, [_, _, s2, ..]) = build();
        let root = dom.root();
        assert_eq!(
            dom.query_selector_in(root, ".first + .mid").unwrap(),
            Some(s2)
        );
        assert!(
            dom.query_selector_in(root, ".first + .last")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn query_selector_general_sibling() {
        let (dom, [_, _, _, p, _]) = build();
        let root = dom.root();
        assert_eq!(
            dom.query_selector_in(root, ".first ~ .last").unwrap(),
            Some(p)
        );
    }

    #[test]
    fn query_selector_all_returns_document_order() {
        let (dom, [_, s1, s2, ..]) = build();
        let root = dom.root();
        let spans = dom.query_selector_all_in(root, "span").unwrap();
        assert_eq!(spans, vec![s1, s2]);
    }

    #[test]
    fn query_selector_list_union() {
        let (dom, [_, _, _, p, em]) = build();
        let root = dom.root();
        let r = dom.query_selector_all_in(root, "p, em").unwrap();
        assert_eq!(r, vec![p, em]);
    }

    #[test]
    fn not_pseudo_excludes_matches() {
        let (dom, _) = build();
        let root = dom.root();
        let r = dom.query_selector_all_in(root, "span:not(.first)").unwrap();
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn first_and_last_child_pseudos() {
        let (dom, [div, s1, _, p, em]) = build();
        let root = dom.root();
        // Every element that IS the first child of its parent: div (first of
        // root), s1 (first of div), em (first of p). Document-order pick: div.
        assert_eq!(
            dom.query_selector_in(root, ":first-child").unwrap(),
            Some(div)
        );
        // `span:first-child` scopes to spans — only s1 qualifies.
        assert_eq!(
            dom.query_selector_in(root, "span:first-child").unwrap(),
            Some(s1)
        );
        // Last children of each parent: div, p, em.
        let lasts = dom.query_selector_all_in(root, ":last-child").unwrap();
        assert!(lasts.contains(&p));
        assert!(lasts.contains(&em));
    }

    #[test]
    fn only_child_pseudo() {
        let (dom, [_, _, _, _, em]) = build();
        let root = dom.root();
        // em is the only child of p.
        assert_eq!(
            dom.query_selector_in(root, "em:only-child").unwrap(),
            Some(em)
        );
    }

    #[test]
    fn empty_pseudo() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let empty = dom.create_element("div");
        let not_empty = dom.create_element("div");
        let t = dom.create_text_node("x");
        dom.append_child(not_empty, t).unwrap();
        dom.append_child(root, empty).unwrap();
        dom.append_child(root, not_empty).unwrap();
        let r = dom.query_selector_all_in(root, "div:empty").unwrap();
        assert_eq!(r, vec![empty]);
    }

    #[test]
    fn root_pseudo() {
        let (dom, _) = build();
        let root = dom.root();
        // `:root` matches only the document root. query_selector scans
        // descendants, so it won't find root itself — use matches / closest.
        // But our root is a Fragment, not an Element. Build a new dom with
        // an element root to exercise :root.
        let mut dom2: Dom = Dom::with_root_tag("html");
        let root2 = dom2.root();
        let body = dom2.create_element("body");
        dom2.append_child(root2, body).unwrap();
        assert!(dom2.matches(root2, ":root").unwrap());
        assert!(!dom2.matches(body, ":root").unwrap());
        // In the original fragment-rooted tree, the root is a fragment so
        // matches_compound returns false regardless.
        assert!(!dom.matches(root, ":root").unwrap());
    }

    #[test]
    fn matches_with_chain() {
        let (dom, [_, _, _, _, em]) = build();
        // em inside .outer via descendant combinator.
        assert!(dom.matches(em, ".outer em").unwrap());
        // child: em's parent is p, not .outer directly.
        assert!(!dom.matches(em, ".outer > em").unwrap());
    }

    #[test]
    fn hover_pseudo_follows_set_hovered() {
        let (mut dom, [div, _, _, _, em]) = build();
        // Nothing hovered → no matches.
        assert!(!dom.matches(div, ":hover").unwrap());
        // Hover div — only div matches.
        dom.set_hovered(Some(div));
        assert!(dom.matches(div, ":hover").unwrap());
        assert!(!dom.matches(em, ":hover").unwrap());
        // Clear hover.
        dom.set_hovered(None);
        assert!(!dom.matches(div, ":hover").unwrap());
    }

    #[test]
    fn focus_pseudo_follows_set_focused() {
        let (mut dom, [div, s1, _, _, _]) = build();
        dom.set_focused(Some(s1));
        assert!(dom.matches(s1, ":focus").unwrap());
        assert!(!dom.matches(div, ":focus").unwrap());
    }

    #[test]
    fn hover_focus_combine_with_other_selectors() {
        let (mut dom, [_, s1, _, _, _]) = build();
        dom.set_hovered(Some(s1));
        // span:hover
        assert!(dom.matches(s1, "span:hover").unwrap());
        // span.first:hover
        assert!(dom.matches(s1, "span.first:hover").unwrap());
        // Non-hovered elements don't match.
        dom.set_hovered(None);
        assert!(!dom.matches(s1, "span:hover").unwrap());
    }

    #[test]
    fn checked_pseudo_matches_attribute_presence() {
        let mut dom: Dom<()> = Dom::new();
        let root = dom.root();
        let cb = dom.create_element("input");
        dom.set_attribute(cb, "type", "checkbox").unwrap();
        dom.append_child(root, cb).unwrap();

        // Absent → no match.
        assert!(!dom.matches(cb, ":checked").unwrap());

        // Empty value (HTML boolean shorthand) → match.
        dom.set_attribute(cb, "checked", "").unwrap();
        assert!(dom.matches(cb, ":checked").unwrap());

        // Any value still matches (presence-only, like the HTML
        // boolean attribute model).
        dom.set_attribute(cb, "checked", "false").unwrap();
        assert!(dom.matches(cb, ":checked").unwrap());

        // Removed → no match.
        dom.remove_attribute(cb, "checked").unwrap();
        assert!(!dom.matches(cb, ":checked").unwrap());
    }

    #[test]
    fn checked_pseudo_combines_with_type_attribute_selector() {
        let mut dom: Dom<()> = Dom::new();
        let root = dom.root();
        let cb = dom.create_element("input");
        dom.set_attribute(cb, "type", "checkbox").unwrap();
        dom.set_attribute(cb, "checked", "").unwrap();
        dom.append_child(root, cb).unwrap();

        assert!(dom.matches(cb, "[type=checkbox]:checked").unwrap());
        assert!(!dom.matches(cb, "[type=radio]:checked").unwrap());
    }

    #[test]
    fn placeholder_shown_matches_when_attribute_set_and_content_empty() {
        let mut dom: Dom<()> = Dom::new();
        let root = dom.root();
        let inp = dom.create_element("input");
        dom.set_attribute(inp, "placeholder", "Search...").unwrap();
        dom.append_child(root, inp).unwrap();

        // No text content → match.
        assert!(dom.matches(inp, ":placeholder-shown").unwrap());

        // Add some content → no match.
        let t = dom.create_text_node("hi");
        dom.append_child(inp, t).unwrap();
        assert!(!dom.matches(inp, ":placeholder-shown").unwrap());
    }

    #[test]
    fn placeholder_shown_requires_non_empty_placeholder_attribute() {
        let mut dom: Dom<()> = Dom::new();
        let root = dom.root();
        let inp = dom.create_element("input");
        dom.append_child(root, inp).unwrap();
        // No placeholder → no match.
        assert!(!dom.matches(inp, ":placeholder-shown").unwrap());

        // Empty placeholder → still no match (HTML rule — blank
        // placeholder doesn't count).
        dom.set_attribute(inp, "placeholder", "").unwrap();
        assert!(!dom.matches(inp, ":placeholder-shown").unwrap());
    }

    #[test]
    fn placeholder_shown_with_whitespace_is_empty_content() {
        // Content::text() concatenates raw strings — whitespace is
        // NOT collapsed for this test. Text node with just spaces
        // is non-empty; matches browsers for real whitespace.
        let mut dom: Dom<()> = Dom::new();
        let root = dom.root();
        let inp = dom.create_element("input");
        dom.set_attribute(inp, "placeholder", "Hint").unwrap();
        let t = dom.create_text_node(" ");
        dom.append_child(inp, t).unwrap();
        dom.append_child(root, inp).unwrap();
        // Space is non-empty text → doesn't match.
        assert!(!dom.matches(inp, ":placeholder-shown").unwrap());
    }

    // ── :indeterminate + :open (Polish #4) ────────────────────────

    #[test]
    fn indeterminate_matches_progress_without_value() {
        let mut dom: Dom<()> = Dom::new();
        let root = dom.root();
        let p = dom.create_element("progress");
        dom.append_child(root, p).unwrap();
        assert!(dom.matches(p, ":indeterminate").unwrap());
        dom.set_attribute(p, "value", "0.5").unwrap();
        assert!(!dom.matches(p, ":indeterminate").unwrap());
    }

    #[test]
    fn indeterminate_does_not_match_other_tags() {
        let mut dom: Dom<()> = Dom::new();
        let root = dom.root();
        let m = dom.create_element("meter");
        dom.append_child(root, m).unwrap();
        // `<meter>` without value is NOT indeterminate (unlike
        // progress) — meter always represents a known measurement.
        assert!(!dom.matches(m, ":indeterminate").unwrap());
    }

    #[test]
    fn open_matches_elements_with_open_attribute() {
        let mut dom: Dom<()> = Dom::new();
        let root = dom.root();
        let d = dom.create_element("details");
        dom.append_child(root, d).unwrap();
        assert!(!dom.matches(d, ":open").unwrap());
        dom.set_attribute(d, "open", "").unwrap();
        assert!(dom.matches(d, ":open").unwrap());
    }

    #[test]
    fn open_works_on_dialog_as_well() {
        let mut dom: Dom<()> = Dom::new();
        let root = dom.root();
        let d = dom.create_element("dialog");
        dom.set_attribute(d, "open", "").unwrap();
        dom.append_child(root, d).unwrap();
        assert!(dom.matches(d, ":open").unwrap());
    }

    #[test]
    fn open_can_combine_with_other_selectors() {
        let mut dom: Dom<()> = Dom::new();
        let root = dom.root();
        let d = dom.create_element("details");
        dom.set_attribute(d, "open", "").unwrap();
        dom.append_child(root, d).unwrap();
        assert!(dom.matches(d, "details:open").unwrap());
        assert!(!dom.matches(d, "dialog:open").unwrap());
    }

    #[test]
    fn closest_walks_up() {
        let (dom, [div, _, _, _, em]) = build();
        // closest(".outer") from em returns div.
        assert_eq!(dom.closest(em, ".outer").unwrap(), Some(div));
        // closest("#nope") from em returns None.
        assert!(dom.closest(em, "#nope").unwrap().is_none());
        // closest("em") from em returns em (inclusive self).
        assert_eq!(dom.closest(em, "em").unwrap(), Some(em));
    }

    #[test]
    fn invalid_selector_errors() {
        let (dom, _) = build();
        let root = dom.root();
        assert!(dom.query_selector_in(root, ":nope").is_err());
        assert!(dom.query_selector_all_in(root, "").is_err());
    }
}
