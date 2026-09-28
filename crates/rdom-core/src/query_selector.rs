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
    /// Whether `id` matches `:placeholder-shown`: it has a non-empty
    /// `placeholder` attribute and no text content, i.e. it is showing
    /// its placeholder hint. Also what decides whether `::placeholder`
    /// rules style anything.
    pub fn is_placeholder_shown(&self, id: NodeId) -> bool {
        self.get_attribute(id, "placeholder")
            .is_some_and(|v| !v.is_empty())
            && self.text_content(id).is_empty()
    }

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
                SimpleSelector::Where(inner) => {
                    // Matches like `:is()` — any complex selector in the list
                    // must match this element as its subject. Specificity is
                    // handled (as zero) by `rdom-style`.
                    if !self.matches_list(id, inner) {
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
            PseudoClass::Hover => self.hovered() == Some(id),
            PseudoClass::Focus => self.focused() == Some(id),
            PseudoClass::FocusVisible => self.focus_visible && self.focused() == Some(id),
            PseudoClass::FocusWithin => {
                // Walk up from the focused node to the root; if
                // `id` is the focused node itself or any ancestor
                // in that chain, it matches. O(depth) per query,
                // which is fine — focus depth is bounded by tree
                // height.
                let Some(focused) = self.focused() else {
                    return false;
                };
                let mut cur = Some(focused);
                while let Some(n) = cur {
                    if n == id {
                        return true;
                    }
                    cur = self.get_node(n).and_then(|node| node.parent);
                }
                false
            }
            PseudoClass::Checked => self
                .get_node(id)
                .map(|n| match &n.data {
                    NodeData::Element { attrs, .. } => attrs.contains_key("checked"),
                    _ => false,
                })
                .unwrap_or(false),
            PseudoClass::PlaceholderShown => self.is_placeholder_shown(id),
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
            PseudoClass::Disabled => self.is_actually_disabled(id),
            PseudoClass::Enabled => self.is_enabled_control(id),
            PseudoClass::Valid => self.constraint_validity(id) == Some(true),
            PseudoClass::Invalid => self.constraint_validity(id) == Some(false),
            PseudoClass::Required => self.is_required_control(id),
            PseudoClass::Optional => self.is_optional_control(id),
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

/// HTML §4.16.2 "case-sensitivity of selectors": attribute selectors on
/// an HTML element treat the values of these attributes as ASCII
/// case-insensitive (a `match`: this runs on every attribute-selector
/// test of the cascade). Every rdom element is an HTML element in an HTML
/// document. (HTML exempts `type` in its rendering section's `ol[type]`
/// rules via the `s` flag, which rdom does not parse.)
fn is_html_case_insensitive_attr(name: &str) -> bool {
    matches!(
        name,
        "accept"
            | "accept-charset"
            | "align"
            | "alink"
            | "axis"
            | "bgcolor"
            | "charset"
            | "checked"
            | "clear"
            | "codetype"
            | "color"
            | "compact"
            | "declare"
            | "defer"
            | "dir"
            | "direction"
            | "disabled"
            | "enctype"
            | "face"
            | "frame"
            | "hreflang"
            | "http-equiv"
            | "lang"
            | "language"
            | "link"
            | "media"
            | "method"
            | "multiple"
            | "nohref"
            | "noresize"
            | "noshade"
            | "nowrap"
            | "readonly"
            | "rel"
            | "rev"
            | "rules"
            | "scope"
            | "scrolling"
            | "selected"
            | "shape"
            | "target"
            | "text"
            | "type"
            | "valign"
            | "valuetype"
            | "vlink"
    )
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
    if is_html_case_insensitive_attr(name) {
        match_value::<AsciiCaseInsensitive>(op, have, want)
    } else {
        match_value::<CaseSensitive>(op, have, want)
    }
}

/// How [`match_value`] compares attribute-value bytes. Comparing bytes
/// is sound for UTF-8: ASCII case folding never touches the bytes of a
/// multi-byte character, so a match starts and ends on char boundaries
/// whenever `want` is valid UTF-8.
trait ValueCase {
    fn eq(a: &[u8], b: &[u8]) -> bool;

    /// `want` (non-empty) occurs in `have`.
    fn contains(have: &str, want: &str) -> bool {
        have.as_bytes()
            .windows(want.len())
            .any(|win| Self::eq(win, want.as_bytes()))
    }
}

struct CaseSensitive;
impl ValueCase for CaseSensitive {
    fn eq(a: &[u8], b: &[u8]) -> bool {
        a == b
    }

    fn contains(have: &str, want: &str) -> bool {
        have.contains(want)
    }
}

/// HTML §4.16.2: ASCII case-insensitive, without allocating.
struct AsciiCaseInsensitive;
impl ValueCase for AsciiCaseInsensitive {
    fn eq(a: &[u8], b: &[u8]) -> bool {
        a.eq_ignore_ascii_case(b)
    }
}

/// Selectors 4 §6.1 / §6.2: the attribute-value operators, comparing
/// per `C`.
fn match_value<C: ValueCase>(op: AttrOp, have: &str, want: &str) -> bool {
    let (h, w) = (have.as_bytes(), want.as_bytes());
    let starts = |h: &[u8]| h.get(..w.len()).is_some_and(|p| C::eq(p, w));
    match op {
        AttrOp::Exact => C::eq(h, w),
        AttrOp::Includes => have
            .split_ascii_whitespace()
            .any(|tok| C::eq(tok.as_bytes(), w)),
        AttrOp::DashMatch => C::eq(h, w) || (starts(h) && h.get(w.len()) == Some(&b'-')),
        AttrOp::Prefix => !w.is_empty() && starts(h),
        AttrOp::Suffix => !w.is_empty() && h.len() >= w.len() && C::eq(&h[h.len() - w.len()..], w),
        AttrOp::Substring => !w.is_empty() && C::contains(have, want),
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
    fn where_pseudo_matches_like_is() {
        let (dom, [_div, s1, _s2, p, em]) = build();
        let root = dom.root();
        // :where(list) matches an element matching any item in the list.
        let r = dom
            .query_selector_all_in(root, ":where(.first, .last)")
            .unwrap();
        assert_eq!(r, vec![s1, p]);
        // Combinators inside :where() are honored (em is a descendant of div).
        assert!(dom.matches(em, ":where(div em)").unwrap());
        assert!(!dom.matches(s1, ":where(div em)").unwrap());
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

    /// Selectors 4 §14.2 (`P7G-CORE-SMALL-1`): `:empty` means no element
    /// children and no text children with non-empty data — a zero-length
    /// text node, a comment do not count; whitespace-only text does
    /// (Level 3, and browsers today).
    #[test]
    fn empty_ignores_zero_length_text_and_comments_but_not_whitespace() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let with = |dom: &mut Dom, kids: &[Option<&str>]| {
            let div = dom.create_element("div");
            for k in kids {
                let child = match k {
                    Some(text) => dom.create_text_node(text),
                    None => dom.create_comment("c"),
                };
                dom.append_child(div, child).unwrap();
            }
            dom.append_child(root, div).unwrap();
            div
        };
        let zero_length = with(&mut dom, &[Some("")]);
        let comment = with(&mut dom, &[None]);
        let both = with(&mut dom, &[Some(""), None, Some("")]);
        let space = with(&mut dom, &[Some(" ")]);
        let text = with(&mut dom, &[Some(""), Some("x")]);
        for id in [zero_length, comment, both] {
            assert!(dom.matches(id, ":empty").unwrap(), "{id:?}");
        }
        for id in [space, text] {
            assert!(!dom.matches(id, ":empty").unwrap(), "{id:?}");
        }
    }

    /// CSS Syntax 3 §4.2: non-ASCII code points are ident code points,
    /// so unquoted values, classes and ids may use them.
    #[test]
    fn non_ascii_identifiers_match() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let el = dom.create_element("p");
        dom.set_attribute(el, "lang", "én-CA").unwrap();
        dom.set_attribute(el, "id", "naïve").unwrap();
        dom.add_class(el, "café").unwrap();
        dom.append_child(root, el).unwrap();
        for sel in ["[lang|=én]", "p.café", "#naïve", "[lang^=é]"] {
            assert!(dom.matches(el, sel).unwrap(), "{sel}");
        }
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
    fn focus_within_matches_focused_and_ancestor_elements() {
        // `:focus-within` matches the focused element AND every
        // ancestor element in the chain. Mirrors CSS Selectors L4
        // §10.1.4. The Document root is not an element so it
        // won't return true via `matches()` (which filters to
        // elements), but every element ancestor between root and
        // the focused node does.
        //
        // Tree shape (from `build`):
        //   root → div.outer → p.last → em
        let (mut dom, [div, _s1, _s2, p, em]) = build();
        dom.set_focused(Some(em));
        assert!(dom.matches(em, ":focus-within").unwrap(), "focused itself");
        assert!(dom.matches(p, ":focus-within").unwrap(), "parent");
        assert!(dom.matches(div, ":focus-within").unwrap(), "grandparent");
    }

    #[test]
    fn focus_within_does_not_match_siblings_or_other_subtrees() {
        // Focus is on em (under p.last). Siblings of p (s1.first,
        // s2.mid) are in a different subtree — they must NOT match.
        let (mut dom, [_div, s1, s2, _p, em]) = build();
        dom.set_focused(Some(em));
        assert!(!dom.matches(s1, ":focus-within").unwrap());
        assert!(!dom.matches(s2, ":focus-within").unwrap());
    }

    #[test]
    fn focus_within_clears_when_focus_cleared() {
        let (mut dom, [div, _, _, _, em]) = build();
        dom.set_focused(Some(em));
        assert!(dom.matches(div, ":focus-within").unwrap());
        dom.set_focused(None);
        assert!(!dom.matches(div, ":focus-within").unwrap());
        assert!(!dom.matches(em, ":focus-within").unwrap());
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

    /// HTML §4.16.2: attribute selectors treat the values of `type`,
    /// `method`, `enctype`, `lang`, … on HTML elements as ASCII
    /// case-insensitive; every other attribute stays case-sensitive.
    #[test]
    fn html_case_insensitive_attribute_values_match_regardless_of_case() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let cb = dom.create_element("input");
        dom.set_attribute(cb, "type", "CheckBox").unwrap();
        dom.set_attribute(cb, "data-kind", "Big").unwrap();
        dom.append_child(root, cb).unwrap();
        let form = dom.create_element("form");
        dom.set_attribute(form, "method", "POST").unwrap();
        dom.set_attribute(form, "lang", "EN-us").unwrap();
        dom.append_child(root, form).unwrap();

        assert!(dom.matches(cb, "input[type=checkbox]").unwrap());
        assert!(dom.matches(cb, "[type^=check]").unwrap());
        assert!(dom.matches(cb, ":not([type=radio])").unwrap());
        assert!(dom.matches(form, "[method=post]").unwrap());
        assert!(dom.matches(form, "[lang|=en]").unwrap());
        assert!(
            !dom.matches(cb, "[data-kind=big]").unwrap(),
            "attributes outside the HTML list stay case-sensitive"
        );
        assert!(dom.matches(cb, "[data-kind=Big]").unwrap());
    }

    /// Every attribute operator honors HTML §4.16.2's case-insensitive
    /// values — `=`, `~=`, `|=`, `^=`, `$=`, `*=` — on a listed attribute
    /// (`rel`, `lang`, `type`), and stays case-sensitive on others.
    #[test]
    fn every_attribute_operator_is_ascii_case_insensitive_on_listed_attributes() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let a = dom.create_element("a");
        dom.set_attribute(a, "rel", "NoOpener External").unwrap();
        dom.set_attribute(a, "lang", "EN-GB").unwrap();
        dom.set_attribute(a, "type", "Text/HTML").unwrap();
        dom.set_attribute(a, "title", "Text/HTML").unwrap();
        dom.append_child(root, a).unwrap();
        for sel in [
            "[type='text/html']",
            "[rel~=external]",
            "[rel~=NOOPENER]",
            "[lang|=en]",
            "[lang|=en-gb]",
            "[type^=TEXT]",
            "[type$='/html']",
            "[type*='T/h']",
        ] {
            assert!(dom.matches(a, sel).unwrap(), "{sel}");
        }
        for sel in [
            "[rel~=noop]",
            "[lang|=e]",
            "[lang|=gb]",
            "[type^=html]",
            "[type$=text]",
            "[type*=xml]",
            "[type^='']",
        ] {
            assert!(!dom.matches(a, sel).unwrap(), "{sel}");
        }
        for sel in [
            "[title='text/html']",
            "[title^=text]",
            "[title$='/html']",
            "[title*='t/h']",
        ] {
            assert!(
                !dom.matches(a, sel).unwrap(),
                "{sel}: title is case-sensitive"
            );
        }
        assert!(dom.matches(a, "[title*='t/H']").unwrap());
        // Non-ASCII letters never fold.
        dom.set_attribute(a, "lang", "ÉN").unwrap();
        assert!(!dom.matches(a, "[lang|='én']").unwrap());
        assert!(dom.matches(a, "[lang|='ÉN']").unwrap());
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

    // ── P7-VALIDATION-SELECTORS-1 ─────────────────────────────────────

    /// HTML §4.16.3: `:required` matches an `<input>` (in a state
    /// `required` applies to), `<select>` or `<textarea>` with
    /// `required`; `:optional` the rest of those three.
    #[test]
    fn required_and_optional_match_the_three_form_controls() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let mk = |dom: &mut Dom, tag: &str, attrs: &[(&str, &str)]| {
            let e = dom.create_element(tag);
            for (k, v) in attrs {
                dom.set_attribute(e, k, v).unwrap();
            }
            dom.append_child(root, e).unwrap();
            e
        };
        let req_text = mk(&mut dom, "input", &[("required", "")]);
        let req_box = mk(&mut dom, "input", &[("type", "checkbox"), ("required", "")]);
        let req_range = mk(&mut dom, "input", &[("type", "range"), ("required", "")]);
        let req_hidden = mk(&mut dom, "input", &[("type", "hidden"), ("required", "")]);
        let plain = mk(&mut dom, "input", &[]);
        let req_select = mk(&mut dom, "select", &[("required", "")]);
        let area = mk(&mut dom, "textarea", &[]);
        let req_div = mk(&mut dom, "div", &[("required", "")]);
        let button = mk(&mut dom, "button", &[("required", "")]);
        for id in [req_text, req_box, req_select] {
            assert!(dom.matches(id, ":required").unwrap(), "{id:?}");
            assert!(!dom.matches(id, ":optional").unwrap(), "{id:?}");
        }
        for id in [req_range, req_hidden, plain, area] {
            assert!(dom.matches(id, ":optional").unwrap(), "{id:?}");
            assert!(!dom.matches(id, ":required").unwrap(), "{id:?}");
        }
        for id in [req_div, button] {
            assert!(!dom.matches(id, ":required").unwrap(), "{id:?}");
            assert!(!dom.matches(id, ":optional").unwrap(), "{id:?}");
        }
    }

    /// Test backend: a candidate is invalid while it has `data-bad`.
    /// Selectors 4 §13.2: `:focus-visible` matches the focused element
    /// while the UA judges its focus should be evident
    /// (`Dom::focus_visible`, on until a backend says otherwise); it
    /// never matches an unfocused element.
    #[test]
    fn focus_visible_matches_the_focused_element_while_focus_is_evident() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let a = dom.create_element("button");
        let b = dom.create_element("button");
        dom.append_child(root, a).unwrap();
        dom.append_child(root, b).unwrap();
        assert!(dom.focus_visible(), "focus is evident by default");
        assert!(
            !dom.matches(a, ":focus-visible").unwrap(),
            "nothing focused"
        );

        dom.set_focused(Some(a));
        assert!(dom.matches(a, ":focus-visible").unwrap());
        assert!(!dom.matches(b, ":focus-visible").unwrap());

        dom.set_focus_visible(false);
        assert!(dom.matches(a, ":focus").unwrap());
        assert!(!dom.matches(a, ":focus-visible").unwrap());
        assert!(dom.matches(a, ":focus:not(:focus-visible)").unwrap());
    }

    fn bad_attr_hook(dom: &Dom, id: NodeId) -> bool {
        !dom.has_attribute(id, "data-bad")
    }

    /// `P7G-VALIDITY-HOOK-DEFAULT-1`: rdom-core has no validity states,
    /// so asking whether a candidate is valid without a backend hook is a
    /// misconfiguration — loud in a debug build rather than a silent
    /// "every control is valid".
    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "no validity hook")]
    fn matching_invalid_on_a_candidate_without_a_validity_hook_panics_in_debug_builds() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let input = dom.create_element("input");
        dom.append_child(root, input).unwrap();
        let _ = dom.matches(input, ":invalid");
    }

    /// Elements that are not candidates answer `:valid` / `:invalid`
    /// without consulting the hook, so they need none.
    #[test]
    fn non_candidates_match_neither_validity_class_without_a_hook() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let div = dom.create_element("div");
        dom.append_child(root, div).unwrap();
        let hidden = dom.create_element("input");
        dom.set_attribute(hidden, "type", "hidden").unwrap();
        dom.append_child(root, hidden).unwrap();
        for id in [div, hidden] {
            assert!(!dom.matches(id, ":valid").unwrap());
            assert!(!dom.matches(id, ":invalid").unwrap());
        }
    }

    /// HTML §4.16.3: `:valid` / `:invalid` match candidates for
    /// constraint validation by the backend's verdict (the validity
    /// hook), forms by their owned candidates and fieldsets by their
    /// descendant candidates; barred and other elements match neither.
    #[test]
    fn valid_and_invalid_follow_the_validity_hook() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let form = dom.create_element("form");
        dom.append_child(root, form).unwrap();
        let fieldset = dom.create_element("fieldset");
        dom.append_child(form, fieldset).unwrap();
        let bad = dom.create_element("input");
        dom.set_attribute(bad, "data-bad", "").unwrap();
        dom.append_child(fieldset, bad).unwrap();
        let good = dom.create_element("textarea");
        dom.append_child(form, good).unwrap();
        let barred = dom.create_element("input");
        dom.set_attribute(barred, "data-bad", "").unwrap();
        dom.set_attribute(barred, "disabled", "").unwrap();
        dom.append_child(form, barred).unwrap();
        let div = dom.create_element("div");
        dom.append_child(form, div).unwrap();
        let empty_form = dom.create_element("form");
        dom.append_child(root, empty_form).unwrap();

        dom.set_validity_hook(Some(bad_attr_hook));
        assert!(dom.matches(bad, ":invalid").unwrap());
        assert!(!dom.matches(bad, ":valid").unwrap());
        assert!(dom.matches(good, ":valid").unwrap());
        assert!(dom.matches(fieldset, ":invalid").unwrap());
        assert!(dom.matches(form, ":invalid").unwrap());
        assert!(dom.matches(empty_form, ":valid").unwrap());
        for id in [barred, div] {
            assert!(!dom.matches(id, ":valid").unwrap(), "{id:?}");
            assert!(!dom.matches(id, ":invalid").unwrap(), "{id:?}");
        }
        assert_eq!(dom.constraint_validity(bad), Some(false));
        assert_eq!(dom.constraint_validity(div), None);

        dom.remove_attribute(bad, "data-bad").unwrap();
        assert!(dom.matches(form, ":valid").unwrap());
        assert!(dom.matches(fieldset, ":valid").unwrap());
    }
}
