//! Why a session step failed.

use std::fmt;

/// Why a session step failed.
#[derive(Debug)]
#[non_exhaustive]
pub enum SessionError {
    /// No app is open: call [`Session::open`](super::Session::open) first.
    NotOpen,
    /// The environment can't be run.
    InvalidEnvironment(String),
    /// The app has no fixture of that name.
    UnknownFixture { name: String, known: Vec<String> },
    /// The app failed to build.
    Build(String),
    /// An input or a step can't be applied as asked.
    InvalidInput(String),
    /// App code panicked; an open app was closed.
    AppPanicked(String),
    /// Drawing a frame failed.
    Io(std::io::Error),
    /// The session cannot be rebuilt from its log (a fork), and why.
    Unreplayable(String),
    /// A fork rebuilt from the log shows another frame than the session
    /// (each named by its grid hash): the app is not deterministic.
    Diverged { expected: String, got: String },
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionError::NotOpen => f.write_str("no app is open; call open first"),
            SessionError::InvalidEnvironment(e) => write!(f, "invalid environment: {e}"),
            SessionError::UnknownFixture { name, known } => {
                write!(f, "no fixture {name:?}; the app has: {}", known.join(", "))
            }
            SessionError::Build(e) => write!(f, "the app failed to build: {e}"),
            SessionError::InvalidInput(e) => write!(f, "invalid input: {e}"),
            SessionError::AppPanicked(e) => {
                write!(
                    f,
                    "app code panicked ({e}); the app is closed, open it again"
                )
            }
            SessionError::Io(e) => write!(f, "drawing a frame failed: {e}"),
            SessionError::Unreplayable(why) => {
                write!(f, "the session cannot be rebuilt from its steps: {why}")
            }
            SessionError::Diverged { expected, got } => write!(
                f,
                "a fork rebuilt from the session's steps shows another frame ({got}, the session {expected}): \
                 the app is not deterministic — another thread feeding it (CAPTURE-INJECT-1)?"
            ),
        }
    }
}

impl std::error::Error for SessionError {}

impl From<std::io::Error> for SessionError {
    fn from(e: std::io::Error) -> Self {
        SessionError::Io(e)
    }
}

/// The message of a caught panic.
pub(super) fn panic_message(panic: &(dyn std::any::Any + Send)) -> String {
    panic
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| panic.downcast_ref::<&str>().map(|s| (*s).to_string()))
        .unwrap_or_else(|| "a panic with no message".to_string())
}
