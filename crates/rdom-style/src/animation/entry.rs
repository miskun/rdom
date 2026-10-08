//! One row of the longhand table (`table.rs`): how a longhand's computed
//! value is compared, tested for interpolability and blended on a
//! [`ComputedStyle`] — built by the `value!` / `steps!` macros — and the
//! fix-ups that keep the companion fields a value carries (the used
//! `border`, the font modifiers, the applied text decorations) in step.

use super::AnimationType;
use super::value::{Cx, discrete, lerp_visibility};
use crate::ComputedStyle;

/// The canvas color a `reset` endpoint stands for (CSS Color Adjust 1
/// §2.1): the background's for `background-color`, the text's for every
/// other color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Role {
    Text,
    Background,
}

/// How a longhand's computed value animates on a [`ComputedStyle`].
#[derive(Clone, Copy)]
pub(super) struct Ops {
    pub differs: fn(&ComputedStyle, &ComputedStyle) -> bool,
    pub interpolable: fn(&ComputedStyle, &ComputedStyle) -> bool,
    pub blend: fn(&ComputedStyle, &ComputedStyle, f64, &Cx, &mut ComputedStyle),
}

/// One longhand.
#[derive(Clone, Copy)]
pub(super) struct Entry {
    pub name: &'static str,
    pub kind: AnimationType,
    pub role: Role,
    /// `None` where rdom keeps no computed value (a background layer
    /// property other than `background-color` / `-clip`: a cell has no
    /// image).
    pub ops: Option<Ops>,
}

/// A value that interpolates by its type (`Animate`).
macro_rules! value {
    ($($f:ident).+ $(=> $fix:path)?) => {
        Some(Ops {
            differs: |a, b| a.$($f).+ != b.$($f).+,
            interpolable: |a, b| interpolable(&a.$($f).+, &b.$($f).+),
            blend: |a, b, p, cx, out| {
                out.$($f).+ = blend(&a.$($f).+, &b.$($f).+, p, cx);
                $($fix(out);)?
            },
        })
    };
}

/// Discrete values: every listed field from the same side.
macro_rules! steps {
    ($($($f:ident).+),+ $(=> $fix:path)?) => {
        Some(Ops {
            differs: |a, b| false $(|| a.$($f).+ != b.$($f).+)+,
            interpolable: |_, _| false,
            blend: |a, b, p, _, out| {
                let side = discrete(&a, &b, p);
                $(out.$($f).+ = side.$($f).+.clone();)+
                $($fix(out);)?
            },
        })
    };
}

pub(super) use {steps, value};

/// `width` / `height`: by computed value, a sizing keyword through
/// `calc-size()` under the element's `interpolate-size` (CSS Values 5
/// §11, read on the after-change style).
macro_rules! size {
    ($f:ident) => {
        Some(Ops {
            differs: |a, b| a.$f != b.$f,
            interpolable: |a, b| super::size::size_interpolable(&a.$f, &b.$f, b.interpolate_size),
            blend: |a, b, p, cx, out| {
                out.$f = super::size::blend_size(&a.$f, &b.$f, p, b.interpolate_size, cx)
            },
        })
    };
}
pub(super) use size;

pub(super) const fn e(name: &'static str, kind: AnimationType, ops: Option<Ops>) -> Entry {
    Entry {
        name,
        kind,
        role: Role::Text,
        ops,
    }
}

/// The used border: the styles with every zero-width side `none`.
pub(super) fn fix_border(out: &mut ComputedStyle) {
    out.border = out.border_style.with_widths(&out.border_width);
}

/// The modifier bits the font draws (bold from weight 600, italic).
pub(super) fn fix_font(out: &mut ComputedStyle) {
    let bold = out.font.weight.is_bold(400.0);
    out.modifiers.set(crate::Modifier::BOLD, bold);
    out.modifiers
        .set(crate::Modifier::ITALIC, out.font.style.is_italic());
}

