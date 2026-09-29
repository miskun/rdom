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

#[cfg(test)]
mod tests {
    //! `P7G-SEED-PRESERVE-1`: inline-style seeding at `App::build` keeps
    //! what a setter or CSSOM write stored before the App existed,
    //! alongside the markup's own `style` declarations.

    use rdom_core::NodeId;
    use rdom_style::{Color, TuiColor, TuiStyle, Value};

    use crate::layout::Size;
    use crate::node::{TuiNodeExt, TuiNodeMutExt};
    use crate::render::{Terminal, TestBackend};
    use crate::runtime::app::App;
    use crate::{Stylesheet, TuiAccessorsMut, TuiDom};

    /// A `<div style="color: red">` in a fresh tree.
    fn styled_div() -> (TuiDom, NodeId) {
        let mut dom: TuiDom = TuiDom::new();
        let div = dom.create_element("div");
        dom.set_attribute(div, "style", "color: red").unwrap();
        dom.append_child(dom.root(), div).unwrap();
        (dom, div)
    }

    fn built(dom: TuiDom) -> App<TestBackend> {
        let terminal = Terminal::new(TestBackend::new(20, 4)).unwrap();
        let mut app = App::with_backend(dom, Stylesheet::bare(), terminal).unwrap();
        app.advance(0).unwrap();
        app
    }

    fn inline(app: &App<TestBackend>, id: NodeId) -> TuiStyle {
        app.dom()
            .node(id)
            .tui_ext()
            .unwrap()
            .inline_style_or_empty()
            .clone()
    }

    const RED: Value<TuiColor> = Value::Specified(TuiColor::Literal(Color::Rgb(255, 0, 0)));

    #[test]
    fn a_setter_before_build_survives_seeding_beside_the_markup_style() {
        let (mut dom, div) = styled_div();
        dom.node_mut(div).set_width(Size::Fixed(7));
        let app = built(dom);
        let style = inline(&app, div);
        assert_eq!(style.width, Some(Value::Specified(Size::Fixed(7))));
        assert_eq!(style.fg, Some(RED), "the markup's declaration too");
    }

    #[test]
    fn a_cssom_write_before_build_survives_seeding_beside_the_markup_style() {
        let (mut dom, div) = styled_div();
        dom.node_mut(div)
            .style_mut()
            .unwrap()
            .set_property("height", "2")
            .unwrap();
        let app = built(dom);
        let style = inline(&app, div);
        assert_eq!(style.height, Some(Value::Specified(Size::Fixed(2))));
        assert_eq!(style.fg, Some(RED));
    }

    /// A value the serializer cannot round-trip (`Color::Indexed`) stays
    /// exact: seeding finds the attribute in sync and keeps the slot.
    #[test]
    fn a_setter_value_that_does_not_round_trip_survives_seeding() {
        let mut dom: TuiDom = TuiDom::new();
        let div = dom.create_element("div");
        dom.append_child(dom.root(), div).unwrap();
        dom.node_mut(div)
            .set_inline_style(TuiStyle::new().fg(Color::Indexed(3)));
        let app = built(dom);
        assert_eq!(
            inline(&app, div).fg,
            Some(Value::Specified(TuiColor::Literal(Color::Indexed(3))))
        );
    }

    /// A raw attribute write after a setter, with no observer to see it,
    /// is newer: seeding takes the attribute (the declarations it states
    /// replace the old ones, as a `style` attribute write does).
    #[test]
    fn a_raw_attribute_write_after_a_setter_wins_at_seeding() {
        let (mut dom, div) = styled_div();
        dom.node_mut(div).set_width(Size::Fixed(7));
        dom.set_attribute(div, "style", "height: 3").unwrap();
        let app = built(dom);
        let style = inline(&app, div);
        assert_eq!(style.height, Some(Value::Specified(Size::Fixed(3))));
        assert_eq!(style.width, None);
        assert_eq!(style.fg, None);
    }
}
