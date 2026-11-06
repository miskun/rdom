//! The stdio server: read a request per line, write a response per line.

use std::io::{self, BufRead, Write};

use serde_json::Value;

use super::methods::dispatch;
use super::{Request, Response, RpcError, code};
use crate::app::CaptureApp;
use crate::session::Session;

/// Serve `app` over stdin and stdout until stdin closes.
///
/// Stdout carries the protocol and nothing else. On Unix the server keeps
/// its own handle on the process's stdout and points file descriptor 1 at
/// stderr, so anything app code prints (`println!`, a library writing to
/// stdout) lands on stderr instead of corrupting a response. Elsewhere an
/// app served this way must not print to stdout.
pub fn serve(app: impl CaptureApp) -> io::Result<()> {
    let stdin = io::stdin();
    let output = protocol_output()?;
    serve_io(app, stdin.lock(), output)
}

/// The protocol's output: the original stdout, with fd 1 moved to stderr.
#[cfg(unix)]
fn protocol_output() -> io::Result<impl Write> {
    use std::os::fd::AsFd;
    io::stdout().flush()?;
    let original = rustix::io::dup(io::stdout().as_fd())?;
    rustix::stdio::dup2_stdout(io::stderr().as_fd())?;
    Ok(io::LineWriter::new(std::fs::File::from(original)))
}

/// The protocol's output: stdout.
#[cfg(not(unix))]
fn protocol_output() -> io::Result<impl Write> {
    Ok(io::stdout())
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
    // A request with an `id` member — `null` included — is answered; one
    // without is a notification (JSON-RPC 2.0 §4.1).
    let answered = value.get("id").is_some();
    let raw_id = value.get("id").cloned().unwrap_or(Value::Null);
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
                raw_id,
                RpcError::new(
                    code::INVALID_REQUEST,
                    format!("not a JSON-RPC request: {e}"),
                ),
            ));
        }
    };
    if request.jsonrpc != "2.0" {
        return Some(Response::error(
            raw_id,
            RpcError::new(code::INVALID_REQUEST, "jsonrpc must be \"2.0\""),
        ));
    }
    let outcome = dispatch(session, &request.method, request.params);
    if !answered {
        return None;
    }
    // `"id": null` deserializes as no id; it is answered with `null`.
    let id = request.id.unwrap_or(Value::Null);
    Some(match outcome {
        Ok(result) => Response::result(id, result),
        Err(error) => Response::error(id, error),
    })
}
