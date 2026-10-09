//! Animation of computed values: each longhand's animation type, as its
//! specification's property definition table states it (Web Animations
//! 1 §5.1 "Animation types"; CSS Values 4 §3 "Combining Values"), and
//! the interpolation of its computed value on a [`ComputedStyle`] —
//! what a running transition composites onto an element's style.
//!
//! [`Longhand`] names one longhand of the dispatch table
//! ([`property_names`](crate::property_dispatch::property_names)); a
//! shorthand or a flow-relative name expands to longhands through
//! [`transition_longhands`]. Renderer-agnostic: the transition engine
//! (`rdom-tui`'s `runtime::animation`) decides *when* a value moves,
//! this module *how*.
//!
//! Whole cells: a length layout reads in cells interpolates exactly and
//! rounds onto the grid half to even, as a `calc()` result does — one
//! rounding, at the computed value (DIVERGENCES §1).

#[cfg(test)]
mod composite_tests;
mod entry;
mod filter;
mod length;
mod size;
mod table;
#[cfg(test)]
mod tests;
mod text;
mod transform;
mod value;

pub use value::{lerp_color, lerp_visibility};

use crate::ComputedStyle;
use crate::color::ColorScheme;
use crate::layout::TextDirection;
use table::LONGHANDS;

/// How a property's values combine (Web Animations 1 §5.1), as the
/// "Animation type" line of its definition gives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AnimationType {
    /// The property does not animate: a transition never starts for it.
    NotAnimatable,
    /// The values step at the midpoint (Web Animations 1 §5.3.1) — and
    /// only under `transition-behavior: allow-discrete` (CSS Transitions
    /// 2 §2). `visibility` is discrete with its own rule (CSS Display 3
    /// §4).
    Discrete,
    /// The computed values interpolate by their type (CSS Values 4 §3);
    /// a pair that cannot (`auto` against a length) is discrete.
    ByComputedValue,
    /// A list repeated to a common length, then interpolated item by
    /// item (CSS Values 4 §3.2 "repeatable list").
    RepeatableList,
    /// `box-shadow`'s "as shadow list" (CSS Backgrounds 3 §6.1).
    ShadowList,
}

/// One longhand property of the dispatch table, as an animation target:
/// a transition runs per longhand and its events name it (CSS
/// Transitions 1 §6: `propertyName` is a longhand).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Longhand(u16);

impl std::fmt::Debug for Longhand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Longhand({})", self.name())
    }
}

