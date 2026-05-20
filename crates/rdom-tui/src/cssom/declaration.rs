//! `StyleDeclaration` + `StyleDeclarationMut` — CSSOM IDL wrappers
//! around an element's inline `TuiStyle`.
//!
//! Reads/writes route through
//! [`rdom_style::property_dispatch`](rdom_style::property_dispatch),
//! the single source of truth for the name→(setter, serializer)
//! mapping shared with the block parser.
//!
//! ## Attribute coherence
//!
//! Write methods on [`StyleDeclarationMut`] update **both**:
//!
//! 1. `TuiExt::inline_style` — the cascade input.
//! 2. The `style="…"` attribute — what `outerHTML` round-trips
//!    and what the future external-write observer (step 28)
//!    listens for.
//!
//! Both writes happen in one method body; from the caller's
//! perspective the two are atomic.

use rdom_core::{NodeMut, NodeRef};
use rdom_style::TuiStyle;
use rdom_style::property_dispatch;
use rdom_style::property_dispatch::DispatchError;

use crate::TuiExt;
use crate::node::TuiNodeExt;

/// Failure modes for [`StyleDeclarationMut::try_set_property`] /
/// [`StyleDeclarationMut::try_set_property_important`].
///
/// The shipped [`StyleDeclarationMut::set_property`] family
/// silently swallows [`Parse`] errors per browser CSSOM —
/// `el.style.color = "not-a-color"` doesn't throw. That's
/// debugger-friendly in a browser (devtools console surfaces the
/// drop) but a real footgun in a terminal app where stdout is
/// often redirected. The `try_*` variants expose the parse
/// channel so callers who want to know can.
///
/// `Tree` wraps a `rdom_core::DomError` from the underlying
/// `set_attribute` write. In practice this only fires when the
/// node has been detached between the borrow check and the
/// attribute write — vanishingly rare for typical author
/// usage, but surfaced rather than swallowed.
///
/// [`Parse`]: SetPropertyError::Parse
#[derive(Debug)]
pub enum SetPropertyError {
    /// `name` wasn't in the property-dispatch table or `value`
    /// failed to parse. Wraps the inner [`DispatchError`].
    Parse(DispatchError),
    /// The attribute-write step (`style="…"`) failed. Wraps
    /// the inner `rdom_core::DomError`.
    Tree(rdom_core::DomError),
}

impl From<DispatchError> for SetPropertyError {
    fn from(e: DispatchError) -> Self {
        Self::Parse(e)
    }
}

impl From<rdom_core::DomError> for SetPropertyError {
    fn from(e: rdom_core::DomError) -> Self {
        Self::Tree(e)
    }
}

impl core::fmt::Display for SetPropertyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse(DispatchError::UnknownProperty) => {
                write!(f, "unknown CSS property")
            }
            Self::Parse(DispatchError::InvalidValue) => {
                write!(f, "invalid value for property")
            }
            Self::Tree(e) => write!(f, "DOM tree error: {e:?}"),
        }
    }
}

impl std::error::Error for SetPropertyError {}

/// Read-only snapshot of an element's inline `TuiStyle` — the
/// CSSOM `getter` half of `el.style`.
///
/// Construct via [`crate::TuiAccessors::style`] (returns `None`
/// for non-element nodes). All reads route through
/// [`rdom_style::property_dispatch::serialize`].
///
/// ## Snapshot semantics
///
/// `StyleDeclaration` owns a `TuiStyle` clone taken at
/// construction time. Subsequent mutations through
/// [`StyleDeclarationMut`] don't appear here — re-fetch via
/// `el.style()` to get a fresh snapshot. This avoids
/// lifetime entanglement with the underlying `NodeRef` so
/// `let style = dom.node(id).style().unwrap(); … style.x()` is a
/// natural usage pattern.
///
/// ## Multi-property reads: take one snapshot
///
/// **Each call to `el.style()` clones the inline `TuiStyle`** (a
/// `Box<>`-free clone of ~30–40 owned fields). For one-shot
/// reads (`dom.node(id).style().unwrap().color()`) the cost is
/// invisible. When you need several properties off the same
/// element, **bind once** rather than re-fetching:
///
/// ```rust,ignore
/// // ✗ Three clones — one per `.style()` call.
/// let fg = dom.node(id).style().unwrap().get_property_value("color");
/// let bg = dom.node(id).style().unwrap().get_property_value("background-color");
/// let pad = dom.node(id).style().unwrap().get_property_value("padding");
///
/// // ✓ One clone — bind once, read many.
/// let style = dom.node(id).style().unwrap();
/// let fg = style.get_property_value("color");
/// let bg = style.get_property_value("background-color");
/// let pad = style.get_property_value("padding");
/// ```
///
/// The clone is a deliberate trade — it sidesteps lifetime
/// entanglement with the underlying `NodeRef` temporary so
/// `let style = dom.node(id).style().unwrap();` works across
/// statements (the natural author pattern).
pub struct StyleDeclaration {
    inline: TuiStyle,
}

