//! Form association — which `<form>` a control belongs to, and what a
//! submission by a given submitter would carry. Names, attributes and
//! tree shape only; no rendering, no runtime.
//!
//! - [`Dom::form_owner`] — HTML §4.10.17.3 "reset the form owner": the
//!   `form="id"` content attribute, else the nearest ancestor `<form>`.
//! - [`Dom::form_listed_elements`] — the listed elements a form owns, in
//!   tree order of the form's whole tree (`form.elements`, §4.10.3).
//! - [`Dom::is_submit_button`] — §4.10.6 / §4.10.5.1.19.
//! - [`Dom::submit_detail`] — the effective `action` / `method` /
//!   `enctype` / `target` / no-validate state (§4.10.19.6).

use crate::dom::Dom;
use crate::event_detail::{FormEnctype, FormMethod, SubmitDetail};
use crate::node_id::NodeId;

/// HTML §4.10.2 *listed* elements: the form-associated elements that
/// take a `form` content attribute and appear in `form.elements`.
fn is_listed(tag: &str) -> bool {
    matches!(
        tag,
        "button" | "fieldset" | "input" | "object" | "output" | "select" | "textarea"
    )
}

impl<Ext> Dom<Ext> {
    /// The form owner of `id` (HTML §4.10.17.3):
    ///
    /// - a *listed* element (`<button>`, `<fieldset>`, `<input>`,
    ///   `<object>`, `<output>`, `<select>`, `<textarea>`) that has a
    ///   `form` attribute and is connected is owned by the first element
    ///   in the document with that id **if it is a `<form>`** — and by
    ///   nothing otherwise, even inside another form;
    /// - anything else is owned by its nearest ancestor `<form>`.
    ///
    /// A form is not its own owner. Dead / non-element ids have none.
    pub fn form_owner(&self, id: NodeId) -> Option<NodeId> {
        let tag = self.get_node(id)?.tag_name()?;
        let mut ancestor = self.parent_element_id(id);
        while let Some(a) = ancestor {
            if self.tag_is(a, "form") {
                break;
            }
            ancestor = self.parent_element_id(a);
        }
        let connected = self.root_of(id) == Some(self.root());
        self.owner_given_ancestor(id, tag, ancestor, connected)
    }

    /// The listed elements whose form owner is `form`, in tree order of
    /// `form`'s whole tree — HTML `form.elements` (§4.10.3), which
    /// includes controls outside the form that name it with `form=`.
    /// Empty for a non-`<form>`. One pass over the tree.
    pub fn form_listed_elements(&self, form: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        if !self.tag_is(form, "form") {
            return out;
        }
        let Some(root) = self.root_of(form) else {
            return out;
        };
        let connected = root == self.root();
        // Pre-order DFS carrying each node's nearest ancestor `<form>`.
        let mut stack: Vec<(NodeId, Option<NodeId>)> = vec![(root, None)];
        while let Some((id, ancestor_form)) = stack.pop() {
            let Some(node) = self.get_node(id) else {
                continue;
            };
            let mut child_ancestor = ancestor_form;
            if let Some(tag) = node.tag_name() {
                if is_listed(tag)
                    && self.owner_given_ancestor(id, tag, ancestor_form, connected) == Some(form)
                {
                    out.push(id);
                }
                if tag == "form" {
                    child_ancestor = Some(id);
                }
            }
            // Push children in reverse so they pop in tree order.
            let first = stack.len();
            let mut c = node.first_child;
            while let Some(cid) = c {
                stack.push((cid, child_ancestor));
                c = self.get_node(cid).and_then(|n| n.next_sibling);
            }
            stack[first..].reverse();
        }
        out
    }

    /// Whether `id` is a *submit button* (HTML §4.10.6, §4.10.5.1.19): a
    /// `<button>` whose `type` is missing, invalid or `submit`, or an
    /// `<input type="submit">`.
    pub fn is_submit_button(&self, id: NodeId) -> bool {
        match self.get_node(id).and_then(|n| n.tag_name()) {
            Some("button") => !matches!(self.get_attribute(id, "type"), Some("reset" | "button")),
            Some("input") => self.get_attribute(id, "type") == Some("submit"),
            _ => false,
        }
    }

