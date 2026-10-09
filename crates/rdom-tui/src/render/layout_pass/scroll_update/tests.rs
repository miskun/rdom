//! A scroll update leaves what a layout at the new offsets would
//! (C15G-SCROLL-NO-RELAYOUT): each fixture is built twice, scrolled the
//! same, then one is updated and the other laid out, and every element's
//! layout state compared — and a scroll costs no layout of any box.

use super::{SCROLLED, Update, update};
use crate::ext::PseudoSlot;
use crate::prelude::*;
use crate::render::layout_pass::{LAYOUTS, ROUNDS};

const VIEW: Rect = Rect {
    x: 0,
    y: 0,
    width: 40,
    height: 12,
};

fn build(markup: &str, css: &str) -> TuiDom {
    let mut dom = TuiDom::new();
    let root = dom.root();
    rdom_parser::parse_into(&mut dom, markup, root).unwrap();
    let sheet = rdom_css::from_css_strict(css).unwrap();
    dom.cascade(&sheet);
    dom.layout_dom(VIEW);
    dom
}

/// Every element's layout state, and what `position-visibility` hides.
fn snapshot(dom: &TuiDom) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![dom.root()];
    while let Some(id) = stack.pop() {
        if let Some(ext) = dom.node(id).ext() {
            let hidden = crate::render::layout_pass::position_hidden(dom, id, None);
            let pseudo_hidden = [PseudoSlot::Before, PseudoSlot::After]
                .map(|s| crate::render::layout_pass::position_hidden(dom, id, Some(s)));
            out.push(format!(
                "{id:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?} {hidden} {pseudo_hidden:?}",
                ext.layout,
                ext.content_layout,
                ext.static_position,
                ext.inline_layout,
                ext.anonymous_blocks,
                ext.kept,
                ext.floated_pseudos,
                ext.positioned_pseudos,
                (ext.scroll_x, ext.scroll_y),
                (ext.scroll_content_width, ext.scroll_content_height),
                ext.scroll_state.as_ref().map(|s| (s.laid_out, s.gutters)),
                ext.margin_chain,
                ext.layout_dirty,
            ));
        }
        stack.extend(dom.node(id).child_nodes().map(|c| c.id()));
    }
    out
}

fn scroll(dom: &mut TuiDom, id: &str, x: i32, y: i32) {
    let n = dom.get_element_by_id(id).unwrap();
    dom.node_mut(n).set_scroll(x, y);
}

/// Scroll a copy of the fixture through `steps` (each a list of `(id, x,
/// y)` writes), updating one and laying the other out after each step:
/// they agree after every step, and each update is `expect`.
fn agrees(markup: &str, css: &str, steps: &[&[(&str, i32, i32)]], expect: Update) {
    let mut a = build(markup, css);
    let mut b = build(markup, css);
    assert_eq!(snapshot(&a), snapshot(&b), "the fixture builds the same");
    for (k, step) in steps.iter().enumerate() {
        for &(id, x, y) in *step {
            scroll(&mut a, id, x, y);
            scroll(&mut b, id, x, y);
        }
        assert_eq!(update(&mut a, VIEW), expect, "step {k}");
        b.layout_dom(VIEW);
        let (sa, sb) = (snapshot(&a), snapshot(&b));
        for (la, lb) in sa.iter().zip(&sb) {
            assert_eq!(la, lb, "step {k}: updated (left) vs laid out (right)");
        }
        assert_eq!(sa.len(), sb.len());
    }
}

const ROWS: &str = r#"<div id="s"><p>a</p><p>b</p><p>c</p><p>d</p><p>e</p><p>f</p><p>g</p><p>h</p><p>i</p><p>j</p><p>k</p><p>l</p></div>"#;

#[test]
fn a_scrolled_block_flow_moves_as_laid_out() {
    agrees(
        ROWS,
        "#s { overflow-y: auto; height: 5; width: 20 } p { margin: 0 }",
        &[&[("s", 0, 3)], &[("s", 0, 7)], &[("s", 0, 0)]],
        Update::Scrolled,
    );
}

#[test]
fn an_out_of_range_offset_is_clamped_as_layout_clamps_it() {
    agrees(
        ROWS,
        "#s { overflow-y: auto; height: 5; width: 20 } p { margin: 0 }",
        &[&[("s", 0, 400)], &[("s", 0, -9)]],
        Update::Scrolled,
    );
}