impl StyleDeclaration {
    /// Wrap the given inline style by clone. `pub(crate)`
    /// because the usual entry point is `TuiAccessors::style`.
    pub(crate) fn new(inline: TuiStyle) -> Self {
        Self { inline }
    }

    /// `el.style.getPropertyValue("color")` — returns the CSS
    /// string form of the property's current value, or `""` if
    /// unset / unknown property (CSSOM convention).
    pub fn get_property_value(&self, name: &str) -> String {
        property_dispatch::serialize(name, &self.inline).unwrap_or_default()
    }

    /// `el.style.getPropertyPriority("color")` — returns
    /// `"important"` if the property carries an `!important`
    /// bit, else `""`. Returns `""` for unknown names.
    pub fn get_property_priority(&self, name: &str) -> &'static str {
        match property_dispatch::property_mask(name) {
            Some(mask) if self.inline.important.contains(mask) => "important",
            _ => "",
        }
    }

    /// `el.style.cssText` — serialize every set property into a
    /// `"name: value [!important]; …"` declaration list in the
    /// canonical [`property_dispatch::property_names`] order.
    pub fn css_text(&self) -> String {
        css_text_of(&self.inline)
    }

    /// `el.style.length` — number of properties currently set
    /// on this declaration (i.e., the number of names in
    /// [`property_dispatch::property_names`] for which
    /// [`property_dispatch::serialize`] returns `Some`).
    pub fn length(&self) -> usize {
        property_dispatch::property_names()
            .iter()
            .filter(|&&name| property_dispatch::serialize(name, &self.inline).is_some())
            .count()
    }

    /// `el.style.item(i)` — name of the i-th set property in
    /// [`property_dispatch::property_names`] iteration order, or
    /// `None` past the end.
    pub fn item(&self, index: usize) -> Option<&'static str> {
        property_dispatch::property_names()
            .iter()
            .copied()
            .filter(|&name| property_dispatch::serialize(name, &self.inline).is_some())
            .nth(index)
    }
}

/// Write-only handle to an element's inline `TuiStyle` — the
/// CSSOM `setter` half of `el.style`.
///
/// Each setter writes both `TuiExt::inline_style` and the
/// `style="…"` attribute (§8.5 lock — see module docstring).
///
/// ## Ergonomic note: bind through `node_mut` for multi-property writes
///
/// In JS, `el.style.color = "red"` is one expression. The Rust
/// equivalent — `dom.node_mut(id).style_mut().unwrap().set_property("color", "red")?`
/// — is verbose for a single write and more so when several
/// writes line up. Two patterns help:
///
/// ```rust,ignore
/// // ✓ Bind the NodeMut once, drive multiple style writes off it.
/// let mut nm = dom.node_mut(id);
/// let mut s = nm.style_mut().unwrap();
/// s.set_property("color", "red")?;
/// s.set_property("background-color", "white")?;
/// s.set_property("padding", "1 2 3 4")?;
///
/// // ✓ For a bulk replacement, prefer `set_css_text` — single
/// // parse, single attribute write, single observer fire.
/// dom.node_mut(id)
///     .style_mut()
///     .unwrap()
///     .set_css_text("color: red; background-color: white; padding: 1 2 3 4")?;
/// ```
///
/// The verbosity is the price of explicit borrow lifetimes —
/// each `set_property` call drives a `set_attribute` write that
/// needs an exclusive `&mut Dom` borrow, so the chain has to
/// rebuild every time without the binding.
pub struct StyleDeclarationMut<'a> {
    node: NodeMut<'a, TuiExt>,
}

impl<'a> StyleDeclarationMut<'a> {
    pub(crate) fn new(node: NodeMut<'a, TuiExt>) -> Self {
        Self { node }
    }

    /// `el.style.setProperty(name, value)` — parses `value` into
    /// the property's typed slot.
    ///
    /// **Silent on parse failure** (browser-faithful CSSOM).
    /// Returns `Ok(())` and leaves the style untouched when:
    /// - `name` isn't in the dispatch table (`"bogus"`),
    /// - `value` fails to parse (`"not-a-color"`),
    /// - this is not an element node.
    ///
    /// The `Err` channel surfaces tree-level failures from the
    /// underlying `set_attribute` write only — in practice this
    /// only fires for detached nodes.
    ///
    /// **Want to know about parse failures?** Use
    /// [`Self::try_set_property`] — it returns a typed
    /// [`SetPropertyError`] for the parse channel. A common
    /// pattern in terminal apps (where stdout is redirected
    /// during raw mode, so a silent drop is a real debugging
    /// trap) is to use `try_set_property` everywhere during
    /// development and `set_property` only at points where
    /// browser-spec parity matters.
    ///
    /// Browser semantics: `setProperty` without an explicit
    /// priority **clears** any prior `!important` bit on the
    /// property. Use [`Self::set_property_important`] when the
    /// bit should be set.
    pub fn set_property(&mut self, name: &str, value: &str) -> rdom_core::Result<()> {
        swallow_parse_errors(self.try_write_inner(name, value, /* important */ false))
    }

