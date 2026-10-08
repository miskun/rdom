//! The transition engine's overrides on an element (`PresentationStyle`)
//! and which of its boxes they drive (`StyleSlot`, `PseudoSlot`).

use std::rc::Rc;

use rdom_style::animation::Longhand;

use super::TuiExt;
use crate::style::ComputedStyle;

/// What running transitions do to one of an element's styles (CSS
/// Transitions 1 §3, Web Animations 1 §5.4.5): the animated value of a
/// longhand is its computed value, so while a transition runs the slot's
/// computed style ([`TuiExt::computed_for`]) is the cascade's style with
/// the running values composited on — what layout, paint, hit-testing
/// and inheritance read. This record keeps the cascade's own style (the
/// *after-change style* the next style change is compared with) and the
/// longhands composited.
///
/// `None` on an element while no transition drives the slot.
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct PresentationStyle {
    /// The cascade's style for the slot, without the running values; set
    /// while [`animated`](Self::animated) is not empty.
    base: Option<Rc<ComputedStyle>>,
    /// The longhands running transitions composite onto the slot.
    animated: Vec<Longhand>,
    /// The running transitions of registered custom properties (name
    /// without dashes → animated value). The cascade applies them on top
    /// of the cascaded values (`ComputedStyle::animated_vars`) so `var()`
    /// consumers follow.
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
    /// True when no transition drives the slot.
    pub fn is_empty(&self) -> bool {
        self.custom_properties.is_none() && self.animated.is_empty()
    }

    /// The longhands running transitions composite onto the slot's
    /// computed style.
    pub fn animated(&self) -> &[Longhand] {
        &self.animated
    }

    /// The cascade's style for the slot under the running values, while
    /// any longhand animates.
    pub fn cascaded(&self) -> Option<&Rc<ComputedStyle>> {
        self.base.as_ref()
    }
}

impl TuiExt {
    /// The cascade's style for `slot` — its computed style without the
    /// running transitions' values: the *after-change style* of CSS
    /// Transitions 1 §3. The computed style itself while nothing runs.
    pub fn cascaded_for(&self, slot: StyleSlot) -> Option<&Rc<ComputedStyle>> {
        self.presentation_for(slot)
            .and_then(PresentationStyle::cascaded)
            .or_else(|| self.computed_for(slot))
    }

    /// `fresh`, a new cascade result for `slot`, with the values the
    /// slot's running transitions hold now (carried from its current
    /// computed style) — `None` while none runs. The cascade's write-back
    /// (`style::cascade`).
    pub(crate) fn overlay(&self, slot: StyleSlot, fresh: &ComputedStyle) -> Option<ComputedStyle> {
        let animated = self.presentation_for(slot)?.animated();
        if animated.is_empty() {
            return None;
        }
        let current = self.computed_for(slot)?;
        let mut out = fresh.clone();
        for l in animated {
            l.copy(current, &mut out);
        }
        Some(out)
    }

    /// Store a cascade result for `slot`: `fresh` as the cascade's style
    /// and `overlaid` ([`overlay`](Self::overlay)) as the computed style
    /// while transitions run.
    pub(crate) fn set_cascaded(
        &mut self,
        slot: StyleSlot,
        fresh: Rc<ComputedStyle>,
        overlaid: Option<Rc<ComputedStyle>>,
    ) {
        match overlaid {
            Some(style) => {
                if let Some(p) = self.presentation_slot(slot).and_then(|p| p.as_deref_mut()) {
                    p.base = Some(fresh);
                }
                self.put_computed(slot, Some(style));
            }
            None => self.put_computed(slot, Some(fresh)),
        }
    }

    /// Keep `fresh` as the cascade's style for `slot` and its computed
    /// style as it is — a restyle that left the composited style alone.
    pub(crate) fn keep_cascaded(&mut self, slot: StyleSlot, fresh: Rc<ComputedStyle>) {
        if self.presentation_for(slot).is_none() {
            return;
        }
        if let Some(p) = self.presentation_slot(slot).and_then(|p| p.as_deref_mut())
            && !p.animated.is_empty()
        {
            p.base = Some(fresh);
        }
    }

    /// Composite the running transitions onto `slot`: `style`, the
    /// cascade's style with the values of `animated` written on, becomes
    /// the computed style; with nothing animated the cascade's style
    /// comes back. Transition-engine plumbing (`runtime::animation`).
    pub(crate) fn composite(
        &mut self,
        slot: StyleSlot,
        animated: Vec<Longhand>,
        style: Option<ComputedStyle>,
    ) {
        let Some(base) = self.cascaded_for(slot).cloned() else {
            return;
        };
        match style.filter(|_| !animated.is_empty()) {
            Some(style) => {
                let Some(p) = self.presentation_for_mut(slot) else {
                    return;
                };
                p.base = Some(base);
                p.animated = animated;
                self.put_computed(slot, Some(Rc::new(style)));
            }
            None => {
                if self.presentation_for(slot).is_none() {
                    return;
                }
                if let Some(p) = self.presentation_slot(slot).and_then(|p| p.as_deref_mut()) {
                    p.base = None;
                    p.animated.clear();
                }
                self.put_computed(slot, Some(base));
                self.release_empty_presentation(slot);
            }
        }
    }

    /// Forget what the transition engine keeps for this element's boxes
    /// once it leaves the document (C12G-DETACHED): the composited values
    /// (each slot's computed style goes back to the cascade's) and the
    /// before-change styles — an element no longer rendered has none (CSS
    /// Transitions 1 §3), so it is rendered afresh when inserted again.
    pub(crate) fn forget_rendering(&mut self) {
        for slot in [StyleSlot::Host, StyleSlot::Before, StyleSlot::After] {
            if let Some(base) = self.presentation_for(slot).and_then(|p| p.base.clone()) {
                self.put_computed(slot, Some(base));
            }
        }
        self.presentation = None;
        self.computed_prev = None;
        self.update_pseudo(false, |p| {
            p.presentation_before = None;
            p.presentation_after = None;
            p.before_prev = None;
            p.after_prev = None;
        });
    }

    fn put_computed(&mut self, slot: StyleSlot, style: Option<Rc<ComputedStyle>>) {
        match slot {
            StyleSlot::Host => self.computed = style,
            StyleSlot::Before => self.computed_before = style,
            StyleSlot::After => self.computed_after = style,
            // No transition drives the other slots (`presentation_slot`).
            StyleSlot::Marker
            | StyleSlot::FirstLetter
            | StyleSlot::BeforeMarker
            | StyleSlot::AfterMarker => {}
        }
    }
}