/// The element's own lines among its applied decorations take its
/// decoration's style and color.
pub(super) fn fix_decorations(out: &mut ComputedStyle) {
    let own = &out.text_decoration;
    let line = crate::layout::AppliedLine {
        style: own.style,
        color: own.color,
    };
    let applied = &mut out.applied_decorations;
    for (on, slot) in [
        (own.line.underline, &mut applied.underline),
        (own.line.overline, &mut applied.overline),
        (own.line.line_through, &mut applied.line_through),
    ] {
        if on {
            *slot = Some(line);
        }
    }
}

/// `display` (CSS Display 4 §2.9, Web Animations 2's display rule):
/// discrete, but a transition to or from `none` shows the other value for
/// every progress strictly between 0 and 1 — `none` only at its end. The
/// value's companions (inner type, list item, the `-webkit-box` and BFC
/// bits) come from the side shown.
pub(super) const DISPLAY: Option<Ops> = Some(Ops {
    differs: |a, b| {
        a.display != b.display
            || a.flow != b.flow
            || a.list_item != b.list_item
            || a.webkit_box != b.webkit_box
    },
    interpolable: |_, _| false,
    blend: |a, b, p, _, out| {
        use crate::layout::Display;
        let side = if p <= 0.0 {
            a
        } else if p >= 1.0 || a.display == Display::None {
            b
        } else if b.display == Display::None {
            a
        } else {
            discrete(&a, &b, p)
        };
        out.display = side.display;
        out.flow = side.flow;
        out.list_item = side.list_item;
        out.webkit_box = side.webkit_box;
        out.establishes_new_bfc = side.establishes_new_bfc;
        out.line_clamp_container = side.line_clamp_container;
    },
});

/// `overlay` (CSS Position 4 §3.4): discrete, but — as `display` with
/// `none` — `auto` for every progress strictly between 0 and 1 when an
/// end is `auto`, so a transition keeps the element in the top layer to
/// its end.
pub(super) const OVERLAY: Option<Ops> = Some(Ops {
    differs: |a, b| a.overlay != b.overlay,
    interpolable: |_, _| false,
    blend: |a, b, p, _, out| {
        use crate::layout::Overlay;
        out.overlay = if p <= 0.0 {
            a.overlay
        } else if p >= 1.0 {
            b.overlay
        } else if a.overlay == Overlay::Auto || b.overlay == Overlay::Auto {
            Overlay::Auto
        } else {
            discrete(&a.overlay, &b.overlay, p)
        };
    },
});

/// `visibility` (CSS Display 3 §4, Web Animations 1 §5.3.2): discrete,
/// but `visible` for the whole interval when an end is.
pub(super) const VISIBILITY: Option<Ops> = Some(Ops {
    differs: |a, b| a.visibility != b.visibility,
    interpolable: |a, b| a.visibility.is_visible() || b.visibility.is_visible(),
    blend: |a, b, p, _, out| out.visibility = lerp_visibility(a.visibility, b.visibility, p as f32),
});

/// CSS Color 4 §13: `opacity` clamps to `[0, 1]` (an overshooting easing
/// may leave it).
pub(super) fn fix_opacity(out: &mut ComputedStyle) {
    out.opacity = out.opacity.clamp(0.0, 1.0);
}

/// CSS Flexbox §7.3: the flex factors are never negative.
pub(super) fn fix_flex(out: &mut ComputedStyle) {
    out.flex_grow = out.flex_grow.max(0.0);
    out.flex_shrink = out.flex_shrink.max(0.0);
}

impl Entry {
    /// Interpolate the longhand `from` → `to` at `progress` into `out`.
    pub(super) fn blend(
        &self,
        from: &ComputedStyle,
        to: &ComputedStyle,
        progress: f64,
        scheme: crate::color::ColorScheme,
        out: &mut ComputedStyle,
    ) {
        let Some(ops) = self.ops else {
            return;
        };
        let (background, text) = scheme.canvas();
        let reset = match self.role {
            Role::Background => background,
            Role::Text => text,
        };
        (ops.blend)(from, to, progress, &Cx { reset }, out);
    }
}
