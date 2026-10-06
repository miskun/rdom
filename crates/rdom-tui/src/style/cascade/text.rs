//! The applicator of the CSS Text properties (CSS Text 3 / 4) and
//! `line-height` (CSS Inline 3): each one computes to its declared value,
//! into the `ComputedStyle::text` group — `line-height` and the
//! `match-parent` alignments fixed up once the ladder has run.

use rdom_core::Dom;
use rdom_style::calc::{UnitContext, Viewport};

use super::apply::{Keywords, apply_value};
use crate::ext::TuiExt;
use crate::layout::{TextAlign, TextAlignLast, TextDirection};
use crate::node::TuiNodeExt;
use crate::style::{ComputedStyle, ImportantMask, TuiStyle};

/// Apply `style`'s CSS Text declarations to `working`, for one ladder
/// pass.
pub(super) fn apply_text(
    working: &mut ComputedStyle,
    style: &TuiStyle,
    important_pass: bool,
    kw: &Keywords<'_>,
) {
    macro_rules! text {
        ($($field:ident: $mask:ident),* $(,)?) => {$(
            apply_value(
                &mut working.text.$field,
                &style.text.$field,
                style.important.contains(ImportantMask::$mask),
                important_pass,
                kw,
                |c| &c.text.$field,
            );
        )*};
    }
    text!(
        white_space_collapse: WHITE_SPACE_COLLAPSE,
        text_wrap_mode: TEXT_WRAP_MODE,
        word_break: WORD_BREAK,
        overflow_wrap: OVERFLOW_WRAP,
        line_break: LINE_BREAK,
        hyphens: HYPHENS,
        tab_size: TAB_SIZE,
        text_transform: TEXT_TRANSFORM,
        text_indent: TEXT_INDENT,
        text_align_all: TEXT_ALIGN_ALL,
        text_align_last: TEXT_ALIGN_LAST,
        text_justify: TEXT_JUSTIFY,
        text_wrap_style: TEXT_WRAP_STYLE,
        line_height: LINE_HEIGHT,
        text_underline_offset: TEXT_UNDERLINE_OFFSET,
        text_underline_position: TEXT_UNDERLINE_POSITION,
        text_decoration_skip_ink: TEXT_DECORATION_SKIP_INK,
    );
    // CSS 2.1 §10.8.1: `vertical-align` (not inherited).
    apply_value(
        &mut working.vertical_align,
        &style.vertical_align,
        style.important.contains(ImportantMask::VERTICAL_ALIGN),
        important_pass,
        kw,
        |c| &c.vertical_align,
    );
}

/// `line-height`'s computed value, and `vertical-align`'s, which reads it
/// (CSS Inline 3 §5.1: a percentage of
/// the font size — one row — or a length, in rows; CSS Values 4 §6.1.1:
/// `lh` in it is the parent's line height, `rlh` the root's, or on the
/// root the initial one row), and the context the element's other
/// lengths resolve their units in: the viewport, its own line height for
/// `lh`, the root's for `rlh` (its own on the root). `root_rows` is the
/// root element's used line height, `None` on the root itself.
pub(super) fn finalize_line_height(
    working: &mut ComputedStyle,
    parent: &ComputedStyle,
    root_rows: Option<u16>,
    viewport: Viewport,
) -> UnitContext {
    let parent_rows = f64::from(parent.text.line_height.rows());
    let rlh = root_rows.map_or(1.0, f64::from);
    let parent_cx = UnitContext::new(viewport).with_line_heights(parent_rows, rlh);
    working.text.line_height = working.text.line_height.computed(&parent_cx);
    let rows = working.text.line_height.rows();
    let own = f64::from(rows);
    let units = UnitContext::new(viewport).with_line_heights(own, root_rows.map_or(own, f64::from));
    // CSS 2.1 §10.8.1: a `vertical-align` percentage is of the element's
    // own line height.
    working.vertical_align = working.vertical_align.computed(rows, &units);
    units
}

/// The used line height of the document's root element, the basis of
/// `rlh` for every other element.
pub(super) fn root_line_height(dom: &Dom<TuiExt>) -> u16 {
    dom.document_element()
        .computed()
        .map_or(1, |c| c.text.line_height.rows())
}

/// `text-align-all` / `text-align-last: match-parent`'s computed value
/// (CSS Text 3 §6.1): the parent's, `start` / `end` resolved against the
/// parent's `direction` to `left` / `right`; on the root element (no
/// element parent), `start`.
pub(super) fn finalize_text_align(working: &mut ComputedStyle, parent: &ComputedStyle, root: bool) {
    let rtl = parent.text_direction == TextDirection::Rtl;
    let text = &mut working.text;
    if text.text_align_all == TextAlign::MatchParent {
        text.text_align_all = if root {
            TextAlign::Start
        } else {
            parent.text.text_align_all.physical(rtl)
        };
    }
    if text.text_align_last == TextAlignLast::MatchParent {
        text.text_align_last = if root {
            TextAlignLast::Start
        } else {
            parent
                .text
                .text_align_last
                .align()
                .map_or(TextAlignLast::Auto, |a| TextAlignLast::of(a.physical(rtl)))
        };
    }
}
