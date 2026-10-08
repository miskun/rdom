//! `accent-color` (CSS UI 4 §6.3): the accent of the form controls' UA
//! chrome. rdom's chrome is in the UA sheet (a toggle's mark is its
//! `::before` text) and the built-ins' paint (a progress bar, a range
//! slider), so each reads the element's used accent here: its
//! `accent-color` resolved against the element — `None` for `auto`, which
//! keeps the UA's own colors.

use rdom_core::{Dom, InputTypeState, NodeId};

use crate::ext::TuiExt;
use crate::layout::AccentColor;
use crate::style::{Color, ComputedStyle};

/// The accent `style` asks for, resolved against its color and used color
/// scheme (`preferred` being the document's): `None` for `auto`.
pub(crate) fn resolve(
    style: &ComputedStyle,
    preferred: rdom_style::color::ColorScheme,
) -> Option<Color> {
    let AccentColor::Color(c) = &style.ui.accent_color else {
        return None;
    };
    let scheme = style.color_scheme.used(preferred);
    let cx = crate::ColorContext::new(style.fg).with_scheme(scheme);
    c.resolve(&style.vars, &cx)
}

/// The used accent of element `id` (its last computed style), `None` for
/// `auto` or an unstyled element.
pub(crate) fn of(dom: &Dom<TuiExt>, id: NodeId) -> Option<Color> {
    let style = dom.node(id).ext()?.computed.as_deref()?;
    resolve(style, crate::style::CascadeExt::color_scheme(dom))
}

/// Tint a toggle's mark: the `::before` of a checked (or indeterminate)
/// checkbox, or of a checked radio, takes the element's accent — the
/// part a browser fills with it. `host` is the element's computed style.
pub(crate) fn tint_mark(
    dom: &Dom<TuiExt>,
    id: NodeId,
    host: &ComputedStyle,
    before: &mut ComputedStyle,
) {
    let toggle = matches!(
        dom.input_type_state(id),
        Some(InputTypeState::Checkbox | InputTypeState::Radio)
    );
    if !toggle || !(dom.has_attribute(id, "checked") || dom.is_indeterminate(id)) {
        return;
    }
    if let Some(accent) = resolve(host, crate::style::CascadeExt::color_scheme(dom)) {
        before.fg = accent;
    }
}
