//! The content sizes measured during one layout pass, on both axes
//! (C5G-PERF-AND-TESTS for the Row axis, C6G-ATOM-COST for the Column
//! axis). `fit-content` and the other intrinsic keywords
//! measure a subtree's min- and max-content sizes, and an enclosing
//! keyword box measures it again, so nested keyword boxes re-walked
//! each subtree once per ancestor. Intrinsic sizes are pure within a
//! pass — the cascaded styles, the text and the table column widths
//! sized before it — so `layout_dom` opens a table for the pass and
//! drops it at the end: nothing is memoized across passes, or outside
//! one (the tree may have changed since). Document data, so `TuiExt`
//! carries nothing for it.

use std::cell::RefCell;
use std::collections::HashMap;

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;

/// What a measurement is for: the element, the axis (`true` for the
/// Row axis), max-content (`true`) or min-content, the cross budget and
/// the containing block's width.
pub(super) type Key = (NodeId, bool, bool, u16, u16);

/// Document data while a layout pass runs: its measurements.
#[derive(Debug, Default)]
struct PassMemo(RefCell<HashMap<Key, u16>>);

/// Open a layout pass: an empty table.
pub(in crate::render::layout_pass) fn begin_pass(dom: &mut Dom<TuiExt>) {
    dom.set_document_data(Some(PassMemo::default()));
}

/// Close the layout pass: its table is dropped.
pub(in crate::render::layout_pass) fn end_pass(dom: &mut Dom<TuiExt>) {
    dom.set_document_data::<Option<PassMemo>>(None);
}

/// The measurement memoized for `key` in the open pass; `None` outside
/// a pass or when not yet measured.
pub(super) fn get(dom: &Dom<TuiExt>, key: Key) -> Option<u16> {
    table(dom)?.0.borrow().get(&key).copied()
}

/// Record a measurement in the open pass (a no-op outside one).
pub(super) fn put(dom: &Dom<TuiExt>, key: Key, value: u16) {
    if let Some(t) = table(dom) {
        t.0.borrow_mut().insert(key, value);
    }
}

fn table(dom: &Dom<TuiExt>) -> Option<&PassMemo> {
    dom.document_data::<Option<PassMemo>>()?.as_ref()
}
