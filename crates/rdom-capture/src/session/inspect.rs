//! The session's inspector steps (`specs/CAPTURE.md` §Edits): `edit`,
//! `force`, `reset`, `patch`, `reload`. Each change is applied as an
//! inspector's (`App::inspect`) and drawn; the log keeps what `reset`
//! undoes and `patch` writes back.

use rdom_tui::capture::DeclarationOrigin;
use rdom_tui::core_api::{ForcedState, NodeId};
use rdom_tui::render::TestBackend;
use rdom_tui::{
    App, RuleContext, RuleOrigin, SourceFile, StyleSelector, Stylesheet, StylesheetId, TuiStyle,
};

use super::{Change, Session, SessionError};
use crate::app::CaptureApp;
use crate::capture::node_key;
use crate::edit::patch::{DeclEdit, apply};
use crate::edit::{EditTarget, Edited, FilePatch, Patch, Record, Unpatched, diff};

fn invalid(msg: impl Into<String>) -> SessionError {
    SessionError::InvalidInput(msg.into())
}

/// The node named `key` in `app`'s document.
pub(crate) fn node_by_key(app: &App<TestBackend>, key: &str) -> Result<NodeId, SessionError> {
    let dom = app.dom();
    std::iter::once(dom.root())
        .chain(dom.descendants(dom.root()))
        .find(|&n| node_key(n) == key)
        .ok_or_else(|| invalid(format!("no node {key:?} in the document")))
}

/// The App's sheets, in cascade order.
fn app_sheets(app: &App<TestBackend>) -> Vec<StylesheetId> {
    let mut out = Vec::new();
    let mut i = 0;
    let mut seen = false;
    loop {
        match app.stylesheet_at(i) {
            Some(id) => {
                out.push(id);
                seen = true;
            }
            None if seen => break,
            None if i > 4096 => break,
            None => {}
        }
        i += 1;
    }
    out
}

/// `location` as `file:line:column`.
fn source(location: Option<rdom_tui::SourceLocation>, name: Option<&str>) -> Option<String> {
    let l = location?;
    Some(match l.file {
        SourceFile::Rust(f) => format!("rust:{f}:{}:{}", l.line, l.column),
        _ => format!("{}:{}:{}", name.unwrap_or("<css>"), l.line, l.column),
    })
}

/// Where an edit goes: the sheet, the rules, and what to record of them.
struct Resolved {
    sheet: StylesheetId,
    rules: Vec<usize>,
    selector: String,
    location: Option<rdom_tui::SourceLocation>,
    source_name: Option<String>,
    new_rule: bool,
}

fn resolve(
    app: &App<TestBackend>,
    target: &EditTarget,
    property: &str,
) -> Result<Resolved, SessionError> {
    match target {
        EditTarget::Winner { node } => {
            let id = node_by_key(app, node)?;
            let trace = app
                .explain_style(id)
                .ok_or_else(|| invalid(format!("node {node} is not a styled element")))?;
            let w = trace.winner(property).ok_or_else(|| {
                invalid(format!(
                    "no declaration of {property} applies to node {node}: edit a rule or add one"
                ))
            })?;
            let rule = match (&w.rule, w.origin) {
                (Some(r), _) => r.clone(),
                (None, DeclarationOrigin::Inline) => {
                    return Err(invalid(
                        "the winner is the element's style attribute: edit the element",
                    ));
                }
                (None, _) => {
                    return Err(invalid(
                        "the winner is a presentational hint: edit the element's attribute",
                    ));
                }
            };
            let sheet = app.stylesheet_at(rule.sheet).ok_or_else(|| {
                invalid("the winning rule is in a <style> element's sheet, edited through its text")
            })?;
            Ok(Resolved {
                sheet,
                rules: vec![rule.index],
                selector: rule.selector,
                location: rule.location,
                source_name: rule.source_name,
                new_rule: false,
            })
        }
        EditTarget::Rule { sheet, index } => {
            let id = app
                .stylesheet_at(*sheet)
                .ok_or_else(|| invalid(format!("sheet {sheet} is not one of the App's sheets")))?;
            let s = app.stylesheet(id).expect("an App sheet");
            let rule = s
                .rules()
                .get(*index)
                .ok_or_else(|| invalid(format!("sheet {sheet} has no rule {index}")))?;
            let source_name = match rule.location.map(|l| l.file) {
                Some(SourceFile::Css(i)) => s.source_name(i).map(str::to_string),
                _ => None,
            };
            Ok(Resolved {
                sheet: id,
                rules: vec![*index],
                selector: rule.source_text.clone(),
                location: rule.location,
                source_name,
                new_rule: false,
            })
        }
        EditTarget::NewRule { selector } => {
            StyleSelector::parse(selector)
                .map_err(|e| invalid(format!("invalid selector {selector:?}: {e}")))?;
            let sheet = *app_sheets(app)
                .last()
                .ok_or_else(|| invalid("the App has no sheet to add a rule to"))?;
            Ok(Resolved {
                sheet,
                rules: Vec::new(),
                selector: selector.clone(),
                location: None,
                source_name: None,
                new_rule: true,
            })
        }
    }
}

