//! Edits (`specs/CAPTURE.md` §Edits): a declaration changed in place in
//! the rule that holds it — the winner an `explain` names, a rule by its
//! place, or a new rule — and elements forced into a user-action state,
//! each applied as an inspector's change (`App::inspect`: app code does
//! not see it), logged so `reset` undoes them exactly and `patch` writes
//! them back to the CSS files they came from.
//!
//! - [`EditTarget`] — which rule an edit changes.
//! - [`Edited`] — what an edit did.
//! - [`Patch`] — the edits as unified diffs against their files, and the
//!   ones that have no file to go to (a UA or Rust-built rule, an unnamed
//!   text), each with why.

pub(crate) mod diff;
pub(crate) mod patch;

use rdom_tui::core_api::{ForcedState, NodeId};
use rdom_tui::{SourceLocation, Stylesheet, StylesheetId};
use serde::{Deserialize, Serialize};

/// Which rule an edit changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum EditTarget {
    /// The rule whose declaration of the edited property an element's
    /// style took (its winner): what `explain` shows.
    Winner { node: String },
    /// A rule by its place: `sheet` as a style trace names it (the sheets
    /// the cascade reads, in order) and its `index` in the sheet.
    Rule { sheet: usize, index: usize },
    /// A new rule for `selector`, at the end of the App's last sheet.
    NewRule { selector: String },
}

/// What one edit did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Edited {
    /// The edit's id (`e1`, `e2`, …).
    pub id: String,
    /// The rule's selector.
    pub selector: String,
    /// Where the rule is written (`file:line:column`), when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub property: String,
    /// The new value; `None`: the declaration was removed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub important: bool,
    /// The rule's declaration before the edit, if it had one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous: Option<String>,
}

/// The edits as patches.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Patch {
    /// One unified diff per file the edits change.
    pub files: Vec<FilePatch>,
    /// The edits no file can take, each with why.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unpatched: Vec<Unpatched>,
    /// Whether the diffs were written to the files.
    pub applied: bool,
}

/// The diff of one file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FilePatch {
    pub path: String,
    pub diff: String,
    /// The edits it carries.
    pub edits: Vec<String>,
}

/// An edit with no file to go to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Unpatched {
    pub edit: String,
    pub reason: String,
}

/// One logged edit.
#[derive(Debug, Clone)]
pub(crate) struct Record {
    pub edited: Edited,
    pub sheet: StylesheetId,
    /// The rule's indices in the sheet (a new rule for a selector list is
    /// one rule per item).
    pub rules: Vec<usize>,
    pub location: Option<SourceLocation>,
    pub source_name: Option<String>,
    /// The edit added the rule.
    pub new_rule: bool,
}

/// A session's edits, forced states, and each edited sheet as it was.
#[derive(Debug, Default)]
pub(crate) struct Edits {
    next: u32,
    pub log: Vec<Record>,
    pub snapshots: Vec<(StylesheetId, Stylesheet)>,
    pub forced: Vec<(NodeId, ForcedState)>,
}

impl Edits {
    /// The next edit's id.
    pub fn next_id(&mut self) -> String {
        self.next += 1;
        format!("e{}", self.next)
    }

    /// Keep `sheet` as it was before its first edit.
    pub fn snapshot(&mut self, id: StylesheetId, sheet: &Stylesheet) {
        if !self.snapshots.iter().any(|(s, _)| *s == id) {
            self.snapshots.push((id, sheet.clone()));
        }
    }
}
