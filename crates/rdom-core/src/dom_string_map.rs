//! `DomStringMap` + `DomStringMapMut` — DOM-faithful read /
//! write wrapper for `data-*` attributes (the `el.dataset` IDL
//! property).
//!
//! ## Naming convention
//!
//! camelCase ↔ kebab-case is the load-bearing JS ↔ Rust mapping
//! (HTML spec, §custom-data-attribute):
//!
//! | JS dataset key | Rust dataset key | Underlying attribute |
//! |---|---|---|
//! | `el.dataset.foo`        | `dataset().get("foo")`        | `data-foo`        |
//! | `el.dataset.fooBar`     | `dataset().get("fooBar")`     | `data-foo-bar`    |
//! | `el.dataset.fooBARbaz`  | `dataset().get("fooBARbaz")`  | `data-foo-b-a-rbaz` |
//!
//! ## Edge cases
//!
//! - `data-` alone (empty key) is excluded.
//! - `data-Foo` is not a valid HTML data attribute name (uppercase
//!   letters aren't allowed). [`DomStringMap::iter`] skips
//!   entries whose attribute name doesn't round-trip through the
//!   conversion.

use crate::accessor::{NodeMut, NodeRef};
use crate::error::Result;

/// Read-side view of `el.dataset`. Borrows the element via
/// `NodeRef`; lookup converts a camelCase key to a `data-*`
/// attribute name on the fly.
pub struct DomStringMap<'a, Ext: 'static> {
    node: NodeRef<'a, Ext>,
}

impl<'a, Ext: 'static> DomStringMap<'a, Ext> {
    /// Construct from a borrowed `NodeRef`. Used by the
    /// `NodeRef::dataset()` integration (M4b step 19) and tests.
    pub fn new(node: NodeRef<'a, Ext>) -> Self {
        Self { node }
    }

    /// Read the `data-*` attribute corresponding to `key`. Returns
    /// `None` when no such attribute is set. DOM
    /// `el.dataset[key]`.
    pub fn get(&self, key: &str) -> Option<&'a str> {
        let attr = camel_to_data_attr(key)?;
        self.node.get_attribute(&attr)
    }

    /// `true` iff a `data-*` attribute corresponding to `key`
    /// exists on the element. DOM `key in el.dataset`.
    pub fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// Iterate `(key, value)` pairs over every `data-*` attribute
    /// that round-trips through the camelCase ↔ kebab-case rule.
    /// Attributes like `data-Foo` (uppercase) or `data-` (empty
    /// key) are skipped because they aren't valid HTML data
    /// attributes and don't have a JS-side key.
    pub fn iter(&self) -> impl Iterator<Item = (String, &'a str)> + 'a {
        // Go through the Dom borrow directly (not via `&self`) so
        // the resulting iterator has lifetime `'a`, not `'self`.
        self.node
            .dom
            .attributes(self.node.id)
            .filter_map(|(name, value)| Some((data_attr_to_camel(name)?, value)))
    }

    /// Count of valid `data-*` attributes (entries that would
    /// appear in [`Self::iter`]).
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    /// `true` iff [`Self::len`] is zero.
    pub fn is_empty(&self) -> bool {
        self.iter().next().is_none()
    }
}

/// Write-side view of `el.dataset`. Borrows the element via
/// `NodeMut`; mutations route through `set_attribute` /
/// `remove_attribute` so the existing attribute-change mutation
/// observer fires.
pub struct DomStringMapMut<'a, Ext: 'static> {
    node: NodeMut<'a, Ext>,
}

impl<'a, Ext: 'static> DomStringMapMut<'a, Ext> {
    /// Construct from a borrowed `NodeMut`. Used by
    /// `NodeMut::dataset_mut()` (M4b step 19) and tests.
    pub fn new(node: NodeMut<'a, Ext>) -> Self {
        Self { node }
    }

    /// Set the `data-*` attribute corresponding to `key`. Errors
    /// only when the underlying node isn't an element. DOM
    /// `el.dataset[key] = value`.
    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        let Some(attr) = camel_to_data_attr(key) else {
            return Ok(()); // empty key — DOM ignores; we match.
        };
        self.node.set_attribute(&attr, value).map(|_| ())
    }

    /// Remove the `data-*` attribute corresponding to `key`.
    /// Returns `true` iff the attribute was present (and is now
    /// gone). DOM `delete el.dataset[key]`.
    pub fn remove(&mut self, key: &str) -> Result<bool> {
        let Some(attr) = camel_to_data_attr(key) else {
            return Ok(false);
        };
        self.node.remove_attribute(&attr)
    }
}

