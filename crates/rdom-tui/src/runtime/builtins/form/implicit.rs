//! Enter in a single-line text field: the field's commit, then implicit
//! submission (HTML §4.10.21.2).
//!
//! HTML leaves when the user *commits* a text control to the user agent
//! (§4.10.5.5); every engine commits a single-line field on Enter — the
//! `change` event and the control's user validity — before the implicit
//! submission the same key starts. Enter in a field whose form owner has
//! a default button activates that button (it submits with itself as
//! the submitter); a disabled one blocks the submission; a form with no
//! submit button submits from itself only when the field is its one field
//! that blocks implicit submission (the text-like, date / time and number
//! inputs).

use rdom_core::{ListenerOptions, NodeId};

use super::{default_button, submit};
use crate::TuiDom;

/// Install the root-level `keydown` listener.
pub(super) fn install(dom: &mut TuiDom) {
    let root = dom.root();
    dom.add_event_listener(root, "keydown", ListenerOptions::default(), move |ctx| {
        if ctx.event.default_prevented() {
            return;
        }
        let Some(focused) = ctx.dom.focused() else {
            return;
        };
        let Some(key) = ctx.event.detail.as_keyboard() else {
            return;
        };
        let no_mods = !key.modifiers.ctrl
            && !key.modifiers.shift
            && !key.modifiers.alt
            && !key.modifiers.meta;
        if !no_mods || key.key != "Enter" {
            return;
        }
        if !blocks_implicit_submission(ctx.dom, focused) {
            return;
        }
        // The commit: `change` (if the user edited the value since the
        // last one) and user validity, form or not.
        crate::runtime::builtins::form_state::commit_pending_change(ctx.dom, focused);
        // A `change` listener may have taken the field out.
        if !ctx.dom.contains(focused) {
            return;
        }
        let Some(form) = ctx.dom.form_owner(focused) else {
            return;
        };
        if let Some(default) = default_button(ctx.dom, form) {
            // The default button's activation behavior does the
            // submitting (and the `method="dialog"` close) through the
            // click listener, with it as the submitter. A disabled
            // default button blocks implicit submission.
            if !ctx.dom.is_actually_disabled(default) {
                use crate::accessors::TuiAccessorsMut;
                ctx.dom.node_mut(default).click();
            }
            return;
        }
        // No submit button: submit from the form itself, only when the
        // focused input is the one field that blocks implicit
        // submission.
        if count_text_inputs(ctx.dom, form) != 1 {
            return;
        }
        submit(ctx.dom, form, None);
    })
    .expect("form implicit-enter submit listener install");
}

/// HTML §4.10.21.2's "field that blocks implicit submission": an
/// `<input>` in the Text, Search, Telephone, URL, Email, Password, Date,
/// Month, Week, Time, Local Date and Time or Number state — the
/// single-line fields Enter submits from.
fn blocks_implicit_submission(dom: &TuiDom, id: NodeId) -> bool {
    use rdom_core::InputTypeState as T;
    matches!(
        dom.input_type_state(id),
        Some(
            T::Text
                | T::Search
                | T::Tel
                | T::Url
                | T::Email
                | T::Password
                | T::Date
                | T::Month
                | T::Week
                | T::Time
                | T::DateTimeLocal
                | T::Number
        )
    )
}

/// How many of the form's owned fields block implicit submission.
fn count_text_inputs(dom: &TuiDom, form: NodeId) -> usize {
    dom.form_listed_elements(form)
        .into_iter()
        .filter(|&id| blocks_implicit_submission(dom, id))
        .count()
}
