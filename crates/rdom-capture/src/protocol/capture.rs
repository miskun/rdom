//! The `capture` method: the capture document with the layers asked for,
//! and its views — the text view and the `png` image
//! (`specs/CAPTURE.md` §Views) — returned, or written beside `out`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::methods::json;
use super::{RpcError, code};
use crate::app::CaptureApp;
use crate::capture::{Capture, Layer};
use crate::profile::Profile;
use crate::raster::{Raster, render};
use crate::session::{CaptureRequest, Session};
use crate::views::base64::base64;
use crate::views::text::{TextOptions, capture_text};

/// The largest `png` scale: 4 × 8×16 pixels a cell is already 32×64.
pub(crate) const MAX_SCALE: u32 = 4;

/// `capture`'s parameters.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct CaptureParams {
    /// The layers to include beside `meta`.
    pub layers: Vec<Layer>,
    /// Include the text view, with these key layers.
    pub text: Option<TextOptions>,
    /// Include the `png` view.
    pub png: Option<PngParams>,
    /// Write the capture to `<out>.json` (and the views to `<out>.txt`,
    /// `<out>.png`) instead of returning them.
    pub out: Option<PathBuf>,
}

/// The `png` view's parameters.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct PngParams {
    /// The terminal profile to draw with (default: the first `describe`
    /// lists).
    pub profile: Option<String>,
    /// Pixels per font pixel, 1–4 (default 1).
    pub scale: Option<u32>,
}

/// `capture`'s result.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct CaptureResult {
    #[serde(flatten)]
    pub capture: Capture,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub png: Option<PngResult>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
}

/// The `png` view: how it was drawn, and the image (base64) unless it was
/// written to a file.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct PngResult {
    pub profile: String,
    pub scale: u32,
    pub width: u32,
    pub height: u32,
    /// The characters the font has no glyph for, drawn as a box with their
    /// code point.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unrendered_glyphs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base64: Option<String>,
}

fn invalid(msg: impl Into<String>) -> RpcError {
    RpcError::new(code::INVALID_PARAMS, msg)
}

/// The profile `name` names, or the default.
fn profile(name: Option<&str>) -> Result<Profile, RpcError> {
    match name {
        None => Ok(Profile::default_profile()),
        Some(n) => Profile::named(n).ok_or_else(|| {
            let known: Vec<String> = Profile::builtin().into_iter().map(|p| p.name).collect();
            invalid(format!("no profile {n:?}: {}", known.join(", ")))
        }),
    }
}

pub(crate) fn capture<A: CaptureApp>(
    session: &Session<A>,
    p: CaptureParams,
) -> Result<Value, RpcError> {
    let mut layers = p.layers.clone();
    // The text view reads the grid, and its owners layer the owners; the
    // image reads the grid.
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
    let png_how = match &p.png {
        Some(o) => {
            let scale = o.scale.unwrap_or(1);
            if !(1..=MAX_SCALE).contains(&scale) {
                return Err(invalid(format!("png scale {scale} is not 1–{MAX_SCALE}")));
            }
            needs(Layer::Grid);
            Some((profile(o.profile.as_deref())?, scale))
        }
        None => None,
    };
    let mut capture = session.capture(&CaptureRequest::with_layers(layers))?;
    let text = p.text.and_then(|o| capture_text(&capture, o));
    let image = png_how.map(|(profile, scale)| {
        let grid = capture.grid.as_ref().expect("the grid was asked for");
        let image = render(grid, Raster::new(&profile, scale));
        let result = PngResult {
            profile: profile.name.clone(),
            scale,
            width: image.width,
            height: image.height,
            unrendered_glyphs: image.missing.iter().map(char::to_string).collect(),
            base64: None,
        };
        (result, image.png())
    });
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
            png: image.map(|(mut r, bytes)| {
                r.base64 = Some(base64(&bytes));
                r
            }),
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
    let png = match image {
        Some((r, bytes)) => {
            files.push(write(with_suffix(&out, ".png"), &bytes)?);
            Some(r)
        }
        None => None,
    };
    capture.grid = None;
    json(CaptureResult {
        capture,
        text: None,
        png,
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

/// `report`'s parameters.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReportParams {
    /// Write the page to `<out>.html`.
    pub out: PathBuf,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub scale: Option<u32>,
}

pub(crate) fn report<A: CaptureApp>(
    session: &Session<A>,
    p: ReportParams,
) -> Result<Value, RpcError> {
    let scale = p.scale.unwrap_or(1);
    if !(1..=MAX_SCALE).contains(&scale) {
        return Err(invalid(format!("scale {scale} is not 1–{MAX_SCALE}")));
    }
    let html = session.report(&profile(p.profile.as_deref())?, scale)?;
    let path = with_suffix(&p.out, ".html");
    std::fs::write(&path, &html)
        .map_err(|e| RpcError::new(code::IO, format!("writing {}: {e}", path.display())))?;
    json(serde_json::json!({ "file": path.display().to_string(), "bytes": html.len() }))
}
