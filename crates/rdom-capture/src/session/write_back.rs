//! Writing the session's edits back to their CSS files
//! (`specs/CAPTURE.md` §Edits, write-back): `patch` — every diff computed
//! from what is on disk before anything is written, applied all at once
//! (each file replaced whole, through a temporary file) and read back — and
//! `reload`, which reads the App's file-backed sheets again.

use std::path::Path;

use rdom_tui::render::TestBackend;
use rdom_tui::{App, RuleOrigin, SourceFile, StylesheetId};

use super::inspect::{drop_replaced, invalid, source};
use super::{Session, SessionError};
use crate::app::CaptureApp;
use crate::edit::patch::{DeclEdit, apply};
use crate::edit::{EditNote, Edits, FilePatch, Patch, Record, Reloaded, diff};

/// A sheet a reload reads again: its id, its file, and whether it holds
/// the UA rules (parsed back with them).
struct Backed {
    id: StylesheetId,
    path: String,
    ua: bool,
}

/// The App's sheets parsed from one named CSS file and nothing else, and
/// the ones that name a file but hold more (each as `path: why`): a
/// reload would drop what the file does not hold. Each sheet is judged as
/// the app made it — before the session's edits, which a reload drops
/// anyway.
fn file_backed(app: &App<TestBackend>, edits: &Edits) -> (Vec<Backed>, Vec<String>) {
    let (mut backed, mut skipped) = (Vec::new(), Vec::new());
    for id in app.stylesheet_ids() {
        let s = edits
            .snapshots
            .iter()
            .find(|(sid, _)| *sid == id)
            .map(|(_, s)| s)
            .or_else(|| app.stylesheet(id))
            .expect("an App sheet");
        let named: Vec<&String> = s.sources().iter().flatten().collect();
        match (s.sources(), named.as_slice()) {
            (_, []) => {}
            ([Some(path)], _) => {
                let ua = s.rules().iter().any(|r| r.origin == RuleOrigin::UserAgent);
                let foreign = s.rules().iter().any(|r| {
                    r.origin != RuleOrigin::UserAgent
                        && !matches!(r.location.map(|l| l.file), Some(SourceFile::Css(0)))
                });
                if foreign {
                    skipped.push(format!(
                        "{path}: the sheet also holds rules built in Rust, which a reload would drop"
                    ));
                } else {
                    backed.push(Backed {
                        id,
                        path: path.clone(),
                        ua,
                    });
                }
            }
            (_, [first, ..]) => skipped.push(format!(
                "{first}: the sheet is parsed from several texts (imports), not one file"
            )),
        }
    }
    (backed, skipped)
}

/// One file's new text: its diff and the edits it carries.
struct Planned {
    path: String,
    old: String,
    new: String,
    edits: Vec<String>,
}

impl<A: CaptureApp> Session<A> {
    /// The edits as unified diffs against the CSS files their rules are
    /// written in (the last value of each property per rule), and the ones
    /// no file can take, each with why.
    ///
    /// `apply` writes the diffs and reads the files back (as
    /// [`reload`](Self::reload)), so the session shows the files from then
    /// on; it is refused, with nothing written, when an edit of a sheet
    /// read back would be lost — one the file cannot take.
    pub fn patch(&mut self, apply_to_files: bool) -> Result<Patch, SessionError> {
        let o = self.open.as_mut().ok_or(SessionError::NotOpen)?;
        drop_replaced(&o.app, &mut o.edits);
        let mut out = Patch {
            unpatched: std::mem::take(&mut o.edits.notes),
            ..Patch::default()
        };
        let (files, lost) = plan(&o.app, &o.edits.log, &mut out.unpatched);
        if apply_to_files {
            let (backed, _) = file_backed(&o.app, &o.edits);
            let lost: Vec<&str> = lost
                .iter()
                .filter(|(sheet, _)| backed.iter().any(|b| b.id == *sheet))
                .map(|(_, id)| id.as_str())
                .collect();
            if !lost.is_empty() {
                return Err(invalid(format!(
                    "applying reads the files back and would lose {} (see `unpatched` in a dry run): reset or redo them first",
                    lost.join(", ")
                )));
            }
            for f in files.iter().filter(|f| f.new != f.old) {
                replace_file(&f.path, &f.new)?;
            }
        }
        out.files = files
            .into_iter()
            .map(|f| FilePatch {
                diff: diff::unified(&f.path, &f.old, &f.new),
                path: f.path,
                edits: f.edits,
            })
            .collect();
        if apply_to_files {
            self.reload()?;
            out.applied = true;
        }
        Ok(out)
    }

    /// Read every App sheet parsed from one named CSS file again from that
    /// file (a patch applied, or the file edited by hand), replacing the
    /// sheet; the edits of a reloaded sheet are dropped. A sheet that also
    /// holds what its file does not (rules built in Rust, imports) is not
    /// reloaded, and is listed with why.
    pub fn reload(&mut self) -> Result<Reloaded, SessionError> {
        let o = self.open.as_ref().ok_or(SessionError::NotOpen)?;
        let (backed, skipped) = file_backed(&o.app, &o.edits);
        let app = &o.app;
        let mut sheets = Vec::new();
        for b in &backed {
            let text = std::fs::read_to_string(&b.path)?;
            let sheet = if b.ua {
                rdom_css::from_css_at(&text, &b.path)
            } else {
                rdom_css::parse_at(&text, &b.path).stylesheet
            };
            sheets.push((b.id, sheet));
        }
        let before = app.terminal().buffer().clone();
        let mut dropped = Vec::new();
        self.step(|o| {
            let ids: Vec<StylesheetId> = sheets.iter().map(|(i, _)| *i).collect();
            o.app.inspect(|a| {
                for (id, sheet) in sheets {
                    a.edit_stylesheet(id, |s| *s = sheet);
                }
            })?;
            for (id, b) in ids.into_iter().zip(&backed) {
                dropped.extend(o.edits.forget_sheet(id).into_iter().map(|edit| EditNote {
                    edit,
                    reason: format!("its sheet was read again from {}", b.path),
                }));
            }
            Ok(())
        })?;
        Ok(Reloaded {
            files: backed.into_iter().map(|b| b.path).collect(),
            dropped,
            skipped,
            change: self.change_since(&before),
        })
    }
}

