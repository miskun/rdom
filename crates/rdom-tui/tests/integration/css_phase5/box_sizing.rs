//! C5-BOX-SIZING — `box-sizing` (CSS UI 3 §3.1, now CSS Sizing 3 "Box
//! Edges for Sizing"): `width` / `height` and their `min-*` / `max-*`
//! measure the content box by default, the border box under
//! `border-box`, where the content box is floored at zero.

use super::{el, lay_out, paint, rect, rows, size};
use rdom_tui::TuiDom;

/// A `div.b` child of the root, laid out under `css` in 30 × 12.
fn one(css: &str) -> (TuiDom, rdom_tui::NodeId) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    lay_out(&mut dom, css, 30, 12);
    (dom, b)
}

// ── content-box, the initial value ─────────────────────────────────

/// CSS UI 3 §3.1: `content-box` (initial) — "the specified width and
/// height … apply to the width and height respectively of the content
/// box of the element. The padding and border of the element are laid
/// out and drawn outside the specified width and height."
#[test]
fn content_box_puts_padding_and_border_outside_the_size() {
    let (dom, b) = one(".b { width: 6; height: 3; padding: 0 1; border: solid }");
    assert_eq!(size(&dom, b), (6 + 2 + 2, 3 + 2));
}

/// CSS UI 3 §3.1: `border-box` — "the specified width and height … on
/// this element determine the border box of the element."
#[test]
fn border_box_puts_padding_and_border_inside_the_size() {
    let (dom, b) =
        one(".b { box-sizing: border-box; width: 6; height: 3; padding: 0 1; border: solid }");
    assert_eq!(size(&dom, b), (6, 3));
}

/// CSS Sizing 3 §5.2: `min-*` / `max-*` measure the same box as
/// `width` / `height` — the content box under `content-box`.
#[test]
fn content_box_min_and_max_measure_the_content_box() {
    let (dom, b) = one(".b { max-width: 4; min-height: 2; padding: 0 1; border: solid }");
    assert_eq!(size(&dom, b), (4 + 2 + 2, 2 + 2));
    let (dom, b) = one(
        ".b { box-sizing: border-box; max-width: 4; min-height: 4; padding: 0 1; border: solid }",
    );
    assert_eq!(size(&dom, b), (4, 4));
}

/// A percentage resolves to the content box's size too (CSS Sizing 3
/// §3.1): `width: 50%` of 30 is a 15-cell content box.
#[test]
fn content_box_percentages_size_the_content_box() {
    let (dom, b) = one(".b { width: 50%; height: 1; padding: 0 2 }");
    assert_eq!(size(&dom, b), (15 + 4, 1));
}

// ── the content box is floored at zero ─────────────────────────────

/// CSS UI 3 §3.1: under `border-box`, "the content width and height are
/// calculated by subtracting the border and padding widths … from the
/// specified width and height … floored at 0". So the used border box
/// is never smaller than its padding plus border: `width: 0; border:
/// solid` draws a 2 × 2 box (it drew nothing — DIVERGENCES §2).
#[test]
fn border_box_never_shrinks_below_padding_and_border() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    let buf = paint(
        &mut dom,
        ".b { box-sizing: border-box; width: 0; height: 0; border: solid }",
        4,
        3,
    );
    assert_eq!(size(&dom, b), (2, 2));
    assert_eq!(rows(&buf, 2, 2), ["┌┐", "└┘"]);

    let (dom, b) = one(".b { box-sizing: border-box; width: 1; height: 1; padding: 1 3 }");
    assert_eq!(size(&dom, b), (6, 2));
    // A `max-*` below the padding and border cannot squeeze it either.
    let (dom, b) = one(".b { box-sizing: border-box; max-width: 1; height: 3; border: solid }");
    assert_eq!(size(&dom, b), (2, 3));
}