#[test]
fn nested_scrollers_move_together() {
    agrees(
        r#"<div id="o"><p>a</p><div id="i"><p>1</p><p>2</p><p>3</p><p>4</p><p>5</p><p>6</p></div><p>b</p><p>c</p><p>d</p><p>e</p><p>f</p></div>"#,
        "#o { overflow: auto; height: 6; width: 30 } #i { overflow: auto; height: 3; width: 10 } p { margin: 0 }",
        &[&[("o", 0, 2), ("i", 0, 2)], &[("i", 0, 1)], &[("o", 0, 4)]],
        Update::Scrolled,
    );
}

#[test]
fn sticky_boxes_stick_as_laid_out() {
    agrees(
        r#"<div id="s"><h1>head</h1><p>a</p><p>b</p><p>c</p><section><h2>sub</h2><p>1</p><p>2</p><p>3</p></section><p>d</p><p>e</p><p>f</p><p>g</p></div>"#,
        "#s { overflow-y: auto; height: 5; width: 20 } p, h1, h2, section { margin: 0 } \
         h1 { position: sticky; top: 0 } h2 { position: sticky; top: 1 } \
         section { position: relative } section::before { content: 'x'; position: absolute; left: 0 }",
        &[
            &[("s", 0, 2)],
            &[("s", 0, 5)],
            &[("s", 0, 9)],
            &[("s", 0, 0)],
        ],
        Update::Scrolled,
    );
}

#[test]
fn positioned_boxes_move_with_their_containing_block_or_stay() {
    agrees(
        r#"<div id="w"><div id="s"><p>a</p><i id="in">in</i><p>b</p><b id="out">out</b><u id="st">st</u><em id="fx">fx</em><p>c</p><p>d</p><p>e</p><p>f</p><p>g</p><p>h</p></div></div>"#,
        "#w { position: relative; padding: 1 } \
         #s { overflow-y: auto; height: 6; width: 24; position: relative } p { margin: 0 } \
         #in { position: absolute; top: 2; left: 10 } \
         #out { position: absolute; top: 0; right: 0 } \
         #st { position: absolute } #fx { position: fixed }",
        &[&[("s", 0, 3)], &[("s", 0, 6)]],
        Update::Scrolled,
    );
    // The scroller is no containing block: the absolutely positioned boxes
    // are the wrapper's, and stay — at their static positions when their
    // insets are `auto`.
    agrees(
        r#"<div id="w"><div id="s"><p>a</p><i id="in">in</i><p>b</p><u id="st">st</u><em id="fx">fx</em><p>c</p><p>d</p><p>e</p><p>f</p><p>g</p><p>h</p></div></div>"#,
        "#w { position: relative; padding: 1 } \
         #s { overflow-y: auto; height: 6; width: 24 } p { margin: 0 } \
         #in { position: absolute; top: 2; left: 10 } #st { position: absolute } \
         #fx { position: fixed }",
        &[&[("s", 0, 3)], &[("s", 0, 1)]],
        Update::Scrolled,
    );
}

#[test]
fn anchored_boxes_follow_their_anchors() {
    let css = "#s { overflow-y: auto; height: 6; width: 20 } p { margin: 0 } \
               #a { anchor-name: --a } \
               #t { position: absolute; position-anchor: --a; top: anchor(bottom); left: anchor(left); \
                    position-try-fallbacks: flip-block } \
               #u { position: fixed; position-anchor: --a; left: anchor(right); top: anchor(top) }";
    agrees(
        r#"<div id="s"><p>a</p><p>b</p><p id="a">anchor</p><p>c</p><p>d</p><p>e</p><p>f</p><p>g</p><p>h</p><p>i</p></div><div id="t">tip</div><div id="u">u</div>"#,
        css,
        &[
            &[("s", 0, 1)],
            &[("s", 0, 2)],
            &[("s", 0, 4)],
            &[("s", 0, 0)],
        ],
        Update::Scrolled,
    );
}

#[test]
fn inline_content_and_atoms_move_with_the_scroll() {
    agrees(
        r#"<div id="s">some words <b>bold</b> and <span class="ib">an atom</span> more words to wrap over lines here and there and on</div><div id="t">line one line two <span class="ib">x</span> line three line four line five</div>"#,
        "#s { overflow-y: auto; height: 3; width: 12 } #t { overflow: auto; height: 2; width: 9 } \
         .ib { display: inline-block; width: 5 }",
        &[&[("s", 0, 2), ("t", 0, 1)], &[("s", 0, 4), ("t", 0, 3)]],
        Update::Scrolled,
    );
}

