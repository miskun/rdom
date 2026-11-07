//! A session's step log and its forks (`specs/CAPTURE.md` §Variants): every
//! step that changed the open app, in order, so the session can be rebuilt
//! — the app built again in the same environment and fixture, the steps
//! applied again. The environment is deterministic, so the rebuilt app
//! shows the same frame; a fork checks that it does, and is then a session
//! of its own: `whatif`, `matrix` and `stress` try changes in forks and
//! leave the session as it was. Phase 9's record and replay build on the
//! same log.
//!
//! A session driven outside its steps (`Session::app_mut`), or whose files
//! a `patch` rewrote (the app reads them again when it is rebuilt), cannot
//! be rebuilt, and says so.

use rdom_tui::core_api::ForcedState;
use serde::{Deserialize, Serialize};

use super::{Session, SessionError};
use crate::app::CaptureApp;
use crate::edit::EditTarget;
use crate::env::Environment;
use crate::input::Input;

/// One step of a session, as its log records it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "step", rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub enum Step {
    /// [`Session::input`].
    Input { inputs: Vec<Input> },
    /// [`Session::advance`].
    Advance { ms: u64 },
    /// [`Session::edit`].
    Edit {
        target: EditTarget,
        property: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        value: Option<String>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        important: bool,
    },
    /// [`Session::force`]; `state` is the pseudo-class's name.
    Force {
        node: String,
        state: String,
        on: bool,
    },
    /// [`Session::reset`].
    Reset,
    /// [`Session::reload`].
    Reload,
}

impl<A: CaptureApp> Session<A> {
    /// The steps that made the open app's frame, in order (empty when no
    /// app is open).
    pub fn log(&self) -> &[Step] {
        self.open.as_ref().map_or(&[], |o| &o.log)
    }

    /// The environment the open app was opened in — what a fork starts
    /// from (a resize step changes the app's environment after).
    pub fn opened_environment(&self) -> Result<&Environment, SessionError> {
        self.open
            .as_ref()
            .map(|o| &o.opened_env)
            .ok_or(SessionError::NotOpen)
    }

    /// Record `step`, which the open app just took.
    pub(super) fn record(&mut self, step: Step) {
        if let Some(o) = self.open.as_mut() {
            o.log.push(step);
        }
    }

    /// Note that the open app can no longer be rebuilt from its log, and
    /// why.
    pub(super) fn unreplayable(&mut self, why: &str) {
        if let Some(o) = self.open.as_mut()
            && o.unreplayable.is_none()
        {
            o.unreplayable = Some(why.to_string());
        }
    }

    /// A new session showing what this one shows: the app built again in
    /// the environment and fixture it was opened with, and every logged
    /// step applied again. Its frame is checked against this session's
    /// ([`SessionError::Diverged`] when they differ — an app fed by other
    /// threads, `CAPTURE-INJECT-1`); from then on the two are independent.
    pub fn fork(&self) -> Result<Session<A>, SessionError> {
        let o = self.open.as_ref().ok_or(SessionError::NotOpen)?;
        let fork = self.rebuild(o.opened_env.clone())?;
        let (want, got) = (&o.frame.hash, &fork.open.as_ref().expect("open").frame.hash);
        if want != got {
            return Err(SessionError::Diverged {
                expected: want.clone(),
                got: got.clone(),
            });
        }
        Ok(fork)
    }

    /// This session's steps applied in `env` instead: the app built there
    /// and every logged step applied again (a `matrix` variant). What it
    /// shows is its own, so nothing is checked against this session.
    pub fn fork_in(&self, env: Environment) -> Result<Session<A>, SessionError> {
        self.rebuild(env)
    }

    fn rebuild(&self, env: Environment) -> Result<Session<A>, SessionError> {
        let o = self.open.as_ref().ok_or(SessionError::NotOpen)?;
        if let Some(why) = &o.unreplayable {
            return Err(SessionError::Unreplayable(why.clone()));
        }
        let mut fork = Session {
            def: self.def.clone(),
            open: None,
            marks: Vec::new(),
        };
        fork.open(env, o.fixture.as_deref())?;
        for step in &o.log {
            fork.apply(step)?;
        }
        Ok(fork)
    }

    /// Take `step` again.
    pub(crate) fn apply(&mut self, step: &Step) -> Result<(), SessionError> {
        match step {
            Step::Input { inputs } => self.input(inputs).map(drop),
            Step::Advance { ms } => self.advance(*ms).map(drop),
            Step::Edit {
                target,
                property,
                value,
                important,
            } => self
                .edit(target, property, value.as_deref(), *important)
                .map(drop),
            Step::Force { node, state, on } => {
                let state = ForcedState::from_name(state).ok_or_else(|| {
                    SessionError::InvalidInput(format!("no forced state {state:?}"))
                })?;
                self.force(node, state, *on).map(drop)
            }
            Step::Reset => self.reset().map(drop),
            Step::Reload => self.reload().map(drop),
        }
    }
}
