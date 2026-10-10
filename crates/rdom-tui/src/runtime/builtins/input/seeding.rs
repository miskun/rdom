//! Seeding: giving every text control the text node it edits.
//!
//! A text-family `<input>` keeps its value in a single text-node child
//! (seeded from its `value` attribute); a `<textarea>` keeps it in its
//! text child (seeded empty when it has none). The editing pipeline, the
//! caret, layout and paint all work on that text node, so a control
//! without one cannot show a caret or take a keystroke.
//!
//! Three moments seed a control:
//!
//! - **mount** — [`seed_all`], from `App::build`, over the whole tree;
//! - **insertion** — [`ControlSeeding`]: a mutation observer queues every
//!   element inserted into the tree (and every `<input>` whose `type`
//!   changes), and the `App` seeds the controls in those subtrees at its
//!   next boundary — before it handles the next event, and in the frame
//!   prelude before the cascade and layout (`INPUT-SEED-ON-INSERT-1`).
//!   A switched-in view's controls are therefore laid out with their
//!   text from their first frame, like the controls present at mount.
//!   Observers may not mutate the tree, so this cannot happen inside the
//!   insertion itself;
//! - **focus** — [`ensure_seeded`], from the focus path, for a control
//!   inserted and focused before that boundary (a listener that mounts a
//!   form and focuses its first field, `[autofocus]` run on a freshly
//!   mounted subtree, a `TuiDom` driven without an `App`). It is the
//!   only seeding those cases get, so it stays.
//!
//! Insertion and focus seed only a control *without* a text node: a
//! control moved within the tree keeps its live (possibly edited) text.

use std::cell::RefCell;
use std::rc::Rc;

use rdom_core::{Mutation, MutationObserver, NodeId, NodeType};

use super::{note_default_value, value};
use crate::{TuiDom, TuiExt};

/// Walk the DOM under `root` and ensure every `<input>` has a
/// single text-node child whose content matches its `value`
/// attribute (or `""` if none). Re-seeding an already-seeded input
/// is idempotent — the text child gets rewritten only when the
/// attribute and text content disagree, or when no text child
/// exists at all (caret needs a Text node to live on).
///
/// Called from `App::build` so parsed templates (`<input value="x">`
/// with no text child) and direct-API users (who may forget to
/// append a text child) both work. Controls inserted later are seeded
/// by the `App` at its next event or frame (module doc).
pub fn seed_all(dom: &mut TuiDom) {
    let mut inputs = Vec::new();
    walk_by_tag(dom, dom.root(), "input", &mut inputs);
    for id in inputs {
        // Only text-family inputs participate in the seed: a
        // checkbox / radio / submit button has no editable text
        // surface. What it displays — a toggle's glyph, a button's
        // `[ label ]` — comes from a UA `::before` content rule.
        if !crate::node::is_text_input(dom, id) {
            continue;
        }
        let want = dom
            .node(id)
            .get_attribute("value")
            .unwrap_or("")
            .to_string();
        let have = value(dom, id);
        if !has_text_child(dom, id) || want != have {
            let _ = crate::node::install_text_content(dom, id, &want);
        }
        note_default_value(dom, id);
    }

    // Textareas need an editable text child too. Unlike `<input>`,
    // a `<textarea>`'s initial content is its existing text child
    // (no `value` attribute), so we only seed when there isn't one.
    let mut textareas = Vec::new();
    walk_by_tag(dom, dom.root(), "textarea", &mut textareas);
    for id in textareas {
        ensure_seeded(dom, id);
    }
}

/// Ensure a single text-family `<input>` / `<textarea>` has its text-node
/// child, seeding it (from the `value` attribute, for an `<input>`) if
/// missing, and record its current value as its `defaultValue` if none is
/// known yet. Idempotent; a no-op on every other element, and it never
/// rewrites an existing text child.
///
/// The `App` seeds inserted controls at its next boundary; the focus
/// path calls this for a control focused before that boundary, so a
/// freshly-focused editable is always typeable, whenever it was created
/// (module doc).
pub fn ensure_seeded(dom: &mut TuiDom, id: NodeId) {
    let initial = match dom.node(id).tag_name() {
        Some("input") if crate::node::is_text_input(dom, id) => dom
            .node(id)
            .get_attribute("value")
            .unwrap_or("")
            .to_string(),
        Some("textarea") => String::new(),
        _ => return,
    };
    if !has_text_child(dom, id) {
        let _ = crate::node::install_text_content(dom, id, &initial);
    }
    note_default_value(dom, id);
}

/// Seed every control in the subtree rooted at `root` (inclusive).
fn seed_subtree(dom: &mut TuiDom, root: NodeId) {
    let mut controls = Vec::new();
    walk_by_tag(dom, root, "input", &mut controls);
    walk_by_tag(dom, root, "textarea", &mut controls);
    for id in controls {
        ensure_seeded(dom, id);
    }
}

fn has_text_child(dom: &TuiDom, id: NodeId) -> bool {
    dom.node(id)
        .child_nodes()
        .any(|c| c.node_type() == NodeType::Text)
}

fn walk_by_tag(dom: &TuiDom, id: NodeId, tag: &str, out: &mut Vec<NodeId>) {
    // Iterative: a DOM may be any depth (C16G-DEPTH-CAPS).
    out.extend(
        std::iter::once(id)
            .chain(dom.descendants(id))
            .filter(|&n| dom.node(n).tag_name() == Some(tag)),
    );
}

/// Elements inserted (or `<input>`s retyped) since the last flush.
type Queue = Rc<RefCell<Vec<NodeId>>>;

/// Seeds the controls inserted after `App::build` (module doc): an
/// installed mutation observer queues the inserted subtrees, and
/// [`ControlSeeding::flush`] seeds them.
pub(crate) struct ControlSeeding {
    queue: Queue,
}

impl ControlSeeding {
    /// Register the observer on `dom`.
    pub(crate) fn install(dom: &mut TuiDom) -> Self {
        let queue = Queue::default();
        dom.add_mutation_observer(Box::new(InsertionObserver {
            queue: queue.clone(),
        }));
        Self { queue }
    }

    /// Seed the controls in every queued subtree still in the document.
    /// The text nodes this installs queue nothing (only elements are
    /// queued). A subtree removed again before the flush is skipped; it
    /// is queued anew when it is reinserted.
    pub(crate) fn flush(&self, dom: &mut TuiDom) {
        let mut queued = std::mem::take(&mut *self.queue.borrow_mut());
        if queued.is_empty() {
            return;
        }
        queued.sort_unstable();
        queued.dedup();
        for root in queued {
            if dom.contains(root) && dom.node(root).is_connected() {
                seed_subtree(dom, root);
            }
        }
    }
}

struct InsertionObserver {
    queue: Queue,
}

impl MutationObserver<TuiExt> for InsertionObserver {
    fn observe(&mut self, dom: &mut TuiDom, record: &Mutation) {
        match record {
            Mutation::ChildListChanged { added, .. } => {
                let mut queue = self.queue.borrow_mut();
                queue.extend(
                    added
                        .iter()
                        .copied()
                        .filter(|&n| dom.node(n).node_type() == NodeType::Element),
                );
            }
            // A `type` change can turn an `<input>` into a text control.
            Mutation::AttributeChanged { id, name, .. }
                if name == "type" && dom.node(*id).tag_name() == Some("input") =>
            {
                self.queue.borrow_mut().push(*id);
            }
            _ => {}
        }
    }
}
