//! The `pattern` attribute (HTML §4.10.5.3.6): the value must match the
//! whole pattern, compiled as `^(?:pattern)$`.
//!
//! rdom compiles it with the `regex` crate, not a JavaScript `v`-flag
//! RegExp engine (DIVERGENCES): the common syntax agrees (including
//! the `v` flag's nested classes and `--` / `&&` set operations), but
//! `\d`, `\w`, `\b` are Unicode-aware, look-around and backreferences
//! are unsupported (such a pattern fails to compile), and `\q{…}` /
//! properties of strings are not parsed. A pattern that does not
//! compile imposes no constraint, as in HTML.

use std::cell::RefCell;

use rdom_core::NodeId;

use crate::TuiDom;

/// Per-control cache of the compiled `pattern`: the source it was
/// compiled from and the result (`None` when it did not compile).
/// Validity is computed on every check and on every `:invalid` match;
/// recompiling each time would dominate. A cache, not state: clones
/// start empty and every two caches compare equal.
#[derive(Debug, Default)]
pub(crate) struct PatternCache(RefCell<Option<(String, Option<regex::Regex>)>>);

impl Clone for PatternCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl PartialEq for PatternCache {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

/// Whether any of `values` fails `id`'s `pattern`. False without a
/// `pattern` or when it does not compile.
pub(super) fn mismatch(dom: &TuiDom, id: NodeId, values: &[&str]) -> bool {
    let Some(source) = dom.node(id).get_attribute("pattern") else {
        return false;
    };
    let Some(ext) = dom.node(id).ext() else {
        return false;
    };
    let mut cache = ext.pattern_cache.0.borrow_mut();
    if cache.as_ref().is_none_or(|(s, _)| s != source) {
        let compiled = regex::Regex::new(&format!("^(?:{source})$")).ok();
        *cache = Some((source.to_string(), compiled));
    }
    let Some((_, Some(re))) = cache.as_ref() else {
        return false;
    };
    values.iter().any(|v| !re.is_match(v))
}
