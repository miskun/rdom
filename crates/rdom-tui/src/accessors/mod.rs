//! `TuiAccessors` / `TuiAccessorsMut` extension traits — author-
//! facing IDL-style accessor methods on `NodeRef<'a, TuiExt>` (read
//! side) and `NodeMut<'a, TuiExt>` (write side).
//!
//! Substrate boundary: accessors that need TUI-only state —
//! runtime focus rules, builtin form helpers, computed cascade,
//! layout rects — live here rather than in `rdom-core`. Pure-DOM
//! accessors stay on the rdom-core wrappers.
//!
//! ## Wrong-tag policy
//!
//! Setters are gated by the tags that own each IDL property in HTML.
//! Calls on a non-owning tag (e.g. `set_value("foo")` on a `<div>`)
//! are silent `Ok(())` no-ops — browser-faithful, since JS assignment
//! to a missing IDL property doesn't throw. The wrong-tag case stays
//! loud on the read side: `value()` returns `None` for a `<div>`.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use rdom_tui::{TuiAccessors, TuiAccessorsMut, TuiDom};
//!
//! let mut dom: TuiDom = TuiDom::new();
//! // ... build a form ...
//! let mut el = dom.node_mut(input_id);
//! let current = el.value();      // read works on NodeMut too (delegates to as_ref)
//! el.set_value("hello")?;        // write — single-block read-then-mutate
//! ```
//!
//! `value()` is the "smart" accessor: dispatches on tag at runtime so
//! `<input>`, `<textarea>`, and `<select>` all reply through one call
//! site. The narrow, tag-prefixed variants (`input_value`,
//! `select_value`, etc.) ship in step 30.

pub mod doc;
mod helpers;
mod read_mut;
mod read_ref;
mod write;

#[cfg(test)]
mod tests;

pub use doc::TuiDocAccessors;

use rdom_core::NodeId;

use crate::Result;