    /// `el.style.setProperty(name, value, "important")` — same
    /// as [`Self::set_property`] but also raises the `!important`
    /// bit for the property. Same silent-parse-error semantics;
    /// see [`Self::try_set_property_important`] for the
    /// surface-errors variant.
    pub fn set_property_important(&mut self, name: &str, value: &str) -> rdom_core::Result<()> {
        swallow_parse_errors(self.try_write_inner(name, value, /* important */ true))
    }

    /// Like [`Self::set_property`] but surfaces parse failures as
    /// [`SetPropertyError::Parse`]. Use this when you want to
    /// know that a typo like `"colour"` or a malformed value like
    /// `"not-a-color"` dropped on the floor — `set_property`
    /// silently swallows those per CSSOM spec, which is a real
    /// debugging trap in terminal apps with no devtools console.
    ///
    /// Non-element nodes still no-op silently (returns `Ok(())`)
    /// — that's a tree-shape question, not a parse error.
    pub fn try_set_property(&mut self, name: &str, value: &str) -> Result<(), SetPropertyError> {
        self.try_write_inner(name, value, /* important */ false)
    }

    /// Like [`Self::set_property_important`] but surfaces parse
    /// failures as [`SetPropertyError::Parse`]. See
    /// [`Self::try_set_property`] for the rationale.
    pub fn try_set_property_important(
        &mut self,
        name: &str,
        value: &str,
    ) -> Result<(), SetPropertyError> {
        self.try_write_inner(name, value, /* important */ true)
    }

    /// `el.style.removeProperty(name)` — clears the property's
    /// typed slot (sets to `None`) and drops its `!important`
    /// bit. Returns the previous serialized value (or `""` if
    /// the property was unset). Re-serializes the `style="…"`
    /// attribute.
    pub fn remove_property(&mut self, name: &str) -> rdom_core::Result<String> {
        let id = self.node.id();
        let dom = self.node.dom_mut();
        let mut nm = dom.node_mut(id);
        let Some(ext) = nm.ext_mut() else {
            return Ok(String::new());
        };
        let prev = property_dispatch::serialize(name, &ext.inline_style).unwrap_or_default();
        let removed = property_dispatch::remove(name, &mut ext.inline_style);
        if !removed {
            return Ok(prev);
        }
        let css_text = css_text_of(&ext.inline_style);
        // Drop the NodeMut borrow before re-borrowing dom for
        // `set_attribute`.
        let _ = nm;
        write_style_attribute(dom, id, &css_text)?;
        Ok(prev)
    }

    /// `el.style.cssText = "..."` — replace the entire inline
    /// style with a fresh parse of `css`. Properties not present
    /// in `css` end up unset.
    ///
    /// Goes through [`rdom_css::parse_inline`] (single source of
    /// truth with `<style>` block parsing). Warnings produced by
    /// the parse are dropped silently — browser CSSOM `cssText =`
    /// doesn't surface them either.
    pub fn set_css_text(&mut self, css: &str) -> rdom_core::Result<()> {
        let id = self.node.id();
        let dom = self.node.dom_mut();
        let parsed = rdom_css::parse_inline(css);
        let mut nm = dom.node_mut(id);
        let Some(ext) = nm.ext_mut() else {
            return Ok(());
        };
        ext.inline_style = parsed.style;
        let css_text = css_text_of(&ext.inline_style);
        let _ = nm;
        write_style_attribute(dom, id, &css_text)?;
        Ok(())
    }

    /// Common write path for the four setProperty variants.
    /// Returns the rich [`SetPropertyError`] so `try_*` callers
    /// see the parse channel; `set_property` /
    /// `set_property_important` swallow `Parse` errors via
    /// [`Self::swallow_parse_errors`] and surface only the tree
    /// channel.
    ///
    /// Non-element nodes return `Ok(())` (no-op) — there's no
    /// `TuiExt` to write into, but that's not a parse-or-tree
    /// failure.
    fn try_write_inner(
        &mut self,
        name: &str,
        value: &str,
        important: bool,
    ) -> Result<(), SetPropertyError> {
        let id = self.node.id();
        let dom = self.node.dom_mut();
        let mut nm = dom.node_mut(id);
        let Some(ext) = nm.ext_mut() else {
            return Ok(());
        };
        // Surface the parse channel verbatim from property_dispatch.
        property_dispatch::set(name, value, &mut ext.inline_style)?;
        // Flip the !important bit for this property's mask.
        if let Some(mask) = property_dispatch::property_mask(name) {
            if important {
                ext.inline_style.important |= mask;
            } else {
                // setProperty without "important" CLEARS any
                // prior important bit (browser semantics).
                ext.inline_style.important = ext.inline_style.important.without(mask);
            }
        }
        let css_text = css_text_of(&ext.inline_style);
        // Drop the NodeMut binding before re-borrowing dom for
        // `set_attribute`.
        let _ = nm;
        write_style_attribute(dom, id, &css_text)?;
        Ok(())
    }
}

