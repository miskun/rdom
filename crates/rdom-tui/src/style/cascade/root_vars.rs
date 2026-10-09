//! The custom properties a cascade pass seeds the root's parent with:
//! every sheet's `vars()` — the programmatic `define_var` variables,
//! beneath every rule (a `:root` rule is an ordinary rule matching the
//! root, `root`; the parse-time mirror of them is gone,
//! C14G-ROOT-ELEMENT) — merged, registered properties at their initial
//! values, and their `var()` / `attr()` substituted against each other
//! (CSS Variables 1 §3). Split out of `walk.rs` (C7G-SIZES).

use rdom_core::{Dom, NodeType};

use super::sheets::Sheets;
use crate::ext::TuiExt;
use crate::style::VarMap;

/// Merge `root_vars` across all registered sheets into a single
/// `VarMap`. Later sheets win per var name — push order is the
/// last-wins tiebreaker. Allocates one fresh `Rc<HashMap>` per call;
/// callers compute this once per cascade pass (in `cascade_all` /
/// `cascade_subtrees_all`) and `Rc::clone` from there per element.
///
/// They are the root's inherited custom properties, so their `attr()`s
/// read the element `:root` matches (CSS Values 5 §8.7, Selectors 4 §14.1): the
/// tree's root when it is an element; a fragment root has no
/// attributes (DIVERGENCES).
pub(super) fn merge_root_vars(dom: &Dom<TuiExt>, sheets: &Sheets<'_>) -> VarMap {
    let mut merged = std::collections::HashMap::new();
    for sheet in sheets.iter() {
        for (k, v) in sheet.vars() {
            merged.insert(k.clone(), v.clone());
        }
    }
    let root = dom.root();
    let root_attrs = |name: &str| dom.node(root).get_attribute(name);
    let attrs: Option<rdom_style::backend::AttrLookup<'_>> =
        (dom.node(root).node_type() == NodeType::Element).then_some(&root_attrs);
    // Their `var()`s substitute against each other (CSS Variables 1 §3).
    // Registered properties start at their initial value and are
    // validated as they resolve, before a dependent reads them
    // (Properties and Values 1 §2.1, §2.4); the root has no parent, so
    // an invalid one is its initial value.
    let registry = sheets.active_registry();
    if !registry.is_empty() {
        registry.seed_root(&mut merged, sheets.viewport_use());
    }
    let names: Vec<String> = merged.keys().cloned().collect();
    let mut cx = rdom_style::backend::SubstitutionContext::new();
    if let Some(attrs) = attrs {
        cx = cx.with_attrs(attrs);
    }
    let no_parent = std::collections::HashMap::new();
    let mut computed =
        |name: &str, value| registry.computed_value(name, value, &no_parent, sheets.viewport_use());
    if !registry.is_empty() {
        cx = cx.with_computed(&mut computed);
    }
    // An invalid one is the guaranteed-invalid value (removed); the
    // cascade does not report why.
    let _invalid = rdom_style::backend::resolve_custom_properties(
        &mut merged,
        names.iter().map(String::as_str),
        cx,
    );
    std::rc::Rc::new(merged)
}