/// Read-side accessor surface for `<el>.{value, checked, …}()`-style
/// calls.
///
/// Implemented for both `NodeRef<'a, TuiExt>` and `NodeMut<'a,
/// TuiExt>`. Having it on `NodeMut` too is what makes the
/// read-then-mutate pattern compile in a single block — `value()`
/// returns owned data, so the immutable borrow ends before the
/// follow-up `set_value()` takes its mutable borrow.
pub trait TuiAccessors<'a> {
    /// Smart form-control value getter. Returns:
    ///
    /// - `<input>` (any type) → the live editing value, mirrored
    ///   from the text-node child seeded by
    ///   `runtime::builtins::input::seed_all`. For button-family
    ///   inputs (`submit`, `reset`, `button`, `hidden`) the seed
    ///   leaves the text child empty, so this returns `""` and
    ///   callers wanting the submit string should read the
    ///   `value` attribute directly.
    /// - `<textarea>` → concatenated descendant text content.
    /// - `<select>` → the selected option's value (or space-joined
    ///   list for `multiple`); empty when nothing is selected.
    /// - other tags → `None`.
    fn value(&self) -> Option<String>;

    /// `[checked]` attribute presence — checkbox / radio state.
    /// False on every other element (and on form controls without
    /// the attribute).
    fn checked(&self) -> bool;

    /// `[indeterminate]` attribute presence — used by the
    /// `:indeterminate` pseudo-class. Browsers expose this as an
    /// IDL-only bit; v1 reflects it via attribute presence so a
    /// single source drives both selector matching and accessor
    /// reads.
    fn indeterminate(&self) -> bool;

    /// `[disabled]` attribute presence.
    fn disabled(&self) -> bool;

    /// `[readonly]` attribute presence. Name is `read_only` (snake)
    /// to match Rust convention; HTML attribute is `readonly`.
    fn read_only(&self) -> bool;

    /// `[inert]` attribute presence. v1 surfaces the bit but does
    /// not yet implement the subtree-disabling semantics (modal
    /// dialog focus trap is deferred).
    fn inert(&self) -> bool;

    /// `HTMLElement.isContentEditable` — effective value with
    /// `contenteditable="inherit"` semantics. Walks ancestors until
    /// it finds an explicit `true`/`""`/`"plaintext-only"` (returns
    /// true) or `"false"` (returns false); falls off the root as
    /// false.
    ///
    /// Distinct from `TuiNodeExt::is_editable`, which is the editing
    /// pipeline's broader notion (also true for native `<input>` /
    /// `<textarea>`).
    fn is_content_editable(&self) -> bool;

    /// Effective tabindex honoring HTML's implicit-focusability
    /// rules (per `runtime::focus::tabindex`):
    ///
    /// - `[disabled]` → never focusable, returns `None`.
    /// - explicit `[tabindex]` → that value.
    /// - implicit focusable tag (`<button>`, `<input>` (non-hidden),
    ///   `<textarea>`, `<select>`, `<summary>`, `<a[href]>`,
    ///   `<area[href]>`) → 0.
    /// - otherwise → `None`.
    ///
    /// Distinct from `NodeRef::tab_index()` (step 19), which returns
    /// only the raw attribute value parsed as `i32`.
    fn effective_tab_index(&self) -> Option<i32>;

    /// `Element.getBoundingClientRect()` — the post-layout, post-
    /// scroll rect this element occupies in its parent's coordinate
    /// space. Returns `None` for non-element nodes (text, comment).
    ///
    /// **Divergence from DOM:** browsers return `DOMRect` with f64
    /// fields; rdom returns [`LayoutRect`] (i32 + u16) because the
    /// substrate is cell-grained. `DomRect` is re-exported below as
    /// a type alias for spec-name parity.
    fn bounding_rect(&self) -> Option<DomRect>;

    /// `Element.scrollTop` — vertical scroll offset in cells.
    /// `None` for non-element nodes; `0` for non-scrollable elements
    /// (browser-faithful: `el.scrollTop` always returns a number;
    /// for non-scrollable elements that number is `0`).
    fn scroll_top(&self) -> Option<i32>;

    /// `Element.scrollLeft` — horizontal scroll offset in cells.
    fn scroll_left(&self) -> Option<i32>;

    /// `Element.scrollWidth` — total content width tracked by the
    /// layout pass for scrollbar sizing. Reports the scrollable
    /// extent, not the viewport.
    fn scroll_width(&self) -> Option<i32>;

    /// `Element.scrollHeight` — total content height; companion to
    /// [`Self::scroll_width`].
    fn scroll_height(&self) -> Option<i32>;

    /// CSSOM-style read view of the element's inline `TuiStyle`
    /// — `el.style.getPropertyValue("color")` and friends, plus
    /// `length` / `item` / `cssText` enumeration. Returns `None`
    /// for non-element nodes.
    ///
    /// Returns an **owned snapshot** of the inline style (clone
    /// of `TuiExt::inline_style`). Re-fetch via `style()` after
    /// mutation to observe new values. The write side is
    /// [`TuiAccessorsMut::style_mut`].
    fn style(&self) -> Option<crate::cssom::StyleDeclaration>;

    // ── Per-tag accessors — `<input>` + `<textarea>` (step 30a) ──
    //
    // Tag-prefixed narrow variants per spec §4.4. Smart
    // counterparts (`value()`, `disabled()`, etc.) already
    // dispatch on tag; these return `Option<T>` with `None` on
    // wrong tag.

    /// Live editing value of an `<input>` — mirror of the
    /// text-node child seeded by
    /// `runtime::builtins::input::seed_all`. `None` for
    /// non-`<input>` elements (including non-element nodes).
    /// Narrow variant of [`Self::value`].
    fn input_value(&self) -> Option<String>;

    /// `<input>` `type` attribute. `None` for non-`<input>`
    /// elements. Returns `Some("text".to_string())` when the
    /// attribute is absent on an `<input>` — HTML's default
    /// type. Other names pass through verbatim.
    fn input_type(&self) -> Option<String>;

    /// `<input>` `name` attribute. `None` for non-`<input>` or
    /// when the attribute is absent.
    fn input_name(&self) -> Option<String>;

    /// `<input>` `placeholder` attribute. `None` for non-
    /// `<input>` or when the attribute is absent.
    fn input_placeholder(&self) -> Option<String>;

    /// `NodeId` of the nearest `<form>` ancestor (or `self` if
    /// already a `<form>`) for an `<input>` element. Returns
    /// `None` when:
    /// - this isn't an `<input>` element, or
    /// - no `<form>` ancestor exists.
    ///
    /// Returns `NodeId` rather than `NodeRef` to avoid
    /// lifetime entanglement with the source `NodeRef`/`NodeMut`
    /// temporary — callers retrieve the form node via
    /// `dom.node(form_id)` themselves.
    fn input_form(&self) -> Option<NodeId>;

    /// Text content of a `<textarea>` — the value used for form
    /// submission and the `:placeholder-shown` selector. `None`
    /// for non-`<textarea>` elements. Narrow variant of
    /// [`Self::value`].
    fn textarea_value(&self) -> Option<String>;

    /// `<textarea>` `name` attribute. `None` for non-
    /// `<textarea>` or when the attribute is absent.
    fn textarea_name(&self) -> Option<String>;

    /// `NodeId` of the nearest `<form>` ancestor for a
    /// `<textarea>` element. Same shape as
    /// [`Self::input_form`].
    fn textarea_form(&self) -> Option<NodeId>;

    // ── Per-tag accessors — `<select>` + `<option>` (step 30b) ───

    /// Value of a `<select>` — single-select returns the
    /// selected option's value (empty when nothing is
    /// selected); multi-select returns space-joined values.
    /// `None` for non-`<select>` elements. Narrow variant of
    /// [`Self::value`].
    fn select_value(&self) -> Option<String>;

    /// All `<option>` descendants of a `<select>` in document
    /// order (descends into `<optgroup>`). `None` for
    /// non-`<select>` elements; `Some(vec![])` when the select
    /// has no options.
    fn select_options(&self) -> Option<Vec<NodeId>>;

    /// `<option>` descendants whose `selected` attribute is
    /// present. `None` for non-`<select>` elements.
    fn select_selected_options(&self) -> Option<Vec<NodeId>>;

    /// Index of the first selected `<option>` in
    /// [`Self::select_options`] order. Returns `Some(-1)` (the
    /// browser `HTMLSelectElement.selectedIndex` sentinel) when
    /// no option carries `[selected]`. `None` for non-
    /// `<select>` elements.
    fn select_selected_index(&self) -> Option<i32>;

    /// Nearest `<form>` ancestor of a `<select>`. Same shape
    /// as [`Self::input_form`].
    fn select_form(&self) -> Option<NodeId>;

    /// Submit value of an `<option>` — the `value` attribute,
    /// falling back to the option's text content per HTML spec
    /// when the attribute is absent. `None` for non-`<option>`
    /// elements.
    fn option_value(&self) -> Option<String>;

    /// Display label of an `<option>` — the `label` attribute,
    /// falling back to the option's text content per HTML spec
    /// when the attribute is absent. `None` for non-`<option>`
    /// elements.
    fn option_label(&self) -> Option<String>;

    /// `true` iff this is an `<option>` element with the
    /// `[selected]` attribute present. `false` for any other
    /// tag — matches the spec §4.4 convention for `bool`-typed
    /// narrow accessors (e.g. `details_open`).
    fn option_selected(&self) -> bool;

    // ── Per-tag accessors — <details>/<dialog>/<button>/<label> ──
    //                                              (step 30c)

    /// `true` iff this is a `<details>` element with the `[open]`
    /// attribute present. `false` on any other tag (per spec §4.4
    /// convention for `bool` narrow accessors).
    fn details_open(&self) -> bool;

    /// `true` iff this is a `<dialog>` element with the `[open]`
    /// attribute present. `false` on any other tag.
    fn dialog_open(&self) -> bool;

    /// `<dialog>` `returnValue` — the string last passed to
    /// `close(value)`, or `""` when the dialog has never been
    /// closed. `None` for non-`<dialog>` elements.
    fn dialog_return_value(&self) -> Option<String>;

    /// Nearest `<form>` ancestor of a `<button>`. Same shape as
    /// [`Self::input_form`].
    fn button_form(&self) -> Option<NodeId>;

    /// `<label>` `for` attribute (the id of the labeled
    /// control). Rust-renamed from `for` to dodge the keyword
    /// clash; in JS this is `label.htmlFor`. `None` for
    /// non-`<label>` elements or when the attribute is absent.
    fn label_html_for(&self) -> Option<String>;

    /// `<label>` `control` — the form-control element this label
    /// associates with. Resolves the explicit `[for="id"]` first
    /// then falls back to the first labelable descendant per
    /// HTML spec. Returns `Option<NodeId>`; `None` for
    /// non-`<label>` or unresolvable. Same id-not-NodeRef shape
    /// as the other `*_form` accessors.
    fn label_control(&self) -> Option<NodeId>;

    // ── Per-tag accessors — `<progress>` + `<meter>` (step 30d) ──
    //
    // All return `Option<f64>`: `None` for wrong tag, `Some(v)`
    // otherwise. Values are the IDL-effective numbers: parsed
    // from the corresponding attribute, or the HTML-spec default
    // when absent (e.g. `progress.max` defaults to `1.0`,
    // `meter.optimum` defaults to `(min+max)/2`).

    /// `<progress>` current value. `None` for non-`<progress>`
    /// elements. When the `value` attribute is absent (the
    /// indeterminate progress case — also matched by
    /// `:indeterminate`) returns `Some(0.0)` per IDL spec.
    fn progress_value(&self) -> Option<f64>;

    /// `<progress>` `max`. Defaults to `1.0` when the attribute
    /// is absent (HTML spec).
    fn progress_max(&self) -> Option<f64>;

    /// `<meter>` `value`. Defaults to `0.0` when absent.
    fn meter_value(&self) -> Option<f64>;

    /// `<meter>` `min`. Defaults to `0.0` when absent.
    fn meter_min(&self) -> Option<f64>;

    /// `<meter>` `max`. Defaults to `1.0` when absent.
    fn meter_max(&self) -> Option<f64>;

    /// `<meter>` `low`. Defaults to [`Self::meter_min`] when the
    /// attribute is absent (per HTML — the "low" boundary
    /// collapses to the min of the range).
    fn meter_low(&self) -> Option<f64>;

    /// `<meter>` `high`. Defaults to [`Self::meter_max`] when
    /// the attribute is absent.
    fn meter_high(&self) -> Option<f64>;

    /// `<meter>` `optimum`. Defaults to `(min + max) / 2.0` when
    /// the attribute is absent.
    fn meter_optimum(&self) -> Option<f64>;

    // ── Per-tag accessors — `<form>` (step 31) ───────────────────

    /// `<form>`'s "listed elements" in document order — every
    /// `<button>`, `<fieldset>`, `<input>`, `<object>`,
    /// `<output>`, `<select>`, or `<textarea>` descendant. The
    /// form itself is excluded (consistent with browser
    /// `form.elements`).
    ///
    /// Returns `Option<Vec<NodeId>>`: `None` for non-`<form>`
    /// elements; `Some(vec![])` when the form has no controls.
    ///
    /// **Deviation from spec §3.1:** the spec sketch listed
    /// `FormControlsCollection<'_, TuiExt>` as the return type.
    /// Browser-shaped, but the borrowed-collection form
    /// entangles the caller's `NodeRef`/`NodeMut` temporary
    /// scope (same issue we hit on `style()`). Returning a
    /// snapshot `Vec<NodeId>` lets `let elts = dom.node(form).
    /// form_elements();` work across statements. Authors who
    /// need `FormControlsCollection::named_item` can construct
    /// it via `FormControlsCollection::from_ids(dom, elts)`.
    fn form_elements(&self) -> Option<Vec<NodeId>>;

    /// `<form>.length` — the number of listed elements.
    /// Equivalent to `form_elements().map(|v| v.len())` but
    /// avoids materializing the `Vec`.
    fn form_length(&self) -> Option<usize>;
}

