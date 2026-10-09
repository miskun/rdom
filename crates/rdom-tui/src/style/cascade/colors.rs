//! The color properties' applicators: `color`, `background-color`,
//! the four `border-*-color`s, `text-decoration-color`, `box-shadow`'s
//! colors and the drop shadows' of `filter` / `backdrop-filter`, resolved at
//! computed-value time (CSS Color 4 §14).
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
use rdom_style::layout::{BoxShadow, FilterList, Sides};

/// The winning color declarations whose value depends on the element
/// (`currentcolor`, `light-dark()`, a color function holding either),
/// kept by the ladder until the element's `color` and `color-scheme`
/// are final.
#[derive(Debug, Default)]
pub(in crate::style::cascade) struct ElementColors {
    fg: Option<TuiColor>,
    bg: Option<TuiColor>,
    border_color: Sides<Option<TuiColor>>,
    /// A declaration of the side's `border-*-color` took part in the
    /// cascade; without one the side takes the initial value.
    border_declared: Sides<bool>,
    /// The winning `box-shadow` list as declared, when its colors wait
    /// for the element's final `color`.
    box_shadow: Option<Vec<BoxShadow>>,
    /// The winning `filter` / `backdrop-filter` as declared (Filter
    /// Effects 1 §5), likewise: a drop shadow's color waits.
    filter: Option<FilterList>,
    backdrop_filter: Option<FilterList>,
    /// `text-decoration-color`, as `border_color`'s sides.
    decoration_color: Option<TuiColor>,
    decoration_declared: bool,
}

/// The computed `border-*-color` of each side, for the CSS-wide
/// keywords' source styles.
const BORDER_COLOR_FIELDS: Sides<fn(&ComputedStyle) -> Color> = Sides::new(
    |c| c.border_color.top,
    |c| c.border_color.right,
    |c| c.border_color.bottom,
    |c| c.border_color.left,
);

/// Each side's `!important` bit.
const BORDER_COLOR_MASKS: Sides<ImportantMask> = Sides::new(
    ImportantMask::BORDER_TOP_COLOR,
    ImportantMask::BORDER_RIGHT_COLOR,
    ImportantMask::BORDER_BOTTOM_COLOR,
    ImportantMask::BORDER_LEFT_COLOR,
);

/// `border-color`'s initial value (CSS Backgrounds 3 §3.1): the one
/// place it is defined, for an element without a declaration and for
/// `border-color: initial`.
const BORDER_COLOR_INITIAL: TuiColor = TuiColor::CurrentColor;

/// `text-decoration-color`'s initial value (CSS Text Decoration 4 §2.4):
/// `currentcolor`.
const DECORATION_COLOR_INITIAL: TuiColor = TuiColor::CurrentColor;

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
        let targets = working.border_color.each_mut();
        let waiting = self.border_color.to_array();
        let declared = self.border_declared.to_array();
        for ((target, color), declared) in targets.into_iter().zip(waiting).zip(declared) {
            let color = if declared {
                color
            } else {
                Some(BORDER_COLOR_INITIAL)
            };
            resolve(color, current, target);
        }
        let decoration = if self.decoration_declared {
            self.decoration_color
        } else {
            Some(DECORATION_COLOR_INITIAL)
        };
        resolve(decoration, current, &mut working.text_decoration.color);
        let cx = ColorContext::new(current).with_scheme(scheme);
        if let Some(shadows) = self.box_shadow {
            working.box_shadow = compute_shadows(shadows, &vars, &cx);
        }
        if let Some(list) = self.filter {
            working.effects.filter = compute_filter(&list, &vars, &cx);
        }
        if let Some(list) = self.backdrop_filter {
            working.effects.backdrop_filter = compute_filter(&list, &vars, &cx);
        }
    }
}

/// A `filter` list's computed value: its drop shadows' colors resolved
/// against `cx` (Filter Effects 1 §5: "as specified, with colors
/// computed"); an unresolved one makes the declaration invalid at
/// computed-value time — `none`, the initial value.
fn compute_filter(
    list: &FilterList,
    vars: &std::collections::HashMap<String, rdom_style::CustomValue>,
    cx: &ColorContext,
) -> FilterList<Color> {
    list.map_colors(|c| c.resolve(vars, cx)).unwrap_or_default()
}

