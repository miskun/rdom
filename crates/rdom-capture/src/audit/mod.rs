//! Audits (`specs/CAPTURE.md` §Audits): checks of how a frame looks and
//! works, each answering with findings — what failed, where (elements by
//! identity, cells), the measured inputs and the threshold — and saying
//! plainly what it could not check and why. Checks read the frame as a
//! terminal profile shows it, after the environment's color depth.
//!
//! - **Look**: `contrast`, `truncation`, `occlusion`,
//!   `alignment`, `palette`, `depth-loss`, `profile`, `rhythm`.
//! - **UX**: `focus-visible`, `reachability`, `targets`,
//!   `landmarks` (`latency` needs a film: phase 8).
//! - **Tokens**: colors shown that are not the design
//!   system's.
//!
//! An app may declare its design system ([`DesignSystem`]: color tokens,
//! a spacing scale, thresholds); checks use it where present and infer
//! from the page otherwise, saying so.

pub(crate) mod color;
mod frame;
pub(crate) use frame::rects;
mod look;
mod tokens;
mod ux;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::app::CaptureApp;
use crate::profile::Profile;
use crate::session::{Session, SessionError};
use frame::Frame;

/// Every check, in the order a full audit runs them.
pub const CHECKS: [&str; 13] = [
    "contrast",
    "truncation",
    "occlusion",
    "alignment",
    "palette",
    "depth-loss",
    "profile",
    "rhythm",
    "focus-visible",
    "reachability",
    "targets",
    "landmarks",
    "tokens",
];

/// What an app declares about its design, for the checks that read it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct DesignSystem {
    /// Color tokens: name and value (`"--accent": "#5f87ff"`). When none
    /// are declared, `tokens` reads the custom properties on the root
    /// element that hold a color.
    pub colors: Vec<(String, String)>,
    /// The spacing scale, in cells (`rhythm`).
    pub spacing: Vec<u16>,
    /// The least WCAG 2 contrast ratio for text (default 4.5); other
    /// glyphs (borders, marks) need 3 (WCAG 1.4.11).
    pub min_contrast: f64,
    /// The least clickable area, `[columns, rows]` (default 3 × 1).
    pub min_target: [u16; 2],
}

impl Default for DesignSystem {
    fn default() -> Self {
        DesignSystem {
            colors: Vec::new(),
            spacing: Vec::new(),
            min_contrast: 4.5,
            min_target: [3, 1],
        }
    }
}

impl DesignSystem {
    /// With color tokens, `(name, value)` each.
    pub fn with_colors(mut self, colors: Vec<(String, String)>) -> Self {
        self.colors = colors;
        self
    }

    /// With a spacing scale, in cells.
    pub fn with_spacing(mut self, spacing: Vec<u16>) -> Self {
        self.spacing = spacing;
        self
    }

    /// With the least contrast ratio for text.
    pub fn with_min_contrast(mut self, ratio: f64) -> Self {
        self.min_contrast = ratio;
        self
    }

    /// With the least clickable area, `[columns, rows]`.
    pub fn with_min_target(mut self, size: [u16; 2]) -> Self {
        self.min_target = size;
        self
    }
}

/// What to audit.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct AuditRequest {
    /// The checks to run (names from [`CHECKS`]); empty: all.
    pub checks: Vec<String>,
    /// The terminal profile the frame is seen through (default: the
    /// default profile).
    pub profile: Option<String>,
    pub design: DesignSystem,
}

impl AuditRequest {
    /// Every check, through the default profile.
    pub fn all() -> Self {
        AuditRequest::default()
    }

    /// Seen through the profile `name`.
    pub fn with_profile(mut self, name: impl Into<String>) -> Self {
        self.profile = Some(name.into());
        self
    }

    /// Against `design`.
    pub fn with_design(mut self, design: DesignSystem) -> Self {
        self.design = design;
        self
    }

    /// Only `checks`.
    pub fn of(checks: &[&str]) -> Self {
        AuditRequest {
            checks: checks.iter().map(|c| (*c).to_string()).collect(),
            ..AuditRequest::default()
        }
    }
}

