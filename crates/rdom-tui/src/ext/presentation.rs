//! The transition engine's overrides on an element (`PresentationStyle`)
//! and which of its boxes they drive (`StyleSlot`, `PseudoSlot`).

use super::TuiExt;
use crate::layout::{Length, Padding, Size, ZIndex};
use crate::style::{Color, ComputedStyle};

/// Sparse override on top of `ComputedStyle`. Only populated for
/// properties that an active transition is currently driving.
/// Paint, layout, and hit-test read these slots before falling
/// back to `ComputedStyle` — see `effective_*` helpers below.
///
/// M3 covers the animatable subset. Discrete properties (display,
/// position, content, etc.) toggle in `ComputedStyle` directly
/// at midpoint and are not covered here.
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct PresentationStyle {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    /// `border-color`'s four sides.
    pub border_color: Option<crate::layout::Sides<Color>>,
    pub width: Option<Size>,
    pub height: Option<Size>,
    pub padding: Option<Padding>,
    /// `row-gap` / `column-gap` while a `gap` transition runs.
    pub row_gap: Option<u16>,
    pub column_gap: Option<u16>,
    pub top: Option<Length>,
    pub right: Option<Length>,
    pub bottom: Option<Length>,
    pub left: Option<Length>,
    pub z_index: Option<ZIndex>,
    /// `visibility` while a transition runs (CSS Display 3 §4: `visible`
    /// for the whole run when either end is).
    pub visibility: Option<crate::layout::Visibility>,
    /// The running transitions of registered custom properties (name
    /// without dashes → animated value). Not read by paint: the cascade
    /// applies them on top of the cascaded values
    /// (`ComputedStyle::animated_vars`) so `var()` consumers follow.
    pub custom_properties: Option<std::collections::HashMap<String, rdom_style::CustomValue>>,
}

/// Which style a transition animates: the element itself or one of
/// its generated pseudo-elements (CSS Transitions 1 §5:
/// `TransitionEvent.pseudoElement`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum StyleSlot {
    #[default]
    Host,
    Before,
    After,
    /// A list item's `::marker` (CSS Lists 3 §3.2). It has no animation
    /// overrides: its transitions do not run (DIVERGENCES §3).
    Marker,
    /// A block container's `::first-letter` (CSS Pseudo-Elements 4
    /// §2.3). It has no animation overrides, as `::marker`.
    FirstLetter,
    /// The `::marker` of a list-item `::before` (CSS Pseudo-Elements 4
    /// §4, CSS Lists 3 §3.1). No animation overrides, as `::marker`.
    BeforeMarker,
    /// The `::marker` of a list-item `::after`.
    AfterMarker,
}

impl StyleSlot {
    /// The `TransitionEvent.pseudoElement` value.
    pub fn pseudo_element(self) -> Option<&'static str> {
        match self {
            StyleSlot::Host => None,
            StyleSlot::Before => Some("::before"),
            StyleSlot::After => Some("::after"),
            StyleSlot::Marker => Some("::marker"),
            StyleSlot::FirstLetter => Some("::first-letter"),
            StyleSlot::BeforeMarker => Some("::before::marker"),
            StyleSlot::AfterMarker => Some("::after::marker"),
        }
    }
}

/// Which generated pseudo-element a piece of generated content belongs
/// to: a [`StyleSlot`] that can never be [`StyleSlot::Host`]. Laid-out
/// generated content (`GeneratedFragment::slot`) carries one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PseudoSlot {
    Before,
    After,
    /// A list item's `::marker` (CSS Lists 3 §3.2).
    Marker,
    /// A block container's `::first-letter` laid out as a box of its own
    /// — a float (CSS Pseudo-Elements 4 §2.3).
    FirstLetter,
    /// The `::marker` of a list-item `::before` (CSS Pseudo-Elements 4
    /// §4, CSS Lists 3 §3.1): it rides the `::before`'s first line.
    BeforeMarker,
    /// The `::marker` of a list-item `::after`.
    AfterMarker,
}

impl From<PseudoSlot> for StyleSlot {
    fn from(slot: PseudoSlot) -> Self {
        match slot {
            PseudoSlot::Before => StyleSlot::Before,
            PseudoSlot::After => StyleSlot::After,
            PseudoSlot::Marker => StyleSlot::Marker,
            PseudoSlot::FirstLetter => StyleSlot::FirstLetter,
            PseudoSlot::BeforeMarker => StyleSlot::BeforeMarker,
            PseudoSlot::AfterMarker => StyleSlot::AfterMarker,
        }
    }
}

