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
//!   text, a removal the text cannot express), each with why.
//! - [`EditNote`] — an edit something happened to, and why: not patched,
//!   dropped by a reset or a reload.
//! - [`Reset`] / [`Reloaded`] — what a reset and a reload did.

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
    pub unpatched: Vec<EditNote>,
    /// Whether the diffs were written to the files — and the files read
    /// back into the session (a reload), so what it shows is the files.
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

/// An edit something happened to — not patched, dropped — and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EditNote {
    pub edit: String,
    pub reason: String,
}

/// What a reset did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Reset {
    pub change: crate::session::Change,
    /// The edits that were not undone because they no longer showed: the
    /// app replaced their sheet since (its sheet stays as the app set it).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dropped: Vec<EditNote>,
}

/// What a reload did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Reloaded {
    /// The files read back.
    pub files: Vec<String>,
    /// The edits of the reloaded sheets, now dropped.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dropped: Vec<EditNote>,
    /// The file-backed sheets not reloaded, each as `path: why`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skipped: Vec<String>,
    pub change: crate::session::Change,
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

/// A session's edits, forced states, each edited sheet as it was, and
/// the version each was left at by the last edit — a different version
/// later means the app replaced the sheet (`StylesheetIntent::Set`), and
/// the edits to it no longer show.
#[derive(Debug, Default)]
pub(crate) struct Edits {
    next: u32,
    pub log: Vec<Record>,
    pub snapshots: Vec<(StylesheetId, Stylesheet)>,
    pub versions: Vec<(StylesheetId, u64)>,
    pub forced: Vec<(NodeId, ForcedState)>,
    /// Edits dropped since the last `reset` or `patch`, which reports them.
    pub notes: Vec<EditNote>,
}

impl Edits {
    /// The next edit's id.
    pub fn next_id(&mut self) -> String {
        self.next += 1;
        format!("e{}", self.next)
    }

    /// Note the version an edit left sheet `id` at.
    pub fn left_at(&mut self, id: StylesheetId, version: u64) {
        self.versions.retain(|(s, _)| *s != id);
        self.versions.push((id, version));
    }

    /// Whether sheet `id` is no longer as the last edit left it.
    pub fn replaced(&self, id: StylesheetId, now: Option<u64>) -> bool {
        self.versions
            .iter()
            .find(|(s, _)| *s == id)
            .is_some_and(|(_, v)| Some(*v) != now)
    }

    /// Forget sheet `id` (reloaded): its snapshot, version and edits; the
    /// ids of the edits dropped.
    pub fn forget_sheet(&mut self, id: StylesheetId) -> Vec<String> {
        self.snapshots.retain(|(s, _)| *s != id);
        self.versions.retain(|(s, _)| *s != id);
        let mut dropped = Vec::new();
        self.log.retain(|r| {
            let keep = r.sheet != id;
            if !keep {
                dropped.push(r.edited.id.clone());
            }
            keep
        });
        dropped
    }

    /// Keep `sheet` as it was before its first edit.
    pub fn snapshot(&mut self, id: StylesheetId, sheet: &Stylesheet) {
        if !self.snapshots.iter().any(|(s, _)| *s == id) {
            self.snapshots.push((id, sheet.clone()));
        }
    }
}

/// Why `text` cannot be written into a CSS file as a property name, a
/// value or a selector: the session writes edits back to their source
/// (`patch`), so an edit is refused when its text would end its
/// declaration or block there and change other rules: a `;`, `{`, `}` or
/// `!` outside quotes (even inside brackets — the write-back splits
/// declarations by `;`), a comment, a bracket or quote not closed.
/// (`!important` is the edit's own flag.)
pub(crate) fn unwritable(text: &str, what: &str) -> Option<String> {
    let mut stack: Vec<char> = Vec::new();
    let mut quote: Option<char> = None;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match (quote, c) {
            (Some(_), '\\') => {
                chars.next();
            }
            (Some(q), _) if c == q => quote = None,
            (Some(_), '\n') => return Some(format!("{what} has a line break inside quotes")),
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '/') if chars.peek() == Some(&'*') => {
                return Some(format!("{what} holds a comment"));
            }
            (None, '(' | '[') => stack.push(c),
            (None, ')') if stack.pop() != Some('(') => {
                return Some(format!("{what} closes a `(` it did not open"));
            }
            (None, ']') if stack.pop() != Some('[') => {
                return Some(format!("{what} closes a `[` it did not open"));
            }
            (None, ';' | '{' | '}' | '!') => {
                return Some(format!(
                    "{what} holds `{c}`, which would end the declaration or the rule in the file"
                ));
            }
            _ => {}
        }
    }
    if quote.is_some() {
        return Some(format!("{what} leaves a quote open"));
    }
    if !stack.is_empty() {
        return Some(format!("{what} leaves a bracket open"));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::unwritable;

    #[test]
    fn text_that_would_end_its_declaration_in_the_file_is_refused() {
        for ok in [
            "red",
            "#123456",
            "var(--a, 1px)",
            "\"a;b\"",
            "calc(1px + 2px)",
            "url(x.png)",
        ] {
            assert_eq!(unwritable(ok, "value"), None, "{ok}");
        }
        for bad in [
            "1; } * { color: red",
            "red !important",
            "a}",
            "f(1",
            "\"open",
            "1 /* x */",
            "x)",
        ] {
            assert!(unwritable(bad, "value").is_some(), "{bad}");
        }
    }
}
