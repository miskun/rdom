//! The landmarks outline (`specs/CAPTURE.md` §Session protocol, the
//! first thing an agent reads): the elements a person navigates by —
//! landmarks, headings, lists, controls, focusable elements — nested as
//! the document nests them, each with its role, its accessible name, where
//! it is on screen and its state. Everything else is passed through: its
//! outline children are its descendants'.
//!
//! Roles follow WAI-ARIA: an explicit `role` attribute, else HTML-AAM's
//! implicit role for the tag (`nav` → `navigation`, `h2` → `heading`,
//! `a[href]` → `link`, a `header` / `footer` not inside sectioning content
//! → `banner` / `contentinfo`, …). The name is computed in the order
//! accname 1.2 §4.3.2 takes for these elements: `aria-labelledby`,
//! `aria-label`, a form control's `<label>`, an image's `alt`, a button
//! input's `value`, the element's text for the roles named from content,
//! then `placeholder` and `title`. This is a navigation
//! outline, not a full accessibility tree: it has no states beyond the
//! ones listed on [`OutlineNode::state`].

use rdom_tui::core_api::{NodeId, NodeType};
use rdom_tui::render::TestBackend;
use rdom_tui::{App, TuiDom};
use serde::{Deserialize, Serialize};

use crate::capture::node_key;

/// One element of the outline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct OutlineNode {
    /// The element's node name.
    pub node: String,
    /// Its role (`navigation`, `heading`, `button`, …).
    pub role: String,
    /// Its accessible name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Its border box, `[x, y, width, height]`.
    pub rect: [i32; 4],
    /// `focused`, `focus-within`, `hovered`, `checked`, `disabled`,
    /// `expanded`, `collapsed`, `selected`, `level-N` (headings).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub state: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<OutlineNode>,
}

/// The outline of `app`'s document; `max_depth` limits nesting (deeper
/// entries are dropped and counted in the second value).
pub(crate) fn outline(app: &App<TestBackend>, max_depth: usize) -> (Vec<OutlineNode>, usize) {
    let dom = app.dom();
    let mut dropped = 0;
    let out = children_of(dom, dom.root(), 0, max_depth, &mut dropped);
    (out, dropped)
}

fn children_of(
    dom: &TuiDom,
    parent: NodeId,
    depth: usize,
    max: usize,
    dropped: &mut usize,
) -> Vec<OutlineNode> {
    let mut out = Vec::new();
    let mut child = dom.node(parent).first_child().map(|c| c.id());
    while let Some(id) = child {
        child = dom.node(id).next_sibling().map(|c| c.id());
        if dom.node(id).node_type() != NodeType::Element {
            continue;
        }
        let Some(boxes) = rdom_tui::capture::box_model(dom, id) else {
            // No box (`display: none`, closed `<details>` contents): not
            // on screen, not in the outline — `display: contents` passes
            // its children through.
            if dom
                .node(id)
                .computed()
                .is_some_and(|c| c.display == rdom_tui::layout::Display::Contents)
            {
                out.extend(children_of(dom, id, depth, max, dropped));
            }
            continue;
        };
        match role(dom, id) {
            Some(role) => {
                if depth >= max {
                    *dropped += 1 + count(dom, id);
                    continue;
                }
                let b = boxes.border;
                out.push(OutlineNode {
                    node: node_key(id),
                    name: name(dom, id, &role),
                    state: state(dom, id, &role),
                    role,
                    rect: [b.x, b.y, i32::from(b.width), i32::from(b.height)],
                    children: children_of(dom, id, depth + 1, max, dropped),
                });
            }
            None => out.extend(children_of(dom, id, depth, max, dropped)),
        }
    }
    out
}

/// How many outline entries `id`'s subtree holds: elements with a role
/// and a box.
fn count(dom: &TuiDom, id: NodeId) -> usize {
    dom.descendants(id)
        .filter(|&n| {
            dom.node(n).node_type() == NodeType::Element
                && role(dom, n).is_some()
                && rdom_tui::capture::box_model(dom, n).is_some()
        })
        .count()
}