/// Map a [`SetPropertyError`] from [`StyleDeclarationMut::try_write_inner`]
/// onto the CSSOM-spec-faithful `rdom_core::Result<()>`: `Parse`
/// errors become `Ok(())` (silent no-op), `Tree` errors propagate
/// as `Err`. Free function (not a method) so callers can chain it
/// inline without re-borrowing `self`.
fn swallow_parse_errors(result: Result<(), SetPropertyError>) -> rdom_core::Result<()> {
    match result {
        Ok(()) | Err(SetPropertyError::Parse(_)) => Ok(()),
        Err(SetPropertyError::Tree(e)) => Err(e),
    }
}

/// Write the `style="…"` attribute under the
/// `CSSOM_REENTRY` guard so the inline-style observer (step 28)
/// doesn't re-parse what we just serialized. Drop semantics
/// restore the guard even on panic.
fn write_style_attribute(
    dom: &mut crate::TuiDom,
    id: rdom_core::NodeId,
    css_text: &str,
) -> rdom_core::Result<()> {
    let _g = super::reentry::ReentryGuard::enter();
    dom.set_attribute(id, "style", css_text)
}

/// Serialize every set property on `style` as a CSSOM-style
/// declaration list: `"name: value [!important]; …"`. Iteration
/// follows [`property_dispatch::property_names`] order.
///
/// **Shorthand/longhand rule (D-M4-2):** when a shorthand family
/// has a representable shorthand form (i.e. `serialize("padding",
/// …)` returns `Some`), the four longhands in the family are
/// suppressed in cssText — the shorthand declaration round-trips
/// the same state through `set_css_text`, and emitting both
/// produces a duplicate-laden, lossy round-trip. The longhands
/// remain available via `get_property_value("padding-top")`; the
/// suppression is purely about cssText shape.
///
/// Per-family deviation note: `padding` storage is consolidated
/// (one `Padding` struct, not four `Option<u16>`), so
/// `set_property("padding-top", "5")` ends up representable as
/// `padding: 5 0 0 0` and emits the shorthand form. Browsers
/// preserve "only padding-top was set" via per-side independent
/// storage; rdom v1 normalizes to the shorthand. Round-trip stays
/// lossless.
fn css_text_of(style: &TuiStyle) -> String {
    let mut out = String::new();
    for &name in property_dispatch::property_names() {
        // Suppress longhand emission when its shorthand fires —
        // see the function-level docstring.
        if let Some(shorthand) = shorthand_family_of(name)
            && property_dispatch::serialize(shorthand, style).is_some()
        {
            continue;
        }
        if let Some(value) = property_dispatch::serialize(name, style) {
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(name);
            out.push_str(": ");
            out.push_str(&value);
            if let Some(mask) = property_dispatch::property_mask(name)
                && style.important.contains(mask)
            {
                out.push_str(" !important");
            }
            out.push(';');
        }
    }
    out
}

/// Map a longhand name to its shorthand parent name. Used by
/// [`css_text_of`] to suppress longhand emission when the
/// shorthand form represents the same state. Returns `None` for
/// non-longhand names (including the shorthand names themselves
/// and standalone properties).
fn shorthand_family_of(name: &str) -> Option<&'static str> {
    match name {
        "padding-top" | "padding-right" | "padding-bottom" | "padding-left" => Some("padding"),
        "top" | "right" | "bottom" | "left" => Some("inset"),
        "overflow-x" | "overflow-y" => Some("overflow"),
        "transition-property"
        | "transition-duration"
        | "transition-timing-function"
        | "transition-delay" => Some("transition"),
        _ => None,
    }
}

/// Helper for [`crate::TuiAccessors::style`] — constructs a
/// snapshot declaration view of `node`'s inline style. Returns
/// `None` for non-element nodes (those without a `TuiExt`).
pub(crate) fn from_node_ref(node: &NodeRef<'_, TuiExt>) -> Option<StyleDeclaration> {
    Some(StyleDeclaration::new(node.tui_ext()?.inline_style.clone()))
}

#[cfg(test)]
mod tests {
    use crate::TuiDom;
    use crate::{TuiAccessors, TuiAccessorsMut};

    fn dom_with(tag: &str) -> (TuiDom, rdom_core::NodeId) {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let el = dom.create_element(tag);
        dom.append_child(root, el).unwrap();
        (dom, el)
    }

    // ── Read side ────────────────────────────────────────────────

    #[test]
    fn get_property_value_returns_empty_when_unset() {
        let (dom, div) = dom_with("div");
        let style = dom.node(div).style().expect("element has style");
        assert_eq!(style.get_property_value("color"), "");
    }

    #[test]
    fn style_returns_none_for_non_element() {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let text = dom.create_text_node("hi");
        dom.append_child(root, text).unwrap();
        // Text nodes have no ext → no style accessor.
        assert!(dom.node(text).style().is_none());
    }

    #[test]
    fn get_property_priority_empty_by_default() {
        let (dom, div) = dom_with("div");
        let style = dom.node(div).style().unwrap();
        assert_eq!(style.get_property_priority("color"), "");
    }

