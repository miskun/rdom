//! `InlineStyleObserver` — internal `MutationObserver` that
//! refreshes `TuiExt::inline_style` whenever the `style="…"`
//! attribute changes.
//!
//! ## What it does
//!
//! Watches for `Mutation::AttributeChanged { name: "style", … }`.
//! When fired, parses the new value via `rdom_css::parse_inline`
//! and writes the resulting [`TuiStyle`] to
//! [`TuiExt::inline_style`](crate::ext::TuiExt::inline_style).
//!
//! Retires the `D-M1-4` tech-debt entry: pre-M4, external
//! `set_attribute("style", "…")` writes after startup were silent
//! — the inline-style cache `seed_inline_styles` populated at
//! `App::new` time didn't refresh.
//!
//! ## What it skips
//!
//! When [`super::reentry::is_in_cssom_write`] is `true`, the
//! observer bails. `StyleDeclarationMut` sets that flag while
//! writing its serialized declaration back to the `style="…"`
//! attribute (per §8.5 lock); the inline_style field has already
//! been updated through the typed path, so re-parsing would be
//! wasted work — and would risk dropping `!important` bits or
//! mangling values the dispatch table parses tightly but the
//! serializer rounds (e.g. `Color::Indexed(n)`).
//!
//! ## Layering
//!
//! Lives in `cssom/` alongside the parse-and-apply helpers
//! ([`crate::cssom::apply`]) because both bridge `rdom-css` and
//! `rdom-tui` — they're the same architectural neighborhood.

use rdom_core::{Dom, Mutation, MutationObserver, ObserverId};
use rdom_style::TuiStyle;

use crate::TuiExt;

/// Internal observer that refreshes `TuiExt::inline_style` on
/// external `style="…"` attribute writes. Returns the
/// [`ObserverId`] for symmetric removal during teardown.
pub fn install(dom: &mut Dom<TuiExt>) -> ObserverId {
    dom.add_mutation_observer(Box::new(InlineStyleObserver))
}

struct InlineStyleObserver;

impl MutationObserver<TuiExt> for InlineStyleObserver {
    fn observe(&mut self, dom: &mut Dom<TuiExt>, record: &Mutation) {
        // Filter: only `style` attribute changes.
        let (id, new_value) = match record {
            Mutation::AttributeChanged { id, name, new, .. } if name == "style" => {
                (*id, new.clone())
            }
            _ => return,
        };

        // Bail if a `StyleDeclarationMut` write is what triggered
        // this mutation — the typed path already updated
        // `inline_style`. See `super::reentry`.
        if super::reentry::is_in_cssom_write() {
            return;
        }

        // Removed attribute (new = None) clears the inline style.
        // Otherwise re-parse via the same path
        // `seed_inline_styles` uses, so author writes match the
        // initial-load behavior.
        let parsed = match new_value {
            Some(text) => rdom_css::parse_inline(&text).style,
            None => TuiStyle::new(),
        };

        // Direct ext write — does NOT fire `Mutation` records,
        // so we don't trip the `is_observing` re-entrancy panic
        // in `Dom::fire_mutation`.
        if let Some(ext) = dom.node_mut(id).ext_mut() {
            ext.inline_style = parsed;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::node::TuiNodeExt;
    use crate::{TuiAccessorsMut, TuiDom};
    use rdom_style::{Color, TuiColor, Value};

    fn dom_with_div() -> (TuiDom, rdom_core::NodeId) {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let div = dom.create_element("div");
        dom.append_child(root, div).unwrap();
        super::install(&mut dom);
        (dom, div)
    }

    #[test]
    fn external_style_attribute_write_refreshes_inline_style() {
        // The §8.4 acceptance criterion: an author's
        // `set_attribute("style", "color: red")` after build
        // updates `TuiExt::inline_style.fg`. Pre-step-28 this
        // was the `D-M1-4` debt.
        let (mut dom, div) = dom_with_div();
        dom.set_attribute(div, "style", "color: red").unwrap();
        let fg = dom.node(div).tui_ext().unwrap().inline_style.fg.clone();
        assert_eq!(
            fg,
            Some(Value::Specified(TuiColor::Literal(Color::Rgb(255, 0, 0))))
        );
    }

    #[test]
    fn external_style_attribute_removal_clears_inline_style() {
        let (mut dom, div) = dom_with_div();
        dom.set_attribute(div, "style", "color: red").unwrap();
        dom.remove_attribute(div, "style").unwrap();
        let fg = dom.node(div).tui_ext().unwrap().inline_style.fg.clone();
        assert!(fg.is_none(), "removing style attribute should clear fg");
    }

    #[test]
    fn cssom_write_does_not_double_parse() {
        // §8.5 acceptance: programmatic
        // `el.style_mut().set_property(...)` already updated
        // `inline_style`. The observer must self-suppress so the
        // typed write doesn't get clobbered by a parse-and-overwrite.
        let (mut dom, div) = dom_with_div();
        dom.node_mut(div)
            .style_mut()
            .unwrap()
            .set_property_important("color", "red")
            .unwrap();
        // The `!important` bit is on the typed inline_style, NOT
        // in the serialized "color: red !important;" attribute as
        // the observer parses it (parse_inline DOES carry
        // !important; if the observer fired it'd preserve the
        // bit). The key check is that the typed value isn't
        // mangled — fg is Red.
        let inline = dom.node(div).tui_ext().unwrap().inline_style.clone();
        assert_eq!(
            inline.fg,
            Some(Value::Specified(TuiColor::Literal(Color::Rgb(255, 0, 0))))
        );
        // And the !important bit is set on the typed style.
        use rdom_style::ImportantMask;
        assert!(inline.important.contains(ImportantMask::FG));
    }

    #[test]
    fn observer_ignores_non_style_attribute_writes() {
        // Sanity: the observer is keyed on `name == "style"`.
        // Other attribute writes shouldn't touch inline_style.
        let (mut dom, div) = dom_with_div();
        dom.set_attribute(div, "class", "hero").unwrap();
        let fg = dom.node(div).tui_ext().unwrap().inline_style.fg.clone();
        assert!(fg.is_none());
    }

    #[test]
    fn invalid_value_clears_or_warns_silently() {
        // `parse_inline("bogus")` returns warnings + an empty
        // style. The observer writes that empty style — the
        // attribute is effectively a no-op for cascade purposes.
        // No panic, no error.
        let (mut dom, div) = dom_with_div();
        dom.set_attribute(div, "style", "bogus-property: x")
            .unwrap();
        // No panic = test passes. Inline style stays empty (no
        // recognized properties parsed).
        let fg = dom.node(div).tui_ext().unwrap().inline_style.fg.clone();
        assert!(fg.is_none());
    }

    #[test]
    fn install_default_observers_wires_inline_style_observer() {
        // D-M4-7 acceptance: `install_default_observers` is the
        // single entry point for apps that build a `TuiDom`
        // directly (bypassing `App::build`). It must install at
        // least the inline-style observer so external
        // `set_attribute("style", …)` writes still refresh the
        // typed `inline_style` field — otherwise the old D-M1-4
        // symptom recurs.
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let div = dom.create_element("div");
        dom.append_child(root, div).unwrap();
        crate::cssom::install_default_observers(&mut dom);

        dom.set_attribute(div, "style", "color: red").unwrap();
        let fg = dom.node(div).tui_ext().unwrap().inline_style.fg.clone();
        assert_eq!(
            fg,
            Some(Value::Specified(TuiColor::Literal(Color::Rgb(255, 0, 0))))
        );
    }
}
