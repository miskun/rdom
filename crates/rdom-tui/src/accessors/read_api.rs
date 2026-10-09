//! The read-side trait [`TuiAccessors`] and its [`DomRect`] alias.
//! Implemented in `read_ref` (for `NodeRef`) and `read_mut` (for
//! `NodeMut`, delegating).

use rdom_core::NodeId;

/// Read-side accessor surface for `<el>.{value, checked, …}()`-style
/// calls.
///
/// Implemented for both `NodeRef<'a, TuiExt>` and `NodeMut<'a,
/// TuiExt>`. Having it on `NodeMut` too is what makes the
/// read-then-mutate pattern compile in a single block — `value()`
/// returns owned data, so the immutable borrow ends before the
/// follow-up `set_value()` takes its mutable borrow.
pub trait TuiAccessors<'a>: crate::sealed::Sealed {
    /// Smart form-control value getter. Returns:
    ///
    /// - `<input>` (any type) → the live editing value, mirrored
    ///   from the text-node child seeded by
    ///   `runtime::builtins::input` (at mount, on insertion, on
    ///   focus). For button-family
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

    /// `defaultValue` of an `<input>` / `<textarea>`: the value a
    /// `<form>` reset restores. For a text control or a range, what it
    /// was authored / seeded with, captured before its first change
    /// (or set with `set_default_value`); before any capture, an
    /// `<input>`'s `value` attribute (`""` when absent) and a
    /// `<textarea>`'s text. For the other `<input>` types (hidden,
    /// submit, checkbox, …) it is the `value` attribute, as in HTML.
    /// `None` for other elements.
    fn default_value(&self) -> Option<String>;

    /// `defaultChecked` of a checkbox / radio: its checkedness before the
    /// first flip; a `<form>` reset restores it. `None` for other
    /// elements.
    fn default_checked(&self) -> Option<bool>;

    /// `defaultSelected` of an `<option>`: its selectedness as authored
    /// (the `selected` attribute before the select's first change),
    /// which a `<form>` reset restores. A pick by the selectedness
    /// setting algorithm is not a default. `None` for other elements.
    fn default_selected(&self) -> Option<bool>;

    /// `[indeterminate]` attribute presence — used by the
    /// `:indeterminate` pseudo-class. Browsers expose this as an
    /// IDL-only bit; v1 reflects it via attribute presence so a
    /// single source drives both selector matching and accessor
    /// reads.
    fn indeterminate(&self) -> bool;

    /// `[disabled]` attribute presence — the reflected IDL attribute.
    /// Whether the control is *actually disabled* (also true inside a
    /// `<fieldset disabled>`) is
    /// [`Dom::is_actually_disabled`](rdom_core::Dom::is_actually_disabled).
    fn disabled(&self) -> bool;

    /// `[readonly]` attribute presence. Name is `read_only` (snake)
    /// to match Rust convention; HTML attribute is `readonly`.
    fn read_only(&self) -> bool;

    /// `[inert]` attribute presence (HTML §6.3.1) — the element's own
    /// attribute. Whether a node *is* inert (an `inert` ancestor, or
    /// outside an open modal dialog) is `Dom::is_inert`, which focus,
    /// sequential navigation, hit-testing and selection honour.
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
    /// - actually disabled (own `disabled`, or inside a `<fieldset
    ///   disabled>`) → never focusable, returns `None`.
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
    /// fields; rdom returns [`LayoutRect`](crate::LayoutRect) (i32 + u16) because the
    /// substrate is cell-grained. `DomRect` is re-exported below as
    /// a type alias for spec-name parity.
    fn bounding_rect(&self) -> Option<DomRect>;

    /// `Element.getClientRects()` (CSSOM View §6) for a box: its border
    /// box's fragments, in flow order — one rect for a box in one piece,
    /// one per fragmentainer for a box a fragmented flow split (CSS
    /// Fragmentation 3: a block across the column boxes of a multi-column
    /// container, each fragment the rows of the box that column holds),
    /// whose [`bounding_rect`](Self::bounding_rect) is their bounding box.
    /// Empty, as §6.1 step 1 has it, for an element with no box —
    /// `display: none` (its own or an ancestor's, or inside skipped
    /// contents such as a closed `<details>`) or `display: contents` — and
    /// for non-element nodes: `client_rects().is_empty()` is "not
    /// rendered".
    ///
    /// **Divergence from DOM:** one rect per box, not per line box — an
    /// inline box that wraps reports its own rect
    /// ([`bounding_rect`](Self::bounding_rect)), where `getClientRects()` lists one rect per line
    /// (DIVERGENCES §2, "DOM API shape").
    fn client_rects(&self) -> Vec<DomRect>;

    /// `Element.scrollTop` — vertical scroll offset in cells, measured
    /// from the scrolling area origin (CSSOM View §4): 0 at the top edge,
    /// or at the bottom edge where that is the origin — a
    /// `column-reverse` flex container's main-start, or a row flex
    /// container's cross-start under `flex-wrap: wrap-reverse` (CSS
    /// Flexbox §5.1 / §5.2) — whose values run negative towards the
    /// overflow above. [`Self::scroll_range`] gives the legal values.
    /// `None` for non-element nodes; `0` for non-scrollable elements
    /// (browser-faithful: `el.scrollTop` always returns a number;
    /// for non-scrollable elements that number is `0`).
    fn scroll_top(&self) -> Option<i32>;

    /// `Element.scrollLeft` — horizontal scroll offset in cells,
    /// measured from the scrolling area origin (CSSOM View §4): 0 at the
    /// left edge, or at the right edge where that is the origin — an
    /// `rtl` box, a flex row whose main-start is its right edge
    /// (`row-reverse` under `ltr`, `row` under `rtl`), a column flex
    /// container whose cross-start is (`rtl` XOR `wrap-reverse`) —
    /// whose values run negative towards the overflow on the left, as in
    /// browsers. [`Self::scroll_range`] gives the legal values.
    fn scroll_left(&self) -> Option<i32>;

    /// The legal `scrollLeft` / `scrollTop` values against the
    /// scrollable overflow area the last layout recorded and the
    /// scrollport — the padding box less the scrollbar gutters (CSSOM
    /// View §4, CSS Overflow 3 §2.2, §5.2): on each axis `0 ..= overflow`, or
    /// `-overflow ..= 0` where the scrolling area origin is the right
    /// (bottom) edge ([`Self::scroll_left`], [`Self::scroll_top`]), so
    /// a consumer that maps offsets to content (a virtual list) need
    /// not re-derive which. `0 ..= 0` on an axis that does not
    /// overflow. `None` for non-element nodes.
    fn scroll_range(&self) -> Option<ScrollRange>;

    /// The used tracks of a laid-out grid container — the analogue of
    /// the resolved `grid-template-columns` / `-rows` (CSS Grid 2
    /// §7.2.6), each track a cell range from the content box
    /// ([`GridTracks`](super::GridTracks)). `None` for an element that is
    /// not a grid container or has not been laid out as one, and for
    /// non-element nodes.
    fn grid_tracks(&self) -> Option<super::GridTracks>;

    /// The used columns and rows of a laid-out table (CSS 2.1 §17.5),
    /// each a cell range from the table box's content edge, in column /
    /// row order ([`TableTracks`](super::TableTracks)) — read from what
    /// the last layout kept, never solved again. `None` for an element
    /// that is not a `table` / `inline-table` or has not been laid out
    /// as one, and for non-element nodes.
    fn table_tracks(&self) -> Option<super::TableTracks>;

    /// `Element.scrollWidth` — the width of the scrolling area (CSSOM
    /// View §4): the scrollport ∪ the content, the content extended by
    /// the end padding (CSS Overflow 3 §2.2), measured from the scrolling
    /// area origin — never less than the scrollport's width. `0` for a
    /// box that is not a scroll container.
    fn scroll_width(&self) -> Option<i32>;

    /// `Element.scrollHeight` — the height of the scrolling area;
    /// companion to [`Self::scroll_width`].
    fn scroll_height(&self) -> Option<i32>;

    /// CSSOM-style read view of the element's inline `TuiStyle`
    /// — `el.style.getPropertyValue("color")` and friends, plus
    /// `length` / `item` / `cssText` enumeration. Returns `None`
    /// for non-element nodes.
    ///
    /// Returns an **owned snapshot** of the inline style (clone
    /// of `TuiExt::inline_style`). Re-fetch via `style()` after
    /// mutation to observe new values. The write side is
    /// [`TuiAccessorsMut::style_mut`](super::TuiAccessorsMut::style_mut).
    fn style(&self) -> Option<crate::cssom::StyleDeclaration>;

    // ── Per-tag accessors — `<input>` + `<textarea>` (step 30a) ──
    //
    // Tag-prefixed narrow variants per spec §4.4. Smart
    // counterparts (`value()`, `disabled()`, etc.) already
    // dispatch on tag; these return `Option<T>` with `None` on
    // wrong tag.

    /// Live editing value of an `<input>` — mirror of the
    /// text-node child seeded by `runtime::builtins::input` (at
    /// mount, on insertion, on focus). `None` for
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

    /// `input.form` — the form owner of an `<input>` (HTML §4.10.17.3,
    /// `Dom::form_owner`): the `<form>` its `form="id"` attribute
    /// names, else its nearest `<form>` ancestor. Returns `None` when:
    /// - this isn't an `<input>` element,
    /// - its `form` attribute names no `<form>`, or
    /// - it has no `form` attribute and no `<form>` ancestor.
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

    /// Form owner of a `<textarea>`. Same shape as
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

    /// Form owner of a `<select>`. Same shape
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

    /// Form owner of a `<button>`. Same shape as
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

    // ── Constraint validation (HTML §4.10.20, P7-VALIDATION-1) ───

    /// `validity` — the control's [`ValidityState`](crate::ValidityState),
    /// computed from its current value and attributes (for a barred
    /// control too, as the web's getters report). `Some` on the
    /// elements HTML gives a `validity` (`<button>`, `<fieldset>`,
    /// `<input>`, `<output>`, `<select>`, `<textarea>`); `None`
    /// elsewhere. See `runtime::builtins::validation` for each state.
    fn validity(&self) -> Option<crate::ValidityState>;

    /// `willValidate` — whether the element is a candidate for
    /// constraint validation (`Dom::will_validate`): a submittable
    /// control that is not disabled, `readonly`, a hidden / reset /
    /// button input, a non-submit button, or inside a `<datalist>`.
    fn will_validate(&self) -> bool;

    /// `validationMessage` — rdom's fixed English message for the first
    /// state the control suffers from (a custom error's own message
    /// first), `Some("")` when it is valid or barred. `None` on the
    /// elements without a `validity`.
    fn validation_message(&self) -> Option<String>;
}

/// Spec-name alias for [`LayoutRect`](crate::LayoutRect) — the type returned by
/// [`TuiAccessors::bounding_rect`]. Browsers return `DOMRect` for
/// the equivalent IDL; this alias lets call sites read `DomRect`
/// without reaching for the layout module.
pub type DomRect = crate::layout::LayoutRect;

/// A box's legal scroll offsets per axis ([`TuiAccessors::scroll_range`]):
/// `scrollLeft` in [`x`](Self::x), `scrollTop` in [`y`](Self::y), each
/// `0 ..= overflow` or `-overflow ..= 0` (CSSOM View §4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollRange {
    x: std::ops::RangeInclusive<i32>,
    y: std::ops::RangeInclusive<i32>,
}

impl ScrollRange {
    /// The range `x` horizontally and `y` vertically.
    pub fn new(x: std::ops::RangeInclusive<i32>, y: std::ops::RangeInclusive<i32>) -> Self {
        Self { x, y }
    }

    /// The legal `scrollLeft` values.
    pub fn x(&self) -> std::ops::RangeInclusive<i32> {
        self.x.clone()
    }

    /// The legal `scrollTop` values.
    pub fn y(&self) -> std::ops::RangeInclusive<i32> {
        self.y.clone()
    }
}