/// `box-shadow`'s computed value: each color resolved against `cx`
/// (CSS Backgrounds 3 §6.1: "as specified, with colors computed"). A
/// color left unresolved — a `var()` chain with no color — makes the
/// declaration invalid at computed-value time, so the property takes
/// its initial value, no shadow (CSS Variables 1 §3.1).
fn compute_shadows(
    shadows: Vec<BoxShadow>,
    vars: &std::collections::HashMap<String, rdom_style::CustomValue>,
    cx: &ColorContext,
) -> Vec<BoxShadow<Color>> {
    shadows
        .into_iter()
        .map(|s| {
            let color = s.color.resolve(vars, cx)?;
            Some(s.with_color(|_| color))
        })
        .collect::<Option<Vec<_>>>()
        .unwrap_or_default()
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
    if let Some(v) = &style.box_shadow
        && matches_pass(
            style.important.contains(ImportantMask::BOX_SHADOW),
            important_pass,
        )
    {
        match kw.resolve(v) {
            Resolved::Specified(list) => {
                // Resolved against what has cascaded so far; finalized
                // against the element's final color.
                working.box_shadow = compute_shadows(list.clone(), &vars, &cx);
                colors.box_shadow = Some(list.clone());
            }
            Resolved::From(source) => {
                working.box_shadow = source.box_shadow.clone();
                colors.box_shadow = None;
            }
        }
    }
    apply_filters(working, colors, style, important_pass, kw, &cx);
    colors.decoration_declared |= style.text_decoration.color.is_some();
    apply_color(
        ColorSlot {
            target: &mut working.text_decoration.color,
            waiting: &mut colors.decoration_color,
            field: |c| c.text_decoration.color,
            initial: Some(DECORATION_COLOR_INITIAL),
        },
        &style.text_decoration.color,
        matches_pass(
            style
                .important
                .contains(ImportantMask::TEXT_DECORATION_COLOR),
            important_pass,
        ),
        kw,
        &vars,
        &cx,
    );
    let sides = working
        .border_color
        .each_mut()
        .into_iter()
        .zip(colors.border_color.each_mut())
        .zip(colors.border_declared.each_mut())
        .zip(style.border_color.each())
        .zip(BORDER_COLOR_FIELDS.to_array())
        .zip(BORDER_COLOR_MASKS.to_array());
    for (((((target, waiting), declared), value), field), mask) in sides {
        *declared |= value.is_some();
        apply_color(
            ColorSlot {
                target,
                waiting,
                field,
                initial: Some(BORDER_COLOR_INITIAL),
            },
            value,
            matches_pass(style.important.contains(mask), important_pass),
            kw,
            &vars,
            &cx,
        );
    }
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

/// `filter` and `backdrop-filter` (Filter Effects 1 §5, 2 §3), whose drop
/// shadows' colors resolve as `box-shadow`'s do.
fn apply_filters(
    working: &mut ComputedStyle,
    colors: &mut ElementColors,
    style: &TuiStyle,
    important_pass: bool,
    kw: &Keywords<'_>,
    cx: &ColorContext,
) {
    let vars = working.vars.clone();
    let slots = [
        (
            &style.effects.filter,
            ImportantMask::FILTER,
            &mut working.effects.filter,
            &mut colors.filter,
            (|c: &ComputedStyle| c.effects.filter.clone())
                as fn(&ComputedStyle) -> FilterList<Color>,
        ),
        (
            &style.effects.backdrop_filter,
            ImportantMask::BACKDROP_FILTER,
            &mut working.effects.backdrop_filter,
            &mut colors.backdrop_filter,
            |c: &ComputedStyle| c.effects.backdrop_filter.clone(),
        ),
    ];
    for (declared, mask, target, waiting, field) in slots {
        let Some(v) = declared else { continue };
        if !matches_pass(style.important.contains(mask), important_pass) {
            continue;
        }
        match kw.resolve(v) {
            Resolved::Specified(list) => {
                *target = compute_filter(list, &vars, cx);
                *waiting = Some(list.clone());
            }
            Resolved::From(source) => {
                *target = field(source);
                *waiting = None;
            }
        }
    }
}
