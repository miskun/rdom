//! The pseudo-element boxes an element's cascade computes before its
//! children (`walk::style_element`): `::marker`, `::before`, `::backdrop`,
//! `::selection`, the scrollbar parts, `::first-line` and
//! `::first-letter`. `::after` waits for the children
//! (`walk::finish_element`): a `counter()` in it sees their increments.

use super::matching::{MatchedRules, Recorder, Slot};
use super::pseudo::{before_targets, compute_pseudo_style};
use super::walk::{ElementCx, compute_box};
use crate::ext::TuiExt;
use crate::style::{ComputedStyle, PseudoElementTarget};

/// The early pseudo-element styles of one element; `None` where it
/// generates no such box.
pub(super) struct EarlyPseudos {
    pub(super) marker: Option<ComputedStyle>,
    pub(super) before: Option<ComputedStyle>,
    backdrop: Option<ComputedStyle>,
    selection: Option<ComputedStyle>,
    scrollbar: Option<ComputedStyle>,
    thumb_vertical: Option<ComputedStyle>,
    thumb_horizontal: Option<ComputedStyle>,
    first_line: Option<ComputedStyle>,
    first_letter: Option<ComputedStyle>,
}

impl EarlyPseudos {
    /// Store them on the element's `ext` (all but `::before`, which the
    /// caller keeps for after the children).
    pub(super) fn write(self, ext: &mut TuiExt) {
        use std::rc::Rc;
        ext.computed_marker = self.marker.map(Rc::new);
        ext.computed_backdrop = self.backdrop.map(Rc::new);
        ext.computed_selection = self.selection.map(Rc::new);
        ext.computed_first_line = self.first_line.map(Rc::new);
        ext.computed_first_letter = self.first_letter.map(Rc::new);
        ext.computed_scrollbar = self.scrollbar.map(Rc::new);
        ext.computed_scrollbar_thumb_vertical = self.thumb_vertical.map(Rc::new);
        ext.computed_scrollbar_thumb_horizontal = self.thumb_horizontal.map(Rc::new);
    }
}

/// The early pseudo-element styles of the element in `cx`, styled
/// `computed`, from `cached` matches or by matching (recorded into
/// `recorder`).
pub(super) fn compute(
    cx: &mut ElementCx<'_, '_>,
    computed: &ComputedStyle,
    cached: Option<&MatchedRules>,
    recorder: &mut Recorder,
) -> EarlyPseudos {
    let (dom, id) = (cx.dom, cx.id);
    let mut pseudo = |cx: &mut ElementCx<'_, '_>,
                      slot,
                      parent: &ComputedStyle,
                      targets: &[PseudoElementTarget]| {
        compute_box(cx, slot, cached, recorder, |cx, rules| {
            compute_pseudo_style(cx, parent, targets, rules)
        })
    };
    // A list item's `::marker` precedes its `::before` (CSS
    // Pseudo-Elements 4 §3.1): its counter reads come first.
    let marker = if computed.list_item {
        pseudo(cx, Slot::Marker, computed, &[PseudoElementTarget::Marker])
    } else {
        None
    };
    let before = pseudo(cx, Slot::Before, computed, before_targets(dom, id));
    let backdrop = pseudo(
        cx,
        Slot::Backdrop,
        computed,
        &[PseudoElementTarget::Backdrop],
    );
    let selection = pseudo(
        cx,
        Slot::Selection,
        computed,
        &[PseudoElementTarget::Selection],
    );
    // Scrollbar pseudos only computed for elements that actually
    // have non-`Visible` overflow on at least one axis — saves a
    // selector-matching pass per element on the (very common)
    // non-scrollable case.
    let shows_bar = |o: crate::layout::Overflow| {
        matches!(
            o,
            crate::layout::Overflow::Scroll | crate::layout::Overflow::Auto
        )
    };
    let needs_scrollbar = shows_bar(computed.overflow_x) || shows_bar(computed.overflow_y);
    let (scrollbar, thumb_vertical, thumb_horizontal) = if needs_scrollbar {
        (
            pseudo(
                cx,
                Slot::Scrollbar,
                computed,
                &[PseudoElementTarget::Scrollbar],
            ),
            pseudo(
                cx,
                Slot::ThumbVertical,
                computed,
                &PseudoElementTarget::thumb_targets(true),
            ),
            pseudo(
                cx,
                Slot::ThumbHorizontal,
                computed,
                &PseudoElementTarget::thumb_targets(false),
            ),
        )
    } else {
        (None, None, None)
    };
    // `::first-line` and `::first-letter` (CSS Pseudo-Elements 4 §2.2,
    // §2.3) exist on block containers, and are matched only where a
    // sheet has such rules; `::first-letter` inherits from
    // `::first-line` (§2.3.1, the fictional tag sequence).
    let (styles_line, styles_letter) = cx.sheets.styles_first();
    let block_container = is_block_container(computed);
    let first_line = (block_container && styles_line)
        .then(|| {
            pseudo(
                cx,
                Slot::FirstLine,
                computed,
                &[PseudoElementTarget::FirstLine],
            )
        })
        .flatten();
    let first_letter = (block_container && styles_letter)
        .then(|| {
            let parent = first_line.as_ref().unwrap_or(computed);
            pseudo(
                cx,
                Slot::FirstLetter,
                parent,
                &[PseudoElementTarget::FirstLetter],
            )
        })
        .flatten();
    EarlyPseudos {
        marker,
        before,
        backdrop,
        selection,
        scrollbar,
        thumb_vertical,
        thumb_horizontal,
        first_line,
        first_letter,
    }
}

/// Whether a box styled `computed` is a block container — the boxes
/// `::first-line` and `::first-letter` apply to (CSS Pseudo-Elements 4
/// §2.2, §2.3): one whose inner display lays its children out in block
/// flow, not inline, box-less or a flex or grid container.
fn is_block_container(computed: &ComputedStyle) -> bool {
    use crate::layout::Display;
    computed.flow.is_block_flow()
        && !matches!(
            computed.display,
            Display::Inline | Display::Contents | Display::None
        )
}
