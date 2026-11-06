//! The stdio server: read a request per line, write a response per line.

use std::io::{self, BufRead, Write};

use serde_json::Value;

use super::methods::dispatch;
use super::{Request, Response, RpcError, code};
use crate::app::CaptureApp;
use crate::session::Session;

/// Serve `app` over stdin and stdout until stdin closes.
pub fn serve(app: impl CaptureApp) -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    serve_io(app, stdin.lock(), stdout.lock())
}

/// Serve `app` over `input` and `output` until `input` ends.
pub fn serve_io(
    app: impl CaptureApp,
    input: impl BufRead,
    mut output: impl Write,
) -> io::Result<()> {
    let mut session = Session::new(app);
    for line in input.lines() {
        let line = line?;
        if let Some(response) = handle_line(&mut session, &line) {
            serde_json::to_writer(&mut output, &response)?;
            output.write_all(b"\n")?;
            output.flush()?;
        }
    }
    Ok(())
}

/// The response to one line: `None` for a blank line or a notification.
pub(crate) fn handle_line<A: CaptureApp>(session: &mut Session<A>, line: &str) -> Option<Response> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let value: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => {
            return Some(Response::error(
                Value::Null,
                RpcError::new(code::PARSE_ERROR, format!("not JSON: {e}")),
            ));
        }
    };
    let id = value.get("id").cloned().unwrap_or(Value::Null);
    if value.is_array() {
        return Some(Response::error(
            Value::Null,
            RpcError::new(
                code::INVALID_REQUEST,
                "batches are not supported; send one request per line",
            ),
        ));
    }
    let request: Request = match serde_json::from_value(value) {
        Ok(r) => r,
        Err(e) => {
            return Some(Response::error(
                id,
                RpcError::new(
                    code::INVALID_REQUEST,
                    format!("not a JSON-RPC request: {e}"),
                ),
            ));
        }
    };
    if request.jsonrpc != "2.0" {
        return Some(Response::error(
            id,
            RpcError::new(code::INVALID_REQUEST, "jsonrpc must be \"2.0\""),
        ));
    }
    let outcome = dispatch(session, &request.method, request.params);
    let id = request.id?;
    Some(match outcome {
        Ok(result) => Response::result(id, result),
        Err(error) => Response::error(id, error),
    })
}
