//! The rarely set pseudo-element state of an element, kept apart from
//! [`TuiExt`] so the common element — no list marker, no first line or
//! letter, no scrollbar, no backdrop, no running
//! pseudo-element transition — pays one pointer for all of it
//! (C10G-TUIEXT-SIDE). The `::highlight()` styles stay on `TuiExt`: a
//! `*::highlight()` rule gives every element one, shared with its
//! parent's, which a box per element would undo. Read through the accessors on `TuiExt`
//! ([`computed_pseudo`](TuiExt::computed_pseudo),
//! [`computed_marker`](TuiExt::computed_marker), …); the cascade and the
//! transition engine write it.

use std::rc::Rc;

use super::{PresentationStyle, TuiExt};
use crate::style::ComputedStyle;

/// An element's rarely set pseudo-element styles and transition state,
/// boxed on [`TuiExt`] only while one of them is set.
#[derive(Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct PseudoStyles {
    /// `::marker` (CSS Lists 3 §3.2): a list item whose marker has
    /// content; its `content` holds the marker text.
    pub marker: Option<Rc<ComputedStyle>>,
    /// `::before::marker` / `::after::marker` (CSS Pseudo-Elements 4 §4):
    /// the marker of a `::before` / `::after` that is a list item, as
    /// [`marker`](Self::marker).
    pub before_marker: Option<Rc<ComputedStyle>>,
    pub after_marker: Option<Rc<ComputedStyle>>,
    /// `::first-line` (CSS Pseudo-Elements 4 §2.2) of a block container a
    /// rule styles it on.
    pub first_line: Option<Rc<ComputedStyle>>,
    /// `::first-letter` (§2.3), inheriting from `::first-line`.
    pub first_letter: Option<Rc<ComputedStyle>>,
    /// `::details-content` (HTML §15.5.20) of a `<details>` element.
    pub details_content: Option<Rc<ComputedStyle>>,
    /// `::backdrop` of a modal `<dialog>` a rule styles it on.
    pub backdrop: Option<Rc<ComputedStyle>>,
    /// `::scrollbar` of a box whose overflow shows a bar.
    pub scrollbar: Option<Rc<ComputedStyle>>,
    /// `::scrollbar-thumb` of the vertical bar.
    pub scrollbar_thumb_vertical: Option<Rc<ComputedStyle>>,
    /// `::scrollbar-thumb` of the horizontal bar.
    pub scrollbar_thumb_horizontal: Option<Rc<ComputedStyle>>,
    /// The previous cascade's `::before` / `::after` styles, which the
    /// transition engine diffs against (`D-M3-3`).
    pub before_prev: Option<Rc<ComputedStyle>>,
    pub after_prev: Option<Rc<ComputedStyle>>,
    /// The running transitions' overrides of `::before` / `::after`.
    pub presentation_before: Option<Box<PresentationStyle>>,
    pub presentation_after: Option<Box<PresentationStyle>>,
    /// The two ends of a `::details-content` box (`render::box_tree::slot`):
    /// on a `<details>`, the node that is its box; on that node, its
    /// `<details>`.
    pub(crate) content_box: ContentBoxLink,
}

/// Which end of a `::details-content` box an element is
/// ([`PseudoStyles::content_box`]).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContentBoxLink {
    #[default]
    None,
    /// The element is a `<details>`; this node is its slot's box.
    Box(rdom_core::NodeId),
    /// The element is the box of this `<details>`'s slot.
    HostedBy(rdom_core::NodeId),
}

impl PseudoStyles {
    /// Nothing is set: the record can go.
    fn is_empty(&self) -> bool {
        *self == PseudoStyles::default()
    }
}

impl TuiExt {
    /// The rarely set pseudo-element styles, `None` while none is set.
    pub fn pseudo_styles(&self) -> Option<&PseudoStyles> {
        self.pseudo.as_deref()
    }