    #[test]
    fn length_zero_on_fresh_element() {
        let (dom, div) = dom_with("div");
        assert_eq!(dom.node(div).style().unwrap().length(), 0);
    }

    #[test]
    fn css_text_empty_on_fresh_element() {
        let (dom, div) = dom_with("div");
        assert_eq!(dom.node(div).style().unwrap().css_text(), "");
    }

    #[test]
    fn item_out_of_range_is_none() {
        let (dom, div) = dom_with("div");
        assert!(dom.node(div).style().unwrap().item(0).is_none());
    }

    // ── Write: set_property ──────────────────────────────────────

    #[test]
    fn set_property_writes_inline_style_field() {
        let (mut dom, div) = dom_with("div");
        dom.node_mut(div)
            .style_mut()
            .unwrap()
            .set_property("color", "red")
            .unwrap();
        // Re-read via the read side.
        assert_eq!(
            dom.node(div).style().unwrap().get_property_value("color"),
            "red"
        );
    }

    #[test]
    fn set_property_writes_style_attribute_atomically() {
        // §8.5 acceptance criterion: programmatic set_property
        // updates BOTH the inline-style field AND the style="…"
        // attribute. This is the "attribute coherence" lock.
        let (mut dom, div) = dom_with("div");
        dom.node_mut(div)
            .style_mut()
            .unwrap()
            .set_property("color", "red")
            .unwrap();
        // (a) inline_style.fg was written.
        assert_eq!(
            dom.node(div).style().unwrap().get_property_value("color"),
            "red"
        );
        // (b) style="…" attribute was re-serialized.
        let attr = dom.node(div).get_attribute("style").unwrap_or("");
        assert!(
            attr.contains("color: red"),
            "style attribute should contain `color: red`, got {attr:?}"
        );
    }

    #[test]
    fn set_property_invalid_value_is_silent_no_op() {
        let (mut dom, div) = dom_with("div");
        let r = dom
            .node_mut(div)
            .style_mut()
            .unwrap()
            .set_property("color", "not-a-color");
        assert!(r.is_ok());
        assert_eq!(
            dom.node(div).style().unwrap().get_property_value("color"),
            "",
            "inline_style.fg should be unset after invalid value"
        );
        // Attribute should not have been touched.
        assert_eq!(dom.node(div).get_attribute("style"), None);
    }

    #[test]
    fn set_property_unknown_name_is_silent_no_op() {
        let (mut dom, div) = dom_with("div");
        let r = dom
            .node_mut(div)
            .style_mut()
            .unwrap()
            .set_property("not-a-property", "x");
        assert!(r.is_ok());
        assert_eq!(dom.node(div).get_attribute("style"), None);
    }

    #[test]
    fn set_property_clears_prior_important_bit() {
        let (mut dom, div) = dom_with("div");
        dom.node_mut(div)
            .style_mut()
            .unwrap()
            .set_property_important("color", "red")
            .unwrap();
        assert_eq!(
            dom.node(div)
                .style()
                .unwrap()
                .get_property_priority("color"),
            "important"
        );
        // Browser: setProperty without "important" clears the bit.
        dom.node_mut(div)
            .style_mut()
            .unwrap()
            .set_property("color", "blue")
            .unwrap();
        assert_eq!(
            dom.node(div)
                .style()
                .unwrap()
                .get_property_priority("color"),
            ""
        );
    }

    // ── Write: set_property_important ────────────────────────────

    #[test]
    fn set_property_important_raises_priority() {
        let (mut dom, div) = dom_with("div");
        dom.node_mut(div)
            .style_mut()
            .unwrap()
            .set_property_important("color", "red")
            .unwrap();
        let style = dom.node(div).style().unwrap();
        assert_eq!(style.get_property_value("color"), "red");
        assert_eq!(style.get_property_priority("color"), "important");
        // !important is emitted in css_text.
        assert!(style.css_text().contains("!important"));
    }

    #[test]
    fn set_property_important_writes_attribute_with_marker() {
        let (mut dom, div) = dom_with("div");
        dom.node_mut(div)
            .style_mut()
            .unwrap()
            .set_property_important("color", "red")
            .unwrap();
        let attr = dom.node(div).get_attribute("style").unwrap_or("");
        assert!(
            attr.contains("!important"),
            "style attribute should include !important, got {attr:?}"
        );
    }

    // ── Write: remove_property ───────────────────────────────────

    #[test]
    fn remove_property_returns_previous_value() {
        let (mut dom, div) = dom_with("div");
        dom.node_mut(div)
            .style_mut()
            .unwrap()
            .set_property("color", "red")
            .unwrap();
        let prev = dom
            .node_mut(div)
            .style_mut()
            .unwrap()
            .remove_property("color")
            .unwrap();
        assert_eq!(prev, "red");
        assert_eq!(
            dom.node(div)
                .style()
                .unwrap()
                .get_property_priority("color"),
            ""
        );
    }

