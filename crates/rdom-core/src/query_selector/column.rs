//! The column selectors (Selectors 4 §16, C13-COLUMN): the column
//! combinator's candidates and `:nth-col()` / `:nth-last-col()`, read from
//! HTML's table model (`table::html`), formed once per table per pass
//! ([`SelectorCaches`](super::SelectorCaches)).

use std::rc::Rc;

use super::matcher::Cx;
use crate::dom::Dom;
use crate::node_id::NodeId;
use crate::selectors::NthColumnSelector;
use crate::table::html::HtmlTableModel;

impl<Ext> Dom<Ext> {
    /// The table model of the table holding the cell `cell`, from the
    /// pass's caches; `None` when `cell` is no cell of an HTML table.
    fn cell_table_model(&self, cell: NodeId, cx: &mut Cx<'_>) -> Option<Rc<HtmlTableModel>> {
        let table = self.html_table_of_cell(cell)?;
        if let Some(model) = cx.caches.tables.get(&table) {
            return Some(model.clone());
        }
        cx.caches.count_table_model();
        let model = Rc::new(self.html_table_model(table));
        cx.caches.tables.insert(table, model.clone());
        Some(model)
    }

    /// Selectors 4 §16.1: the column elements the cell `cell` belongs to —
    /// each `<col>` / `<colgroup>` representing a column it spans — the
    /// candidates a column combinator relates it to.
    pub(super) fn column_elements_of(&self, cell: NodeId, cx: &mut Cx<'_>) -> Vec<NodeId> {
        self.cell_table_model(cell, cx)
            .map(|m| m.column_elements(cell).collect())
            .unwrap_or_default()
    }

    /// Selectors 4 §16.2 / §16.3: whether the cell `id` spans a column
    /// with `An+B - 1` columns before it (after it, for `last`).
    pub(super) fn matches_nth_column(
        &self,
        id: NodeId,
        nth: &NthColumnSelector,
        cx: &mut Cx<'_>,
    ) -> bool {
        let Some(model) = self.cell_table_model(id, cx) else {
            return false;
        };
        let Some((first, span)) = model.cell(id) else {
            return false;
        };
        (first..first + span).any(|c| {
            let index = if nth.last { model.width - c } else { c + 1 };
            u32::try_from(index).is_ok_and(|i| nth.matches_index(i))
        })
    }
}
