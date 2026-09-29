//! The one write path for an element's inline declarations
//! (`P7G-SETTER-MUTATION-1`, `P7G-SEED-PRESERVE-1`).
//!
//! The `style` attribute is the source of truth (CSSOM §6.7 "style
//! attribute"): `TuiExt::inline_style` is its parsed cache, and every
//! typed write — the CSSOM ([`super::StyleDeclarationMut`]) and the
//! direct setters (`TuiNodeMutExt::set_width`, …, `set_inline_style`) —
//! serializes the result back into the attribute. That attribute write
//! is a DOM mutation, so the dirty tracker queues the element for the
//! next cascade wherever the write came from; the typed value stays
//! exact in the cache even where the serialization rounds.
//!
//! **In sync** means the attribute reads exactly what serializing the
//! cache gives. Anything else — markup parsed before the App existed, a
//! raw `set_attribute("style", …)` with no inline-style observer
//! installed — means the attribute is newer, and
//! [`sync_from_attribute`] re-parses it before a write builds on the
//! cache. Seeding at `App::build` is the same sync over the tree, so a
//! setter's value written before the App survives it.

use rdom_core::NodeId;
use rdom_css::{Warning, parse_inline};
use rdom_style::TuiStyle;

use super::declaration::css_text_of;
use crate::TuiDom;

/// Bring `id`'s inline-style cache up to its `style` attribute when the
/// two disagree (module doc): the cache is replaced by the attribute's
/// parse. No attribute, or an attribute that already reads as the
/// cache's serialization, leaves the cache alone. Returns the parse's
/// warnings (none when nothing was parsed).
pub(crate) fn sync_from_attribute(dom: &mut TuiDom, id: NodeId) -> Vec<Warning> {
    let node = dom.node(id);
    let (Some(text), Some(ext)) = (node.get_attribute("style"), node.ext()) else {
        return Vec::new();
    };
    if text == css_text_of(ext.inline_style_or_empty()) {
        return Vec::new();
    }
    let parsed = parse_inline(text);
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.set_inline_style(parsed.style);
    }
    parsed.warnings
}

/// Apply `f` to `id`'s inline declarations and reflect the result into
/// its `style` attribute (module doc). A no-op on a node that is not an
/// element. The attribute write is the `AttributeChanged` the dirty
/// tracker restyles from; the inline-style observer skips it (the cache
/// is already current).
pub(crate) fn write_inline_style(dom: &mut TuiDom, id: NodeId, f: impl FnOnce(&mut TuiStyle)) {
    if dom.node(id).ext().is_none() {
        return;
    }
    sync_from_attribute(dom, id);
    let css_text = {
        let mut node = dom.node_mut(id);
        let ext = node.ext_mut().expect("checked above: an element");
        f(ext.inline_style_mut());
        // An emptied style is stored as `None`, as `set_inline_style` does.
        let style = ext.inline_style.take().map(|b| *b).unwrap_or_default();
        ext.set_inline_style(style);
        ext.style_dirty = true;
        css_text_of(ext.inline_style_or_empty())
    };
    write_style_attribute(dom, id, &css_text).expect("an element accepts a `style` attribute");
}

/// Write the `style="…"` attribute under the `CSSOM_REENTRY` guard so
/// the inline-style observer doesn't re-parse what was just serialized.
/// Drop semantics restore the guard even on panic.
pub(crate) fn write_style_attribute(
    dom: &mut TuiDom,
    id: NodeId,
    css_text: &str,
) -> rdom_core::Result<()> {
    let _g = super::reentry::ReentryGuard::enter();
    dom.set_attribute(id, "style", css_text)
}
