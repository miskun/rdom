//! Evaluating a media feature (Media Queries 4 §4–§7, 5 §12) against the
//! terminal's media environment, and a container size feature (CSS
//! Conditional 5 §6.5) against a query container — split from the parse
//! and the serialization (`super`).

use super::super::{MediaEnvironment, PointerAccuracy, Truth};
use super::{MediaFeature, Test, Value};
// Pixels to a column and to a row in a feature value (DIVERGENCES §2,
// C14G-PX-BREAKPOINTS).
use crate::pixels::{PX_PER_COLUMN, PX_PER_ROW};

impl MediaFeature {
    /// Evaluate against `env` (Media Queries 4 §4–§7, 5 §12), by the
    /// terminal mapping in the module doc of `media_env`.
    pub(crate) fn evaluate(&self, env: &MediaEnvironment) -> Truth {
        let p = &env.preferences;
        let vp = env.viewport;
        let size = matches!(
            self.name.as_str(),
            "width"
                | "device-width"
                | "height"
                | "device-height"
                | "aspect-ratio"
                | "device-aspect-ratio"
                | "orientation"
        );
        if size && !env.viewport_known {
            return Truth::Unknown;
        }
        match self.name.as_str() {
            "width" | "device-width" => self.range(Kind::Columns, f64::from(vp.cols)),
            "height" | "device-height" => self.range(Kind::Rows, f64::from(vp.rows)),
            "aspect-ratio" | "device-aspect-ratio" => {
                self.range(Kind::Ratio, f64::from(vp.cols) / f64::from(vp.rows))
            }
            // §6.1–§6.3: the terminal's color depth (C16G-COLOR-DEPTH); a
            // terminal with no color shows one bit — the default or not.
            "color" => self.range(Kind::Integer, f64::from(env.color_depth.bits())),
            "color-index" => self.range(Kind::Integer, f64::from(env.color_depth.index_entries())),
            "monochrome" => {
                let mono = env.color_depth == crate::color::ColorDepth::NoColor;
                self.range(Kind::Integer, if mono { 1.0 } else { 0.0 })
            }
            // §4.4: a grid device — a terminal is the spec's example.
            "grid" => self.range(Kind::Integer, 1.0),
            // A terminal has no pixel density: the concept does not exist
            // on the device, so the feature is false (§2.4).
            "resolution" => Truth::False,
            "orientation" => {
                let v = if vp.rows > vp.cols {
                    "portrait"
                } else {
                    "landscape"
                };
                self.discrete(v, &["portrait", "landscape"], None)
            }
            "update" => self.discrete("fast", &["none", "slow", "fast"], Some("none")),
            "overflow-block" => self.discrete("scroll", &["none", "scroll", "paged"], Some("none")),
            "overflow-inline" => self.discrete("scroll", &["none", "scroll"], Some("none")),
            "color-gamut" | "video-color-gamut" => {
                // `srgb` holds for a device that covers sRGB; the wider
                // gamuts do not.
                self.discrete("srgb", &["srgb", "p3", "rec2020"], None)
            }
            "dynamic-range" | "video-dynamic-range" => {
                self.discrete("standard", &["standard", "high"], None)
            }
            "environment-blending" => {
                self.discrete("opaque", &["opaque", "additive", "subtractive"], None)
            }
            "scripting" => self.discrete(
                "enabled",
                &["none", "initial-only", "enabled"],
                Some("none"),
            ),
            "display-mode" => self.discrete(
                "standalone",
                &[
                    "fullscreen",
                    "standalone",
                    "minimal-ui",
                    "browser",
                    "picture-in-picture",
                ],
                None,
            ),
            "hover" | "any-hover" => {
                let v = if p.hover { "hover" } else { "none" };
                self.discrete(v, &["none", "hover"], Some("none"))
            }
            "pointer" | "any-pointer" => {
                let v = match p.pointer {
                    PointerAccuracy::None => "none",
                    PointerAccuracy::Coarse => "coarse",
                    PointerAccuracy::Fine => "fine",
                };
                self.discrete(v, &["none", "coarse", "fine"], Some("none"))
            }
            "prefers-color-scheme" => {
                let v = match env.color_scheme {
                    crate::color::ColorScheme::Light => "light",
                    crate::color::ColorScheme::Dark => "dark",
                };
                self.discrete(v, &["light", "dark"], None)
            }
            "prefers-reduced-motion" => self.preference(p.reduced_motion),
            "prefers-reduced-transparency" => self.preference(p.reduced_transparency),
            "prefers-reduced-data" => self.preference(p.reduced_data),
            "prefers-contrast" => {
                let v = match p.contrast {
                    super::super::Contrast::NoPreference => "no-preference",
                    super::super::Contrast::More => "more",
                    super::super::Contrast::Less => "less",
                    super::super::Contrast::Custom => "custom",
                };
                self.discrete(
                    v,
                    &["no-preference", "more", "less", "custom"],
                    Some("no-preference"),
                )
            }
            "forced-colors" => {
                let v = if p.forced_colors { "active" } else { "none" };
                self.discrete(v, &["none", "active"], Some("none"))
            }
            "inverted-colors" => {
                let v = if p.inverted_colors {
                    "inverted"
                } else {
                    "none"
                };
                self.discrete(v, &["none", "inverted"], Some("none"))
            }
            _ => Truth::Unknown,
        }
    }