    /// Change the record with `f`: created when `sets` (`f` writes a
    /// value), dropped again when `f` left it empty — so writing `None`s
    /// to an element without one allocates nothing.
    pub(crate) fn update_pseudo(&mut self, sets: bool, f: impl FnOnce(&mut PseudoStyles)) {
        if self.pseudo.is_none() && !sets {
            return;
        }
        let record = self.pseudo.get_or_insert_with(Default::default);
        f(record);
        if record.is_empty() {
            self.pseudo = None;
        }
    }

    /// Keep this cascade's `::before` / `::after` styles as the previous
    /// ones the next diff reads (`runtime::animation`).
    pub(crate) fn snapshot_pseudo_prev(&mut self) {
        let (before, after) = (self.computed_before.clone(), self.computed_after.clone());
        self.update_pseudo(before.is_some() || after.is_some(), |p| {
            p.before_prev = before;
            p.after_prev = after;
        });
    }

    /// `::marker`'s computed style (CSS Lists 3 §3.2).
    pub fn computed_marker(&self) -> Option<&Rc<ComputedStyle>> {
        self.pseudo.as_ref()?.marker.as_ref()
    }

    /// `::before::marker`'s computed style (CSS Pseudo-Elements 4 §4).
    pub fn computed_before_marker(&self) -> Option<&Rc<ComputedStyle>> {
        self.pseudo.as_ref()?.before_marker.as_ref()
    }

    /// `::after::marker`'s computed style.
    pub fn computed_after_marker(&self) -> Option<&Rc<ComputedStyle>> {
        self.pseudo.as_ref()?.after_marker.as_ref()
    }

    /// `::first-line`'s computed style (CSS Pseudo-Elements 4 §2.2).
    pub fn computed_first_line(&self) -> Option<&Rc<ComputedStyle>> {
        self.pseudo.as_ref()?.first_line.as_ref()
    }

    /// `::first-letter`'s computed style (§2.3).
    pub fn computed_first_letter(&self) -> Option<&Rc<ComputedStyle>> {
        self.pseudo.as_ref()?.first_letter.as_ref()
    }

    /// `::details-content`'s computed style (HTML §15.5.20).
    pub fn computed_details_content(&self) -> Option<&Rc<ComputedStyle>> {
        self.pseudo.as_ref()?.details_content.as_ref()
    }

    /// Which end of a `::details-content` box this element is.
    pub(crate) fn content_box_link(&self) -> ContentBoxLink {
        self.pseudo
            .as_ref()
            .map_or(ContentBoxLink::None, |p| p.content_box)
    }

    /// Set which end of a `::details-content` box this element is.
    pub(crate) fn set_content_box_link(&mut self, link: ContentBoxLink) {
        self.update_pseudo(link != ContentBoxLink::None, |p| p.content_box = link);
    }

    /// `::backdrop`'s computed style.
    pub fn computed_backdrop(&self) -> Option<&Rc<ComputedStyle>> {
        self.pseudo.as_ref()?.backdrop.as_ref()
    }

    /// `::scrollbar`'s computed style.
    pub fn computed_scrollbar(&self) -> Option<&Rc<ComputedStyle>> {
        self.pseudo.as_ref()?.scrollbar.as_ref()
    }

    /// The vertical bar's `::scrollbar-thumb` computed style.
    pub fn computed_scrollbar_thumb_vertical(&self) -> Option<&Rc<ComputedStyle>> {
        self.pseudo.as_ref()?.scrollbar_thumb_vertical.as_ref()
    }

    /// The horizontal bar's `::scrollbar-thumb` computed style.
    pub fn computed_scrollbar_thumb_horizontal(&self) -> Option<&Rc<ComputedStyle>> {
        self.pseudo.as_ref()?.scrollbar_thumb_horizontal.as_ref()
    }

    /// The previous cascade's `::before` style (`D-M3-3`).
    pub fn computed_before_prev(&self) -> Option<&Rc<ComputedStyle>> {
        self.pseudo.as_ref()?.before_prev.as_ref()
    }

    /// The previous cascade's `::after` style (`D-M3-3`).
    pub fn computed_after_prev(&self) -> Option<&Rc<ComputedStyle>> {
        self.pseudo.as_ref()?.after_prev.as_ref()
    }
}