#[test]
fn flex_grid_table_float_and_columns_move_as_laid_out() {
    agrees(
        r#"<div id="s"><div class="f"><i>1</i><i>2</i></div><div class="g"><i>1</i><i>2</i><i>3</i></div><table><tr><td>a</td><td>b</td></tr><tr><td>c</td><td>d</td></tr></table><div class="fl">float</div><p>beside the float, wrapping</p><div class="mc">one two three four five six seven eight</div><p>end</p><p>end</p><p>end</p></div>"#,
        "#s { overflow-y: auto; height: 6; width: 30 } p { margin: 0 } \
         .f { display: flex; flex-direction: row-reverse } .g { display: grid; grid-template-columns: 3 3 } \
         .fl { float: left; width: 6; height: 3 } .mc { columns: 2; column-rule: solid }",
        &[&[("s", 0, 3)], &[("s", 0, 8)]],
        Update::Scrolled,
    );
}

#[test]
fn reversed_and_rtl_scrollers_move_as_laid_out() {
    agrees(
        r#"<div id="c"><p>1</p><p>2</p><p>3</p><p>4</p><p>5</p><p>6</p></div><div id="r"><span class="w">wide content line</span></div>"#,
        "#c { display: flex; flex-direction: column-reverse; overflow: auto; height: 3; width: 8 } \
         p { margin: 0 } #r { direction: rtl; overflow-x: auto; width: 6; height: 2 } \
         .w { white-space: nowrap }",
        &[&[("c", 0, -2), ("r", -4, 0)], &[("c", 0, -1), ("r", -2, 0)]],
        Update::Scrolled,
    );
}

#[test]
fn pseudo_elements_move_as_laid_out() {
    agrees(
        r#"<div id="s"><p class="r">a</p><p>b</p><p class="k">c</p><p>d</p><p>e</p><p>f</p><p>g</p><p>h</p></div>"#,
        "#s { overflow-y: auto; height: 4; width: 20; position: relative } p { margin: 0 } \
         .r::before { content: 'R'; position: relative; left: 2 } \
         .k::after { content: 'K'; position: sticky; top: 0; display: block } \
         #s::after { content: 'A'; position: absolute; top: 0; right: 0 } \
         #s::before { content: 'B'; position: absolute }",
        &[&[("s", 0, 2)], &[("s", 0, 3)]],
        Update::Scrolled,
    );
}

#[test]
fn translated_and_clamped_boxes_move_as_laid_out() {
    agrees(
        r#"<div id="s"><p class="t">moved</p><div class="lc">one two three four five six seven</div><p>x</p><p>y</p><p>z</p></div>"#,
        "#s { overflow-y: auto; height: 3; width: 10 } p { margin: 0 } \
         .t { translate: 2 1 } .lc { line-clamp: 2 }",
        &[&[("s", 0, 2)], &[("s", 0, 1)]],
        Update::Scrolled,
    );
}

