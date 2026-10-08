//! The document's highlight layers (CSS Pseudo-Elements 4 §3.5, CSS
//! Custom Highlight API 1 §5.2) indexed by text node: each registered
//! highlight's ranges by priority then registration order, then the
//! selection's range, the topmost layer. A painted fragment reads the
//! layers that start and end in its own text node and those that span
//! nodes — not every range of every highlight (C10G-HIGHLIGHT-COST).
//!
//! The layout pass builds the index for the paints after it
//! ([`prepare`], document data), keyed by the registry's generation
//! (`HighlightRegistry::generation`) and the selection; a paint after a
//! change no layout saw builds its own ([`current`]).

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use rdom_core::{Dom, NodeId, Range};

use crate::ext::TuiExt;

/// What one overlay layer paints in.
#[derive(Debug, Clone)]
pub(crate) enum Source {
    /// The registered highlight of that name.
    Highlight(Arc<str>),
    /// The document selection.
    Selection,
}

/// One range of one layer.
#[derive(Debug, Clone)]
pub(crate) struct Layer {
    pub(crate) range: Range,
    pub(crate) source: Source,
}

/// What an index was built from: the registry's generation and the
/// selection's range.
type Key = (u64, Option<Range>);

/// The document's overlay layers, bottom first, indexed by text node.
#[derive(Debug, Clone)]
pub(crate) struct OverlayIndex {
    key: Key,
    layers: Vec<Layer>,
    /// The layers whose range starts and ends in one node, by that node,
    /// in paint order.
    within: HashMap<NodeId, Vec<u32>>,
    /// The layers whose range spans nodes, in paint order: they may cover
    /// any text node, wholly or at an end.
    spanning: Vec<u32>,
}

/// The index the last layout built (document data).
#[derive(Debug, Default)]
struct Prepared(Option<OverlayIndex>);

/// The key of the document's overlays now; `None` with nothing to
/// highlight.
fn key_of(dom: &Dom<TuiExt>) -> Option<Key> {
    let selection = dom.selection_range().filter(|r| !r.is_collapsed());
    let registry = dom.highlights();
    if registry.is_empty() && selection.is_none() {
        return None;
    }
    Some((registry.generation(), selection))
}

impl OverlayIndex {
    /// The document's overlays under `key`: each registered highlight's
    /// ranges, by priority then registration order, then the selection.
    fn build(dom: &Dom<TuiExt>, key: Key) -> Self {
        let registry = dom.highlights();
        let mut order: Vec<(i32, usize, &str, &rdom_core::Highlight)> = registry
            .iter()
            .enumerate()
            .map(|(k, (name, h))| (h.priority(), k, name, h))
            .collect();
        order.sort_by_key(|&(priority, k, ..)| (priority, k));
        let mut index = OverlayIndex {
            key: key.clone(),
            layers: Vec::new(),
            within: HashMap::new(),
            spanning: Vec::new(),
        };
        for (.., name, h) in order {
            let name: Arc<str> = name.into();
            for range in h.ranges() {
                index.push(range.clone(), Source::Highlight(name.clone()));
            }
        }
        if let Some(range) = key.1 {
            index.push(range, Source::Selection);
        }
        index
    }

    fn push(&mut self, range: Range, source: Source) {
        #[cfg(test)]
        cost::RANGE_COPIES.with(|c| c.set(c.get() + 1));
        let k = self.layers.len() as u32;
        if range.start.node == range.end.node {
            self.within.entry(range.start.node).or_default().push(k);
        } else {
            self.spanning.push(k);
        }
        self.layers.push(Layer { range, source });
    }

    /// The layers that may cover text of `node`, bottom first: those
    /// within it and those spanning nodes, merged in paint order.
    pub(crate) fn layers_of(&self, node: NodeId) -> impl Iterator<Item = &Layer> {
        let within = self.within.get(&node).map_or(&[][..], Vec::as_slice);
        let (mut a, mut b) = (within.iter().peekable(), self.spanning.iter().peekable());
        std::iter::from_fn(move || {
            let next = match (a.peek(), b.peek()) {
                (Some(&&x), Some(&&y)) if x < y => a.next(),
                (Some(_), Some(_)) => b.next(),
                (Some(_), None) => a.next(),
                (None, _) => b.next(),
            };
            next.map(|&k| &self.layers[k as usize])
        })
    }
}

/// Build the overlay index for the paints after a layout (document
/// data): kept while its key holds, so an unchanged document builds it
/// once.
pub(crate) fn prepare(dom: &mut Dom<TuiExt>) {
    let key = key_of(dom);
    let current = dom
        .document_data::<Prepared>()
        .and_then(|p| p.0.as_ref())
        .map(|i| &i.key);
    if current == key.as_ref() {
        return;
    }
    if key.is_none() && dom.document_data::<Prepared>().is_none() {
        return;
    }
    let index = key.map(|key| OverlayIndex::build(dom, key));
    dom.set_document_data(Prepared(index));
}

/// The document's highlight layers now: the prepared index while it is
/// current, else one built for this paint. `None` when there is nothing
/// to highlight.
pub(crate) fn current(dom: &Dom<TuiExt>) -> Option<Cow<'_, OverlayIndex>> {
    let key = key_of(dom)?;
    let prepared = dom
        .document_data::<Prepared>()
        .and_then(|p| p.0.as_ref())
        .filter(|i| i.key == key);
    Some(match prepared {
        Some(index) => Cow::Borrowed(index),
        None => Cow::Owned(OverlayIndex::build(dom, key)),
    })
}

/// Test-only counters of the highlight layers' work.
#[cfg(test)]
pub(crate) mod cost {
    thread_local! {
        /// Ranges copied into an index.
        pub(crate) static RANGE_COPIES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
        /// Ranges tested against a painted fragment
        /// (`paint_pass::inline_paint::highlight_overlay`).
        pub(crate) static RANGE_TESTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }
}