    #[test]
    fn remove_property_clears_field_and_important_bit() {
        let (mut dom, div) = dom_with("div");
        {
            let mut nm = dom.node_mut(div);
            let mut sd = nm.style_mut().unwrap();
            sd.set_property_important("color", "red").unwrap();
            sd.remove_property("color").unwrap();
        }
        let style = dom.node(div).style().unwrap();
        assert_eq!(style.get_property_value("color"), "");
        assert_eq!(style.get_property_priority("color"), "");
    }

    #[test]
    fn remove_property_unset_returns_empty_string() {
        let (mut dom, div) = dom_with("div");
        let prev = dom
            .node_mut(div)
            .style_mut()
            .unwrap()
            .remove_property("color")
            .unwrap();
        assert_eq!(prev, "");
    }

    #[test]
    fn remove_property_unknown_returns_empty_string() {
        let (mut dom, div) = dom_with("div");
        let prev = dom
            .node_mut(div)
            .style_mut()
            .unwrap()
            .remove_property("bogus")
            .unwrap();
        assert_eq!(prev, "");
    }

    // ── Write: set_css_text ──────────────────────────────────────

    #[test]
    fn set_css_text_replaces_entire_declaration() {
        let (mut dom, div) = dom_with("div");
        {
            let mut nm = dom.node_mut(div);
            let mut sd = nm.style_mut().unwrap();
            sd.set_property("color", "red").unwrap();
            sd.set_css_text("display: block; gap: 2").unwrap();
        }
        // color was cleared; display + gap are set.
        let style = dom.node(div).style().unwrap();
        assert_eq!(style.get_property_value("color"), "");
        assert_eq!(style.get_property_value("display"), "block");
        assert_eq!(style.get_property_value("gap"), "2");
    }

    #[test]
    fn set_css_text_serializes_back_to_attribute() {
        let (mut dom, div) = dom_with("div");
        dom.node_mut(div)
            .style_mut()
            .unwrap()
            .set_css_text("color: red; gap: 3")
            .unwrap();
        let attr = dom.node(div).get_attribute("style").unwrap_or("");
        assert!(attr.contains("color: red"));
        assert!(attr.contains("gap: 3"));
    }

    // ── length / item ────────────────────────────────────────────

    #[test]
    fn length_counts_set_properties() {
        let (mut dom, div) = dom_with("div");
        {
            let mut nm = dom.node_mut(div);
            let mut sd = nm.style_mut().unwrap();
            sd.set_property("color", "red").unwrap();
            sd.set_property("gap", "2").unwrap();
        }
        assert_eq!(dom.node(div).style().unwrap().length(), 2);
    }

    #[test]
    fn item_returns_property_names_in_canonical_order() {
        // property_names() lists "color" before "gap"; item(0)
        // should hit the first set name in that order.
        let (mut dom, div) = dom_with("div");
        {
            let mut nm = dom.node_mut(div);
            let mut sd = nm.style_mut().unwrap();
            sd.set_property("gap", "2").unwrap();
            sd.set_property("color", "red").unwrap();
        }
        let style = dom.node(div).style().unwrap();
        assert_eq!(style.item(0), Some("color"));
        assert_eq!(style.item(1), Some("gap"));
        assert_eq!(style.item(2), None);
    }

    // ── css_text + round-trip ────────────────────────────────────

    #[test]
    fn css_text_round_trips_through_set_css_text() {
        let (mut dom, div) = dom_with("div");
        {
            let mut nm = dom.node_mut(div);
            let mut sd = nm.style_mut().unwrap();
            sd.set_property("color", "red").unwrap();
            sd.set_property("gap", "2").unwrap();
        }
        let serialized = dom.node(div).style().unwrap().css_text();

        // Build a second element, apply the serialized text,
        // assert the same property values.
        let root = dom.root();
        let other = dom.create_element("div");
        dom.append_child(root, other).unwrap();
        dom.node_mut(other)
            .style_mut()
            .unwrap()
            .set_css_text(&serialized)
            .unwrap();
        let s = dom.node(other).style().unwrap();
        assert_eq!(s.get_property_value("color"), "red");
        assert_eq!(s.get_property_value("gap"), "2");
    }

    #[test]
    fn css_text_includes_important_marker() {
        let (mut dom, div) = dom_with("div");
        dom.node_mut(div)
            .style_mut()
            .unwrap()
            .set_property_important("color", "red")
            .unwrap();
        let text = dom.node(div).style().unwrap().css_text();
        assert!(text.contains("color: red !important"));
    }

    // ── Build-script camelCase aliases (step 27) ─────────────────

    #[test]
    fn alias_color_delegates_to_get_property_value() {
        let (mut dom, div) = dom_with("div");
        dom.node_mut(div)
            .style_mut()
            .unwrap()
            .set_property("color", "red")
            .unwrap();
        let style = dom.node(div).style().unwrap();
        assert_eq!(style.color(), "red");
        assert_eq!(style.color(), style.get_property_value("color"));
    }

