//! The variant methods (`specs/CAPTURE.md` §Variants): `mark` and `diff`,
//! `whatif`, `matrix` — their parameters and the calls into the session.

use std::path::PathBuf;

use serde::Deserialize;
use serde_json::Value;

use super::capture::{invalid, profile, with_suffix};
use super::methods::json;
use super::{RpcError, code};
use crate::app::CaptureApp;
use crate::capture::Grid;
use crate::query::fit;
use crate::raster::{Raster, render, sheet::sheet};
use crate::session::Session;
use crate::variants::stress::Rewrite;
use crate::variants::whatif::Candidate;
use crate::views::base64::base64;

/// `mark`'s parameters.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MarkParams {
    pub name: String,
}

/// `diff`'s parameters: from a mark, to another or to now.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DiffParams {
    pub from: String,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub budget: Option<usize>,
}

/// An image of each answer: a profile and a scale.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Images {
    pub profile: Option<String>,
    pub scale: Option<u32>,
}

/// `whatif`'s parameters.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WhatifParams {
    pub candidates: Vec<Candidate>,
    /// How long the base and each candidate run before they are compared
    /// (default 500 ms: transitions end).
    #[serde(default = "settle")]
    pub settle_ms: u64,
    #[serde(default)]
    pub png: Option<Images>,
    /// Audit the base and each candidate, and rank by findings fixed
    /// minus introduced.
    #[serde(default)]
    pub audit: Option<crate::audit::AuditRequest>,
    #[serde(default)]
    pub budget: Option<usize>,
}

/// `matrix`'s parameters.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MatrixParams {
    pub axes: Vec<Vec<Value>>,
    #[serde(default = "settle")]
    pub settle_ms: u64,
    /// The contact sheet's image (default: the default profile, scale 1).
    #[serde(default)]
    pub png: Option<Images>,
    /// Tiles to a row (default: as square as the variants allow).
    #[serde(default)]
    pub columns: Option<usize>,
    /// Write the sheet to `<out>.png` instead of returning it.
    #[serde(default)]
    pub out: Option<PathBuf>,
}

/// `stress`'s parameters.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StressParams {
    pub select: String,
    /// Default: every rewrite.
    #[serde(default)]
    pub rewrites: Option<Vec<Rewrite>>,
    #[serde(default = "settle")]
    pub settle_ms: u64,
    #[serde(default)]
    pub budget: Option<usize>,
}

/// `mockup`'s parameters.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MockupParams {
    pub text: String,
    #[serde(default)]
    pub at: [u16; 2],
    #[serde(default)]
    pub any: Option<char>,
    #[serde(default)]
    pub budget: Option<usize>,
}

fn settle() -> u64 {
    500
}

/// `grid` as a PNG, base64.
fn png_of(grid: &Grid, images: &Images) -> Result<String, RpcError> {
    let profile = profile(images.profile.as_deref())?;
    let image = render(grid, Raster::new(&profile, images.scale.unwrap_or(1))?)?;
    Ok(base64(&image.png()))
}

pub(crate) fn mark<A: CaptureApp>(
    session: &mut Session<A>,
    p: MarkParams,
) -> Result<Value, RpcError> {
    session.mark(&p.name)?;
    json(serde_json::json!({ "marked": p.name }))
}

pub(crate) fn diff<A: CaptureApp>(session: &Session<A>, p: DiffParams) -> Result<Value, RpcError> {
    let mut answer = json(session.diff(&p.from, p.to.as_deref())?)?;
    fit(&mut answer, p.budget);
    Ok(answer)
}

pub(crate) fn whatif<A: CaptureApp>(
    session: &Session<A>,
    p: WhatifParams,
) -> Result<Value, RpcError> {
    let outcomes = session.whatif(&p.candidates, p.settle_ms, p.audit.as_ref())?;
    let mut answer = Vec::with_capacity(outcomes.len());
    for o in &outcomes {
        let mut v = json(o)?;
        if let (Some(images), Some(grid)) = (&p.png, &o.grid) {
            v["png"] = Value::String(png_of(grid, images)?);
        }
        answer.push(v);
    }
    let mut answer = serde_json::json!({ "settle_ms": p.settle_ms, "outcomes": answer });
    fit(&mut answer, p.budget);
    Ok(answer)
}

pub(crate) fn matrix<A: CaptureApp>(
    session: &Session<A>,
    p: MatrixParams,
) -> Result<Value, RpcError> {
    let variants = session.matrix(&p.axes, p.settle_ms)?;
    let tiles: Vec<(String, &Grid)> = variants
        .iter()
        .filter_map(|v| Some((v.label.clone(), v.grid.as_ref()?)))
        .collect();
    let columns = p
        .columns
        .unwrap_or_else(|| (tiles.len() as f64).sqrt().ceil() as usize)
        .max(1);
    let mut answer = serde_json::json!({ "variants": json(&variants)? });
    if !tiles.is_empty() {
        let images = p.png.unwrap_or_default();
        let profile = profile(images.profile.as_deref())?;
        let image = render(
            &sheet(&tiles, columns),
            Raster::new(&profile, images.scale.unwrap_or(1))?,
        )?;
        let bytes = image.png();
        let mut s = serde_json::json!({
            "profile": profile.name, "columns": columns,
            "width": image.width(), "height": image.height(),
        });
        match p.out {
            Some(out) => {
                let path = with_suffix(&out, ".png");
                std::fs::write(&path, &bytes).map_err(|e| {
                    RpcError::new(code::IO, format!("writing {}: {e}", path.display()))
                })?;
                s["file"] = Value::String(path.display().to_string());
            }
            None => s["base64"] = Value::String(base64(&bytes)),
        }
        answer["sheet"] = s;
    } else if p.out.is_some() {
        return Err(invalid(
            "no variant could be shown, so there is no sheet to write",
        ));
    }
    Ok(answer)
}