/// Whether `id` sits inside sectioning content or `main` (`article`,
/// `aside`, `main`, `nav`, `section`).
fn in_sectioning(dom: &TuiDom, id: NodeId) -> bool {
    let mut cur = dom.node(id).parent_node().map(|p| p.id());
    while let Some(p) = cur {
        let tag = dom.node(p).tag_name().map(str::to_ascii_lowercase);
        if matches!(
            tag.as_deref(),
            Some("article" | "aside" | "main" | "nav" | "section")
        ) {
            return true;
        }
        cur = dom.node(p).parent_node().map(|q| q.id());
    }
    false
}

use rdom_tui::TuiNodeExt;

/// The outline role of `id`: an explicit `role`, else the tag's implicit
/// one; `None` for an element the outline passes through.
pub(crate) fn role(dom: &TuiDom, id: NodeId) -> Option<String> {
    let n = dom.node(id);
    if let Some(r) = n
        .get_attribute("role")
        .and_then(|r| r.split_whitespace().next())
    {
        return (!matches!(r, "none" | "presentation" | "generic")).then(|| r.to_string());
    }
    let tag = n.tag_name()?.to_ascii_lowercase();
    let named =
        || n.get_attribute("aria-label").is_some() || n.get_attribute("aria-labelledby").is_some();
    let role = match tag.as_str() {
        // HTML-AAM: a `header` / `footer` scoped to sectioning content or
        // `main` is generic, not the page's banner / contentinfo.
        "header" if !in_sectioning(dom, id) => "banner",
        "footer" if !in_sectioning(dom, id) => "contentinfo",
        "nav" => "navigation",
        "main" => "main",
        "aside" => "complementary",
        "form" => "form",
        "search" => "search",
        "dialog" => "dialog",
        "section" if named() => "region",
        "article" => "article",
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => "heading",
        "button" => "button",
        "a" if n.get_attribute("href").is_some() => "link",
        "textarea" => "textbox",
        "select" => "combobox",
        "option" => "option",
        "ul" | "ol" | "menu" => "list",
        "li" => "listitem",
        "table" => "table",
        "tr" => "row",
        "th" => "columnheader",
        "td" => "cell",
        "details" => "group",
        "summary" => "button",
        "progress" => "progressbar",
        "meter" => "meter",
        "img" => "img",
        "fieldset" => "group",
        "input" => match n
            .get_attribute("type")
            .unwrap_or("text")
            .to_ascii_lowercase()
            .as_str()
        {
            "checkbox" => "checkbox",
            "radio" => "radio",
            "range" => "slider",
            "number" => "spinbutton",
            "search" => "searchbox",
            "button" | "submit" | "reset" => "button",
            "hidden" => return None,
            _ => "textbox",
        },
        _ if rdom_tui::runtime::focus::tabindex::is_focusable(dom, id) => "focusable",
        _ => return None,
    };
    Some(role.to_string())
}

/// Roles whose name comes from their content (WAI-ARIA §5.2.8.6).
fn named_from_content(role: &str) -> bool {
    matches!(
        role,
        "button"
            | "link"
            | "heading"
            | "listitem"
            | "option"
            | "cell"
            | "columnheader"
            | "row"
            | "treeitem"
            | "tab"
            | "menuitem"
            | "checkbox"
            | "radio"
            | "switch"
    )
}

/// `text` with whitespace collapsed and cut to 60 characters.
fn tidy(text: &str) -> Option<String> {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return None;
    }
    let mut chars = collapsed.chars();
    let cut: String = chars.by_ref().take(60).collect();
    Some(if chars.next().is_some() {
        format!("{cut}…")
    } else {
        cut
    })
}

