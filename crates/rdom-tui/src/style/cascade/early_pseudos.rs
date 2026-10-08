//! The pseudo-element boxes an element's cascade computes before its
//! children (`walk::style_element`): `::marker`, `::before`, `::backdrop`,
//! `::selection`, the scrollbar parts, `::first-line`,
//! `::first-letter`, `::details-content` and `::highlight()`. `::after` waits for the children
//! (`walk::finish_element`): a `counter()` in it sees their increments.

use super::matching::{MatchedRules, Recorder, Rules, Slot};
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
    details_content: Option<ComputedStyle>,
    highlights: Option<crate::ext::HighlightStyles>,
}

impl EarlyPseudos {
    /// Store them on the element's `ext` (all but `::before`, which the
    /// caller keeps for after the children).
    pub(super) fn write(self, ext: &mut TuiExt) {
        use std::rc::Rc;
        ext.computed_selection = self.selection.map(Rc::new);
        ext.computed_highlights = self.highlights;
        let sets = self.marker.is_some()
            || self.backdrop.is_some()
            || self.first_line.is_some()
            || self.first_letter.is_some()
            || self.details_content.is_some()
            || self.scrollbar.is_some()
            || self.thumb_vertical.is_some()
            || self.thumb_horizontal.is_some();
        ext.update_pseudo(sets, |p| {
            p.marker = self.marker.map(Rc::new);
            p.backdrop = self.backdrop.map(Rc::new);
            p.first_line = self.first_line.map(Rc::new);
            p.first_letter = self.first_letter.map(Rc::new);
            p.details_content = self.details_content.map(Rc::new);
            p.scrollbar = self.scrollbar.map(Rc::new);
            p.scrollbar_thumb_vertical = self.thumb_vertical.map(Rc::new);
            p.scrollbar_thumb_horizontal = self.thumb_horizontal.map(Rc::new);
        });
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
    // `::details-content` (HTML §15.5.20): a `<details>` element's slot.
    let details_content = (dom.node(id).tag_name() == Some("details"))
        .then(|| {
            pseudo(
                cx,
                Slot::DetailsContent,
                computed,
                &[PseudoElementTarget::DetailsContent],
            )
        })
        .flatten();
    let highlights = highlight_styles(cx, computed, cached, recorder);
    EarlyPseudos {
        details_content,
        highlights,
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

/// `::highlight(name)` (CSS Custom Highlight API 1 §5.1), for each name
/// the sheets style: from `cached` matches or by matching (recorded into
/// `recorder`, the same matches shared across elements). A style equal to
/// the parent element's for that name is the parent's, and a list equal to
/// the parent's is the parent's: a `*::highlight(search)` rule over a
/// document allocates per element only where its style differs.
fn highlight_styles(
    cx: &mut ElementCx<'_, '_>,
    computed: &ComputedStyle,
    cached: Option<&MatchedRules>,
    recorder: &mut Recorder,
) -> Option<crate::ext::HighlightStyles> {
    use std::rc::Rc;
    let names = cx.sheets.highlight_names();
    if names.is_empty() {
        return None;
    }
    let parent = cx
        .dom
        .node(cx.id)
        .parent_node()
        .and_then(|p| p.ext())
        .and_then(|e| e.computed_highlights.clone());
    let mut refs = std::mem::take(&mut cx.scratch.highlight_buf);
    let mut styles = std::mem::take(&mut cx.scratch.highlight_styles);
    refs.clear();
    styles.clear();
    for (k, name) in names.iter().enumerate() {
        let rules = cached.map_or(Rules::Match, |m| m.highlight_rules(k));
        let target = [PseudoElementTarget::Highlight(name.clone())];
        let style = compute_pseudo_style(cx, computed, &target, rules);
        if let Rules::Match = rules {
            refs.push(cx.scratch.intern_highlight(k));
        }
        if let Some(style) = style {
            let shared = parent
                .as_deref()
                .and_then(|p| p.iter().find(|(n, _)| n == name))
                .filter(|(_, s)| **s == style)
                .map(|(_, s)| s.clone());
            styles.push((name.clone(), shared.unwrap_or_else(|| Rc::new(style))));
        }
    }
    if refs.len() == names.len() {
        recorder.record_highlights(cx.scratch.intern_highlights(&refs));
    }
    let out = match &parent {
        _ if styles.is_empty() => None,
        Some(p)
            if p.len() == styles.len()
                && p.iter().zip(&styles).all(|(a, b)| Rc::ptr_eq(&a.1, &b.1)) =>
        {
            Some(p.clone())
        }
        _ => Some(Rc::new(styles.clone())),
    };
    cx.scratch.highlight_buf = refs;
    cx.scratch.highlight_styles = styles;
    out
}

/// Whether a box styled `computed` is a block container — the boxes
/// `::first-line` and `::first-letter` apply to (CSS Pseudo-Elements 4
/// §2.2, §2.3): one whose inner display lays its children out in block
/// flow, not inline, box-less or a flex or grid container.
pub(crate) fn is_block_container(computed: &ComputedStyle) -> bool {
    use crate::layout::Display;
    computed.flow.is_block_flow()
        && !matches!(
            computed.display,
            Display::Inline | Display::Contents | Display::None
        )
}