/// Spec-name alias for [`LayoutRect`] — the type returned by
/// [`TuiAccessors::bounding_rect`]. Browsers return `DOMRect` for
/// the equivalent IDL; this alias lets call sites read `DomRect`
/// without reaching for the layout module.
pub type DomRect = crate::layout::LayoutRect;

/// Write-side accessor surface paired with [`TuiAccessors`].
///
/// Wrong-tag setters are silent `Ok(())` no-ops — calling
/// `set_value("x")` on a `<div>` neither errors nor mutates the
/// tree. Each setter documents which tags it owns; everything
/// else short-circuits before touching the arena.
///
/// `set_value` accepts `impl Into<String>` so call sites pass `&str`,
/// `String`, or anything that converts, matching the ergonomics of
/// the browser IDL setter.
pub trait TuiAccessorsMut<'a> {
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

    /// Set the `[indeterminate]` attribute presence on `<input>`.
    /// No-op on other tags. Browsers expose this as an IDL-only
    /// bit; v1 reflects it via the attribute so a single source
    /// drives selector matching + accessor reads + writes.
    fn set_indeterminate(&mut self, value: bool) -> Result<()>;

    /// Set the `[disabled]` attribute presence on tags with the
    /// `disabled` IDL property: `<button>`, `<input>`, `<select>`,
    /// `<textarea>`, `<option>`, `<optgroup>`, `<fieldset>`. No-op
    /// on other tags. The cascade picks up `[disabled]` changes via
    /// the existing dirty tracker, so `:disabled` / UA `[disabled]
    /// { dim }` re-resolve on next cascade.
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
    /// element is not focusable, this method does nothing"). When
    /// it does fire, runs the standard focus ceremony via
    /// [`runtime::focus::focus_node`](crate::runtime::focus::focus_node):
    /// `blur` + `focusout` on the old target, commit the new focus
    /// (which drives the `:focus` cascade), then `focus` + `focusin`
    /// on this element.
    ///
    /// Lives on the mut trait taking `&mut self` because the literal
    /// shape `focus(&self, ctx: &mut TuiEventCtx<'_>)` hits a borrow-
    /// checker conflict at the call site (`ctx.dom.node(id).focus(
    /// &mut ctx)` reborrows `ctx.dom` shared via the `NodeRef`, then
    /// needs `ctx` mut). The `&mut Dom` borrow is taken once via
    /// `node_mut` and released when the method returns.
    fn focus(&mut self);

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
    /// label, …) fire. Built-in handlers gate on `[disabled]`
    /// themselves, so clicking a disabled checkbox is a tree-level
    /// no-op even though the event still walks the listener chain.
    fn click(&mut self);

    /// `Element.scrollTop = n` — set the vertical scroll offset.
    /// Value is clamped to `[0, scroll_height - viewport_height]`.
    /// On non-scrollable elements (no scrollable content) the
    /// clamp range collapses to `[0, 0]`, so the call is a no-op
    /// — browser-faithful.
    fn set_scroll_top(&mut self, value: i32) -> Result<()>;

    /// `Element.scrollLeft = n` — horizontal companion to
    /// [`Self::set_scroll_top`].
    fn set_scroll_left(&mut self, value: i32) -> Result<()>;

    /// `Element.scrollTo(x, y)` — set both axes in one call. Each
    /// value is clamped independently.
    fn scroll_to(&mut self, x: i32, y: i32) -> Result<()>;

    /// `Element.scrollBy(dx, dy)` — add the deltas to the current
    /// scroll offsets. Each axis re-clamps after the add.
    fn scroll_by(&mut self, dx: i32, dy: i32) -> Result<()>;

    /// `Element.scrollIntoView()` (no options form). Walks up to
    /// the nearest scrollable ancestor and adjusts its scroll
    /// offsets so this element appears at the top-left of the
    /// ancestor's content area. No-op when this element has no
    /// scrollable ancestor.
    ///
    /// M4 ships only the no-args form; `ScrollIntoViewOptions`
    /// (`{block, inline, behavior}`) is polish.
    fn scroll_into_view(&mut self) -> Result<()>;

    /// CSSOM-style write handle to the element's inline
    /// `TuiStyle` — `el.style.setProperty(name, value)`,
    /// `el.style.cssText = "…"`, etc. Returns `None` for
    /// non-element nodes.
    ///
    /// The read side is [`TuiAccessors::style`]. Writes through
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

    /// `<form>.requestSubmit(submitter?)` — fires a synthetic
    /// `submit` event on this form with typed
    /// `EventDetail::Submit { submitter }`. The `submitter`
    /// argument is the `NodeId` of the `<button>` / `<input
    /// type=submit>` that should be reported as the trigger
    /// (`None` for implicit-Enter / programmatic submits).
    ///
    /// Returns `Ok(true)` when the event was
    /// `preventDefault`-ed, `Ok(false)` otherwise. No-op on
    /// non-`<form>` elements (returns `Ok(false)`).
    ///
    /// **Skips form validation** — the validation layer (per
    /// spec §3.2) is polish. Browser `requestSubmit` runs the
    /// "constraint validation" step before firing the event;
    /// rdom doesn't yet have that step, so this method is a
    /// straight pass-through to the existing fire path that
    /// implicit-Enter and button-click already use.
    fn form_request_submit(&mut self, submitter: Option<NodeId>) -> Result<bool>;
}
