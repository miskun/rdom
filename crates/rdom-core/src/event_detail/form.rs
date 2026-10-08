//! The form and editing event payloads: `beforeinput` / `input`
//! ([`InputDetail`]) and `submit` ([`SubmitDetail`], with the
//! submission's [`FormMethod`] and [`FormEnctype`]).

use super::InputType;
use crate::NodeId;

/// `beforeinput` / `input` event payload, per UI Events / Input
/// Events Level 2.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct InputDetail {
    /// What kind of edit produced this event. See [`InputType`].
    pub input_type: InputType,
    /// Text being inserted, or `None` for deletion-style events.
    pub data: Option<String>,
    /// `true` if this event fires as part of an IME composition
    /// sequence. rdom doesn't model IME directly; always `false`
    /// in M4. Reserved for future polish.
    pub is_composing: bool,
}

impl InputDetail {
    /// An input event payload (`InputEventInit`) outside a composition
    /// (`is_composing` false).
    pub fn new(input_type: InputType, data: Option<String>) -> Self {
        Self {
            input_type,
            data,
            is_composing: false,
        }
    }
}

/// `submit` event payload, per HTML §4.10.21.3 form submission.
///
/// rdom has no navigation, so nothing is submitted *to* anything: the
/// payload carries what a browser would have submitted with — the
/// submitter and the effective `action` / `method` / `enctype` /
/// `target` / no-validate state, each the submitter's `form*` override
/// when it has one, else the form's attribute (HTML §4.10.19.6). Build
/// it with [`Dom::submit_detail`](crate::Dom::submit_detail).
///
/// `#[non_exhaustive]`: construct with [`SubmitDetail::new`] or
/// `Dom::submit_detail`, then set fields.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct SubmitDetail {
    /// The element that triggered submission (the `<button>` /
    /// `<input type="submit">` that was activated, or the form's default
    /// button on implicit submission), or `None` when the form itself
    /// submitted: implicit submission from a form with no submit button,
    /// or `form.requestSubmit()` without a submitter.
    pub submitter: Option<NodeId>,
    /// The submitter's `formaction`, else the form's `action` attribute,
    /// else `""`. The raw attribute value: rdom has no document URL to
    /// resolve it against (DIVERGENCES).
    pub action: String,
    /// The submitter's `formmethod`, else the form's `method` (missing
    /// and invalid values are [`FormMethod::Get`]).
    pub method: FormMethod,
    /// The submitter's `formenctype`, else the form's `enctype` (missing
    /// and invalid values are [`FormEnctype::UrlEncoded`]).
    pub enctype: FormEnctype,
    /// The submitter's `formtarget`, else the form's `target`, else `""`.
    pub target: String,
    /// The submitter's no-validate state: it has `formnovalidate`, or the
    /// form has `novalidate`. Constraint validation is skipped when set.
    pub no_validate: bool,
}

impl SubmitDetail {
    /// A payload for `submitter` with every other field at its HTML
    /// default: empty `action` / `target`, [`FormMethod::Get`],
    /// [`FormEnctype::UrlEncoded`], validation on. Prefer
    /// [`Dom::submit_detail`](crate::Dom::submit_detail), which reads the
    /// form and submitter attributes.
    pub fn new(submitter: Option<NodeId>) -> Self {
        Self {
            submitter,
            action: String::new(),
            method: FormMethod::Get,
            enctype: FormEnctype::UrlEncoded,
            target: String::new(),
            no_validate: false,
        }
    }
}

/// A form's submission method — the `method` / `formmethod` enumerated
/// attribute (HTML §4.10.19.6). Keywords are ASCII case-insensitive;
/// the missing and invalid value default is `Get`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum FormMethod {
    /// `get` — the default.
    #[default]
    Get,
    /// `post`.
    Post,
    /// `dialog` — submission closes the form's nearest ancestor
    /// `<dialog>` (HTML §4.10.21.3 step "method is dialog").
    Dialog,
}

impl FormMethod {
    /// Parse an attribute value; anything but `post` / `dialog` (ASCII
    /// case-insensitive) is `Get`.
    pub fn from_attribute(value: &str) -> Self {
        if value.eq_ignore_ascii_case("post") {
            Self::Post
        } else if value.eq_ignore_ascii_case("dialog") {
            Self::Dialog
        } else {
            Self::Get
        }
    }

    /// The canonical keyword (`"get"`, `"post"`, `"dialog"`) — the
    /// `form.method` IDL value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "get",
            Self::Post => "post",
            Self::Dialog => "dialog",
        }
    }
}

/// A form's entry-list encoding — the `enctype` / `formenctype`
/// enumerated attribute (HTML §4.10.19.6). Keywords are ASCII
/// case-insensitive; the missing and invalid value default is
/// `UrlEncoded`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum FormEnctype {
    /// `application/x-www-form-urlencoded` — the default.
    #[default]
    UrlEncoded,
    /// `multipart/form-data`.
    MultipartFormData,
    /// `text/plain`.
    TextPlain,
}

impl FormEnctype {
    /// Parse an attribute value; unknown values are `UrlEncoded`.
    pub fn from_attribute(value: &str) -> Self {
        if value.eq_ignore_ascii_case("multipart/form-data") {
            Self::MultipartFormData
        } else if value.eq_ignore_ascii_case("text/plain") {
            Self::TextPlain
        } else {
            Self::UrlEncoded
        }
    }

    /// The canonical MIME type — the `form.enctype` IDL value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UrlEncoded => "application/x-www-form-urlencoded",
            Self::MultipartFormData => "multipart/form-data",
            Self::TextPlain => "text/plain",
        }
    }
}
