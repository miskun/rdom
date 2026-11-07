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
use crate::capture::{Capture, ColorValue, Layer, SCHEMA, node_key};
use crate::env::Environment;
use crate::input::Input;
use crate::query::{Detail, fit};
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
        name: "outline",
        summary: "The landmarks outline: landmarks, headings, lists, controls and focusable elements, nested, each with role, name, rect and state. Params: {depth?: integer, detail?: \"outline\"|\"summary\"|\"full\", budget?: tokens}. Read it first.",
    },
    MethodInfo {
        name: "explain",
        summary: "Why a cell or an element looks as it does: the cell's paint operations, the element whose paint it shows, and for each relevant property the winning declaration (value, selector, specificity, file:line), what it overrode, or the ancestor it inherits from. Params: one of {at: [x, y]} | {select: css} | {node: id}; properties?: [names]; detail?; budget?.",
    },
    MethodInfo {
        name: "query",
        summary: "Reverse lookups on the current frame. Params: one of {selector: css} (elements it matches) | {color: \"#rrggbb\"|\"ansi:N\"} (who paints it, fg/bg, how many cells) | {rect: [x, y, w, h]} (what is painted there) | {text: string} (where it is on screen, who painted it); budget?.",
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
        "outline" => outline(session, params(p)?),
        "explain" => explain(session, params(p)?),
        "query" => query(session, params(p)?),
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

/// `outline`'s parameters.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct OutlineParams {
    /// How deep the outline nests (default by detail: 3, 8, unlimited).
    pub depth: Option<usize>,
    pub detail: Detail,
    pub budget: Option<usize>,
}

/// `explain`'s parameters: exactly one target.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct ExplainParams {
    pub at: Option<[u16; 2]>,
    pub select: Option<String>,
    pub node: Option<String>,
    /// The properties to explain (default: the ones the painted part
    /// reads, or every declared one for an element).
    pub properties: Vec<String>,
    pub detail: Detail,
    pub budget: Option<usize>,
}

/// `query`'s parameters: exactly one question.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct QueryParams {
    pub selector: Option<String>,
    pub color: Option<ColorValue>,
    pub rect: Option<[u16; 4]>,
    pub text: Option<String>,
    pub budget: Option<usize>,
}

fn invalid(msg: impl Into<String>) -> RpcError {
    RpcError::new(code::INVALID_PARAMS, msg)
}

/// The open app, or `NOT_OPEN`.
fn open_app<A: CaptureApp>(
    session: &Session<A>,
) -> Result<&rdom_tui::App<rdom_tui::render::TestBackend>, RpcError> {
    Ok(session.app()?)
}

fn outline<A: CaptureApp>(session: &Session<A>, p: OutlineParams) -> Result<Value, RpcError> {
    let app = open_app(session)?;
    let depth = p.depth.unwrap_or(match p.detail {
        Detail::Outline => 3,
        Detail::Summary => 8,
        _ => usize::MAX,
    });
    let (nodes, dropped) = crate::query::outline::outline(app, depth);
    let mut v = serde_json::json!({ "outline": nodes });
    if dropped > 0 {
        v["deeper"] = Value::from(dropped);
    }
    fit(&mut v, p.budget);
    Ok(v)
}

/// The node named `key` in the open app.
fn node_named<A: CaptureApp>(
    session: &Session<A>,
    key: &str,
) -> Result<rdom_tui::core_api::NodeId, RpcError> {
    let dom = open_app(session)?.dom();
    std::iter::once(dom.root())
        .chain(dom.descendants(dom.root()))
        .find(|&n| node_key(n) == key)
        .ok_or_else(|| invalid(format!("no node {key:?} in the document")))
}

fn explain<A: CaptureApp>(session: &Session<A>, p: ExplainParams) -> Result<Value, RpcError> {
    let app = open_app(session)?;
    let targets = [p.at.is_some(), p.select.is_some(), p.node.is_some()];
    if targets.iter().filter(|t| **t).count() != 1 {
        return Err(invalid("explain takes exactly one of at, select, node"));
    }
    let answer = if let Some([x, y]) = p.at {
        crate::query::explain::cell(app, x, y, &p.properties, p.detail)
            .ok_or_else(|| invalid(format!("nothing is painted at ({x}, {y})")))?
    } else {
        let id = match (&p.select, &p.node) {
            (Some(sel), _) => app
                .dom()
                .query_selector_in(app.dom().root(), sel)
                .map_err(|e| invalid(format!("invalid selector {sel:?}: {e:?}")))?
                .ok_or_else(|| invalid(format!("{sel:?} matches no element")))?,
            (_, Some(key)) => node_named(session, key)?,
            _ => unreachable!("one target"),
        };
        crate::query::explain::element(app, id, &p.properties, None, p.detail)
    };
    let mut v = json(answer)?;
    fit(&mut v, p.budget);
    Ok(v)
}

fn query<A: CaptureApp>(session: &Session<A>, p: QueryParams) -> Result<Value, RpcError> {
    let app = open_app(session)?;
    let asked = [
        p.selector.is_some(),
        p.color.is_some(),
        p.rect.is_some(),
        p.text.is_some(),
    ];
    if asked.iter().filter(|a| **a).count() != 1 {
        return Err(invalid(
            "query takes exactly one of selector, color, rect, text",
        ));
    }
    use crate::query::find;
    let found = if let Some(sel) = &p.selector {
        find::by_selector(app, sel).map_err(invalid)?
    } else {
        let c = session.capture(&CaptureRequest::with_layers([Layer::Grid, Layer::Owners]))?;
        let (grid, owners) = (c.grid.expect("asked for"), c.owners.expect("asked for"));
        if let Some(color) = p.color {
            find::by_color(app, &grid, &owners, color)
        } else if let Some([x, y, w, h]) = p.rect {
            find::in_rect(app, &grid, &owners, rdom_tui::Rect::new(x, y, w, h))
        } else {
            find::by_text(app, &grid, &owners, p.text.as_deref().unwrap_or_default())
        }
    };
    let mut v = serde_json::json!({ "found": found });
    fit(&mut v, p.budget);
    Ok(v)
}