/// A `content-visibility: auto` element changing relevance re-cascades: the
/// update lays out.
#[test]
fn a_relevance_change_lays_out() {
    let mut markup = String::from(r#"<div id="s">"#);
    for _ in 0..8 {
        markup.push_str(r#"<section><p>x</p></section>"#);
    }
    markup.push_str("</div>");
    agrees(
        &markup,
        "#s { overflow-y: auto; height: 3; width: 10 } p { margin: 0 } \
         section { content-visibility: auto; contain-intrinsic-size: auto 1 }",
        &[&[("s", 0, 4)]],
        Update::LaidOut,
    );
}

/// An offset on a box that is not a scroll container is dropped by layout.
#[test]
fn an_offset_on_a_box_that_does_not_scroll_lays_out() {
    agrees(ROWS, "p { margin: 0 }", &[&[("s", 0, 2)]], Update::LaidOut);
}

#[test]
fn nothing_moved_is_nothing_to_do() {
    let mut dom = build(ROWS, "#s { overflow-y: auto; height: 5 } p { margin: 0 }");
    assert_eq!(update(&mut dom, VIEW), Update::Unmoved);
}

/// A scroll in a large document lays no box out — with sticky and anchored
/// boxes too, which are moved, not laid out.
#[test]
fn a_scroll_lays_nothing_out() {
    let mut markup = String::from(r#"<div id="s"><h1>head</h1>"#);
    for i in 0..400 {
        markup.push_str(&format!(r#"<p id="p{i}">row {i} <b>bold</b> text</p>"#));
    }
    markup.push_str(r#"</div><div id="t">tip</div>"#);
    let css = "#s { overflow-y: auto; height: 10; width: 30 } p { margin: 0 } \
               h1 { position: sticky; top: 0 } #p5 { anchor-name: --a } \
               #t { position: absolute; position-anchor: --a; top: anchor(bottom); left: anchor(left) }";
    let mut dom = build(&markup, css);
    for y in 1..=20 {
        scroll(&mut dom, "s", 0, y);
        LAYOUTS.with(|c| c.set(0));
        ROUNDS.with(|c| c.set(0));
        SCROLLED.with(|c| c.set(0));
        assert_eq!(update(&mut dom, VIEW), Update::Scrolled);
        assert_eq!(
            (
                LAYOUTS.with(std::cell::Cell::get),
                ROUNDS.with(std::cell::Cell::get),
                SCROLLED.with(std::cell::Cell::get)
            ),
            (0, 0, 1),
            "tick {y}: boxes laid out, layout runs, scroll updates"
        );
    }
}

/// Boxes layout writes at their parent's content origin (`display:
/// contents`, an inline box of a line) or not at all (a nested inline box,
/// a `display: none` subtree) move as layout moves them.
#[test]
fn boxless_and_unwritten_boxes_stay_as_laid_out() {
    agrees(
        r#"<div id="s"><span class="c"><b>in <i>nested <span class="ib">atom</span></i></b> text</span><br><em hidden>gone <b>x</b></em> more text to wrap a bit</div><div id="o"><p>a</p><div class="ifc">x <b>y <i>z</i></b> <span class="c"><u>w</u></span></div><div class="c"><p>in contents</p></div><p>b</p><p hidden>none</p><p>c</p><p>d</p><p>e</p></div>"#,
        "#s { overflow: auto; height: 2; width: 10 } #o { overflow: auto; height: 3; width: 12 } \
         p { margin: 0 } .c { display: contents } .ib { display: inline-block }",
        &[&[("s", 0, 1), ("o", 0, 2)], &[("s", 0, 2), ("o", 0, 4)]],
        Update::Scrolled,
    );
}

/// An absolutely positioned box in a grid area, one inside a sticky box,
/// and a horizontal scroll.
#[test]
fn grid_areas_sticky_containers_and_horizontal_scrolls_move_as_laid_out() {
    agrees(
        r#"<div id="s"><div class="g"><i>1</i><i>2</i><b class="a">abs</b></div><div class="st"><u class="a2">u</u>sticky</div><p>1</p><p>2</p><p>3</p><p>4</p><p>5</p><p>6</p></div><div id="h"><span>a long line that does not wrap at all</span><b class="hb">x</b></div>"#,
        "#s { overflow-y: auto; height: 5; width: 20 } p { margin: 0 } \
         .g { display: grid; grid-template-columns: 4 4; position: relative } \
         .a { position: absolute; grid-column: 2; top: 0; left: 1 } \
         .st { position: sticky; top: 0 } .a2 { position: absolute; right: 0 } \
         #h { overflow-x: auto; width: 10; height: 3; white-space: nowrap; position: relative } \
         .hb { position: absolute; left: 12; top: 1 }",
        &[&[("s", 0, 3), ("h", 5, 0)], &[("s", 0, 6), ("h", 9, 0)]],
        Update::Scrolled,
    );
}

/// `content-visibility: auto` elements that keep their relevance cost no
/// layout.
#[test]
fn relevance_kept_scrolls_without_a_layout() {
    let mut markup = String::from(r#"<div id="s">"#);
    for _ in 0..8 {
        markup.push_str(r#"<section><p>x</p></section>"#);
    }
    markup.push_str("</div>");
    agrees(
        &markup,
        "#s { overflow-y: auto; height: 4; width: 10 } p { margin: 0 } \
         section { content-visibility: auto; contain-intrinsic-size: auto 1 }",
        &[&[("s", 0, 1)]],
        Update::Scrolled,
    );
}
