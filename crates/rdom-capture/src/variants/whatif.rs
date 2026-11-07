//! `whatif` (`specs/CAPTURE.md` §Variants): candidate changes, each tried
//! from the same base in a fork of the session — the session itself is
//! left as it was — and answered with what it changed on screen (a
//! [`Diff`] against the base, both settled the same time) and the patch
//! its edits would make.
//!
//! A candidate is a list of [`Step`]s: edits most often, but a forced
//! state or an input is a step too. With an audit, each candidate also
//! answers the findings it fixes and introduces against the base's — a
//! finding is the same one when its check and elements are — and the
//! candidates are ranked by fixed minus introduced (ties keep their
//! order); without one, they are answered in the order given.

use serde::{Deserialize, Serialize};

use super::diff::{Diff, diff};
use crate::app::CaptureApp;
use crate::audit::{Audit, AuditRequest, Check, Finding};
use crate::capture::Grid;
use crate::edit::Patch;
use crate::session::{Session, SessionError, Step};

/// The most candidates one `whatif` tries.
pub const MAX_CANDIDATES: usize = 16;

/// One candidate: its name and its steps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Candidate {
    pub name: String,
    pub steps: Vec<Step>,
}

impl Candidate {
    pub fn new(name: impl Into<String>, steps: Vec<Step>) -> Self {
        Candidate {
            name: name.into(),
            steps,
        }
    }
}

/// What one candidate did.
#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct Outcome {
    pub name: String,
    /// Its frame against the base's, after both settled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<Diff>,
    /// What its edits would write to their files.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch: Option<Patch>,
    /// Why it could not be tried (a step refused, the app panicked).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// What it does to the audit's findings, when one was asked for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub findings: Option<FindingDelta>,
    /// Its settled grid, for an image of it.
    #[serde(skip)]
    pub grid: Option<Grid>,
}

/// The audit findings a candidate fixes and introduces, over the checks
/// that ran on both the base and the candidate.
#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct FindingDelta {
    /// Fixed minus introduced: what the ranking sorts by.
    pub score: i64,
    pub fixed: Vec<Finding>,
    pub introduced: Vec<Finding>,
    /// The checks that ran on only one of the two, so are not compared
    /// (a check that stops running fixes nothing).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub incomparable: Vec<Check>,
}

/// A finding's identity across moments: its check and its elements (or,
/// with none, its message).
fn same(a: &Finding, b: &Finding) -> bool {
    a.check == b.check
        && if a.nodes.is_empty() && b.nodes.is_empty() {
            a.message == b.message
        } else {
            a.nodes == b.nodes
        }
}

/// The findings `now` fixes and introduces against `base`: each finding
/// matched to at most one (a multiset), over the checks both ran.
fn delta(base: &Audit, now: &Audit) -> FindingDelta {
    let both = |c: &Check| base.checked.contains(c) && now.checked.contains(c);
    let mut incomparable: Vec<Check> = base
        .checked
        .iter()
        .chain(&now.checked)
        .filter(|c| !both(c))
        .copied()
        .collect();
    incomparable.sort();
    incomparable.dedup();
    let (base, mut now): (Vec<&Finding>, Vec<&Finding>) = (
        base.findings.iter().filter(|f| both(&f.check)).collect(),
        now.findings.iter().filter(|f| both(&f.check)).collect(),
    );
    let mut fixed = Vec::new();
    for b in base {
        match now.iter().position(|n| same(b, n)) {
            Some(i) => {
                now.remove(i);
            }
            None => fixed.push(b.clone()),
        }
    }
    let introduced: Vec<Finding> = now.into_iter().cloned().collect();
    FindingDelta {
        score: fixed.len() as i64 - introduced.len() as i64,
        fixed,
        introduced,
        incomparable,
    }
}