    #[test]
    fn alias_background_color_handles_kebab_to_snake_conversion() {
        // Single-hyphen property name. The generated alias is
        // `background_color` (snake), reading the kebab key
        // `"background-color"` internally.
        let (mut dom, div) = dom_with("div");
        dom.node_mut(div)
            .style_mut()
            .unwrap()
            .set_property("background-color", "blue")
            .unwrap();
        let style = dom.node(div).style().unwrap();
        assert_eq!(style.background_color(), "blue");
        assert_eq!(
            style.background_color(),
            style.get_property_value("background-color")
        );
    }

    #[test]
    fn alias_transition_timing_function_handles_multi_hyphen() {
        // Multi-hyphen property name → multi-underscore method.
        let (mut dom, div) = dom_with("div");
        dom.node_mut(div)
            .style_mut()
            .unwrap()
            .set_property("transition-timing-function", "ease-in-out")
            .unwrap();
        let style = dom.node(div).style().unwrap();
        assert_eq!(style.transition_timing_function(), "ease-in-out");
    }

    #[test]
    fn alias_z_index_handles_short_compound() {
        let (mut dom, div) = dom_with("div");
        dom.node_mut(div)
            .style_mut()
            .unwrap()
            .set_property("z-index", "3")
            .unwrap();
        assert_eq!(dom.node(div).style().unwrap().z_index(), "3");
    }

    #[test]
    fn alias_returns_empty_when_property_unset() {
        // Aliases delegate to get_property_value, which returns
        // "" for unset properties (CSSOM convention). Generated
        // aliases inherit that behavior.
        let (dom, div) = dom_with("div");
        let style = dom.node(div).style().unwrap();
        assert_eq!(style.color(), "");
        assert_eq!(style.padding(), "");
        assert_eq!(style.font_weight(), "");
    }

    // ── cssText: shorthand suppresses longhands (D-M4-2) ─────────

    #[test]
    fn css_text_padding_shorthand_skips_longhands() {
        // Pre-D-M4-2 bug: cssText emitted padding AND each of the
        // four padding-* longhands because they all read from
        // style.padding. Round-tripping cssText through set_css_text
        // back to cssText would not be lossless. Browser-faithful:
        // emit shorthand only when its name comes up; skip longhands.
        let (mut dom, div) = dom_with("div");
        dom.node_mut(div)
            .style_mut()
            .unwrap()
            .set_property("padding", "1 2 3 4")
            .unwrap();
        let css = dom.node(div).style().unwrap().css_text();
        assert!(
            css.contains("padding: 1 2 3 4;"),
            "shorthand should emit, got {css:?}"
        );
        assert!(
            !css.contains("padding-top"),
            "padding-top longhand must not be emitted when shorthand fires, got {css:?}"
        );
        assert!(
            !css.contains("padding-right"),
            "padding-right longhand must not be emitted when shorthand fires, got {css:?}"
        );
        assert!(
            !css.contains("padding-bottom"),
            "padding-bottom longhand must not be emitted when shorthand fires, got {css:?}"
        );
        assert!(
            !css.contains("padding-left"),
            "padding-left longhand must not be emitted when shorthand fires, got {css:?}"
        );
    }

    #[test]
    fn css_text_inset_shorthand_skips_longhands_when_all_four_set() {
        // inset fires only when all four sides are set. Then the
        // four longhand (top/right/bottom/left) declarations must
        // be skipped — same rule as padding.
        let (mut dom, div) = dom_with("div");
        dom.node_mut(div)
            .style_mut()
            .unwrap()
            .set_property("inset", "1 2 3 4")
            .unwrap();
        let css = dom.node(div).style().unwrap().css_text();
        assert!(css.contains("inset:"), "inset shorthand should emit");
        assert!(
            !css.contains("top: 1"),
            "top longhand must not emit when inset shorthand fires, got {css:?}"
        );
        assert!(
            !css.contains("right: 2"),
            "right longhand must not emit when inset shorthand fires, got {css:?}"
        );
        assert!(
            !css.contains("bottom: 3"),
            "bottom longhand must not emit when inset shorthand fires, got {css:?}"
        );
        assert!(
            !css.contains("left: 4"),
            "left longhand must not emit when inset shorthand fires, got {css:?}"
        );
    }

    #[test]
    fn css_text_inset_longhand_only_emits_set_sides() {
        // Counter to the previous test: when only one side is set,
        // the inset shorthand serializes None, so the lone longhand
        // emits on its own. This is the *correct* behavior pre-fix
        // too — confirming the fix doesn't regress it.
        let (mut dom, div) = dom_with("div");
        dom.node_mut(div)
            .style_mut()
            .unwrap()
            .set_property("top", "5")
            .unwrap();
        let css = dom.node(div).style().unwrap().css_text();
        assert!(css.contains("top: 5"), "lone top should emit, got {css:?}");
        assert!(
            !css.contains("inset:"),
            "inset must not emit when only top is set, got {css:?}"
        );
    }