fn previous_of(sheet: &Stylesheet, rule: usize, property: &str) -> Option<String> {
    sheet
        .rule_declaration(rule, property)
        .map(|(v, imp)| if imp { format!("{v} !important") } else { v })
}

impl<A: CaptureApp> Session<A> {
    /// Change `property` of the rule `target` names to `value`
    /// (`!important` when `important`; `None` removes the declaration), as
    /// an inspector's change, and draw the frame it causes.
    pub fn edit(
        &mut self,
        target: &EditTarget,
        property: &str,
        value: Option<&str>,
        important: bool,
    ) -> Result<(Edited, Change), SessionError> {
        let r = resolve(self.app()?, target, property)?;
        if r.new_rule && value.is_none() {
            return Err(invalid("a new rule needs a value"));
        }
        let before = self.app()?.terminal().buffer().clone();
        let mut outcome: Result<(Vec<usize>, Option<String>), SessionError> =
            Ok((Vec::new(), None));
        self.step(|o| {
            let snapshot = o.app.stylesheet(r.sheet).expect("an App sheet").clone();
            o.edits.snapshot(r.sheet, &snapshot);
            let result = o.app.inspect(|app| {
                app.edit_stylesheet(
                    r.sheet,
                    |s| -> Result<(Vec<usize>, Option<String>), SessionError> {
                        let rules = if r.new_rule {
                            let from = s.rules().len();
                            let sel = StyleSelector::parse(&r.selector).expect("checked");
                            s.add_style_rule(&sel, TuiStyle::new(), RuleContext::default());
                            (from..s.rules().len()).collect()
                        } else {
                            r.rules.clone()
                        };
                        let previous = rules.first().and_then(|&i| previous_of(s, i, property));
                        for &i in &rules {
                            match value {
                                Some(v) => s.set_rule_declaration(i, property, v, important),
                                None => s.remove_rule_declaration(i, property).map(|_| ()),
                            }
                            .map_err(|e| invalid(format!("{property}: {e}")))?;
                        }
                        Ok((rules, previous))
                    },
                )
                .expect("an App sheet")
            })?;
            outcome = result;
            Ok(())
        })?;
        let (rules, previous) = outcome?;
        let o = self.open.as_mut().expect("open");
        let edited = Edited {
            id: o.edits.next_id(),
            selector: r.selector.clone(),
            source: source(r.location, r.source_name.as_deref()),
            property: property.to_string(),
            value: value.map(str::to_string),
            important,
            previous,
        };
        o.edits.log.push(Record {
            edited: edited.clone(),
            sheet: r.sheet,
            rules,
            location: r.location,
            source_name: r.source_name,
            new_rule: r.new_rule,
        });
        Ok((edited, self.change_since(&before)))
    }

    /// Force the element named `node` into `state` (`on`) or stop forcing
    /// it, as an inspector's change.
    pub fn force(
        &mut self,
        node: &str,
        state: ForcedState,
        on: bool,
    ) -> Result<Change, SessionError> {
        let id = node_by_key(self.app()?, node)?;
        let before = self.app()?.terminal().buffer().clone();
        self.step(|o| {
            o.app.inspect(|a| a.force_state(id, state, on))?;
            o.edits.forced.retain(|(n, s)| !(*n == id && *s == state));
            if on {
                o.edits.forced.push((id, state));
            }
            Ok(())
        })?;
        Ok(self.change_since(&before))
    }

    /// Undo every edit and forced state: each edited sheet back as it was.
    pub fn reset(&mut self) -> Result<Change, SessionError> {
        let before = self.app()?.terminal().buffer().clone();
        self.step(|o| {
            let snapshots = std::mem::take(&mut o.edits.snapshots);
            o.app.inspect(|a| {
                for (id, sheet) in snapshots {
                    a.edit_stylesheet(id, |s| *s = sheet);
                }
                a.clear_forced_states();
            })?;
            o.edits.log.clear();
            o.edits.forced.clear();
            Ok(())
        })?;
        Ok(self.change_since(&before))
    }

