//! Chrome substitution — the seam between the inline paint path and
//! the UA chrome the built-in elements supply.
//!
//! A few built-ins paint something other than their own text: a
//! `<progress>` / `<meter>` paints a bar, a closed `<select>` dropdown
//! echoes its selected option's label, an `<input type="password">`
//! paints bullets. The paint pass must not know how any of that is
//! decided (which attribute marks a dropdown open, how a meter picks
//! its zone color, …). It knows only the contract in this file: ask
//! for an element's [`ChromeText`]; if one comes back, paint it as a
//! single row in place of the element's own text, converting the
//! optional fg override into a paint `Style` here, where
//! `ComputedStyle` → `Style` conversion lives.
//!
//! The suppliers are the built-ins' `inline_chrome` adapters, gathered
//! into one table in `runtime::builtins::inline_chrome`. That table is
//! compiled in rather than registered per `Dom` because chrome must
//! paint for a bare `Dom<TuiExt>` that never went through `App::build`
//! — the paint pass's own tests, and any consumer driving `cascade` →
//! `layout_dom` → `paint_dom` by hand; a per-`Dom` registration would
//! silently drop the chrome for them. [`inline_chrome`] below is the
//! only place `render` names `runtime::builtins` for chrome.

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::render::Style;
use crate::render::paint_pass::text::glyph_style_from_computed;
use crate::style::{Color, ComputedStyle};

/// What a built-in paints in place of an element's own text: the
/// single-row `text`, and optionally a foreground that overrides the
/// element's cascaded `color` (a `<meter>` outside its optimum zone).
/// `None` for `fg` keeps the cascade's fg.
pub(crate) struct ChromeText {
    pub(crate) text: String,
    pub(crate) fg: Option<Color>,
}

/// A built-in's chrome supplier: given an element and the single-row
/// width available to it, the text to paint instead of the element's
/// own text — or `None` when the element is not one of that
/// built-in's.
pub(crate) type InlineChromeFn = fn(&Dom<TuiExt>, NodeId, u16) -> Option<ChromeText>;

/// The replacement text and its glyph style for `id`, or `None` when
/// no built-in substitutes chrome for it and the element paints its
/// own text. `width` is the single row the chrome may fill.
///
/// The style omits `bg` (`glyph_style_from_computed`): the element's
/// `fill_bg` already owns the cells' background.
pub(super) fn inline_chrome(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    width: u16,
) -> Option<(String, Style)> {
    let chrome = crate::runtime::builtins::inline_chrome::lookup(dom, id, width)?;
    let style = match chrome.fg {
        Some(fg) => {
            let mut overridden = computed.clone();
            overridden.fg = fg;
            glyph_style_from_computed(&overridden)
        }
        None => glyph_style_from_computed(computed),
    };
    Some((chrome.text, style))
}