    #[test]
    fn css_text_overflow_shorthand_skips_longhands() {
        let (mut dom, div) = dom_with("div");
        dom.node_mut(div)
            .style_mut()
            .unwrap()
            .set_property("overflow", "scroll")
            .unwrap();
        let css = dom.node(div).style().unwrap().css_text();
        assert!(css.contains("overflow: scroll"));
        assert!(
            !css.contains("overflow-x"),
            "overflow-x longhand must not emit when overflow shorthand fires, got {css:?}"
        );
        assert!(
            !css.contains("overflow-y"),
            "overflow-y longhand must not emit when overflow shorthand fires, got {css:?}"
        );
    }

    #[test]
    fn css_text_transition_shorthand_skips_longhands() {
        let (mut dom, div) = dom_with("div");
        dom.node_mut(div)
            .style_mut()
            .unwrap()
            .set_property("transition", "color 200ms ease 0ms")
            .unwrap();
        let css = dom.node(div).style().unwrap().css_text();
        assert!(
            css.contains("transition:"),
            "transition shorthand should emit, got {css:?}"
        );
        assert!(
            !css.contains("transition-property"),
            "transition-property longhand must not emit when shorthand fires, got {css:?}"
        );
        assert!(
            !css.contains("transition-duration"),
            "transition-duration longhand must not emit when shorthand fires, got {css:?}"
        );
        assert!(
            !css.contains("transition-timing-function"),
            "transition-timing-function longhand must not emit when shorthand fires, got {css:?}"
        );
        assert!(
            !css.contains("transition-delay"),
            "transition-delay longhand must not emit when shorthand fires, got {css:?}"
        );
    }

    // ── try_set_property surfaces parse failures (A-M4-2) ────────

    #[test]
    fn try_set_property_returns_unknown_property() {
        use crate::cssom::{DispatchError, SetPropertyError};
        let (mut dom, div) = dom_with("div");
        let mut nm = dom.node_mut(div);
        let r = nm.style_mut().unwrap().try_set_property("colour", "red");
        assert!(matches!(
            r,
            Err(SetPropertyError::Parse(DispatchError::UnknownProperty))
        ));
    }

    #[test]
    fn try_set_property_returns_invalid_value() {
        use crate::cssom::{DispatchError, SetPropertyError};
        let (mut dom, div) = dom_with("div");
        let mut nm = dom.node_mut(div);
        let r = nm
            .style_mut()
            .unwrap()
            .try_set_property("color", "not-a-color");
        assert!(matches!(
            r,
            Err(SetPropertyError::Parse(DispatchError::InvalidValue))
        ));
    }

    #[test]
    fn try_set_property_succeeds_on_valid_input() {
        let (mut dom, div) = dom_with("div");
        {
            let mut nm = dom.node_mut(div);
            nm.style_mut()
                .unwrap()
                .try_set_property("color", "red")
                .unwrap();
        }
        assert_eq!(
            dom.node(div).style().unwrap().get_property_value("color"),
            "red"
        );
    }

    #[test]
    fn try_set_property_important_raises_priority_and_surfaces_errors() {
        use crate::cssom::{DispatchError, SetPropertyError};
        let (mut dom, div) = dom_with("div");

        // Success path raises !important.
        dom.node_mut(div)
            .style_mut()
            .unwrap()
            .try_set_property_important("color", "red")
            .unwrap();
        assert_eq!(
            dom.node(div)
                .style()
                .unwrap()
                .get_property_priority("color"),
            "important"
        );

        // Failure path surfaces the parse error.
        let r = dom
            .node_mut(div)
            .style_mut()
            .unwrap()
            .try_set_property_important("colour", "red");
        assert!(matches!(
            r,
            Err(SetPropertyError::Parse(DispatchError::UnknownProperty))
        ));
    }

    #[test]
    fn set_property_remains_silent_on_parse_failure() {
        // A-M4-2 acceptance: try_set_property surfaces errors,
        // but plain set_property must keep its browser-faithful
        // silence so existing call sites don't change behavior.
        let (mut dom, div) = dom_with("div");
        let r = dom
            .node_mut(div)
            .style_mut()
            .unwrap()
            .set_property("colour", "red");
        assert!(r.is_ok(), "set_property must silently swallow parse errors");
        assert_eq!(
            dom.node(div).style().unwrap().get_property_value("color"),
            ""
        );
    }

    #[test]
    fn css_text_round_trip_padding_is_lossless() {
        // The headline correctness test for D-M4-2. cssText output
        // must parse back into an equivalent inline style.
        let (mut dom, div) = dom_with("div");
        dom.node_mut(div)
            .style_mut()
            .unwrap()
            .set_property("padding", "1 2 3 4")
            .unwrap();
        let css_first = dom.node(div).style().unwrap().css_text();
        // Round-trip: feed cssText back through set_css_text and
        // confirm the result re-serializes the same string.
        let (mut dom2, div2) = dom_with("div");
        dom2.node_mut(div2)
            .style_mut()
            .unwrap()
            .set_css_text(&css_first)
            .unwrap();
        let css_second = dom2.node(div2).style().unwrap().css_text();
        assert_eq!(
            css_first, css_second,
            "cssText round-trip must be lossless (was broken pre-D-M4-2)"
        );
    }
}
