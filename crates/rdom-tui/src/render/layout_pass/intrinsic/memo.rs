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
//!
//! One size is not pure within a pass: a subgrid's (CSS Grid 2 §9) takes
//! its parent's laid-out tracks (`grid::subgrid::from_parent`), which the
//! parent's arrangement writes during the pass. It is never put in the
//! first table (`grid::reads_parent_lines`, C7G-MEMO-PURITY); the second
//! keys a subgrid's size by the tracks it inherits, so what it holds is
//! pure again.

use std::cell::RefCell;
use std::collections::HashMap;

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;

/// What a measurement is for: the element, the axis (`true` for the
/// Row axis), max-content (`true`) or min-content, the cross budget and
/// the containing block's width.
pub(super) type Key = (NodeId, bool, bool, u16, u16);

/// Document data while a layout pass runs: its measurements, and its
/// subgrid work (`grid::SubgridMemo`, keyed by what each subgrid
/// inherits), so nested subgrids are measured and flattened once a pass
/// whatever their depth.
#[derive(Debug, Default)]
struct PassMemo(
    RefCell<HashMap<Key, u16>>,
    RefCell<crate::render::layout_pass::grid::SubgridMemo>,
);

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

/// `f` on the open pass's subgrid memo; `None` outside a pass. The memo
/// is borrowed only while `f` runs: `f` must not measure.
pub(in crate::render::layout_pass) fn with_subgrids<R>(
    dom: &Dom<TuiExt>,
    f: impl FnOnce(&mut crate::render::layout_pass::grid::SubgridMemo) -> R,
) -> Option<R> {
    Some(f(&mut table(dom)?.1.borrow_mut()))
}

fn table(dom: &Dom<TuiExt>) -> Option<&PassMemo> {
    dom.document_data::<Option<PassMemo>>()?.as_ref()
}
