//! [`NodeMap`]: a JSON object keyed by node name that keeps document
//! order — a `BTreeMap<String, _>` would put `"10"` before `"9"`.

use std::fmt;
use std::marker::PhantomData;

use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Entries keyed by node name, in document order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeMap<T>(Vec<(String, T)>);

impl<T> Default for NodeMap<T> {
    fn default() -> Self {
        NodeMap(Vec::new())
    }
}

impl<T> NodeMap<T> {
    /// The entry of node `key`.
    pub fn get(&self, key: &str) -> Option<&T> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// Every entry, in document order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &T)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// How many entries.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<T> FromIterator<(String, T)> for NodeMap<T> {
    fn from_iter<I: IntoIterator<Item = (String, T)>>(iter: I) -> Self {
        NodeMap(iter.into_iter().collect())
    }
}

impl<T> std::ops::Index<&str> for NodeMap<T> {
    type Output = T;
    fn index(&self, key: &str) -> &T {
        self.get(key).unwrap_or_else(|| panic!("no node {key:?}"))
    }
}

impl<T: Serialize> Serialize for NodeMap<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for NodeMap<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V<T>(PhantomData<T>);
        impl<'de, T: Deserialize<'de>> Visitor<'de> for V<T> {
            type Value = NodeMap<T>;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an object keyed by node name")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
                let mut out = Vec::new();
                while let Some(entry) = access.next_entry()? {
                    out.push(entry);
                }
                Ok(NodeMap(out))
            }
        }
        deserializer.deserialize_map(V(PhantomData))
    }
}
