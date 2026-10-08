//! The write-side trait [`TuiAccessorsMut`]. Implemented in `write`
//! (for `NodeMut`).

use rdom_core::NodeId;

use super::{ScrollIntoViewOptions, ScrollToOptions};
use crate::Result;

/// Write-side accessor surface paired with [`TuiAccessors`](super::TuiAccessors).
///
/// Wrong-tag setters are silent `Ok(())` no-ops — calling
/// `set_value("x")` on a `<div>` neither errors nor mutates the
/// tree. Each setter documents which tags it owns; everything
/// else short-circuits before touching the arena.
///
/// `set_value` accepts `impl Into<String>` so call sites pass `&str`,
/// `String`, or anything that converts, matching the ergonomics of
/// the browser IDL setter.
pub trait TuiAccessorsMut<'a>: crate::sealed::Sealed {
    /// Set the form-control value. Owning tags: `<input>` (any
    /// type), `<textarea>`, `<select>`.
    ///
    /// - Text-family `<input>` → writes the `value` attribute AND
    ///   reseats the text-node child via
    ///   `runtime::builtins::input::set_value`, keeping the editing
    ///   pipeline + paint in lockstep.
    /// - Other `<input>` types (`submit`, `button`, `hidden`, …) →
    ///   writes only the `value` attribute (no text child, since
    ///   the UA `::before` provides the glyph).
    /// - `<textarea>` → replaces all children with a single text
    ///   node holding `value`.
    /// - `<select>` → marks the first matching `<option>` (by value
    ///   attribute, or text content if no `value` attribute)
    ///   `selected` and clears `selected` from the rest. No match
    ///   clears every selection (matching HTMLSelectElement.value
    ///   setter).
    /// - Any other tag → silent `Ok(())` no-op.
    fn set_value(&mut self, value: impl Into<String>) -> Result<()>;

    /// Set the `[checked]` attribute presence on `<input>`. No-op
    /// on other tags. v1 collapses HTML's `checked` attribute and
    /// IDL `.checked` property into one source — flipping false
    /// removes the attribute, mirroring the runtime toggle handler.
    fn set_checked(&mut self, value: bool) -> Result<()>;

    /// Set `defaultValue` — the value a `<form>` reset restores (HTML
    /// `defaultValue = …`). For `<textarea>`, text-family `<input>`s
    /// and `<input type=range>` it replaces the recorded default and
    /// leaves the live value alone (rdom has no dirty-value flag, so
    /// every control behaves as a dirty one; DIVERGENCES). For the
    /// other `<input>` types the `value` attribute is both value and
    /// default, so it is written. No-op on other tags.
    fn set_default_value(&mut self, value: impl Into<String>) -> Result<()>;

    /// Set `defaultChecked` of a checkbox / radio — the checkedness a
    /// `<form>` reset restores (HTML `defaultChecked = …`). The live
    /// `checked` attribute is left alone (no dirty-checkedness flag;
    /// DIVERGENCES). No-op on other elements.
    fn set_default_checked(&mut self, value: bool) -> Result<()>;

    /// Set `defaultSelected` of an `<option>` — the selectedness a
    /// `<form>` reset restores (HTML `defaultSelected = …`). The live
    /// `selected` attribute is left alone (DIVERGENCES). No-op on other
    /// elements.
    fn set_default_selected(&mut self, value: bool) -> Result<()>;

    /// Set the `[indeterminate]` attribute presence on `<input>`.
    /// No-op on other tags. Browsers expose this as an IDL-only
    /// bit; v1 reflects it via the attribute so a single source
    /// drives selector matching + accessor reads + writes.
    fn set_indeterminate(&mut self, value: bool) -> Result<()>;

    /// Set the `[disabled]` attribute presence on tags with the
    /// `disabled` IDL property: `<button>`, `<input>`, `<select>`,
    /// `<textarea>`, `<option>`, `<optgroup>`, `<fieldset>`. No-op
    /// on other tags. The cascade picks up `[disabled]` changes via
    /// the existing dirty tracker, so `:disabled` (and the UA
    /// `:disabled` muting) re-resolve on next cascade.
    fn set_disabled(&mut self, value: bool) -> Result<()>;

    /// Set the `[readonly]` attribute presence on `<input>` /
    /// `<textarea>`. No-op on other tags. The editing pipeline
    /// already honors `[readonly]` — see
    /// `runtime::editing::perform`.
    fn set_read_only(&mut self, value: bool) -> Result<()>;

    /// Set the `[inert]` attribute presence. Unlike the other
    /// setters this applies to every element — `inert` is an
    /// HTMLElement-level global attribute. v1 reflects the bit
    /// but does not yet implement subtree-disabling semantics.
    fn set_inert(&mut self, value: bool) -> Result<()>;

    /// Focus this element. No-op if the element isn't focusable
    /// (matches `HTMLElement.focus()` browser semantics — "if the
    /// element is not focusable, this method does nothing"), decided
    /// against up-to-date style: on a document an `App` runs, the
    /// element's dirty style is flushed first
    /// ([`runtime::style_flush::flush_style`](crate::runtime::style_flush::flush_style)),
    /// so a panel shown by the same handler lets its input take the
    /// focus. When
    /// it does fire, runs the standard focus ceremony via
    /// [`runtime::focus::focus_node`](crate::runtime::focus::focus_node):
    /// `blur` + `focusout` on the old target, commit the new focus
    /// (which drives the `:focus` cascade), then `focus` + `focusin`
    /// on this element — and scrolls it into view (HTML's focusing steps,
    /// `nearest` on both axes within each scroll container's
    /// `scroll-padding`; under a running `App` at its next layout, where
    /// the element is shown, DIVERGENCES). [`focus_with`](Self::focus_with)
    /// with `prevent_scroll` focuses without scrolling.
    ///
    /// Lives on the mut trait taking `&mut self` because the literal
    /// shape `focus(&self, ctx: &mut TuiEventCtx<'_>)` hits a borrow-
    /// checker conflict at the call site (`ctx.dom.node(id).focus(
    /// &mut ctx)` reborrows `ctx.dom` shared via the `NodeRef`, then
    /// needs `ctx` mut). The `&mut Dom` borrow is taken once via
    /// `node_mut` and released when the method returns.
    fn focus(&mut self);

    /// [`focus`](Self::focus) with `options` (HTML `focus(options)`):
    /// `FocusOptions::new().prevent_scroll(true)` focuses without
    /// scrolling the element into view.
    fn focus_with(&mut self, options: crate::runtime::focus::FocusOptions);

    /// Blur this element. Per `HTMLElement.blur()`: only fires
    /// `blur` / `focusout` if this element is currently focused;
    /// otherwise silently no-op.
    fn blur(&mut self);

    /// Dispatch a synthetic `click` event on this element.
    ///
    /// Builds an `EventDetail::Mouse(MouseDetail { button: Left,
    /// buttons: 0, client_x: 0, client_y: 0, delta_x: 0, delta_y:
    /// 0, modifiers: default })` payload and marks the event
    /// synthetic via `Event::with_synthetic(true)`. Matches
    /// `HTMLElement.click()` — the canonical "main-button click at
    /// the origin" shape browsers synthesize.
    ///
    /// Dispatches through the standard capture → target → bubble
    /// walk, so any author + built-in listeners (toggle, button,
    /// label, …) fire. Built-in handlers gate on
    /// `Dom::is_actually_disabled` themselves, so clicking a disabled checkbox is a tree-level
    /// no-op even though the event still walks the listener chain.
    fn click(&mut self);

    /// `Element.scrollTop = n` — set the vertical scroll offset.
    /// Clamped to the legal values,
    /// [`TuiAccessors::scroll_range`](super::TuiAccessors::scroll_range)'s
    /// `y`: `0 ..= overflow`, or `-overflow ..= 0` where the scrolling
    /// area origin is the bottom edge
    /// ([`TuiAccessors::scroll_top`](super::TuiAccessors::scroll_top)).
    /// On non-scrollable elements (no scrollable content) the
    /// clamp range collapses to `[0, 0]`, so the call is a no-op
    /// — browser-faithful.
    ///
    /// Like every programmatic scroll here (CSSOM View "perform a
    /// scroll" with behavior `auto`), it animates when the element's
    /// computed `scroll-behavior` is `smooth`: the offset then moves
    /// over the following frames of the `App`
    /// (`runtime::smooth_scroll`), and reads return the intermediate
    /// position.
    fn set_scroll_top(&mut self, value: i32) -> Result<()>;

    /// `Element.scrollLeft = n` — horizontal companion to
    /// [`Self::set_scroll_top`]. Clamped to `0 ..= overflow`, or to
    /// `-overflow ..= 0` where the scrolling area origin is the right
    /// edge — an `rtl` box, a reversed flex axis
    /// ([`TuiAccessors::scroll_left`](super::TuiAccessors::scroll_left),
    /// [`TuiAccessors::scroll_range`](super::TuiAccessors::scroll_range)).
    fn set_scroll_left(&mut self, value: i32) -> Result<()>;

    /// `Element.scrollTo(x, y)` — set both axes in one call. Each
    /// value is clamped independently.
    fn scroll_to(&mut self, x: i32, y: i32) -> Result<()>;

    /// `Element.scrollBy(dx, dy)` — add the deltas to the current
    /// scroll offsets. Each axis re-clamps after the add.
    fn scroll_by(&mut self, dx: i32, dy: i32) -> Result<()>;

    /// `Element.scroll(options)` / `scrollTo(options)` — scroll to
    /// `left` / `top` (an absent axis stays put) with
    /// `options.behavior`: `Instant` jumps, `Smooth` animates, `Auto`
    /// follows the computed `scroll-behavior`.
    fn scroll_with(&mut self, options: ScrollToOptions) -> Result<()>;

    /// `Element.scrollBy(options)` — add `left` / `top` to the current
    /// offsets, with `options.behavior` as in [`Self::scroll_with`].
    fn scroll_by_with(&mut self, options: ScrollToOptions) -> Result<()>;

    /// `Element.scrollIntoView()` — the no-argument form, i.e.
    /// `{block: "start", inline: "nearest"}` with behavior `auto`
    /// ([`ScrollIntoViewOptions::new`]). See
    /// [`Self::scroll_into_view_with`].
    fn scroll_into_view(&mut self) -> Result<()>;

    /// `Element.scrollIntoView(options)` — CSSOM View §5.2: scroll
    /// every scroll container on the ancestor chain, innermost first,
    /// so this element is aligned per `options.block` (vertical) and
    /// `options.inline` (horizontal), each scroll with
    /// `options.behavior` (`auto` animates under the container's
    /// `scroll-behavior: smooth`). The legacy
    /// `scrollIntoView(alignToTop)` form is
    /// `scroll_into_view_with(align_to_top.into())`. No-op for an
    /// element that is not rendered or has no scroll container.
    fn scroll_into_view_with(&mut self, options: ScrollIntoViewOptions) -> Result<()>;

    /// CSSOM-style write handle to the element's inline
    /// `TuiStyle` — `el.style.setProperty(name, value)`,
    /// `el.style.cssText = "…"`, etc. Returns `None` for
    /// non-element nodes.
    ///
    /// The read side is [`TuiAccessors::style`](super::TuiAccessors::style). Writes through
    /// this handle update both `TuiExt::inline_style` and the
    /// `style="…"` attribute.
    fn style_mut(&mut self) -> Option<crate::cssom::StyleDeclarationMut<'_>>;

    // ── Per-tag setters — <details>/<dialog> (step 30c) ──────────

    /// Set the `<details>` `[open]` attribute presence. No-op on
    /// other tags (silent `Ok(())` per §3.4.1). Toggling does
    /// NOT fire the `toggle` event — use the runtime path
    /// (`details::toggle`) when you want the event.
    fn set_details_open(&mut self, value: bool) -> Result<()>;

    /// Direct assignment to `<dialog>` `returnValue` — the
    /// IDL `dialog.returnValue = "x"` setter. Does NOT close
    /// the dialog or fire events; just stores the value for
    /// subsequent reads. No-op on other tags.
    fn set_dialog_return_value(&mut self, value: impl Into<String>) -> Result<()>;

    // ── Per-tag setters — `<form>` (step 31) ─────────────────────

    /// `<form>.requestSubmit(submitter?)` (HTML §4.10.3) — runs the
    /// form submission algorithm, the same path a submit-button click
    /// and implicit submission take (`form::submit`): interactive
    /// constraint validation unless `novalidate` / the submitter's
    /// `formnovalidate` applies (an invalid control stops it with
    /// `SubmitOutcome::Invalid`), a cancelable `submit` event with
    /// `EventDetail::Submit(Dom::submit_detail(form, submitter))`, then,
    /// unless canceled, the method-`dialog` close.
    /// `submitter` is the submit button to report (`None` submits from
    /// the form itself). A form that is not connected does nothing
    /// (`SubmitOutcome::Disconnected`), and a call from one of the
    /// form's own `invalid` / `submit` listeners, while that submission
    /// is firing its events, does nothing
    /// (`SubmitOutcome::AlreadySubmitting`).
    ///
    /// Errors, as the web throws them:
    /// - `DomError::Type` — `submitter` is not a submit button;
    /// - `DomError::NotFound` — `submitter`'s form owner is not this
    ///   form (the web's `NotFoundError`).
    ///
    /// Returns what happened ([`SubmitOutcome`](crate::SubmitOutcome));
    /// on a non-`<form>` element it does nothing and returns
    /// `Ok(SubmitOutcome::NotAForm)`.
    fn form_request_submit(
        &mut self,
        submitter: Option<NodeId>,
    ) -> Result<crate::runtime::builtins::form::SubmitOutcome>;

    // ── Constraint validation (HTML §4.10.20, P7-VALIDATION-1) ───

    /// `checkValidity()`. On a control: when it is a candidate that
    /// does not satisfy its constraints, fire a cancelable,
    /// non-bubbling `invalid` event at it and return `false`; else
    /// `true`. On a `<form>`: fire `invalid` at every such control the
    /// form owns, in tree order, and return whether there were none.
    /// `true` on any other element.
    fn check_validity(&mut self) -> bool;

    /// `reportValidity()`: [`check_validity`](Self::check_validity),
    /// then report the problem. Browsers show a bubble with the
    /// validation message; rdom has none, so it focuses the invalid
    /// control (on a `<form>`, the first one) whose `invalid` event was
    /// not canceled.
    fn report_validity(&mut self) -> bool;

    /// `setCustomValidity(message)`: a non-empty `message` makes the
    /// control suffer from a custom error (`validity().custom_error`,
    /// `validation_message()` = `message`); `""` clears it. Owning tags
    /// as for [`TuiAccessors::validity`](super::TuiAccessors::validity); elsewhere a silent `Ok(())`.
    fn set_custom_validity(&mut self, message: &str) -> Result<()>;
}