impl Longhand {
    fn entry(self) -> &'static entry::Entry {
        &LONGHANDS[usize::from(self.0)]
    }

    /// Every longhand, in the dispatch table's order.
    pub fn all() -> impl Iterator<Item = Longhand> {
        (0..LONGHANDS.len()).map(|i| Longhand(i as u16))
    }

    /// Its position in [`all`](Self::all), for tables indexed by
    /// longhand.
    pub fn index(self) -> usize {
        usize::from(self.0)
    }

    /// The longhand spelled `name` (ASCII case-insensitive); `None` for a
    /// shorthand, an alias or an unknown name.
    pub fn from_name(name: &str) -> Option<Longhand> {
        let name = crate::property_dispatch::canonical_property_name(name);
        LONGHANDS
            .iter()
            .position(|e| e.name == name)
            .map(|i| Longhand(i as u16))
    }

    /// The CSS name.
    pub fn name(self) -> &'static str {
        self.entry().name
    }

    /// The CSS name — what a transition event's `propertyName` carries.
    pub fn css_name(self) -> &'static str {
        self.name()
    }

    /// Its animation type.
    pub fn animation_type(self) -> AnimationType {
        self.entry().kind
    }

    /// Whether rdom keeps a computed value for it (every longhand but the
    /// background layer properties a cell cannot draw).
    pub fn has_computed_value(self) -> bool {
        self.entry().ops.is_some()
    }

    /// Whether a change of it reaches the element's descendants: it
    /// inherits, or it propagates (the text decorations, CSS Text
    /// Decoration 4 §2.1).
    pub fn reaches_descendants(self) -> bool {
        crate::property_dispatch::inherits(self.name())
            || self.name().starts_with("text-decoration")
    }

    /// Whether `a` and `b` hold different values of it.
    pub fn differs(self, a: &ComputedStyle, b: &ComputedStyle) -> bool {
        self.entry().ops.is_some_and(|ops| (ops.differs)(a, b))
    }

    /// Whether the pair `a` → `b` interpolates — a transition of it starts
    /// under `transition-behavior: normal` (CSS Transitions 2 §2: a pair
    /// that is not interpolable transitions only with `allow-discrete`).
    /// A not-animatable longhand never does.
    pub fn interpolable(self, a: &ComputedStyle, b: &ComputedStyle) -> bool {
        let entry = self.entry();
        entry.kind != AnimationType::NotAnimatable
            && entry.ops.is_some_and(|ops| (ops.interpolable)(a, b))
    }

    /// Whether it animates at all — not [`AnimationType::NotAnimatable`],
    /// and rdom keeps its value.
    pub fn is_animatable(self) -> bool {
        self.entry().kind != AnimationType::NotAnimatable && self.has_computed_value()
    }

    /// Write its value `progress` of the way from `from` to `to` into
    /// `out` (an `out` holding `to`'s other values is the usual target).
    /// `scheme` is the element's used color scheme: a `reset` color
    /// endpoint interpolates as its canvas color for the property's role.
    /// A progress outside `[0, 1]` (an overshooting easing) extrapolates
    /// where the type allows and clamps to the property's range.
    pub fn interpolate(
        self,
        from: &ComputedStyle,
        to: &ComputedStyle,
        progress: f64,
        scheme: ColorScheme,
        out: &mut ComputedStyle,
    ) {
        self.entry().blend(from, to, progress, scheme, out);
    }

    /// Whether `style` declares it — itself, through a shorthand, or
    /// through a flow-relative property mapped by `direction` (a
    /// declaration kept for substitution counts) — as a keyframe block
    /// names the longhands its keyframe animates (CSS Animations 1 §3).
    pub fn declared_in(self, style: &crate::TuiStyle, direction: TextDirection) -> bool {
        crate::property_dispatch::sets_any_field(style, self.name())
            || style
                .pending
                .iter()
                .any(|d| transition_longhands(&d.name, direction).contains(&self))
    }

    /// Whether a change of its value can move a box — anything but the
    /// colors, `opacity` and the shadows, which paint reads alone. A
    /// frame whose running animations move only paint-only longhands
    /// is painted without a layout.
    pub fn affects_layout(self) -> bool {
        !matches!(
            self.name(),
            "color"
                | "background-color"
                | "border-top-color"
                | "border-right-color"
                | "border-bottom-color"
                | "border-left-color"
                | "opacity"
                | "text-decoration-color"
                | "caret-color"
                | "caret-text-color"
                | "caret-shape"
                | "caret-animation"
                | "accent-color"
                | "appearance"
                | "resize"
                | "scrollbar-color"
                | "box-shadow"
                // CSS UI 4 §5: an outline takes no room.
                | "outline-style"
                | "outline-width"
                | "outline-color"
                | "outline-offset"
                // §4.1: the pointer's shape.
                | "cursor"
                // CSS Transforms 1 §6: the origin of what a grid does not
                // draw (rotation, scaling).
                | "transform-origin"
        )
    }

    /// Whether the change `a` → `b` of it can move a box: it
    /// [affects layout](Self::affects_layout) and its values differ — a
    /// transform's only in what a cell grid draws of it (CSS Transforms 1
    /// §2, Transforms 2 §6): its translation, and whether the box is
    /// transformed at all (a stacking context and a containing block); a
    /// filter's only in whether there is one (Filter Effects 1 §5). A
    /// spinner's turning `rotate()` and a pulsing `brightness()` move
    /// nothing.
    pub fn moves_boxes(self, a: &ComputedStyle, b: &ComputedStyle) -> bool {
        if !self.affects_layout() {
            return false;
        }
        match self.name() {
            "translate" | "rotate" | "scale" | "transform" | "filter" | "backdrop-filter" => {
                a.effects.layout_differs(&b.effects)
            }
            _ => self.differs(a, b),
        }
    }

    /// Write `underlying + value` into `out` (Web Animations 1 §5.4.4:
    /// the composite of an `add` or `accumulate` effect value — the two
    /// agree for every type rdom adds: numbers, lengths, colors), the
    /// value clamped to the property's range; `false` when the type
    /// defines no addition (a discrete value), where the composite
    /// replaces. `scheme` resolves a `reset` color as for
    /// [`interpolate`](Self::interpolate).
    pub fn add(
        self,
        underlying: &ComputedStyle,
        value: &ComputedStyle,
        scheme: ColorScheme,
        out: &mut ComputedStyle,
    ) -> bool {
        self.entry().add(underlying, value, scheme, out)
    }

    /// Copy its value from `from` into `out`.
    pub fn copy(self, from: &ComputedStyle, out: &mut ComputedStyle) {
        self.interpolate(from, from, 1.0, ColorScheme::Dark, out);
    }
}

/// The animation type of the longhand `name`; `None` for a shorthand or
/// an unknown name.
pub fn animation_type(name: &str) -> Option<AnimationType> {
    Longhand::from_name(name).map(Longhand::animation_type)
}

/// The longhands a `transition-property` entry naming `name` covers
/// (CSS Transitions 1 §2.1): a longhand itself, a shorthand's longhands,
/// a flow-relative property's physical twin under `direction`. Empty for
/// an unknown name. Built once for every name of the dispatch table.
pub fn transition_longhands(name: &str, direction: TextDirection) -> &'static [Longhand] {
    /// Per name: the longhands under `ltr`, then under `rtl`.
    type Map = std::collections::HashMap<&'static str, [Vec<Longhand>; 2]>;
    static MAP: std::sync::OnceLock<Map> = std::sync::OnceLock::new();
    let map = MAP.get_or_init(|| {
        crate::property_dispatch::property_names()
            .iter()
            .map(|&n| {
                let ltr = expand(n, TextDirection::Ltr);
                (n, [ltr, expand(n, TextDirection::Rtl)])
            })
            .collect()
    });
    let name = crate::property_dispatch::canonical_property_name(name);
    map.get(&*name).map_or(&[], |both| {
        both[usize::from(direction == TextDirection::Rtl)].as_slice()
    })
}

fn expand(name: &str, direction: TextDirection) -> Vec<Longhand> {
    if let Some(physical) = crate::property_dispatch::physical_names(name, direction) {
        return physical.iter().flat_map(|p| expand(p, direction)).collect();
    }
    Longhand::all()
        .filter(|l| crate::property_dispatch::covers(name, l.name()))
        .collect()
}
