//! C10-FIRST — CSS Pseudo-Elements 4 §2.2 `::first-line` and §2.3
//! `::first-letter`: the first formatted line of a block container and
//! its first typographic letter unit, styled through the packer's style
//! switch at the line / letter boundary — the DOM text is never split,
//! so selection, the caret, copy and hit-testing stay on the source.

use rdom_tui::layout::{Float, TextCase};
use rdom_tui::prelude::*;

use super::{el, lay_out, text_el};

/// §2.2.1 / §2.3.1: `::first-line` exists on a block container whose
/// rules match it, its computed style inheriting from the block; the
/// properties outside its subset do not reach it. `::first-letter`
/// inherits from `::first-line` (the fictional tag sequence, §2.3.1:
/// it sits inside the first line's pseudo-element). An inline element
/// is no block container: it has neither.
#[test]
fn first_line_and_first_letter_are_cascaded() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = text_el(&mut dom, root, "p", "", "hello ");
    let span = text_el(&mut dom, p, "span", "", "world");
    lay_out(
        &mut dom,
        "p { color: green; letter-spacing: 1 } \
         p::first-line, span::first-line { color: red; padding: 3 } \
         p::first-letter, span::first-letter { float: left; text-transform: uppercase }",
        20,
        3,
    );
    let ext = dom.node(p).ext().unwrap();
    let line = ext
        .computed_first_line
        .as_deref()
        .expect("p has a ::first-line");
    assert_eq!(line.fg, rdom_tui::Color::Rgb(255, 0, 0));
    assert_eq!(
        line.text.letter_spacing.cells(),
        1,
        "inherited from the block"
    );
    assert_eq!(line.padding.top.resolve(20), 0, "padding does not apply");
    let letter = ext
        .computed_first_letter
        .as_deref()
        .expect("p has a ::first-letter");
    assert_eq!(
        letter.fg,
        rdom_tui::Color::Rgb(255, 0, 0),
        "from ::first-line"
    );
    assert_eq!(letter.float, Float::Left);
    assert_eq!(letter.text.text_transform.case, TextCase::Uppercase);
    let span_ext = dom.node(span).ext().unwrap();
    assert!(span_ext.computed_first_line.is_none());
    assert!(span_ext.computed_first_letter.is_none());
    let _ = el;
}