    /// The `submit` event payload for `form` submitted by `submitter`
    /// (HTML §4.10.21.3 with the §4.10.19.6 attributes): each of
    /// `action` / `method` / `enctype` / `target` is the submitter's
    /// `formaction` / `formmethod` / `formenctype` / `formtarget` when the
    /// submitter is a submit button carrying it, else the form's
    /// `action` / `method` / `enctype` / `target`. `no_validate` is the
    /// submitter's `formnovalidate` or the form's `novalidate`. A
    /// `submitter` that is not a submit button contributes no override
    /// but is still reported.
    pub fn submit_detail(&self, form: NodeId, submitter: Option<NodeId>) -> SubmitDetail {
        let button = submitter.filter(|&s| self.is_submit_button(s));
        let attr = |over: &str, own: &str| -> Option<&str> {
            button
                .and_then(|b| self.get_attribute(b, over))
                .or_else(|| self.get_attribute(form, own))
        };
        let mut d = SubmitDetail::new(submitter);
        d.action = attr("formaction", "action").unwrap_or("").to_string();
        d.method = attr("formmethod", "method")
            .map(FormMethod::from_attribute)
            .unwrap_or_default();
        d.enctype = attr("formenctype", "enctype")
            .map(FormEnctype::from_attribute)
            .unwrap_or_default();
        d.target = attr("formtarget", "target").unwrap_or("").to_string();
        d.no_validate = button.is_some_and(|b| self.has_attribute(b, "formnovalidate"))
            || self.has_attribute(form, "novalidate");
        d
    }

    /// The owner rule of [`Self::form_owner`], given the element's
    /// nearest ancestor `<form>` and whether it is connected.
    fn owner_given_ancestor(
        &self,
        id: NodeId,
        tag: &str,
        ancestor_form: Option<NodeId>,
        connected: bool,
    ) -> Option<NodeId> {
        if is_listed(tag)
            && connected
            && let Some(form_id) = self.get_attribute(id, "form")
        {
            // `get_element_by_id` answers the first connected element in
            // tree order; a detached hit means no element in the
            // document carries the id.
            return self
                .get_element_by_id(form_id)
                .filter(|&f| self.tag_is(f, "form") && self.root_of(f) == Some(self.root()));
        }
        ancestor_form
    }
}

#[cfg(test)]
mod tests {
    use crate::node_id::NodeId;
    use crate::{Dom, FormEnctype, FormMethod};

    fn el(dom: &mut Dom, parent: NodeId, tag: &str, attrs: &[(&str, &str)]) -> NodeId {
        let e = dom.create_element(tag);
        for (k, v) in attrs {
            dom.set_attribute(e, k, v).unwrap();
        }
        dom.append_child(parent, e).unwrap();
        e
    }

