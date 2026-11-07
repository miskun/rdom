//! The session's moments for diffs: [`Session::snapshot`] takes one —
//! the grid and every element by identity, with its border box and
//! declared styles — and [`Session::mark`] keeps one by name for
//! [`Session::diff`].

use super::{CaptureRequest, Session, SessionError};
use crate::app::CaptureApp;
use crate::capture::identity::identities;
use crate::capture::{Layer, node_key, nodes};
use crate::variants::diff::{Diff, Element, Snapshot, diff};

/// The most marks a session keeps; marking more drops the oldest.
pub const MAX_MARKS: usize = 32;

impl<A: CaptureApp> Session<A> {
    /// The current moment, for [`crate::variants::diff::diff`].
    pub fn snapshot(&self) -> Result<Snapshot, SessionError> {
        let capture = self.capture(&CaptureRequest::with_layers([Layer::Grid]))?;
        let o = self.open.as_ref().ok_or(SessionError::NotOpen)?;
        let dom = o.app.dom();
        let ids = identities(dom);
        let boxes: std::collections::HashMap<_, _> = nodes::boxes(dom).into_iter().collect();
        let elements = nodes::elements(dom)
            .into_iter()
            .filter_map(|id| {
                let identity = ids.get(&id)?;
                let mut styles: Vec<(String, String)> = o
                    .app
                    .explain_style(id)
                    .map(|t| {
                        t.properties()
                            .into_iter()
                            .filter_map(|p| Some((p.to_string(), t.winner(p)?.value.clone())))
                            .collect()
                    })
                    .unwrap_or_default();
                styles.sort();
                Some(Element {
                    key: identity.key.clone(),
                    by: identity.by,
                    node: node_key(id),
                    path: nodes::node_info(dom, id).path,
                    border: boxes.get(&id).map(|b| b.border),
                    styles,
                })
            })
            .collect();
        Ok(Snapshot {
            time_ms: o.time_ms,
            grid_hash: capture.meta.grid_hash,
            grid: capture.grid.expect("asked for"),
            elements,
        })
    }

    /// Keep the current moment as `name` (replacing one of that name).
    pub fn mark(&mut self, name: &str) -> Result<(), SessionError> {
        let snap = self.snapshot()?;
        self.marks.retain(|(n, _)| n != name);
        if self.marks.len() == MAX_MARKS {
            self.marks.remove(0);
        }
        self.marks.push((name.to_string(), snap));
        Ok(())
    }

    /// How the moment marked `from` differs from the one marked `to`, or
    /// from the current one.
    pub fn diff(&self, from: &str, to: Option<&str>) -> Result<Diff, SessionError> {
        let mark = |name: &str| {
            self.marks
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, s)| s)
                .ok_or_else(|| {
                    let known: Vec<&str> = self.marks.iter().map(|(n, _)| n.as_str()).collect();
                    SessionError::InvalidInput(format!(
                        "no mark {name:?}; marked: {}",
                        if known.is_empty() {
                            "none".to_string()
                        } else {
                            known.join(", ")
                        }
                    ))
                })
        };
        let a = mark(from)?;
        let now;
        let b = match to {
            Some(name) => mark(name)?,
            None => {
                now = self.snapshot()?;
                &now
            }
        };
        Ok(diff(a, b))
    }
}