/// CSS 2.1 §10.3.3: an `auto` width fills the containing block, but the
/// content width cannot be negative — a box whose padding and border
/// exceed the space overflows instead of shrinking under them.
#[test]
fn an_auto_width_box_is_at_least_its_padding_and_border() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let outer = el(&mut dom, root, "div", "outer");
    let inner = el(&mut dom, outer, "div", "inner");
    lay_out(
        &mut dom,
        ".outer { width: 3 } .inner { height: 1; padding: 0 3 }",
        30,
        12,
    );
    assert_eq!(size(&dom, inner), (6, 1));
}

// ── flex items, positioned boxes, intrinsic sizes ──────────────────

/// CSS Flexbox §9.2: a definite main size is the item's flex base size
/// — its content box under `content-box`, so the outer size adds the
/// padding and border; the cross size likewise.
#[test]
fn flex_item_sizes_follow_box_sizing() {
    let css = ".row { display: flex; flex-direction: row; height: 6 } \
               .a { width: 4; height: 2; padding: 0 1; border: solid; flex-shrink: 0 } \
               .b { width: 4; height: 2; padding: 0 1; border: solid; flex-shrink: 0; box-sizing: border-box }";
    let mut dom = TuiDom::new();
    let root = dom.root();
    let row = el(&mut dom, root, "div", "row");
    let a = el(&mut dom, row, "div", "a");
    let b = el(&mut dom, row, "div", "b");
    lay_out(&mut dom, css, 30, 12);
    assert_eq!(size(&dom, a), (4 + 2 + 2, 2 + 2));
    assert_eq!(size(&dom, b), (4, 2));
    assert_eq!(rect(&dom, b).x, 8);
}

/// CSS 2.1 §10.3.7 / §10.6.4 with CSS UI 3 §3.1: an absolutely
/// positioned box's declared size is its content box too.
#[test]
fn positioned_box_sizes_follow_box_sizing() {
    let (dom, b) =
        one(".b { position: absolute; top: 1; left: 1; width: 4; height: 1; border: solid }");
    assert_eq!(size(&dom, b), (6, 3));
    let (dom, b) = one(
        ".b { position: absolute; top: 1; left: 1; width: 4; height: 1; border: solid; box-sizing: border-box }",
    );
    assert_eq!(size(&dom, b), (4, 2));
}

/// CSS Sizing 3 §5.1: a box's max-content contribution is its outer
/// size — a fixed-width child counts its padding and border under
/// `content-box`, so a shrink-to-fit parent wraps all of it.
#[test]
fn intrinsic_contributions_follow_box_sizing() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let wrap = el(&mut dom, root, "div", "wrap");
    let child = el(&mut dom, wrap, "div", "child");
    lay_out(
        &mut dom,
        ".wrap { position: absolute; top: 0; left: 0 } .child { width: 4; height: 1; border: solid }",
        30,
        12,
    );
    assert_eq!(size(&dom, child), (6, 3));
    assert_eq!(size(&dom, wrap), (6, 3));
}

/// CSS Sizing 4 §5.1: "the aspect-ratio … applies to the box specified
/// by box-sizing". Under `content-box` the ratio relates the content
/// boxes: a 4-cell content width at 2 / 1 is a 2-row content box; under
/// `border-box` the 4-cell border box (2-cell content) is 2 / 1 → 2 rows.
#[test]
fn aspect_ratio_applies_to_the_box_sizing_box() {
    let css = ".row { display: flex; flex-direction: row; height: 8 } \
               .a { width: 4; aspect-ratio: 2; padding: 0 1; flex-shrink: 0 } \
               .b { width: 4; aspect-ratio: 2; padding: 0 1; flex-shrink: 0; box-sizing: border-box }";
    let mut dom = TuiDom::new();
    let root = dom.root();
    let row = el(&mut dom, root, "div", "row");
    let a = el(&mut dom, row, "div", "a");
    let b = el(&mut dom, row, "div", "b");
    lay_out(&mut dom, css, 30, 12);
    assert_eq!(size(&dom, a), (6, 2));
    assert_eq!(size(&dom, b), (4, 2));
}

