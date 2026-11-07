//! The nodes and boxes layers: what each element a capture names is, and
//! where its boxes are.
//!
//! A node is named by its arena id as a string — `"12"`, or `"12@2"` for a
//! slot reused — valid within one capture (`specs/CAPTURE.md` §Node
//! identity; matching nodes across captures is the diff's job).

use rdom_tui::TuiDom;
use rdom_tui::capture::{BoxModel, box_model};
use rdom_tui::core_api::NodeId;
use rdom_tui::layout::LayoutRect;
use serde::{Deserialize, Serialize};

/// A node's name in a capture.
pub fn node_key(id: NodeId) -> String {
    match id.generation() {
        1 => id.as_u32().to_string(),
        g => format!("{}@{g}", id.as_u32()),
    }
}

/// What an element is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NodeInfo {
    /// Its tag name.
    pub tag: String,
    /// Its `id` attribute.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Its classes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<String>,
    /// Its parent element.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// A selector-like path from the root: `main > section#settings.panel`.
    pub path: String,
    /// Its identity across captures (`capture::identity`): what a diff
    /// matches it by.
    pub key: String,
    /// Which rule named it: its `id`, its `data-key`, or its fingerprint.
    pub key_by: super::By,
}

/// An element's boxes, each `[x, y, width, height]` in cells.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Boxes {
    pub border: [i32; 4],
    pub padding: [i32; 4],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<[i32; 4]>,
    /// The scroll container's scrollport, offset `[left, top]` and
    /// scrollable size `[width, height]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scroll: Option<Scroll>,
    /// The fragments of a box a fragmented flow split.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fragments: Vec<[i32; 4]>,
}

/// A scroll container's scroll state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Scroll {
    pub port: [i32; 4],
    pub offset: [i32; 2],
    pub size: [u32; 2],
}

fn rect(r: LayoutRect) -> [i32; 4] {
    [r.x, r.y, i32::from(r.width), i32::from(r.height)]
}

impl From<BoxModel> for Boxes {
    fn from(b: BoxModel) -> Self {
        Boxes {
            border: rect(b.border),
            padding: rect(b.padding),
            content: b.content.map(rect),
            scroll: b.scroll.map(|s| Scroll {
                port: rect(s.port),
                offset: [s.offset.0, s.offset.1],
                size: [s.size.0, s.size.1],
            }),
            fragments: b.fragments.into_iter().map(rect).collect(),
        }
    }
}

/// `id` as an element's selector step: `tag#id.class.class`.
fn step(dom: &TuiDom, id: NodeId) -> String {
    let n = dom.node(id);
    let mut s = n.tag_name().unwrap_or_default().to_ascii_lowercase();
    if let Some(i) = n.get_attribute("id") {
        s.push('#');
        s.push_str(&escape_ident(i));
    }
    for c in dom.class_list(id) {
        s.push('.');
        s.push_str(&escape_ident(c));
    }
    s
}

/// The element ancestors of `id` and `id`, root first.
fn element_path(dom: &TuiDom, id: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    let mut cur = Some(id);
    while let Some(n) = cur {
        if dom.node(n).node_type() == rdom_tui::core_api::NodeType::Element {
            out.push(n);
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    }
    out.reverse();
    out
}

/// Element `id`'s path from the root: `main > section#settings.panel`.
pub(crate) fn path(dom: &TuiDom, id: NodeId) -> String {
    element_path(dom, id)
        .iter()
        .map(|p| step(dom, *p))
        .collect::<Vec<_>>()
        .join(" > ")
}

/// What element `id` is, its identity looked up in `identities` (computed
/// once per request: `identity::identities` walks the document).
pub(crate) fn node_info_in(
    dom: &TuiDom,
    id: NodeId,
    identities: &std::collections::HashMap<NodeId, super::identity::Identity>,
) -> NodeInfo {
    let n = dom.node(id);
    let identity = identities.get(&id);
    let path = element_path(dom, id);
    NodeInfo {
        tag: n.tag_name().unwrap_or_default().to_ascii_lowercase(),
        id: n.get_attribute("id").map(str::to_string),
        classes: dom.class_list(id).map(str::to_string).collect(),
        parent: path.iter().rev().nth(1).map(|p| node_key(*p)),
        path: path
            .iter()
            .map(|p| step(dom, *p))
            .collect::<Vec<_>>()
            .join(" > "),
        key: identity.map_or_else(String::new, |i| i.key.clone()),
        key_by: identity.map_or(super::By::Fingerprint, |i| i.by),
    }
}

/// The boxes of every element that has one, in document order.
pub(crate) fn boxes(dom: &TuiDom) -> Vec<(NodeId, Boxes)> {
    dom.descendants(dom.root())
        .filter_map(|id| box_model(dom, id).map(|b| (id, Boxes::from(b))))
        .collect()
}

/// Every element of the document, in document order.
pub(crate) fn elements(dom: &TuiDom) -> Vec<NodeId> {
    dom.descendants(dom.root())
        .filter(|&id| dom.node(id).node_type() == rdom_tui::core_api::NodeType::Element)
        .collect()
}

/// What each of `ids` is, keyed by node name, in document order.
pub(crate) fn nodes(
    dom: &TuiDom,
    ids: impl IntoIterator<Item = NodeId>,
) -> super::NodeMap<NodeInfo> {
    let identities = super::identity::identities(dom);
    ids.into_iter()
        .map(|id| (node_key(id), node_info_in(dom, id, &identities)))
        .collect()
}

/// `s` as a CSS identifier (CSSOM §2.1 `serialize an identifier`, for the
/// characters a class or id holds): a character outside `[A-Za-z0-9_-]`
/// and non-ASCII is escaped with a backslash, a leading digit as a code
/// point.
fn escape_ident(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for (i, c) in s.chars().enumerate() {
        let leading_digit = i == 0 && c.is_ascii_digit();
        if leading_digit {
            out.push_str(&format!("\\{:x} ", u32::from(c)));
        } else if c.is_ascii_alphanumeric() || c == '_' || c == '-' || !c.is_ascii() {
            out.push(c);
        } else {
            out.push('\\');
            out.push(c);
        }
    }
    out
}