impl<A: CaptureApp> Session<A> {
    /// Try each candidate from this session's moment in a fork of its own,
    /// letting the base and each candidate run `settle_ms` (transitions
    /// end, timers fire), and answer what each changed and would patch.
    /// The session is left as it was. A candidate that fails says why; the
    /// others are still tried.
    ///
    /// With `audit`, each candidate's findings are compared with the
    /// base's, and the candidates ranked by findings fixed minus
    /// introduced.
    pub fn whatif(
        &self,
        candidates: &[Candidate],
        settle_ms: u64,
        audit: Option<&AuditRequest>,
    ) -> Result<Vec<Outcome>, SessionError> {
        if candidates.len() > MAX_CANDIDATES {
            return Err(SessionError::InvalidInput(format!(
                "{} candidates: at most {MAX_CANDIDATES}",
                candidates.len()
            )));
        }
        let mut base_fork = self.fork()?;
        base_fork.advance(settle_ms)?;
        let base = base_fork.snapshot()?;
        let base_audit = match audit {
            Some(req) => Some(base_fork.audit(req)?),
            None => None,
        };
        let mut outcomes: Vec<Outcome> = candidates
            .iter()
            .map(|c| {
                let tried = (|| -> Result<_, SessionError> {
                    let mut fork = self.fork()?;
                    for step in &c.steps {
                        fork.apply(step)?;
                    }
                    fork.advance(settle_ms)?;
                    let snap = fork.snapshot()?;
                    let patch = fork.patch(false)?;
                    let findings = match (audit, &base_audit) {
                        (Some(req), Some(base)) => Some(delta(base, &fork.audit(req)?)),
                        _ => None,
                    };
                    Ok((diff(&base, &snap), patch, findings, snap.grid))
                })();
                match tried {
                    Ok((d, patch, findings, grid)) => Outcome {
                        name: c.name.clone(),
                        diff: Some(d),
                        patch: Some(patch),
                        error: None,
                        findings,
                        grid: Some(grid),
                    },
                    Err(e) => Outcome {
                        name: c.name.clone(),
                        diff: None,
                        patch: None,
                        error: Some(e.to_string()),
                        findings: None,
                        grid: None,
                    },
                }
            })
            .collect();
        if audit.is_some() {
            // Best first; a failed candidate last; ties keep their order.
            outcomes.sort_by_key(|o| {
                std::cmp::Reverse(o.findings.as_ref().map_or(i64::MIN, |f| f.score))
            });
        }
        Ok(outcomes)
    }
}

#[cfg(test)]
mod tests {
    use super::delta;
    use crate::audit::{Audit, Check, Finding, Severity};

    fn finding(check: Check, node: &str) -> Finding {
        Finding {
            check,
            severity: Severity::Error,
            message: "m".into(),
            nodes: vec![node.into()],
            cells: Vec::new(),
            inputs: serde_json::Value::Null,
            threshold: serde_json::Value::Null,
        }
    }

    fn audit(checked: Vec<Check>, findings: Vec<Finding>) -> Audit {
        Audit {
            profile: "p".into(),
            checked,
            not_checked: Vec::new(),
            partial: Vec::new(),
            findings,
        }
    }

    #[test]
    fn a_check_that_stops_running_fixes_nothing() {
        let base = audit(vec![Check::Tokens], vec![finding(Check::Tokens, "a")]);
        let now = audit(Vec::new(), Vec::new());
        let d = delta(&base, &now);
        assert_eq!((d.score, d.fixed.len()), (0, 0));
        assert_eq!(d.incomparable, [Check::Tokens]);
    }

    #[test]
    fn findings_are_matched_one_to_one() {
        let two = vec![finding(Check::Contrast, "a"), finding(Check::Contrast, "a")];
        let base = audit(vec![Check::Contrast], two);
        let now = audit(vec![Check::Contrast], vec![finding(Check::Contrast, "a")]);
        let d = delta(&base, &now);
        assert_eq!((d.score, d.fixed.len(), d.introduced.len()), (1, 1, 0));
    }
}
