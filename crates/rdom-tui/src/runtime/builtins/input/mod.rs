//! `<input>` (text-family) value model.
//!
//! ## The `value`-attribute model
//!
//! HTML's `<input>` carries its current state in the `value`
//! attribute (technically `defaultValue` per spec, but this v1
//! collapses both into one). Editing infrastructure (Phase B)
//! operates on text nodes — so an `<input>` needs a text-node child
//! that mirrors the `value` attribute.
//!
//! Two integration points keep them in lockstep:
//!
//! - [`seed_all`] — walks the DOM once and gives every `<input>` a
//!   single text-child reflecting its `value` attribute (or empty
//!   string when absent). Called from `App::build` so parsed pages
//!   and direct API users land in the same shape.
//! - [`mirror_to_attribute`] — called after a successful edit on an
//!   `<input>` text child, copies the new text content back into the
//!   `value` attribute so `dom.node(input).get_attribute("value")`
//!   stays accurate.
//!
//! ## Limitations (v1)
//!
//! - Programmatic `set_attribute(input, "value", "x")` AFTER seed
//!   does NOT update the text content. Use [`set_value`] instead,
//!   which writes both the attribute and the text child.
//! - Only the text-family `type` values participate (text, password,
//!   email, url, tel, search, default). `type="checkbox"` etc. land
//!   in C.4b and use a different model.

use rdom_core::NodeId;
use unicode_segmentation::UnicodeSegmentation;

use crate::TuiDom;
use crate::render::paint_pass::ChromeText;

/// Read the live value of an `<input>`. Reads the text content of
/// the input's first text-node child (i.e., what the editing
/// pipeline has written). Returns `""` when the input has no text
/// child (typically a programmatically-created `<input>` that
/// wasn't routed through [`seed_all`] or [`set_value`]).
pub fn value(dom: &TuiDom, input: NodeId) -> String {
    let mut out = String::new();
    for child in dom.node(input).child_nodes() {
        if child.node_type() == rdom_core::NodeType::Text
            && let Some(s) = child.node_value()
        {
            out.push_str(s);
        }
    }
    out
}

/// Write the value of an `<input>` programmatically. Sets the
/// `value` attribute AND replaces the text-child contents so the
/// editing pipeline + paint pass agree.
///
/// Errors silently when `input` isn't an element node — matches the
/// generally-forgiving builder-chain style used elsewhere in
/// rdom-tui's helper API.
pub fn set_value(dom: &mut TuiDom, input: NodeId, new_value: &str) {
    // Setting the value programmatically leaves the default alone
    // (HTML: `.value =` does not touch `defaultValue`), so record the
    // authored value first if nothing has yet.
    note_default_value(dom, input);
    let _ = dom.set_attribute(input, "value", new_value);
    // Errors discarded at the boundary — see the function-level
    // docstring. The canonical helper (`crate::node::install_text_content`)
    // propagates, callers that want the forgiving style swallow here.
    let _ = crate::node::install_text_content(dom, input, new_value);
}

/// Walk the DOM under `root` and ensure every `<input>` has a
/// single text-node child whose content matches its `value`
/// attribute (or `""` if none). Re-seeding an already-seeded input
/// is idempotent — the text child gets rewritten only when the
/// attribute and text content disagree, or when no text child
/// exists at all (caret needs a Text node to live on).
///
/// Called from `App::build` so parsed templates (`<input value="x">`
/// with no text child) and direct-API users (who may forget to
/// append a text child) both work.
pub fn seed_all(dom: &mut TuiDom) {
    let inputs: Vec<NodeId> = collect_inputs(dom, dom.root());
    for id in inputs {
        // Only text-family inputs participate in the seed: a
        // checkbox / radio / submit button has no editable text
        // surface. What it displays — a toggle's glyph, a button's
        // `[ label ]` — comes from a UA `::before` content rule.
        if !is_text_family_input(dom, id) {
            continue;
        }
        let want = dom
            .node(id)
            .get_attribute("value")
            .unwrap_or("")
            .to_string();
        let have = value(dom, id);
        let has_text_child = dom
            .node(id)
            .child_nodes()
            .any(|c| c.node_type() == rdom_core::NodeType::Text);
        if !has_text_child || want != have {
            let _ = crate::node::install_text_content(dom, id, &want);
        }
        note_default_value(dom, id);
    }

    // Textareas need an editable text child too. Unlike `<input>`,
    // a `<textarea>`'s initial content is its existing text child
    // (no `value` attribute), so we only seed when there isn't one.
    let textareas: Vec<NodeId> = collect_textareas(dom, dom.root());
    for id in textareas {
        let has_text_child = dom
            .node(id)
            .child_nodes()
            .any(|c| c.node_type() == rdom_core::NodeType::Text);
        if !has_text_child {
            let _ = crate::node::install_text_content(dom, id, "");
        }
        note_default_value(dom, id);
    }
}

/// Does `id` keep a `defaultValue` apart from its live value — HTML's
/// value mode "value"? `<textarea>`, text-family `<input>`s and
/// `<input type=range>`. Every other `<input>` (hidden, submit, the
/// toggles, …) *is* its `value` attribute: its default is that
/// attribute and nothing is recorded.
pub(crate) fn keeps_default_value(dom: &TuiDom, id: NodeId) -> bool {
    match dom.node(id).tag_name() {
        Some("textarea") => true,
        Some("input") => is_text_family_input(dom, id) || is_range_input(dom, id),
        _ => false,
    }
}

fn is_range_input(dom: &TuiDom, id: NodeId) -> bool {
    dom.input_type_state(id) == Some(rdom_core::InputTypeState::Range)
}