// ── the UA sheet ───────────────────────────────────────────────────

/// HTML §15.5 ("Form controls", the rendering section's UA sheet):
/// `input:is([type=radio], [type=checkbox], [type=reset], [type=button],
/// [type=submit], [type=color], [type=search]), select, button {
/// box-sizing: border-box }`; Chromium's `html.css` adds `meter` and
/// `progress`. A text `<input>` and `<textarea>` stay `content-box`:
/// their 20-cell width is the content box, the padding sits outside.
#[test]
fn ua_form_controls_follow_the_html_rendering_rules() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let text = el(&mut dom, root, "input", "");
    let search = el(&mut dom, root, "input", "");
    dom.set_attribute(search, "type", "search").unwrap();
    let select = el(&mut dom, root, "select", "");
    let progress = el(&mut dom, root, "progress", "");
    let meter = el(&mut dom, root, "meter", "");
    let area = el(&mut dom, root, "textarea", "");
    lay_out(&mut dom, "", 40, 30);
    assert_eq!(size(&dom, text), (20 + 2, 1));
    assert_eq!(size(&dom, search), (20, 1));
    assert_eq!(size(&dom, select), (20, 1));
    assert_eq!(size(&dom, progress), (20, 1));
    assert_eq!(size(&dom, meter), (20, 1));
    assert_eq!(size(&dom, area), (20 + 2, 4));
}

/// The HTML rendering section's `hr` rule declares a border and no
/// height (the content box is empty): rdom's rule is its one-cell top
/// border, one row, under either `box-sizing`.
#[test]
fn hr_is_one_row_under_either_box_sizing() {
    for css in ["", "* { box-sizing: border-box }"] {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let hr = el(&mut dom, root, "hr", "");
        lay_out(&mut dom, css, 10, 4);
        assert_eq!(size(&dom, hr), (10, 1), "{css:?}");
    }
}

/// The migration the CHANGELOG gives: `*, ::before, ::after {
/// box-sizing: border-box }` restores the old sizing for every box.
#[test]
fn the_border_box_reset_restores_border_box_sizing() {
    let (dom, b) = one("*, *::before, *::after { box-sizing: border-box } \
         .b { width: 6; height: 3; padding: 0 1; border: solid }");
    assert_eq!(size(&dom, b), (6, 3));
}

/// The node setter writes the inline declaration, the accessor reads it
/// back, and layout follows it — as `set_width` does.
#[test]
fn node_setter_and_accessor_drive_box_sizing() {
    use rdom_tui::prelude::*;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    dom.node_mut(b)
        .set_width(6u16)
        .set_padding(Padding::new(0, 1, 0, 1))
        .set_box_sizing(BoxSizing::BorderBox);
    assert_eq!(dom.node(b).box_sizing(), Some(BoxSizing::BorderBox));
    lay_out(&mut dom, "", 30, 12);
    assert_eq!(size(&dom, b).0, 6);
    dom.node_mut(b).set_box_sizing(BoxSizing::ContentBox);
    lay_out(&mut dom, "", 30, 12);
    assert_eq!(size(&dom, b).0, 8);
}

/// CSS 2.1 §10.4 / §10.7 apply to absolutely positioned boxes too: the
/// tentative width / height is clamped by `max-*`, then `min-*` — each
/// measured as `box-sizing` says (found by C5-INTRINSIC: positioned boxes
/// ignored both).
#[test]
fn positioned_boxes_honour_min_and_max() {
    let (dom, b) = one(
        ".b { position: absolute; top: 0; left: 0; width: 10; max-width: 4; height: 1; min-height: 3 }",
    );
    assert_eq!(size(&dom, b), (4, 3));
    let (dom, b) = one(
        ".b { position: absolute; top: 0; left: 0; right: 0; bottom: 0; max-width: 6; max-height: 2; border: solid }",
    );
    assert_eq!(size(&dom, b), (6 + 2, 2 + 2));
}
