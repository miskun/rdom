//! [`StyleDeclaration`] — the read-only CSSOM getter half of
//! `el.style`: a snapshot of an element's inline `TuiStyle` with
//! `getPropertyValue` / `getPropertyPriority` / `cssText` /
//! `length` / `item`.

use rdom_style::TuiStyle;
use rdom_style::property_dispatch;

use super::serialize::css_text_of;

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
/// [`StyleDeclarationMut`](super::StyleDeclarationMut) don't appear here — re-fetch via
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