/// How bad a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Severity {
    /// Worth a look.
    Info,
    /// Likely a problem.
    Warning,
    /// A problem: a threshold failed.
    Error,
}

/// One thing a check found.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Finding {
    pub check: &'static str,
    pub severity: Severity,
    pub message: String,
    /// The elements involved, by identity.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<String>,
    /// The cells involved, as `[x, y, width, height]` rectangles.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cells: Vec<[u16; 4]>,
    /// What was measured.
    #[serde(skip_serializing_if = "Value::is_null")]
    pub inputs: Value,
    /// What it was held to.
    #[serde(skip_serializing_if = "Value::is_null")]
    pub threshold: Value,
}

/// A check that did not run, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct NotChecked {
    pub check: String,
    pub reason: String,
}

/// An audit's answer.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Audit {
    pub profile: String,
    /// The checks that ran.
    pub checked: Vec<&'static str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub not_checked: Vec<NotChecked>,
    /// Most severe first, then by check.
    pub findings: Vec<Finding>,
}

/// What a check answers: its findings, or why it could not run.
pub(crate) type Checked = Result<Vec<Finding>, String>;

impl<A: CaptureApp> Session<A> {
    /// Run the checks `request` asks for on the current frame. A check
    /// that cannot run (nothing to check against, a fork refused) is
    /// listed with why; the others still run.
    pub fn audit(&self, request: &AuditRequest) -> Result<Audit, SessionError> {
        let unknown: Vec<&str> = request
            .checks
            .iter()
            .map(String::as_str)
            .filter(|c| !CHECKS.contains(c) && *c != "latency")
            .collect();
        if !unknown.is_empty() {
            return Err(SessionError::InvalidInput(format!(
                "no check {}: {}",
                unknown.join(", "),
                CHECKS.join(", ")
            )));
        }
        let profile = match &request.profile {
            None => Profile::default_profile(),
            Some(n) => Profile::named(n)
                .ok_or_else(|| SessionError::InvalidInput(format!("no profile {n:?}")))?,
        };
        let frame = Frame::of(self, &profile)?;
        let wanted = |c: &str| request.checks.is_empty() || request.checks.iter().any(|w| w == c);
        let mut audit = Audit {
            profile: profile.name.clone(),
            checked: Vec::new(),
            not_checked: Vec::new(),
            findings: Vec::new(),
        };
        for check in CHECKS {
            if !wanted(check) {
                continue;
            }
            let d = &request.design;
            let result: Checked = match check {
                "contrast" => look::contrast(&frame, d),
                "truncation" => look::truncation(&frame),
                "occlusion" => look::occlusion(&frame),
                "alignment" => look::alignment(&frame),
                "palette" => look::palette(&frame),
                "depth-loss" => look::depth_loss(&frame),
                "profile" => look::profile(&frame),
                "rhythm" => look::rhythm(&frame, d),
                "focus-visible" => ux::focus_visible(self, &frame),
                "reachability" => ux::reachability(self, &frame),
                "targets" => ux::targets(&frame, d),
                "landmarks" => ux::landmarks(&frame),
                "tokens" => tokens::tokens_check(&frame, d),
                _ => unreachable!("CHECKS lists what this match answers"),
            };
            match result {
                Ok(findings) => {
                    audit.checked.push(check);
                    audit.findings.extend(findings);
                }
                Err(reason) => audit.not_checked.push(NotChecked {
                    check: check.to_string(),
                    reason,
                }),
            }
        }
        if wanted("latency") && request.checks.iter().any(|c| c == "latency") {
            audit.not_checked.push(NotChecked {
                check: "latency".into(),
                reason: "frames from an input to its first visible change need a film (phase 8)"
                    .into(),
            });
        }
        audit
            .findings
            .sort_by(|a, b| b.severity.cmp(&a.severity).then(a.check.cmp(b.check)));
        Ok(audit)
    }
}
