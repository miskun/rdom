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
    let outcomes = session.whatif(&p.candidates, p.settle_ms)?;
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
