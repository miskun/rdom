//! `matrix` (`specs/CAPTURE.md` §Variants): this session's steps replayed
//! across a cross product of environments — viewport sizes, schemes,
//! color depths, media preferences — each a fork of its own, and answered
//! as one frame per variant, ready for a contact sheet
//! (`raster::sheet::sheet`).
//!
//! An axis is a list of partial environments (`{"color_scheme": "light"}`,
//! `{"viewport": {"width": 60, "height": 20}}`), so every environment
//! field is an axis without a list of axes to keep: a variant is the
//! session's environment with one value of each axis laid over it.

use serde::Serialize;
use serde_json::Value;

use crate::app::CaptureApp;
use crate::capture::Grid;
use crate::env::Environment;
use crate::session::{CaptureRequest, Session, SessionError};

/// The most variants one `matrix` makes.
pub const MAX_VARIANTS: usize = 64;

/// One variant: what it laid over the environment, and its frame.
#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct Variant {
    /// What it changed, `key=value` each (`color_scheme=light`).
    pub label: String,
    pub env: Environment,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grid_hash: Option<String>,
    /// Why it could not be shown (an environment the app cannot run in,
    /// a step refused there).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip)]
    pub grid: Option<Grid>,
}

/// `patch` laid over `base`: an object's fields over the base's,
/// recursively; any other value replaces it.
fn lay(base: &mut Value, patch: &Value) {
    match (base, patch) {
        (Value::Object(b), Value::Object(p)) => {
            for (k, v) in p {
                lay(b.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (b, p) => *b = p.clone(),
    }
}

/// `patch` as `key=value` pairs, nested keys dotted.
fn label_of(patch: &Value, prefix: &str, out: &mut Vec<String>) {
    match patch {
        Value::Object(m) => {
            for (k, v) in m {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                label_of(v, &key, out);
            }
        }
        Value::String(s) => out.push(format!("{prefix}={s}")),
        v => out.push(format!("{prefix}={v}")),
    }
}

impl<A: CaptureApp> Session<A> {
    /// This session's steps replayed in every combination of `axes` (each
    /// a list of partial environments laid over the session's), each run
    /// `settle_ms`. A variant that cannot run says why; the others are
    /// still made. The session is left as it was.
    pub fn matrix(
        &self,
        axes: &[Vec<Value>],
        settle_ms: u64,
    ) -> Result<Vec<Variant>, SessionError> {
        let invalid = |m: String| SessionError::InvalidInput(m);
        let mut combos: Vec<Vec<&Value>> = vec![Vec::new()];
        for axis in axes {
            if axis.is_empty() {
                return Err(invalid("an axis has no values".into()));
            }
            if let Some(v) = axis.iter().find(|v| !v.is_object()) {
                return Err(invalid(format!(
                    "an axis value is a partial environment (an object), not {v}"
                )));
            }
            combos = combos
                .into_iter()
                .flat_map(|c| axis.iter().map(move |v| [c.clone(), vec![v]].concat()))
                .collect();
            if combos.len() > MAX_VARIANTS {
                return Err(invalid(format!(
                    "more than {MAX_VARIANTS} variants: fewer axes or values"
                )));
            }
        }
        let base =
            serde_json::to_value(self.opened_environment()?).expect("an environment serializes");
        Ok(combos
            .into_iter()
            .map(|patches| {
                let mut env = base.clone();
                let mut label = Vec::new();
                for p in &patches {
                    lay(&mut env, p);
                    label_of(p, "", &mut label);
                }
                let label = label.join(" ");
                let made = (|| -> Result<(Environment, Grid, String), String> {
                    let env: Environment = serde_json::from_value(env)
                        .map_err(|e| format!("not an environment: {e}"))?;
                    let mut fork = self.fork_in(env.clone()).map_err(|e| e.to_string())?;
                    fork.advance(settle_ms).map_err(|e| e.to_string())?;
                    let c = fork
                        .capture(&CaptureRequest::with_layers([crate::capture::Layer::Grid]))
                        .map_err(|e| e.to_string())?;
                    Ok((env, c.grid.expect("asked for"), c.meta.grid_hash))
                })();
                match made {
                    Ok((env, grid, hash)) => Variant {
                        label,
                        env,
                        grid_hash: Some(hash),
                        error: None,
                        grid: Some(grid),
                    },
                    Err(e) => Variant {
                        label,
                        env: Environment::default(),
                        grid_hash: None,
                        error: Some(e),
                        grid: None,
                    },
                }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{label_of, lay};

    #[test]
    fn a_partial_environment_is_laid_over_the_base() {
        let mut base = json!({"viewport": {"width": 80, "height": 24}, "color_scheme": "dark"});
        lay(&mut base, &json!({"viewport": {"width": 40}}));
        assert_eq!(
            base,
            json!({"viewport": {"width": 40, "height": 24}, "color_scheme": "dark"})
        );
        let mut label = Vec::new();
        label_of(
            &json!({"viewport": {"width": 40}, "reduced_motion": true}),
            "",
            &mut label,
        );
        // Keys in a partial are in serde_json's order: sorted.
        assert_eq!(label, ["reduced_motion=true", "viewport.width=40"]);
    }
}
