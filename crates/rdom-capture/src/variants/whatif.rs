//! `whatif` (`specs/CAPTURE.md` §Variants): candidate changes, each tried
//! from the same base in a fork of the session — the session itself is
//! left as it was — and answered with what it changed on screen (a
//! [`Diff`] against the base, both settled the same time) and the patch
//! its edits would make.
//!
//! A candidate is a list of [`Step`]s: edits most often, but a forced
//! state or an input is a step too. Ranking candidates by the audit
//! findings they fix and introduce comes with the audits (phase 7); until
//! then they are answered in the order given.

use serde::{Deserialize, Serialize};

use super::diff::{Diff, diff};
use crate::app::CaptureApp;
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
    /// Its settled grid, for an image of it.
    #[serde(skip)]
    pub grid: Option<Grid>,
}

impl<A: CaptureApp> Session<A> {
    /// Try each candidate from this session's moment in a fork of its own,
    /// letting the base and each candidate run `settle_ms` (transitions
    /// end, timers fire), and answer what each changed and would patch.
    /// The session is left as it was. A candidate that fails says why; the
    /// others are still tried.
    pub fn whatif(
        &self,
        candidates: &[Candidate],
        settle_ms: u64,
    ) -> Result<Vec<Outcome>, SessionError> {
        if candidates.len() > MAX_CANDIDATES {
            return Err(SessionError::InvalidInput(format!(
                "{} candidates: at most {MAX_CANDIDATES}",
                candidates.len()
            )));
        }
        let mut base = self.fork()?;
        base.advance(settle_ms)?;
        let base = base.snapshot()?;
        Ok(candidates
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
                    Ok((diff(&base, &snap), patch, snap.grid))
                })();
                match tried {
                    Ok((d, patch, grid)) => Outcome {
                        name: c.name.clone(),
                        diff: Some(d),
                        patch: Some(patch),
                        error: None,
                        grid: Some(grid),
                    },
                    Err(e) => Outcome {
                        name: c.name.clone(),
                        diff: None,
                        patch: None,
                        error: Some(e.to_string()),
                        grid: None,
                    },
                }
            })
            .collect())
    }
}