impl TuiExt {
    /// The animation overrides for `slot`; `None` while no transition
    /// drives it.
    pub fn presentation_for(&self, slot: StyleSlot) -> Option<&PresentationStyle> {
        match slot {
            StyleSlot::Host => self.presentation.as_deref(),
            StyleSlot::Before => self.pseudo.as_ref()?.presentation_before.as_deref(),
            StyleSlot::After => self.pseudo.as_ref()?.presentation_after.as_deref(),
            StyleSlot::Marker
            | StyleSlot::FirstLetter
            | StyleSlot::BeforeMarker
            | StyleSlot::AfterMarker => None,
        }
    }

    /// The animation overrides for `slot`, boxed on first use; `None`
    /// for a slot that takes none (`::marker`). Transition-engine
    /// plumbing (`runtime::animation`).
    pub(crate) fn presentation_for_mut(
        &mut self,
        slot: StyleSlot,
    ) -> Option<&mut PresentationStyle> {
        self.presentation_slot(slot)
            .map(|boxed| &mut **boxed.get_or_insert_with(Default::default))
    }

    /// Drop `slot`'s override box once no property is overridden, so an
    /// element whose transitions finished is back to one `None`.
    /// Transition-engine plumbing (`runtime::animation`).
    pub(crate) fn release_empty_presentation(&mut self, slot: StyleSlot) {
        if slot != StyleSlot::Host && self.pseudo.is_none() {
            return;
        }
        if let Some(boxed) = self.presentation_slot(slot)
            && boxed.as_deref().is_some_and(PresentationStyle::is_empty)
        {
            *boxed = None;
        }
        // An empty side record goes with the last override in it.
        self.update_pseudo(false, |_| {});
    }

    /// The computed style of `slot`: the element's own, or one of its
    /// pseudo-elements' ([`computed_pseudo`](Self::computed_pseudo)).
    pub fn computed_for(&self, slot: StyleSlot) -> Option<&std::rc::Rc<ComputedStyle>> {
        match slot {
            StyleSlot::Host => self.computed.as_ref(),
            StyleSlot::Before => self.computed_before.as_ref(),
            StyleSlot::After => self.computed_after.as_ref(),
            StyleSlot::Marker => self.computed_marker(),
            StyleSlot::FirstLetter => self.computed_first_letter(),
            StyleSlot::BeforeMarker => self.computed_before_marker(),
            StyleSlot::AfterMarker => self.computed_after_marker(),
        }
    }

    /// The computed style of the `slot` pseudo-element — `::before`,
    /// `::after`, a `::marker` — `None` when it generates no box.
    pub fn computed_pseudo(&self, slot: PseudoSlot) -> Option<&std::rc::Rc<ComputedStyle>> {
        match slot {
            PseudoSlot::Before => self.computed_before.as_ref(),
            PseudoSlot::After => self.computed_after.as_ref(),
            PseudoSlot::Marker => self.computed_marker(),
            PseudoSlot::FirstLetter => self.computed_first_letter(),
            PseudoSlot::BeforeMarker => self.computed_before_marker(),
            PseudoSlot::AfterMarker => self.computed_after_marker(),
        }
    }

    fn presentation_slot(
        &mut self,
        slot: StyleSlot,
    ) -> Option<&mut Option<Box<PresentationStyle>>> {
        match slot {
            StyleSlot::Host => Some(&mut self.presentation),
            StyleSlot::Before => Some(
                &mut self
                    .pseudo
                    .get_or_insert_with(Default::default)
                    .presentation_before,
            ),
            StyleSlot::After => Some(
                &mut self
                    .pseudo
                    .get_or_insert_with(Default::default)
                    .presentation_after,
            ),
            StyleSlot::Marker
            | StyleSlot::FirstLetter
            | StyleSlot::BeforeMarker
            | StyleSlot::AfterMarker => None,
        }
    }
}

impl PresentationStyle {
    /// True when no animation is currently driving any property.
    /// The hot path uses this to skip the override read.
    pub fn is_empty(&self) -> bool {
        self.custom_properties.is_none()
            && self.fg.is_none()
            && self.bg.is_none()
            && self.border_color.is_none()
            && self.width.is_none()
            && self.height.is_none()
            && self.padding.is_none()
            && self.row_gap.is_none()
            && self.column_gap.is_none()
            && self.top.is_none()
            && self.right.is_none()
            && self.bottom.is_none()
            && self.left.is_none()
            && self.z_index.is_none()
            && self.visibility.is_none()
    }

    /// Drop every override. Called by the engine when an
    /// animation reaches its end value (so paint sees the
    /// committed `ComputedStyle` from the next cascade onward).
    pub fn clear(&mut self) {
        *self = PresentationStyle::default();
    }
}
