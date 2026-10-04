//! The color properties' applicators: `color`, `background-color`,
//! `border-color`, resolved at computed-value time (CSS Color 4 §14).
//!
//! A color resolves against a [`ColorContext`]: the element's color
//! for `currentcolor` (§6.4; in `color` itself, the parent's) and its
//! used color scheme for `light-dark()` and the canvas model (CSS Color
//! Adjust 1 §2.1). Both are only final once the ladder has run — a
//! later rule may set `color` or `color-scheme` — so a winning value
//! that depends on them waits in [`ElementColors`] and is resolved by
//! [`ElementColors::finalize`]; until then the value resolved against
//! what has cascaded so far stands in.

use super::apply::{Keywords, Resolved, matches_pass};
use crate::style::{Color, ColorContext, ComputedStyle, ImportantMask, TuiColor, TuiStyle, Value};
use rdom_style::color::ColorScheme;

/// The winning color declarations whose value depends on the element
/// (`currentcolor`, `light-dark()`, a color function holding either),
/// kept by the ladder until the element's `color` and `color-scheme`
/// are final.
#[derive(Debug, Default)]
pub(in crate::style::cascade) struct ElementColors {
    fg: Option<TuiColor>,
    bg: Option<TuiColor>,
    border_fg: Option<TuiColor>,
    /// A declaration of `border-color` took part in the cascade; without
    /// one the property takes its initial value.
    border_declared: bool,
}

/// `border-color`'s initial value (CSS Backgrounds 3 §3.1): the one
/// place it is defined, for an element without a declaration and for
/// `border-color: initial`.
const BORDER_COLOR_INITIAL: TuiColor = TuiColor::CurrentColor;

impl ElementColors {
    /// Resolve the waiting colors once the ladder has run: `color`
    /// against `parent_color` (its `currentcolor`), then the others
    /// against the element's final color — an undeclared `border-color`
    /// its initial `currentcolor` — all under the element's used
    /// color scheme given the document's `preferred` one.
    pub(in crate::style::cascade) fn finalize(
        self,
        working: &mut ComputedStyle,
        parent_color: Color,
        preferred: ColorScheme,
    ) {
        let scheme = working.color_scheme.used(preferred);
        let vars = working.vars.clone();
        let resolve = |color: Option<TuiColor>, current: Color, target: &mut Color| {
            let cx = ColorContext::new(current).with_scheme(scheme);
            if let Some(c) = color.and_then(|c| c.resolve(&vars, &cx)) {
                *target = c;
            }
        };
        resolve(self.fg, parent_color, &mut working.fg);
        let current = working.fg;
        resolve(self.bg, current, &mut working.bg);
        let border = if self.border_declared {
            self.border_fg
        } else {
            Some(BORDER_COLOR_INITIAL)
        };
        resolve(border, current, &mut working.border_fg);
    }
}

/// Apply one block's color declarations for one ladder pass.
pub(in crate::style::cascade) fn apply_colors(
    working: &mut ComputedStyle,
    colors: &mut ElementColors,
    style: &TuiStyle,
    important_pass: bool,
    kw: &Keywords<'_>,
) {
    let vars = working.vars.clone();
    let scheme = working.color_scheme.used(kw.preferred_scheme);
    apply_color(
        ColorSlot {
            target: &mut working.fg,
            waiting: &mut colors.fg,
            field: |c| c.fg,
            initial: None,
        },
        &style.fg,
        matches_pass(style.important.contains(ImportantMask::FG), important_pass),
        kw,
        &vars,
        &ColorContext::new(kw.parent.fg).with_scheme(scheme),
    );
    let cx = ColorContext::new(working.fg).with_scheme(scheme);
    apply_color(
        ColorSlot {
            target: &mut working.bg,
            waiting: &mut colors.bg,
            field: |c| c.bg,
            initial: None,
        },
        &style.bg,
        matches_pass(style.important.contains(ImportantMask::BG), important_pass),
        kw,
        &vars,
        &cx,
    );
    colors.border_declared |= style.border_fg.is_some();
    apply_color(
        ColorSlot {
            target: &mut working.border_fg,
            waiting: &mut colors.border_fg,
            field: |c| c.border_fg,
            initial: Some(BORDER_COLOR_INITIAL),
        },
        &style.border_fg,
        matches_pass(
            style.important.contains(ImportantMask::BORDER_FG),
            important_pass,
        ),
        kw,
        &vars,
        &cx,
    );
}

/// One color property's computed field and its bookkeeping.
struct ColorSlot<'w> {
    target: &'w mut Color,
    /// Where a value that depends on the element waits for
    /// [`ElementColors::finalize`].
    waiting: &'w mut Option<TuiColor>,
    /// The field in another computed style (`inherit`, `revert`, …).
    field: fn(&ComputedStyle) -> Color,
    /// The property's initial value where it is not the initial
    /// table's (`border-color`: `currentcolor`).
    initial: Option<TuiColor>,
}

/// A color property (`in_pass`: the declaration's importance matches
/// the pass), resolved against `vars` and `cx`. A `var()` chain that
/// finds no color takes the parent's value.
fn apply_color(
    slot: ColorSlot<'_>,
    value: &Option<Value<TuiColor>>,
    in_pass: bool,
    kw: &Keywords<'_>,
    vars: &std::collections::HashMap<String, rdom_style::CustomValue>,
    cx: &ColorContext,
) {
    let Some(v) = value else { return };
    if !in_pass {
        return;
    }
    let specified = match (v, &slot.initial) {
        (Value::Initial, Some(initial)) => Some(initial),
        _ => match kw.resolve(v) {
            Resolved::Specified(tc) => Some(tc),
            Resolved::From(source) => {
                *slot.target = (slot.field)(source);
                None
            }
        },
    };
    *slot.waiting = match specified.and_then(|tc| tc.substitute_vars(vars)) {
        Some(color) => {
            *slot.target = color
                .resolve(vars, cx)
                .unwrap_or_else(|| (slot.field)(kw.parent));
            color.depends_on_element().then_some(color)
        }
        None => {
            if specified.is_some() {
                *slot.target = (slot.field)(kw.parent);
            }
            None
        }
    };
}