/// The accessible name of `id` in `role`.
pub(crate) fn name(dom: &TuiDom, id: NodeId, role: &str) -> Option<String> {
    let n = dom.node(id);
    if let Some(ids) = n.get_attribute("aria-labelledby") {
        let text: Vec<String> = ids
            .split_whitespace()
            .filter_map(|i| dom.get_element_by_id(i))
            .map(|r| dom.text_content(r))
            .collect();
        if let Some(t) = tidy(&text.join(" ")) {
            return Some(t);
        }
    }
    if let Some(l) = n.get_attribute("aria-label").and_then(tidy) {
        return Some(l);
    }
    if let Some(label) = label_text(dom, id) {
        return Some(label);
    }
    let tag = n.tag_name().map(str::to_ascii_lowercase);
    if tag.as_deref() == Some("img")
        && let Some(alt) = n.get_attribute("alt").and_then(tidy)
    {
        return Some(alt);
    }
    if tag.as_deref() == Some("input")
        && matches!(n.get_attribute("type"), Some("button" | "submit" | "reset"))
        && let Some(v) = n.get_attribute("value").and_then(tidy)
    {
        return Some(v);
    }
    if named_from_content(role)
        && let Some(t) = tidy(&dom.text_content(id))
    {
        return Some(t);
    }
    n.get_attribute("placeholder")
        .and_then(tidy)
        .or_else(|| n.get_attribute("title").and_then(tidy))
}

/// The text of the `<label>` of a form control: one naming it by `for`, or
/// one wrapping it.
fn label_text(dom: &TuiDom, id: NodeId) -> Option<String> {
    let n = dom.node(id);
    let tag = n.tag_name()?.to_ascii_lowercase();
    if !matches!(
        tag.as_str(),
        "input" | "select" | "textarea" | "meter" | "progress"
    ) {
        return None;
    }
    if let Some(own) = n.get_attribute("id") {
        let by_for = dom.descendants(dom.root()).find(|&l| {
            dom.node(l)
                .tag_name()
                .is_some_and(|t| t.eq_ignore_ascii_case("label"))
                && dom.node(l).get_attribute("for") == Some(own)
        });
        if let Some(l) = by_for {
            return tidy(&dom.text_content(l));
        }
    }
    let mut cur = n.parent_node().map(|p| p.id());
    while let Some(p) = cur {
        if dom
            .node(p)
            .tag_name()
            .is_some_and(|t| t.eq_ignore_ascii_case("label"))
        {
            return tidy(&dom.text_content(p));
        }
        cur = dom.node(p).parent_node().map(|q| q.id());
    }
    None
}

/// The states of `id` the outline reports.
fn state(dom: &TuiDom, id: NodeId, role: &str) -> Vec<String> {
    let n = dom.node(id);
    let mut out = Vec::new();
    match dom.focused() {
        Some(f) if f == id => out.push("focused".to_string()),
        Some(f) if dom.descendants(id).any(|d| d == f) => out.push("focus-within".to_string()),
        _ => {}
    }
    if dom.hovered() == Some(id) {
        out.push("hovered".to_string());
    }
    let attr = |a: &str| n.get_attribute(a);
    if attr("checked").is_some() || attr("aria-checked") == Some("true") {
        out.push("checked".to_string());
    }
    if attr("disabled").is_some() || attr("aria-disabled") == Some("true") {
        out.push("disabled".to_string());
    }
    match (
        n.tag_name().map(str::to_ascii_lowercase).as_deref(),
        attr("aria-expanded"),
    ) {
        (_, Some("true")) => out.push("expanded".to_string()),
        (_, Some("false")) => out.push("collapsed".to_string()),
        (Some("details"), _) => out.push(
            if attr("open").is_some() {
                "expanded"
            } else {
                "collapsed"
            }
            .to_string(),
        ),
        _ => {}
    }
    if attr("aria-selected") == Some("true") || attr("selected").is_some() {
        out.push("selected".to_string());
    }
    if role == "heading"
        && let Some(level) = n
            .tag_name()
            .and_then(|t| t.strip_prefix(['h', 'H']))
            .and_then(|l| l.parse::<u8>().ok())
            .or_else(|| attr("aria-level").and_then(|l| l.parse().ok()))
    {
        out.push(format!("level-{level}"));
    }
    out
}
