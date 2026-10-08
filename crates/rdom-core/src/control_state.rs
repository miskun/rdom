//! Form-control state the backend holds and the input pseudo-classes
//! read (Selectors 4 §14, HTML §4.16.3): the defaults a control keeps
//! beside its live state, and whether its value lies within its range
//! limitations. Values, their parsing and the defaults live in the
//! backend (rdom-tui's form builtins), so the substrate asks through a
//! [`ControlStateHook`] — a plain `fn`, as the validity hook is, so
//! matching (`&self`) can call it.

use crate::dom::Dom;
use crate::node_id::NodeId;

/// A question about one control's state the substrate asks its backend
/// ([`Dom::set_control_state_hook`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ControlState {
    /// A checkbox's or radio's `defaultChecked` — the checkedness its
    /// `checked` content attribute gave it (`:default`). Without a hook:
    /// whether it has a `checked` attribute.
    DefaultChecked,
    /// An `<option>`'s `defaultSelected` — the selectedness its
    /// `selected` content attribute gave it (`:default`). Without a hook:
    /// whether it has a `selected` attribute.
    DefaultSelected,
    /// The control *has range limitations* (HTML §4.16.3: a `min` or
    /// `max` that applies, or a default one — `type=range`). Without a
    /// hook: never.
    RangeLimited,
    /// The control *suffers from an underflow or an overflow* (HTML
    /// §4.10.20.1 `rangeUnderflow` / `rangeOverflow`). Without a hook:
    /// never.
    OutOfRange,
    /// The control's *user validity* (HTML §4.10.18.1): set when the
    /// user commits a change to it or its form's submission is
    /// attempted, cleared by a form reset (`:user-valid` /
    /// `:user-invalid`). Without a hook: never.
    UserValidity,
}

/// A backend's answer to a [`ControlState`] question about control `id`.
/// Installed with [`Dom::set_control_state_hook`].
pub type ControlStateHook<Ext> = fn(&Dom<Ext>, NodeId, ControlState) -> bool;

/// Storage for the hook with a `Debug` impl.
pub(crate) struct ControlStateSlot<Ext: 'static>(pub(crate) Option<ControlStateHook<Ext>>);

impl<Ext: 'static> std::fmt::Debug for ControlStateSlot<Ext> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if self.0.is_some() {
            "ControlStateSlot(Some(hook))"
        } else {
            "ControlStateSlot(None)"
        })
    }
}

impl<Ext: 'static> Dom<Ext> {
    /// Install (or remove, with `None`) the backend's answers to
    /// [`ControlState`] questions, behind `:default`, `:in-range`,
    /// `:out-of-range`, `:user-valid` and `:user-invalid`. Without one, each question has the answer its
    /// variant documents: the defaults are the content attributes and no
    /// control has range limitations. rdom-tui's `App` installs its hook
    /// at construction; a bare `TuiDom` cascaded without an `App` calls
    /// `rdom_tui::runtime::builtins::validation::install`.
    pub fn set_control_state_hook(&mut self, hook: Option<ControlStateHook<Ext>>) {
        self.control_state_hook = ControlStateSlot(hook);
    }
}

impl<Ext> Dom<Ext> {
    /// The backend's answer to `state` for `id`, or the substrate's
    /// default without a hook (see each [`ControlState`] variant).
    pub fn control_state(&self, id: NodeId, state: ControlState) -> bool {
        if let Some(hook) = self.control_state_hook.0 {
            return hook(self, id, state);
        }
        match state {
            ControlState::DefaultChecked => self.has_attribute(id, "checked"),
            ControlState::DefaultSelected => self.has_attribute(id, "selected"),
            ControlState::RangeLimited | ControlState::OutOfRange | ControlState::UserValidity => {
                false
            }
        }
    }
}
