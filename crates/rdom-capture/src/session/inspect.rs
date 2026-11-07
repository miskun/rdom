//! The session's inspector steps (`specs/CAPTURE.md` §Edits): `edit`,
//! `force`, `reset` (`patch` and `reload` are in `write_back`). Each change
//! is applied as an inspector's (`App::inspect`) and drawn; the log keeps
//! what `reset` undoes and `patch` writes back.

use rdom_tui::capture::DeclarationOrigin;
use rdom_tui::core_api::{ForcedState, NodeId};
use rdom_tui::render::TestBackend;
use rdom_tui::{App, RuleContext, SourceFile, StyleSelector, Stylesheet, StylesheetId, TuiStyle};

use super::{Change, Session, SessionError, Step};
use crate::app::CaptureApp;
use crate::capture::node_key;
use crate::edit::{EditNote, EditTarget, Edited, Edits, Record, Reset, unwritable};

pub(super) fn invalid(msg: impl Into<String>) -> SessionError {
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

/// Element `id`'s identity (`capture::identity`): how the log names it.
pub(crate) fn identity_of(app: &App<TestBackend>, id: NodeId) -> String {
    crate::capture::identity::identities(app.dom())
        .remove(&id)
        .map_or_else(|| node_key(id), |i| i.key)
}

/// The node key of the element whose identity is `key`.
pub(crate) fn by_identity(app: &App<TestBackend>, key: &str) -> Result<String, SessionError> {
    crate::capture::identity::identities(app.dom())
        .into_iter()
        .find(|(_, i)| i.key == key)
        .map(|(id, _)| node_key(id))
        .ok_or_else(|| invalid(format!("no element {key:?} in the document")))
}

/// `location` as `file:line:column`.
pub(super) fn source(
    location: Option<rdom_tui::SourceLocation>,
    name: Option<&str>,
) -> Option<String> {
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
            let s = app.stylesheet(sheet).expect("an App sheet");
            Ok(Resolved {
                sheet,
                rules: block_of(s, rule.index),
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
                rules: block_of(s, *index),
                selector: rule.source_text.clone(),
                location: rule.location,
                source_name,
                new_rule: false,
            })
        }
        EditTarget::NewRule { selector } => {
            StyleSelector::parse(selector)
                .map_err(|e| invalid(format!("invalid selector {selector:?}: {e}")))?;
            let sheet = *app
                .stylesheet_ids()
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

/// The rules written as one block with rule `index` — the items of a
/// selector list (`a, b { … }`) are one rule each, at one place in the
/// CSS text — so an edit changes them all, as its write-back will. A rule
/// with no CSS location is its own block.
fn block_of(sheet: &Stylesheet, index: usize) -> Vec<usize> {
    match sheet.rules().get(index).and_then(|r| r.location) {
        Some(at) if matches!(at.file, SourceFile::Css(_)) => sheet
            .rules()
            .iter()
            .enumerate()
            .filter(|(_, r)| r.location == Some(at))
            .map(|(i, _)| i)
            .collect(),
        _ => vec![index],
    }
}

fn previous_of(sheet: &Stylesheet, rule: usize, property: &str) -> Option<String> {
    sheet
        .rule_declaration(rule, property)
        .map(|(v, imp)| if imp { format!("{v} !important") } else { v })
}

/// Drop the edits of every sheet the app replaced or removed since its
/// last edit (`Edits::replaced`): they no longer show, so `reset` has
/// nothing to undo there and `patch` nothing to write. Each is kept as a
/// note for the next `reset` or `patch` to report.
pub(super) fn drop_replaced(app: &App<TestBackend>, edits: &mut Edits) {
    let replaced: Vec<StylesheetId> = edits
        .versions
        .iter()
        .map(|(id, _)| *id)
        .filter(|id| edits.replaced(*id, app.stylesheet(*id).map(Stylesheet::version)))
        .collect();
    for id in replaced {
        for edit in edits.forget_sheet(id) {
            edits.notes.push(EditNote {
                edit,
                reason: "the app replaced or removed its sheet since: the edit no longer shows"
                    .into(),
            });
        }
    }
}

impl<A: CaptureApp> Session<A> {
    /// Apply `f` to the open app as an inspector's change (`App::inspect`)
    /// and draw its frame, as one step.
    pub(crate) fn step_inspected(
        &mut self,
        f: impl FnOnce(&mut App<TestBackend>) -> Result<(), SessionError>,
    ) -> Result<(), SessionError> {
        let mut out = Ok(());
        self.step(|o| {
            out = o.app.inspect(f)?;
            Ok(())
        })?;
        out
    }

    /// Change `property` of the rule `target` names to `value`
    /// (`!important` when `important`; `None` removes the declaration), as
    /// an inspector's change, and draw the frame it causes. Every rule
    /// written as one block with it changes too (a selector list's
    /// items). Text that would not stay one declaration (or one selector)
    /// in its CSS file is refused.
    pub fn edit(
        &mut self,
        target: &EditTarget,
        property: &str,
        value: Option<&str>,
        important: bool,
    ) -> Result<(Edited, Change), SessionError> {
        let checks = [
            Some((property, "the property")),
            value.map(|v| (v, "the value")),
            match target {
                EditTarget::NewRule { selector } => Some((selector.as_str(), "the selector")),
                _ => None,
            },
        ];
        for (text, what) in checks.into_iter().flatten() {
            if let Some(why) = unwritable(text, what) {
                return Err(invalid(why));
            }
        }
        {
            let o = self.open.as_mut().ok_or(SessionError::NotOpen)?;
            drop_replaced(&o.app, &mut o.edits);
        }
        let r = resolve(self.app()?, target, property)?;
        let app = self.app()?;
        match (r.new_rule, value) {
            (true, None) => return Err(invalid("a new rule needs a value")),
            // Tried on a scratch sheet first, so a refused value leaves no
            // empty rule behind.
            (true, Some(v)) => {
                let mut scratch = Stylesheet::bare();
                let sel = StyleSelector::parse(&r.selector).expect("checked by resolve");
                scratch.add_style_rule(&sel, TuiStyle::new(), RuleContext::default());
                scratch
                    .set_rule_declaration(0, property, v, important)
                    .map_err(|e| invalid(format!("{property}: {e}")))?;
            }
            (false, None) => {
                let s = app.stylesheet(r.sheet).expect("an App sheet");
                if r.rules
                    .iter()
                    .all(|&i| s.rule_declaration(i, property).is_none())
                {
                    return Err(invalid(format!("the rule declares no {property}")));
                }
            }
            (false, Some(_)) => {}
        }
        let before = app.terminal().buffer().clone();
        let mut outcome: Result<(Vec<usize>, Option<String>), SessionError> =
            Ok((Vec::new(), None));
        self.step(|o| {
            let snapshot = o.app.stylesheet(r.sheet).expect("an App sheet").clone();
            o.edits.snapshot(r.sheet, &snapshot);
            outcome = o.app.inspect(|app| {
                // On a copy, swapped in only when every rule took the edit:
                // a block's rules change together or not at all.
                app.edit_stylesheet(r.sheet, |s| {
                    let mut copy = s.clone();
                    let out = edit_rules(&mut copy, &r, property, value, important)?;
                    *s = copy;
                    Ok(out)
                })
                .expect("an App sheet")
            })?;
            let version = o.app.stylesheet(r.sheet).map(Stylesheet::version);
            o.edits.left_at(r.sheet, version.expect("an App sheet"));
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
        // The log holds the rule the edit changed, not how it was found:
        // a replay elsewhere (another environment) edits the same rule.
        let logged = if r.new_rule {
            target.clone()
        } else {
            let app = self.app()?;
            EditTarget::Rule {
                sheet: app.stylesheet_index(r.sheet).expect("an App sheet"),
                index: r.rules[0],
            }
        };
        self.record(Step::Edit {
            target: logged,
            property: property.to_string(),
            value: value.map(str::to_string),
            important,
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
        self.record(Step::Force {
            node: identity_of(self.app()?, id),
            state: state.name().to_string(),
            on,
        });
        Ok(self.change_since(&before))
    }

    /// Undo every edit and forced state: each edited sheet back as it was.
    /// An edit to a sheet the app has since replaced is not undone — the
    /// app's sheet stays — and is listed with why.
    pub fn reset(&mut self) -> Result<Reset, SessionError> {
        let before = self.app()?.terminal().buffer().clone();
        let mut dropped = Vec::new();
        self.step(|o| {
            drop_replaced(&o.app, &mut o.edits);
            dropped = std::mem::take(&mut o.edits.notes);
            let snapshots = std::mem::take(&mut o.edits.snapshots);
            o.app.inspect(|a| {
                for (id, sheet) in snapshots {
                    a.edit_stylesheet(id, |s| *s = sheet);
                }
                a.clear_forced_states();
            })?;
            o.edits.log.clear();
            o.edits.versions.clear();
            o.edits.forced.clear();
            Ok(())
        })?;
        self.record(Step::Reset);
        Ok(Reset {
            change: self.change_since(&before),
            dropped,
        })
    }
}

/// Apply one edit to the rules `r` names in sheet `s` (adding the new rule
/// first): the rules changed and the first one's declaration before.
fn edit_rules(
    s: &mut Stylesheet,
    r: &Resolved,
    property: &str,
    value: Option<&str>,
    important: bool,
) -> Result<(Vec<usize>, Option<String>), SessionError> {
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
}
