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

/// A check: an audit's (a frame's) or a film's (motion).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Check {
    Contrast,
    Truncation,
    Occlusion,
    Alignment,
    Palette,
    DepthLoss,
    Profile,
    Rhythm,
    FocusVisible,
    Reachability,
    Targets,
    Landmarks,
    Tokens,
    /// Frames from an input to its first visible change (not built).
    Latency,
    /// A film's: a cell that changes and changes back within a frame.
    Flicker,
    /// A film's: a spinner's pace and width.
    Spinner,
    /// A film's: frames a link cannot carry in time.
    Cost,
    /// A film's: motion under `prefers-reduced-motion: reduce`.
    ReducedMotion,
}

impl Check {
    /// The checks an audit of a frame runs, in order.
    pub const AUDIT: [Check; 14] = [
        Check::Contrast,
        Check::Truncation,
        Check::Occlusion,
        Check::Alignment,
        Check::Palette,
        Check::DepthLoss,
        Check::Profile,
        Check::Rhythm,
        Check::FocusVisible,
        Check::Reachability,
        Check::Targets,
        Check::Landmarks,
        Check::Tokens,
        Check::Latency,
    ];

    /// Its name (`depth-loss`).
    pub fn name(self) -> &'static str {
        match self {
            Check::Contrast => "contrast",
            Check::Truncation => "truncation",
            Check::Occlusion => "occlusion",
            Check::Alignment => "alignment",
            Check::Palette => "palette",
            Check::DepthLoss => "depth-loss",
            Check::Profile => "profile",
            Check::Rhythm => "rhythm",
            Check::FocusVisible => "focus-visible",
            Check::Reachability => "reachability",
            Check::Targets => "targets",
            Check::Landmarks => "landmarks",
            Check::Tokens => "tokens",
            Check::Latency => "latency",
            Check::Flicker => "flicker",
            Check::Spinner => "spinner",
            Check::Cost => "cost",
            Check::ReducedMotion => "reduced-motion",
        }
    }

    /// The audit check `name` names.
    pub fn from_name(name: &str) -> Option<Check> {
        Check::AUDIT.into_iter().find(|c| c.name() == name)
    }
}

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
    /// The checks to run ([`Check::AUDIT`]); empty: all.
    pub checks: Vec<Check>,
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
    pub fn of(checks: &[Check]) -> Self {
        AuditRequest {
            checks: checks.to_vec(),
            ..AuditRequest::default()
        }
    }
}

/// How bad a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Severity {
    /// Worth a look: a heuristic matched.
    Info,
    /// Likely a problem: a threshold of this crate's (or the design's)
    /// failed.
    Warning,
    /// An accessibility requirement failed (WCAG 2: text contrast, a
    /// visible focus, keyboard reach).
    Error,
}

/// One thing a check found.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Finding {
    pub check: Check,
    pub severity: Severity,
    pub message: String,
    /// The elements involved, by identity.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<String>,
    /// The cells involved, as `[x, y, width, height]` rectangles (at most
    /// 16; `inputs` counts them all where it matters).
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
    pub checked: Vec<Check>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub not_checked: Vec<NotChecked>,
    /// Checks that ran on part of what they could, and what they left
    /// out (a cap on elements or colors).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub partial: Vec<NotChecked>,
    /// Most severe first, then by check.
    pub findings: Vec<Finding>,
}

/// What a check answers: its findings, or why it could not run.
pub(crate) type Checked = Result<Vec<Finding>, String>;

/// A check's findings, and what it left out, if anything.
pub(crate) struct Ran {
    pub findings: Vec<Finding>,
    pub partial: Option<String>,
}

impl From<Vec<Finding>> for Ran {
    fn from(findings: Vec<Finding>) -> Self {
        Ran {
            findings,
            partial: None,
        }
    }
}

impl<A: CaptureApp> Session<A> {
    /// Run the checks `request` asks for on the current frame. A check
    /// that cannot run (nothing to check against, a fork refused) is
    /// listed with why; the others still run.
    pub fn audit(&self, request: &AuditRequest) -> Result<Audit, SessionError> {
        // Declared tokens must be colors: a token that is none is the
        // caller's mistake, not a page's finding.
        for (name, value) in &request.design.colors {
            if rdom_style::parse_color(value).is_none() {
                return Err(SessionError::InvalidInput(format!(
                    "design token {name}: {value:?} is not a color"
                )));
            }
        }
        let profile = match &request.profile {
            None => Profile::default_profile(),
            Some(n) => Profile::named(n)
                .ok_or_else(|| SessionError::InvalidInput(format!("no profile {n:?}")))?,
        };
        let frame = Frame::of(self, &profile)?;
        let wanted = |c: Check| request.checks.is_empty() || request.checks.contains(&c);
        let mut audit = Audit {
            profile: profile.name.clone(),
            checked: Vec::new(),
            not_checked: Vec::new(),
            partial: Vec::new(),
            findings: Vec::new(),
        };
        let d = &request.design;
        // focus-visible and reachability share one Tab walk, in a fork.
        let walk = if wanted(Check::FocusVisible) || wanted(Check::Reachability) {
            Some(ux::walk(self, &frame))
        } else {
            None
        };
        for check in Check::AUDIT {
            if !wanted(check) {
                continue;
            }
            let result: Result<Ran, String> = match check {
                Check::Contrast => look::contrast(&frame, d).map(Ran::from),
                Check::Truncation => look::truncation(&frame).map(Ran::from),
                Check::Occlusion => look::occlusion(&frame).map(Ran::from),
                Check::Alignment => look::alignment(&frame),
                Check::Palette => look::palette(&frame),
                Check::DepthLoss => look::depth_loss(&frame).map(Ran::from),
                Check::Profile => look::profile(&frame).map(Ran::from),
                Check::Rhythm => look::rhythm(&frame, d),
                Check::FocusVisible => ux::focus_visible(walk.as_ref().expect("walked"), &frame),
                Check::Reachability => ux::reachability(walk.as_ref().expect("walked"), &frame),
                Check::Targets => ux::targets(&frame, d).map(Ran::from),
                Check::Landmarks => ux::landmarks(&frame).map(Ran::from),
                Check::Tokens => tokens::tokens_check(&frame, d).map(Ran::from),
                Check::Latency => Err(
                    "frames from an input to its first visible change: not built yet (a film's timelines hold them)"
                        .into(),
                ),
                _ => Err("a film's check: run film".into()),
            };
            match result {
                Ok(ran) => {
                    audit.checked.push(check);
                    audit.findings.extend(ran.findings);
                    if let Some(left) = ran.partial {
                        audit.partial.push(NotChecked {
                            check: check.name().to_string(),
                            reason: left,
                        });
                    }
                }
                Err(reason) => audit.not_checked.push(NotChecked {
                    check: check.name().to_string(),
                    reason,
                }),
            }
        }
        audit
            .findings
            .sort_by(|a, b| b.severity.cmp(&a.severity).then(a.check.cmp(&b.check)));
        Ok(audit)
    }
}