pub(crate) fn stress<A: CaptureApp>(
    session: &Session<A>,
    p: StressParams,
) -> Result<Value, RpcError> {
    let rewrites = p.rewrites.unwrap_or_else(|| Rewrite::ALL.to_vec());
    let mut answer = serde_json::json!({
        "results": json(session.stress(&p.select, &rewrites, p.settle_ms)?)?,
    });
    fit(&mut answer, p.budget);
    Ok(answer)
}

pub(crate) fn mockup<A: CaptureApp>(
    session: &Session<A>,
    p: MockupParams,
) -> Result<Value, RpcError> {
    let mut answer = json(session.mockup(&p.text, p.at, p.any)?)?;
    fit(&mut answer, p.budget);
    Ok(answer)
}

/// `audit`'s parameters: the request, and a token budget.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct AuditParams {
    #[serde(flatten)]
    pub request: crate::audit::AuditRequest,
    pub budget: Option<usize>,
}

pub(crate) fn audit<A: CaptureApp>(
    session: &Session<A>,
    p: AuditParams,
) -> Result<Value, RpcError> {
    let mut answer = json(session.audit(&p.request)?)?;
    fit(&mut answer, p.budget);
    Ok(answer)
}

/// `film`'s parameters: the film, and what to make of it.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct FilmParams {
    #[serde(flatten)]
    pub request: crate::motion::FilmRequest,
    /// A filmstrip of the frames (at most 24, evenly spaced), as an image.
    pub filmstrip: Option<Images>,
    /// The asciicast of the bytes sent.
    pub cast: bool,
    /// Write the filmstrip to `<out>.png` and the cast to `<out>.cast`
    /// instead of returning them.
    pub out: Option<PathBuf>,
    pub budget: Option<usize>,
}

pub(crate) fn film<A: CaptureApp>(session: &Session<A>, p: FilmParams) -> Result<Value, RpcError> {
    let film = session.film(&p.request)?;
    let mut answer = json(&film)?;
    let write = |path: PathBuf, bytes: &[u8]| -> Result<String, RpcError> {
        std::fs::write(&path, bytes)
            .map_err(|e| RpcError::new(code::IO, format!("writing {}: {e}", path.display())))?;
        Ok(path.display().to_string())
    };
    if let Some(images) = &p.filmstrip {
        let tiles = strip(&film);
        let profile = profile(images.profile.as_deref())?;
        let columns = (tiles.len() as f64).sqrt().ceil() as usize;
        let image = render(
            &sheet(&tiles, columns),
            Raster::new(&profile, images.scale.unwrap_or(1))?,
        )?;
        let bytes = image.png();
        answer["filmstrip"] = match &p.out {
            Some(out) => serde_json::json!({ "file": write(with_suffix(out, ".png"), &bytes)? }),
            None => serde_json::json!({ "base64": base64(&bytes) }),
        };
    }
    if p.cast {
        let text = crate::motion::cast::asciicast(&film);
        answer["cast"] = match &p.out {
            Some(out) => {
                serde_json::json!({ "file": write(with_suffix(out, ".cast"), text.as_bytes())? })
            }
            None => Value::String(text),
        };
    }
    // The budget covers the whole answer, the inline cast and image too.
    fit(&mut answer, p.budget);
    Ok(answer)
}

/// At most 24 of a film's frames, evenly spaced, the last always among
/// them (the end state), each labelled by its time.
fn strip(film: &crate::motion::Film) -> Vec<(String, &Grid)> {
    let n = film.grids.len();
    let every = n.div_ceil(24).max(1);
    let mut picks: Vec<usize> = (0..n).step_by(every).collect();
    if n > 0 && picks.last() != Some(&(n - 1)) {
        if picks.len() == 24 {
            picks.pop();
        }
        picks.push(n - 1);
    }
    picks
        .into_iter()
        .map(|i| (format!("{} ms", film.frames[i].t_ms), &film.grids[i]))
        .collect()
}

/// `replay`'s parameters: the recording, as text or a file.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct ReplayParams {
    pub log: Option<String>,
    pub path: Option<PathBuf>,
    pub until_ms: Option<u64>,
}

pub(crate) fn replay<A: CaptureApp>(
    session: &mut Session<A>,
    p: ReplayParams,
) -> Result<Value, RpcError> {
    let log = match (p.log, p.path) {
        (Some(log), None) => log,
        (None, Some(path)) => std::fs::read_to_string(&path)
            .map_err(|e| RpcError::new(code::IO, format!("reading {}: {e}", path.display())))?,
        _ => {
            return Err(invalid(
                "replay takes one of log (the text) or path (a file)",
            ));
        }
    };
    json(session.replay(&log, p.until_ms)?)
}

/// `follow`'s parameters.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FollowParams {
    pub path: PathBuf,
}

pub(crate) fn follow<A: CaptureApp>(
    session: &mut Session<A>,
    p: FollowParams,
) -> Result<Value, RpcError> {
    json(session.follow(&p.path)?)
}
