//! Every protocol method: its parameters, its result, and the call into
//! the [`Session`]. [`METHODS`] is the list `describe` returns;
//! [`dispatch`] answers exactly the methods it lists (a test holds the two
//! together).

use std::path::PathBuf;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{PROTOCOL, RpcError, code};
use crate::app::{CaptureApp, FixtureInfo};
use crate::capture::{Capture, Layer, SCHEMA};
use crate::env::Environment;
use crate::input::Input;
use crate::session::{CaptureRequest, Session};
use crate::views::text::{TextOptions, capture_text};

/// A method's name and what it does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) struct MethodInfo {
    pub name: &'static str,
    pub summary: &'static str,
}

/// Every method the session answers, in the order `describe` lists them.
pub(crate) const METHODS: &[MethodInfo] = &[
    MethodInfo {
        name: "describe",
        summary: "The protocol, the capture schema, the methods, the app's fixtures and the default environment.",
    },
    MethodInfo {
        name: "open",
        summary: "Build the app. Params: {env?: Environment, fixture?: string}. Returns the first frame's Change.",
    },
    MethodInfo {
        name: "input",
        summary: "Apply inputs in order, each followed by its frame. Params: {inputs: [Input]}. Returns the Change.",
    },
    MethodInfo {
        name: "advance",
        summary: "Move the virtual clock. Params: {ms: integer}. Returns the Change.",
    },
    MethodInfo {
        name: "capture",
        summary: "Capture the current frame. Params: {layers?: [\"grid\" | \"owners\" | \"boxes\" | \"nodes\"], text?: {bg?, fg?, owners?}, out?: path}. Layers that name elements bring a nodes table. With out, writes <out>.json (and <out>.txt) and returns their paths instead of the content.",
    },
    MethodInfo {
        name: "close",
        summary: "Close the app.",
    },
];

/// `describe`'s result.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct DescribeResult {
    pub protocol: &'static str,
    pub schema: &'static str,
    pub rdom_capture: &'static str,
    pub methods: &'static [MethodInfo],
    pub fixtures: Vec<FixtureInfo>,
    pub default_env: Environment,
    pub open: bool,
}

/// `open`'s parameters.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct OpenParams {
    pub env: Environment,
    pub fixture: Option<String>,
}

/// `input`'s parameters.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InputParams {
    pub inputs: Vec<Input>,
}

/// `advance`'s parameters.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AdvanceParams {
    pub ms: u64,
}

/// `capture`'s parameters.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct CaptureParams {
    /// The layers to include beside `meta`.
    pub layers: Vec<Layer>,
    /// Include the text view, with these key layers.
    pub text: Option<TextOptions>,
    /// Write the capture to `<out>.json` (and the text view to
    /// `<out>.txt`) instead of returning them.
    pub out: Option<PathBuf>,
}

/// `capture`'s result.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct CaptureResult {
    #[serde(flatten)]
    pub capture: Capture,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
}

/// `params` as `T`; absent params read as `{}`.
fn params<T: DeserializeOwned>(params: Option<Value>) -> Result<T, RpcError> {
    let value = params.unwrap_or_else(|| Value::Object(Default::default()));
    serde_json::from_value(value)
        .map_err(|e| RpcError::new(code::INVALID_PARAMS, format!("invalid params: {e}")))
}

/// `value` as a JSON value.
fn json(value: impl Serialize) -> Result<Value, RpcError> {
    serde_json::to_value(value)
        .map_err(|e| RpcError::new(code::IO, format!("result does not serialize: {e}")))
}

/// Answer `method` with `p` on `session`.
pub(crate) fn dispatch<A: CaptureApp>(
    session: &mut Session<A>,
    method: &str,
    p: Option<Value>,
) -> Result<Value, RpcError> {
    match method {
        "describe" => json(DescribeResult {
            protocol: PROTOCOL,
            schema: SCHEMA,
            rdom_capture: env!("CARGO_PKG_VERSION"),
            methods: METHODS,
            fixtures: session.fixtures()?,
            default_env: Environment::default(),
            open: session.is_open(),
        }),
        "open" => {
            let p: OpenParams = params(p)?;
            json(session.open(p.env, p.fixture.as_deref())?)
        }
        "input" => {
            let p: InputParams = params(p)?;
            json(session.input(&p.inputs)?)
        }
        "advance" => {
            let p: AdvanceParams = params(p)?;
            json(session.advance(p.ms)?)
        }
        "capture" => capture(session, params(p)?),
        "close" => {
            let _: Empty = params(p)?;
            session.close();
            Ok(Value::Object(Default::default()))
        }
        other => Err(RpcError::new(
            code::METHOD_NOT_FOUND,
            format!("no method {other:?}; call describe for the list"),
        )),
    }
}

/// A method that takes no parameters.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}

/// `capture`.
fn capture<A: CaptureApp>(session: &Session<A>, p: CaptureParams) -> Result<Value, RpcError> {
    let mut layers = p.layers.clone();
    // The text view reads the grid, and its owners layer the owners.
    let mut needs = |l: Layer| {
        if !layers.contains(&l) {
            layers.push(l);
        }
    };
    if let Some(o) = p.text {
        needs(Layer::Grid);
        if o.owners {
            needs(Layer::Owners);
        }
    }
    let mut capture = session.capture(&CaptureRequest::with_layers(layers))?;
    let text = p.text.and_then(|o| capture_text(&capture, o));
    if !p.layers.contains(&Layer::Grid) {
        capture.grid = None;
    }
    if !p.layers.contains(&Layer::Owners) {
        capture.owners = None;
        if !p.layers.contains(&Layer::Boxes) && !p.layers.contains(&Layer::Nodes) {
            capture.nodes = None;
        }
    }
    let Some(out) = p.out else {
        return json(CaptureResult {
            capture,
            text,
            files: Vec::new(),
        });
    };
    let write = |path: PathBuf, bytes: &[u8]| -> Result<String, RpcError> {
        std::fs::write(&path, bytes)
            .map_err(|e| RpcError::new(code::IO, format!("writing {}: {e}", path.display())))?;
        Ok(path.display().to_string())
    };
    let mut files = Vec::new();
    let body = serde_json::to_vec_pretty(&capture)
        .map_err(|e| RpcError::new(code::IO, format!("capture does not serialize: {e}")))?;
    files.push(write(with_suffix(&out, ".json"), &body)?);
    if let Some(t) = &text {
        files.push(write(with_suffix(&out, ".txt"), t.as_bytes())?);
    }
    capture.grid = None;
    json(CaptureResult {
        capture,
        text: None,
        files,
    })
}

/// `path` with `suffix` appended to its last component (`shot.v2` →
/// `shot.v2.json`, where `with_extension` would drop `.v2`).
fn with_suffix(path: &std::path::Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_os_string();
    s.push(suffix);
    PathBuf::from(s)
}
