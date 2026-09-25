//! [`StyleDeclarationMut`] — the CSSOM setter half of `el.style`.
//! Every write updates both `TuiExt::inline_style` and the
//! `style="…"` attribute (under the CSSOM re-entry guard), so the
//! two stay coherent.

use rdom_core::NodeMut;
use rdom_style::property_dispatch;

use super::SetPropertyError;
use super::serialize::css_text_of;
use crate::TuiExt;

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
        // Custom properties carry importance per declaration (`set`
        // stored the value already, rendered from tokens like CSS source).
        if let Some(custom) = name.strip_prefix("--")
            && let Some(d) = ext
                .inline_style
                .custom_properties
                .iter_mut()
                .find(|d| d.name == custom)
        {
            d.important = important;
        }
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
    let _g = crate::cssom::reentry::ReentryGuard::enter();
    dom.set_attribute(id, "style", css_text)
}