/// Convert a JS-side dataset key (e.g. `"fooBar"`) to the
/// underlying HTML attribute name (e.g. `"data-foo-bar"`). Returns
/// `None` for the empty string (DOM excludes the empty key).
fn camel_to_data_attr(key: &str) -> Option<String> {
    if key.is_empty() {
        return None;
    }
    let mut out = String::with_capacity(key.len() + 6);
    out.push_str("data-");
    for c in key.chars() {
        if c.is_ascii_uppercase() {
            out.push('-');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    Some(out)
}

/// Convert an HTML attribute name (e.g. `"data-foo-bar"`) to the
/// JS-side dataset key (e.g. `"fooBar"`). Returns `None` when:
///
/// - The name doesn't start with `"data-"`.
/// - The name is just `"data-"` (no key part).
/// - The name contains uppercase letters (not a valid HTML data
///   attribute name).
/// - The name has a trailing hyphen or a hyphen followed by a
///   non-lowercase-letter (wouldn't round-trip back).
fn data_attr_to_camel(name: &str) -> Option<String> {
    let suffix = name.strip_prefix("data-")?;
    if suffix.is_empty() {
        return None;
    }
    // Reject any uppercase — HTML data attribute names are all
    // lowercase, and a stray uppercase wouldn't round-trip.
    if suffix.chars().any(|c| c.is_ascii_uppercase()) {
        return None;
    }
    let mut out = String::with_capacity(suffix.len());
    let mut chars = suffix.chars();
    while let Some(c) = chars.next() {
        if c == '-' {
            // Hyphen — must be followed by lowercase ASCII letter
            // to round-trip. Anything else (digit, hyphen, end)
            // means the source attribute name isn't a valid
            // round-tripable data attribute.
            match chars.next() {
                Some(next) if next.is_ascii_lowercase() => {
                    out.push(next.to_ascii_uppercase());
                }
                _ => return None,
            }
        } else {
            out.push(c);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Dom;

    // ── Conversion helpers ────────────────────────────────────────────

    #[test]
    fn camel_to_data_attr_simple_key() {
        assert_eq!(camel_to_data_attr("foo"), Some("data-foo".into()));
    }

    #[test]
    fn camel_to_data_attr_uppercase_becomes_hyphen_lowercase() {
        // Canonical step-12 mapping: "fooBar" → "data-foo-bar".
        assert_eq!(camel_to_data_attr("fooBar"), Some("data-foo-bar".into()));
    }

    #[test]
    fn camel_to_data_attr_consecutive_uppercase_each_gets_hyphen() {
        // "fooBARbaz" → "data-foo-b-a-rbaz" per the HTML spec.
        assert_eq!(
            camel_to_data_attr("fooBARbaz"),
            Some("data-foo-b-a-rbaz".into())
        );
    }

    #[test]
    fn camel_to_data_attr_empty_key_rejected() {
        assert_eq!(camel_to_data_attr(""), None);
    }

    #[test]
    fn data_attr_to_camel_simple() {
        assert_eq!(data_attr_to_camel("data-foo"), Some("foo".into()));
    }

    #[test]
    fn data_attr_to_camel_hyphen_lowercase_becomes_uppercase() {
        assert_eq!(data_attr_to_camel("data-foo-bar"), Some("fooBar".into()));
    }

    #[test]
    fn data_attr_to_camel_round_trips_consecutive_uppercase_pattern() {
        assert_eq!(
            data_attr_to_camel("data-foo-b-a-rbaz"),
            Some("fooBARbaz".into())
        );
    }

    #[test]
    fn data_attr_to_camel_rejects_non_data_prefix() {
        assert_eq!(data_attr_to_camel("foo"), None);
        assert_eq!(data_attr_to_camel("class"), None);
    }

    #[test]
    fn data_attr_to_camel_rejects_empty_suffix() {
        assert_eq!(data_attr_to_camel("data-"), None);
    }

    #[test]
    fn data_attr_to_camel_rejects_uppercase_in_attr() {
        // `data-Foo` is not a valid HTML data attribute name.
        assert_eq!(data_attr_to_camel("data-Foo"), None);
    }

    #[test]
    fn data_attr_to_camel_rejects_trailing_hyphen() {
        // `data-foo-` would convert to `foo` then a stray hyphen
        // that doesn't round-trip cleanly.
        assert_eq!(data_attr_to_camel("data-foo-"), None);
    }

    #[test]
    fn data_attr_to_camel_rejects_double_hyphen() {
        // `data-foo--bar` — hyphen followed by hyphen has no
        // letter to uppercase, so reject.
        assert_eq!(data_attr_to_camel("data-foo--bar"), None);
    }

    // ── DomStringMap (read) ───────────────────────────────────────────

    fn element_with_attrs(pairs: &[(&str, &str)]) -> (Dom, crate::node_id::NodeId) {
        let mut dom: Dom = Dom::new();
        let el = dom.create_element("div");
        for (k, v) in pairs {
            dom.set_attribute(el, k, v).unwrap();
        }
        (dom, el)
    }

    #[test]
    fn get_foo_bar_reads_data_foo_bar() {
        // Canonical step-12 failing test: dataset().get("fooBar")
        // returns the value of the data-foo-bar attribute.
        let (dom, el) = element_with_attrs(&[("data-foo-bar", "value!")]);
        let ds = DomStringMap::new(dom.node(el));
        assert_eq!(ds.get("fooBar"), Some("value!"));
        assert!(ds.contains_key("fooBar"));
    }

    #[test]
    fn get_missing_returns_none() {
        let (dom, el) = element_with_attrs(&[]);
        let ds = DomStringMap::new(dom.node(el));
        assert_eq!(ds.get("foo"), None);
        assert!(!ds.contains_key("foo"));
    }

    #[test]
    fn iter_returns_round_tripable_attrs_with_camel_keys() {
        let (dom, el) = element_with_attrs(&[
            ("data-foo", "1"),
            ("data-foo-bar", "2"),
            ("class", "ignored"),
            ("id", "ignored"),
        ]);
        let ds = DomStringMap::new(dom.node(el));
        let mut entries: Vec<(String, &str)> = ds.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(
            entries,
            vec![("foo".to_string(), "1"), ("fooBar".to_string(), "2"),]
        );
    }

    #[test]
    fn iter_skips_invalid_data_attrs() {
        let (dom, el) = element_with_attrs(&[("data-foo", "ok"), ("data-", "skipped-empty")]);
        let ds = DomStringMap::new(dom.node(el));
        let entries: Vec<(String, &str)> = ds.iter().collect();
        assert_eq!(entries, vec![("foo".to_string(), "ok")]);
    }

    #[test]
    fn len_counts_only_valid_data_attrs() {
        let (dom, el) = element_with_attrs(&[
            ("data-foo", "1"),
            ("data-bar-baz", "2"),
            ("class", "ignored"),
        ]);
        let ds = DomStringMap::new(dom.node(el));
        assert_eq!(ds.len(), 2);
        assert!(!ds.is_empty());
    }

    #[test]
    fn empty_dataset_is_empty() {
        let (dom, el) = element_with_attrs(&[("class", "not-data")]);
        let ds = DomStringMap::new(dom.node(el));
        assert_eq!(ds.len(), 0);
        assert!(ds.is_empty());
    }

    // ── DomStringMapMut (write) ──────────────────────────────────────

    #[test]
    fn set_writes_camel_key_as_kebab_data_attr() {
        let (mut dom, el) = element_with_attrs(&[]);
        let mut ds = DomStringMapMut::new(dom.node_mut(el));
        ds.set("fooBar", "X").unwrap();
        // Read back via the underlying attribute.
        assert_eq!(dom.node(el).get_attribute("data-foo-bar"), Some("X"));
    }

    #[test]
    fn set_then_get_round_trips() {
        let (mut dom, el) = element_with_attrs(&[]);
        {
            let mut ds = DomStringMapMut::new(dom.node_mut(el));
            ds.set("fooBARbaz", "weird").unwrap();
        }
        let ds_read = DomStringMap::new(dom.node(el));
        assert_eq!(ds_read.get("fooBARbaz"), Some("weird"));
    }

    #[test]
    fn remove_returns_true_when_present() {
        let (mut dom, el) = element_with_attrs(&[("data-foo", "1")]);
        let mut ds = DomStringMapMut::new(dom.node_mut(el));
        assert!(ds.remove("foo").unwrap());
        assert!(!ds.remove("foo").unwrap(), "second remove finds nothing");
    }

    #[test]
    fn set_empty_key_is_ignored() {
        let (mut dom, el) = element_with_attrs(&[]);
        let mut ds = DomStringMapMut::new(dom.node_mut(el));
        ds.set("", "value").unwrap();
        // No `data-` attribute should have been written.
        assert!(dom.node(el).get_attribute("data-").is_none());
    }
}
