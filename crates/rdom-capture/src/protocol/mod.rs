//! The session protocol: JSON-RPC 2.0, one request per line in, one
//! response per line out (`specs/CAPTURE.md` §Session protocol).
//!
//! ```text
//! → {"jsonrpc":"2.0","id":1,"method":"open","params":{"fixture":"acid-1"}}
//! ← {"jsonrpc":"2.0","id":1,"result":{"time_ms":0,"cells_changed":4812,…}}
//! → {"jsonrpc":"2.0","id":2,"method":"input","params":{"inputs":[{"type":"key","key":"tab"}]}}
//! → {"jsonrpc":"2.0","id":3,"method":"capture","params":{"layers":["grid"],"text":{}}}
//! ```
//!
//! The Rust types in `methods` are the single source of every method's
//! parameters and results; `METHODS` lists them with a summary, and the
//! `describe` method returns that list, so a client can discover the
//! protocol from the session itself.
//!
//! A request without an `id` is a notification: it runs, and gets no
//! response; one with `"id": null` is answered with that `null`. Batches
//! (a JSON array) are not supported and are answered with an `invalid
//! request` error. Stdout carries the protocol and nothing else
//! ([`serve`] moves what app code prints to stderr, on Unix).

mod inspector;
mod methods;
mod serve;

#[cfg(test)]
pub(crate) use methods::METHODS;
#[cfg(test)]
pub(crate) use serve::handle_line;
pub use serve::{serve, serve_io};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::session::SessionError;

/// The protocol's identifier, returned by `describe`.
pub const PROTOCOL: &str = "rdom-capture-session/1";

/// A request.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Request {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

/// A response: a result or an error, for the request with `id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Response {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

/// A JSON-RPC error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct RpcError {
    pub code: i64,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// The error codes: JSON-RPC's own, then the session's.
pub mod code {
    pub const PARSE_ERROR: i64 = -32700;
    pub const INVALID_REQUEST: i64 = -32600;
    pub const METHOD_NOT_FOUND: i64 = -32601;
    pub const INVALID_PARAMS: i64 = -32602;
    /// No app is open.
    pub const NOT_OPEN: i64 = -32000;
    /// The app has no fixture of that name.
    pub const UNKNOWN_FIXTURE: i64 = -32001;
    /// The app failed to build.
    pub const BUILD_FAILED: i64 = -32002;
    /// App code panicked; the session closed.
    pub const APP_PANICKED: i64 = -32003;
    /// Drawing a frame or writing a file failed.
    pub const IO: i64 = -32004;
    /// The environment can't be run.
    pub const INVALID_ENVIRONMENT: i64 = -32005;
    /// An input can't be turned into terminal events.
    pub const INVALID_INPUT: i64 = -32006;
}

impl RpcError {
    /// An error with `code` and `message`.
    pub(crate) fn new(code: i64, message: impl Into<String>) -> Self {
        RpcError {
            code,
            message: message.into(),
            data: None,
        }
    }
}

impl From<SessionError> for RpcError {
    fn from(e: SessionError) -> Self {
        let c = match &e {
            SessionError::NotOpen => code::NOT_OPEN,
            SessionError::UnknownFixture { .. } => code::UNKNOWN_FIXTURE,
            SessionError::Build(_) => code::BUILD_FAILED,
            SessionError::AppPanicked(_) => code::APP_PANICKED,
            SessionError::Io(_) => code::IO,
            SessionError::InvalidEnvironment(_) => code::INVALID_ENVIRONMENT,
            SessionError::InvalidInput(_) => code::INVALID_INPUT,
        };
        let data = match &e {
            SessionError::UnknownFixture { known, .. } => {
                Some(serde_json::json!({ "known": known }))
            }
            _ => None,
        };
        RpcError {
            code: c,
            message: e.to_string(),
            data,
        }
    }
}

impl Response {
    /// A result for `id`.
    pub(crate) fn result(id: Value, result: Value) -> Self {
        Response {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// An error for `id` (`null` when the request's id is unknown).
    pub(crate) fn error(id: Value, error: RpcError) -> Self {
        Response {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}
