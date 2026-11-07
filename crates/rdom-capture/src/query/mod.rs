//! The answering methods (`specs/CAPTURE.md` §Session protocol): the
//! landmarks [`outline`](outline::outline), `explain` and `query` — each
//! sized to the question with a [`Detail`] level and an optional token
//! budget ([`fit`]), and saying when it cut an answer short.

pub(crate) mod explain;
pub(crate) mod find;
pub(crate) mod outline;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// How much an answer says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Detail {
    /// The least that answers: winners only, the frame's operations only,
    /// a shallow outline.
    Outline,
    /// The default: winners with a few of what they overrode.
    #[default]
    Summary,
    /// Everything.
    Full,
}

/// About four characters a token: the budget's unit, approximate on
/// purpose (no tokenizer is shipped).
const CHARS_PER_TOKEN: usize = 4;

/// Cut `answer` to about `budget` tokens of compact JSON, longest lists
/// first, from their ends; each cut is recorded under `"truncated"` as
/// `{path: dropped}`, so nothing disappears silently. `None`: no budget.
pub(crate) fn fit(answer: &mut Value, budget: Option<usize>) {
    let Some(budget) = budget else {
        return;
    };
    let limit = budget.saturating_mul(CHARS_PER_TOKEN);
    let mut dropped: Vec<(String, usize)> = Vec::new();
    while serde_json::to_string(answer).map_or(0, |s| s.len()) > limit {
        let Some((path, len)) = longest_list(answer, String::new()) else {
            break;
        };
        if len <= 1 {
            break;
        }
        let keep = len / 2;
        if let Some(list) = list_at(answer, &path) {
            list.truncate(keep);
        }
        match dropped.iter_mut().find(|(p, _)| *p == path) {
            Some(d) => d.1 += len - keep,
            None => dropped.push((path, len - keep)),
        }
    }
    if !dropped.is_empty()
        && let Value::Object(map) = answer
    {
        let cut: serde_json::Map<String, Value> = dropped
            .into_iter()
            .map(|(p, n)| (p, Value::from(n)))
            .collect();
        map.insert("truncated".to_string(), Value::Object(cut));
    }
}

/// The path (`a.b.0.c`) and length of the longest list in `v`.
fn longest_list(v: &Value, at: String) -> Option<(String, usize)> {
    let here = match v {
        Value::Array(a) => Some((at.clone(), a.len())),
        _ => None,
    };
    let children: Box<dyn Iterator<Item = (String, &Value)>> = match v {
        Value::Array(a) => Box::new(a.iter().enumerate().map(|(i, c)| (i.to_string(), c))),
        Value::Object(o) => Box::new(o.iter().map(|(k, c)| (k.clone(), c))),
        _ => Box::new(std::iter::empty()),
    };
    children
        .filter_map(|(k, c)| {
            let path = if at.is_empty() {
                k
            } else {
                format!("{at}.{k}")
            };
            longest_list(c, path)
        })
        .chain(here)
        .max_by_key(|(_, n)| *n)
}

fn list_at<'a>(v: &'a mut Value, path: &str) -> Option<&'a mut Vec<Value>> {
    let mut cur = v;
    if !path.is_empty() {
        for k in path.split('.') {
            cur = match cur {
                Value::Array(a) => a.get_mut(k.parse::<usize>().ok()?)?,
                Value::Object(o) => o.get_mut(k)?,
                _ => return None,
            };
        }
    }
    match cur {
        Value::Array(a) => Some(a),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::fit;

    #[test]
    fn a_budget_cuts_the_longest_lists_and_says_so() {
        let mut v = json!({"a": (0..100).collect::<Vec<_>>(), "b": [1, 2], "keep": "x"});
        fit(&mut v, Some(20));
        assert!(
            serde_json::to_string(&v).unwrap().len() <= 20 * 4 + 40,
            "{v}"
        );
        assert!(v["a"].as_array().unwrap().len() < 100);
        assert!(v["truncated"]["a"].as_u64().unwrap() > 0, "{v}");
        assert_eq!(v["keep"], "x");
        let mut small = json!({"a": [1]});
        fit(&mut small, Some(1000));
        assert!(small.get("truncated").is_none());
    }
}