    /// The edits as unified diffs against the CSS files their rules are
    /// written in (the last value of each property per rule), and the ones
    /// with no file; `apply` writes the diffs to the files.
    pub fn patch(&mut self, apply_to_files: bool) -> Result<Patch, SessionError> {
        let o = self.open.as_ref().ok_or(SessionError::NotOpen)?;
        let mut out = Patch {
            applied: apply_to_files,
            ..Patch::default()
        };
        // Per file: its edits, latest per (rule, property).
        let mut files: Vec<(String, Vec<&Record>)> = Vec::new();
        let mut appended: Vec<(String, &Record)> = Vec::new();
        for (k, rec) in o.edits.log.iter().enumerate() {
            let superseded = o.edits.log[k + 1..].iter().any(|later| {
                later.sheet == rec.sheet
                    && later.rules == rec.rules
                    && later.edited.property == rec.edited.property
            });
            if superseded {
                continue;
            }
            let unpatched = |reason: String| Unpatched {
                edit: rec.edited.id.clone(),
                reason,
            };
            if rec.new_rule {
                let sheet = o.app.stylesheet(rec.sheet);
                let named: Vec<&str> = sheet
                    .map(|s| s.sources().iter().flatten().map(String::as_str).collect())
                    .unwrap_or_default();
                match named.as_slice() {
                    [one] => appended.push(((*one).to_string(), rec)),
                    _ => out.unpatched.push(unpatched(
                        "a new rule goes to its sheet's file, and the sheet has no one named file"
                            .into(),
                    )),
                }
                continue;
            }
            match (rec.location.map(|l| l.file), &rec.source_name) {
                (Some(SourceFile::Css(_)), Some(path)) => match files.iter_mut().find(|(p, _)| p == path) {
                    Some((_, list)) => list.push(rec),
                    None => files.push((path.clone(), vec![rec])),
                },
                (Some(SourceFile::Rust(f)), _) => out.unpatched.push(unpatched(format!(
                    "the rule is built in Rust at {} — change its declaration there",
                    source(rec.location, None).unwrap_or_else(|| f.to_string())
                ))),
                (Some(_), None) => out.unpatched.push(unpatched(
                    "the rule's CSS text has no file name (parse it with rdom_css::from_css_at or css_file!)".into(),
                )),
                _ => out.unpatched.push(unpatched(match o.app.stylesheet(rec.sheet).and_then(|s| s.rules().get(rec.rules[0])) {
                    Some(r) if r.origin == RuleOrigin::UserAgent => "a user-agent rule has no file".to_string(),
                    _ => "the rule records no location".to_string(),
                })),
            }
        }
        for (path, _) in &appended {
            if !files.iter().any(|(p, _)| p == path) {
                files.push((path.clone(), Vec::new()));
            }
        }
        for (path, mut recs) in files {
            let old = std::fs::read_to_string(&path)?;
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
                match apply(&new, l.line, l.column, &edit) {
                    Ok(text) => {
                        new = text;
                        ids.push(rec.edited.id.clone());
                    }
                    Err(reason) => out.unpatched.push(Unpatched {
                        edit: rec.edited.id.clone(),
                        reason: format!("{path}: {reason} (the file changed since it was parsed?)"),
                    }),
                }
            }
            for (_, rec) in appended.iter().filter(|(p, _)| *p == path) {
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
            let d = diff::unified(&path, &old, &new);
            if apply_to_files && !d.is_empty() {
                std::fs::write(&path, &new)?;
            }
            out.files.push(FilePatch {
                path,
                diff: d,
                edits: ids,
            });
        }
        Ok(out)
    }

    /// Read every App sheet parsed from one named CSS file again from that
    /// file (a patch applied, or the file edited by hand), replacing the
    /// sheet; the edits of a reloaded sheet are dropped. Returns the files
    /// read, the edits dropped, and the change.
    pub fn reload(&mut self) -> Result<(Vec<String>, Vec<String>, Change), SessionError> {
        let app = self.app()?;
        let mut plan: Vec<(StylesheetId, String, bool)> = Vec::new();
        for id in app_sheets(app) {
            let s = app.stylesheet(id).expect("an App sheet");
            if let [Some(path)] = s.sources() {
                let ua = s.rules().iter().any(|r| r.origin == RuleOrigin::UserAgent);
                plan.push((id, path.clone(), ua));
            }
        }
        let mut sheets = Vec::new();
        for (id, path, ua) in &plan {
            let text = std::fs::read_to_string(path)?;
            let sheet = if *ua {
                rdom_css::from_css_at(&text, path)
            } else {
                rdom_css::parse_at(&text, path).stylesheet
            };
            sheets.push((*id, sheet));
        }
        let before = self.app()?.terminal().buffer().clone();
        let mut dropped = Vec::new();
        self.step(|o| {
            let ids: Vec<StylesheetId> = sheets.iter().map(|(i, _)| *i).collect();
            o.app.inspect(|a| {
                for (id, sheet) in sheets {
                    a.edit_stylesheet(id, |s| *s = sheet);
                }
            })?;
            o.edits.snapshots.retain(|(i, _)| !ids.contains(i));
            o.edits.log.retain(|r| {
                let keep = !ids.contains(&r.sheet);
                if !keep {
                    dropped.push(r.edited.id.clone());
                }
                keep
            });
            Ok(())
        })?;
        let paths = plan.into_iter().map(|(_, p, _)| p).collect();
        Ok((paths, dropped, self.change_since(&before)))
    }
}