    #[test]
    fn owner_is_the_nearest_ancestor_form_without_a_form_attribute() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let form = el(&mut dom, root, "form", &[]);
        let div = el(&mut dom, form, "div", &[]);
        let input = el(&mut dom, div, "input", &[]);
        let outside = el(&mut dom, root, "input", &[]);
        assert_eq!(dom.form_owner(input), Some(form));
        assert_eq!(dom.form_owner(outside), None);
        assert_eq!(dom.form_owner(form), None, "a form is not its own owner");
    }

    /// HTML §4.10.17.3: `form="id"` naming a `<form>` wins over the
    /// ancestor, and reaches controls outside any form.
    #[test]
    fn form_attribute_names_the_owner() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let a = el(&mut dom, root, "form", &[("id", "a")]);
        let b = el(&mut dom, root, "form", &[("id", "b")]);
        let in_a_for_b = el(&mut dom, a, "input", &[("form", "b")]);
        let outside_for_a = el(&mut dom, root, "select", &[("form", "a")]);
        assert_eq!(dom.form_owner(in_a_for_b), Some(b));
        assert_eq!(dom.form_owner(outside_for_a), Some(a));
    }

    /// The `form` attribute's "otherwise" is no owner at all: an unknown
    /// id, or an id naming a non-form, leaves even a nested control
    /// unowned.
    #[test]
    fn form_attribute_that_names_no_form_means_no_owner() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let form = el(&mut dom, root, "form", &[("id", "f")]);
        el(&mut dom, root, "div", &[("id", "d")]);
        let unknown = el(&mut dom, form, "input", &[("form", "nope")]);
        let div_id = el(&mut dom, form, "button", &[("form", "d")]);
        let empty = el(&mut dom, form, "textarea", &[("form", "")]);
        assert_eq!(dom.form_owner(unknown), None);
        assert_eq!(dom.form_owner(div_id), None);
        assert_eq!(dom.form_owner(empty), None);
    }

    /// Only listed elements take `form=`; a disconnected control falls
    /// back to its ancestor (HTML: the attribute applies when connected).
    #[test]
    fn form_attribute_ignored_on_non_listed_and_disconnected_elements() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let f = el(&mut dom, root, "form", &[("id", "f")]);
        let g = el(&mut dom, root, "form", &[("id", "g")]);
        let label = el(&mut dom, g, "label", &[("form", "f")]);
        assert_eq!(dom.form_owner(label), Some(g));

        let detached = dom.create_element("form");
        let input = dom.create_element("input");
        dom.set_attribute(input, "form", "f").unwrap();
        dom.append_child(detached, input).unwrap();
        assert_eq!(dom.form_owner(input), Some(detached));
        let _ = f;
    }

    /// `form.elements`: tree order of the whole document, including
    /// controls outside the form that point at it, excluding controls
    /// inside it that point elsewhere.
    #[test]
    fn listed_elements_follow_document_tree_order() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let before = el(&mut dom, root, "input", &[("form", "f")]);
        let form = el(&mut dom, root, "form", &[("id", "f")]);
        let fs = el(&mut dom, form, "fieldset", &[]);
        let inner = el(&mut dom, fs, "input", &[]);
        el(&mut dom, form, "input", &[("form", "other")]);
        el(&mut dom, form, "div", &[]);
        let after = el(&mut dom, root, "button", &[("form", "f")]);
        el(&mut dom, root, "input", &[]);
        assert_eq!(
            dom.form_listed_elements(form),
            vec![before, fs, inner, after]
        );
    }

    #[test]
    fn submit_detail_reads_the_form_attributes() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let form = el(
            &mut dom,
            root,
            "form",
            &[
                ("action", "/save"),
                ("method", "POST"),
                ("enctype", "multipart/form-data"),
                ("target", "_blank"),
                ("novalidate", ""),
            ],
        );
        let plain = el(&mut dom, form, "button", &[]);
        let d = dom.submit_detail(form, Some(plain));
        assert_eq!(d.submitter, Some(plain));
        assert_eq!(d.action, "/save");
        assert_eq!(d.method, FormMethod::Post);
        assert_eq!(d.enctype, FormEnctype::MultipartFormData);
        assert_eq!(d.target, "_blank");
        assert!(d.no_validate);

        let bare = el(&mut dom, root, "form", &[("method", "bogus")]);
        let d = dom.submit_detail(bare, None);
        assert_eq!(
            (
                d.action.as_str(),
                d.method,
                d.enctype,
                d.target.as_str(),
                d.no_validate
            ),
            ("", FormMethod::Get, FormEnctype::UrlEncoded, "", false)
        );
    }

    /// §4.10.19.6: a submit button's `form*` attributes override the
    /// form's; `formmethod` with an invalid value is GET (no missing
    /// value default, so presence alone overrides).
    #[test]
    fn submit_detail_prefers_the_submit_buttons_overrides() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let form = el(
            &mut dom,
            root,
            "form",
            &[("action", "/a"), ("method", "post"), ("target", "t")],
        );
        let over = el(
            &mut dom,
            form,
            "input",
            &[
                ("type", "submit"),
                ("formaction", "/b"),
                ("formmethod", "dialog"),
                ("formenctype", "text/plain"),
                ("formtarget", "_self"),
                ("formnovalidate", ""),
            ],
        );
        let d = dom.submit_detail(form, Some(over));
        assert_eq!(d.action, "/b");
        assert_eq!(d.method, FormMethod::Dialog);
        assert_eq!(d.enctype, FormEnctype::TextPlain);
        assert_eq!(d.target, "_self");
        assert!(d.no_validate);

        let invalid = el(&mut dom, form, "button", &[("formmethod", "bogus")]);
        assert_eq!(
            dom.submit_detail(form, Some(invalid)).method,
            FormMethod::Get
        );

        // A non-submit button contributes no override.
        let reset = el(
            &mut dom,
            form,
            "button",
            &[("type", "reset"), ("formaction", "/r")],
        );
        let d = dom.submit_detail(form, Some(reset));
        assert_eq!((d.submitter, d.action.as_str()), (Some(reset), "/a"));
    }
}
