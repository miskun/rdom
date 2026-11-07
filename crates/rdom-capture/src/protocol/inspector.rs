//! The inspector methods (`specs/CAPTURE.md` §Edits): `edit`, `force`,
//! `reset`, `patch`, `reload` — their parameters and the calls into the
//! session.

use rdom_tui::core_api::ForcedState;
use serde::Deserialize;
use serde_json::Value;

use super::methods::{json, params};
use super::{RpcError, code};
use crate::app::CaptureApp;
use crate::edit::EditTarget;
use crate::session::Session;

/// `edit`'s parameters: the property, its new value (`null` removes it),
/// and which rule — `node` (the winner of the property on that element),
/// `rule` (`[sheet, index]` as `explain` names it) or `selector` (a new
/// rule).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct EditParams {
    pub property: String,
    pub value: Option<String>,
    pub important: bool,
    pub node: Option<String>,
    pub rule: Option<[usize; 2]>,
    pub selector: Option<String>,
}

/// `force`'s parameters.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ForceParams {
    pub node: String,
    /// `hover`, `active`, `focus`, `focus-visible`, `focus-within`.
    pub state: String,
    #[serde(default = "yes")]
    pub on: bool,
}

fn yes() -> bool {
    true
}

/// `patch`'s parameters: `apply` writes the diffs (default: a dry run).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct PatchParams {
    pub apply: bool,
}

fn invalid(msg: impl Into<String>) -> RpcError {
    RpcError::new(code::INVALID_PARAMS, msg)
}

pub(crate) fn edit<A: CaptureApp>(
    session: &mut Session<A>,
    p: Option<Value>,
) -> Result<Value, RpcError> {
    let p: EditParams = params(p)?;
    if p.property.is_empty() {
        return Err(invalid("edit needs a property"));
    }
    if !p.property.starts_with("--")
        && !rdom_style::property_dispatch::property_names().contains(&p.property.as_str())
    {
        return Err(invalid(format!("no property {:?}", p.property)));
    }
    let target = match (p.node, p.rule, p.selector) {
        (Some(node), None, None) => EditTarget::Winner { node },
        (None, Some([sheet, index]), None) => EditTarget::Rule { sheet, index },
        (None, None, Some(selector)) => EditTarget::NewRule { selector },
        _ => return Err(invalid("edit takes exactly one of node, rule, selector")),
    };
    let (edited, change) = session.edit(&target, &p.property, p.value.as_deref(), p.important)?;
    json(serde_json::json!({ "edit": edited, "change": change }))
}

pub(crate) fn force<A: CaptureApp>(
    session: &mut Session<A>,
    p: Option<Value>,
) -> Result<Value, RpcError> {
    let p: ForceParams = params(p)?;
    let state = ForcedState::from_name(&p.state).ok_or_else(|| {
        invalid(format!(
            "no state {:?}: hover, active, focus, focus-visible, focus-within",
            p.state
        ))
    })?;
    json(session.force(&p.node, state, p.on)?)
}

pub(crate) fn reset<A: CaptureApp>(session: &mut Session<A>) -> Result<Value, RpcError> {
    json(session.reset()?)
}

pub(crate) fn patch<A: CaptureApp>(
    session: &mut Session<A>,
    p: Option<Value>,
) -> Result<Value, RpcError> {
    let p: PatchParams = params(p)?;
    json(session.patch(p.apply)?)
}

pub(crate) fn reload<A: CaptureApp>(session: &mut Session<A>) -> Result<Value, RpcError> {
    json(session.reload()?)
}