/// Every file's new text, from what is on disk now — nothing written —
/// and the edits no file takes, as `(sheet, edit id)` (their reasons go
/// to `unpatched`).
fn plan(
    app: &App<TestBackend>,
    log: &[Record],
    unpatched: &mut Vec<EditNote>,
) -> (Vec<Planned>, Vec<(StylesheetId, String)>) {
    let mut lost: Vec<(StylesheetId, String)> = Vec::new();
    let mut note = |rec: &Record, reason: String| {
        lost.push((rec.sheet, rec.edited.id.clone()));
        unpatched.push(EditNote {
            edit: rec.edited.id.clone(),
            reason,
        });
    };
    // Per file: its edits (the latest per rule and property) and its new
    // rules.
    let mut files: Vec<(String, Vec<&Record>, Vec<&Record>)> = Vec::new();
    fn file<'r>(files: &mut Vec<(String, Vec<&'r Record>, Vec<&'r Record>)>, path: &str) -> usize {
        match files.iter().position(|(p, _, _)| p == path) {
            Some(i) => i,
            None => {
                files.push((path.to_string(), Vec::new(), Vec::new()));
                files.len() - 1
            }
        }
    }
    for (k, rec) in log.iter().enumerate() {
        let superseded = log[k + 1..].iter().any(|later| {
            later.sheet == rec.sheet
                && later.rules == rec.rules
                && later.edited.property == rec.edited.property
        });
        if superseded {
            continue;
        }
        if rec.new_rule {
            let named: Vec<&str> = app
                .stylesheet(rec.sheet)
                .map(|s| s.sources().iter().flatten().map(String::as_str).collect())
                .unwrap_or_default();
            match named.as_slice() {
                [one] => {
                    let i = file(&mut files, one);
                    files[i].2.push(rec);
                }
                _ => note(
                    rec,
                    "a new rule goes to its sheet's file, and the sheet has no one named file"
                        .into(),
                ),
            }
            continue;
        }
        match (rec.location.map(|l| l.file), &rec.source_name) {
            (Some(SourceFile::Css(_)), Some(path)) => {
                let i = file(&mut files, path);
                files[i].1.push(rec);
            }
            (Some(SourceFile::Rust(f)), _) => note(
                rec,
                format!(
                    "the rule is built in Rust at {} — change its declaration there",
                    source(rec.location, None).unwrap_or_else(|| f.to_string())
                ),
            ),
            (Some(_), None) => note(
                rec,
                "the rule's CSS text has no file name (parse it with rdom_css::from_css_at or css_file!)"
                    .into(),
            ),
            _ => {
                let ua = app
                    .stylesheet(rec.sheet)
                    .and_then(|s| s.rules().get(rec.rules[0]))
                    .is_some_and(|r| r.origin == RuleOrigin::UserAgent);
                note(
                    rec,
                    if ua {
                        "a user-agent rule has no file".into()
                    } else {
                        "the rule records no location".into()
                    },
                );
            }
        }
    }
    let mut planned = Vec::new();
    for (path, mut recs, appended) in files {
        let old = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) => {
                for rec in recs.iter().chain(&appended) {
                    note(rec, format!("cannot read {path}: {e}"));
                }
                continue;
            }
        };
        // Last in the text first, so an earlier rule's position holds.
        recs.sort_by_key(|r| std::cmp::Reverse(r.location.map(|l| (l.line, l.column))));
        let mut new = old.clone();
        let mut ids = Vec::new();
        for rec in recs {
            let l = rec.location.expect("located");
            let edit = DeclEdit {
                property: rec.edited.property.clone(),
                value: rec.edited.value.clone(),
                important: rec.edited.important,
            };
            match apply(&new, l.line, l.column, &rec.edited.selector, &edit) {
                Ok(text) => {
                    new = text;
                    ids.push(rec.edited.id.clone());
                }
                Err(reason) => note(rec, format!("{path}: {reason}")),
            }
        }
        for rec in appended {
            if !new.ends_with('\n') && !new.is_empty() {
                new.push('\n');
            }
            new.push_str(&format!(
                "\n{} {{\n  {}: {}{};\n}}\n",
                rec.edited.selector,
                rec.edited.property,
                rec.edited.value.as_deref().unwrap_or_default(),
                if rec.edited.important {
                    " !important"
                } else {
                    ""
                }
            ));
            ids.push(rec.edited.id.clone());
        }
        ids.sort();
        planned.push(Planned {
            path,
            old,
            new,
            edits: ids,
        });
    }
    (planned, lost)
}

/// Replace `path` with `text` whole: written beside it, then renamed over
/// it, so a failure leaves the old file, never half of the new one.
fn replace_file(path: &str, text: &str) -> std::io::Result<()> {
    let target = Path::new(path);
    let mut tmp = target.as_os_str().to_owned();
    tmp.push(".rdom-capture-tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, target).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}