    /// Evaluate as a container size feature (CSS Conditional 5 §6.5)
    /// against a query container's content box: `width` / `inline-size`
    /// read `width`, `height` / `block-size` read `height`, `aspect-ratio`
    /// and `orientation` read both — `None`, an axis the container does
    /// not answer on, is unknown, as is any other feature.
    pub(crate) fn evaluate_size(&self, width: Option<f64>, height: Option<f64>) -> Truth {
        let unknown = |v: Option<f64>| v.is_none();
        match self.name.as_str() {
            "width" | "inline-size" => match width {
                Some(w) => self.range(Kind::Columns, w),
                None => Truth::Unknown,
            },
            "height" | "block-size" => match height {
                Some(h) => self.range(Kind::Rows, h),
                None => Truth::Unknown,
            },
            "aspect-ratio" | "orientation" if unknown(width) || unknown(height) => Truth::Unknown,
            "aspect-ratio" => self.range(Kind::Ratio, width.unwrap_or(0.0) / height.unwrap_or(0.0)),
            "orientation" => {
                let v = if height > width {
                    "portrait"
                } else {
                    "landscape"
                };
                self.discrete(v, &["portrait", "landscape"], None)
            }
            _ => Truth::Unknown,
        }
    }

    /// A `no-preference | reduce` preference.
    fn preference(&self, reduce: bool) -> Truth {
        let v = if reduce { "reduce" } else { "no-preference" };
        self.discrete(v, &["no-preference", "reduce"], Some("no-preference"))
    }

    /// A discrete feature whose value is `actual`, one of `values`;
    /// `falsy`: the value that is false in the boolean context (else
    /// every value is true). A range test, or a value not among
    /// `values`, is unknown.
    fn discrete(&self, actual: &str, values: &[&str], falsy: Option<&str>) -> Truth {
        match &self.test {
            Test::Boolean => Truth::from_bool(falsy != Some(actual)),
            Test::Equals(Value::Ident(v)) => {
                let v = v.to_ascii_lowercase();
                if values.contains(&v.as_str()) {
                    Truth::from_bool(v == actual)
                } else {
                    Truth::Unknown
                }
            }
            _ => Truth::Unknown,
        }
    }

    /// A range feature whose value is `actual`.
    fn range(&self, kind: Kind, actual: f64) -> Truth {
        let value = |v: &Value| -> Option<f64> {
            match (kind, v) {
                (Kind::Columns | Kind::Rows, Value::Number(n) | Value::Cells(n)) => Some(*n),
                (Kind::Columns, Value::Pixels { px, .. }) => Some(px / PX_PER_COLUMN),
                (Kind::Rows, Value::Pixels { px, .. }) => Some(px / PX_PER_ROW),
                (Kind::Ratio, Value::Ratio(a, b)) => Some(a / b),
                (Kind::Ratio, Value::Number(n)) => Some(*n),
                (Kind::Integer, Value::Number(n)) if n.fract() == 0.0 => Some(*n),
                _ => None,
            }
        };
        match &self.test {
            Test::Boolean => Truth::from_bool(actual != 0.0 && !actual.is_nan()),
            Test::Equals(v) => match value(v) {
                Some(n) => Truth::from_bool(actual == n),
                None => Truth::Unknown,
            },
            Test::Range(pairs) => {
                let mut result = Truth::True;
                for (op, v) in pairs {
                    result = result.and(match value(v) {
                        Some(n) => Truth::from_bool(op.holds(actual, n)),
                        None => Truth::Unknown,
                    });
                }
                result
            }
        }
    }
}

/// What a range feature's values are.
#[derive(Clone, Copy)]
enum Kind {
    /// Columns: a number, `ch`, or pixels at 8 a column.
    Columns,
    /// Rows: a number, `ch`, or pixels at 16 a row.
    Rows,
    /// `<ratio>` (or a number, `n / 1`).
    Ratio,
    /// `<integer>`.
    Integer,
}