/// The live value a default is captured from: a text control's text,
/// a range's `value` attribute (`""` when absent).
fn live_value(dom: &TuiDom, id: NodeId) -> String {
    if is_range_input(dom, id) {
        return dom
            .node(id)
            .get_attribute("value")
            .unwrap_or("")
            .to_string();
    }
    value(dom, id)
}

/// Record the control's current value as its `defaultValue` unless a
/// default is already known (`FORM-DEFAULTS-1`). Called when a control
/// is seeded and before its first change, so the authored value is the
/// one a `<form>` reset restores. No-op on controls without a separate
/// default ([`keeps_default_value`]).
pub(crate) fn note_default_value(dom: &mut TuiDom, control: NodeId) {
    if !keeps_default_value(dom, control) {
        return;
    }
    let known = dom
        .node(control)
        .ext()
        .is_some_and(|e| e.default_value.is_some());
    if known {
        return;
    }
    let current = live_value(dom, control);
    if let Some(ext) = dom.node_mut(control).ext_mut() {
        ext.default_value = Some(current);
    }
}

/// Restore a control to its `defaultValue`: a text control's text
/// content and, for an `<input>`, the mirrored `value` attribute; a
/// range's `value` attribute (removed when the default is `""`, which
/// puts the thumb back at the midpoint, as a browser sanitizes an empty
/// value). No-op without a recorded default.
pub(crate) fn reset_to_default(dom: &mut TuiDom, control: NodeId) {
    let Some(default) = dom
        .node(control)
        .ext()
        .and_then(|e| e.default_value.clone())
    else {
        return;
    };
    if is_range_input(dom, control) {
        if default.is_empty() {
            let _ = dom.remove_attribute(control, "value");
        } else {
            let _ = dom.set_attribute(control, "value", &default);
        }
        return;
    }
    let _ = crate::node::install_text_content(dom, control, &default);
    if dom.node(control).tag_name() == Some("input") {
        let _ = dom.set_attribute(control, "value", &default);
    }
}

/// Ensure a single editable `<input>` / `<textarea>` has its text-node child,
/// seeding it (from the `value` attribute, for text-family inputs) if missing.
/// Idempotent.
///
/// [`seed_all`] only runs once at `App::build`, so an editable added to the
/// DOM *after* that (a dynamically-mounted view, a runtime-built form) had no
/// text child — and editing/caret seeding silently no-op'd against it. The
/// focus path calls this so a freshly-focused editable is always typeable,
/// whenever it was created.
pub fn ensure_seeded(dom: &mut TuiDom, id: NodeId) {
    let has_text_child = dom
        .node(id)
        .child_nodes()
        .any(|c| c.node_type() == rdom_core::NodeType::Text);
    if has_text_child {
        return;
    }
    match dom.node(id).tag_name() {
        Some("input") if is_text_family_input(dom, id) => {
            let want = dom
                .node(id)
                .get_attribute("value")
                .unwrap_or("")
                .to_string();
            let _ = crate::node::install_text_content(dom, id, &want);
        }
        Some("textarea") => {
            let _ = crate::node::install_text_content(dom, id, "");
        }
        _ => {}
    }
}

fn is_text_family_input(dom: &TuiDom, id: NodeId) -> bool {
    crate::node::is_text_input(dom, id)
}

/// Mirror the input's current text content into its `value`
/// attribute. Called from `perform_edit` after a successful edit
/// commits, so apps reading `get_attribute("value")` always see
/// the live value.
///
/// `editable` is the editable element id from `perform_edit` (i.e.,
/// the `<input>` itself, since it IS the editable). No-op for any
/// non-`<input>` editable.
pub fn mirror_to_attribute(dom: &mut TuiDom, editable: NodeId) {
    if dom.node(editable).tag_name() != Some("input") {
        return;
    }
    let live = value(dom, editable);
    let _ = dom.set_attribute(editable, "value", &live);
}

// ── Internals ──────────────────────────────────────────────────────

/// Recursively collect every `<input>` element id under `root`
/// (inclusive). Used by `seed_all`.
fn collect_inputs(dom: &TuiDom, root: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    walk_by_tag(dom, root, "input", &mut out);
    out
}

/// Recursively collect every `<textarea>` element id under `root`
/// (inclusive). Used by `seed_all`.
fn collect_textareas(dom: &TuiDom, root: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    walk_by_tag(dom, root, "textarea", &mut out);
    out
}

fn walk_by_tag(dom: &TuiDom, id: NodeId, tag: &str, out: &mut Vec<NodeId>) {
    if dom.node(id).tag_name() == Some(tag) {
        out.push(id);
    }
    for child in dom.node(id).child_nodes() {
        walk_by_tag(dom, child.id(), tag, out);
    }
}

/// True iff `id` is `<input type="password">`.
fn is_password(dom: &TuiDom, id: NodeId) -> bool {
    dom.input_type_state(id) == Some(rdom_core::InputTypeState::Password)
}

/// The paint pass's chrome for a password input (see
/// `runtime::builtins::inline_chrome`): every grapheme cluster of the
/// live value replaced by the bullet `•`, so the value never paints.
/// The bullet is a single-cell glyph in monospace fonts, so the
/// masked width matches the grapheme count (not the display width —
/// wide chars mask to a single bullet, matching browser behavior).
/// `None` for every other element.
pub(crate) fn password_inline_chrome(dom: &TuiDom, id: NodeId, _width: u16) -> Option<ChromeText> {
    if !is_password(dom, id) {
        return None;
    }
    Some(ChromeText {
        text: value(dom, id).graphemes(true).map(|_| "\u{2022}").collect(),
        fg: None,
    })
}

#[cfg(test)]
mod tests;
