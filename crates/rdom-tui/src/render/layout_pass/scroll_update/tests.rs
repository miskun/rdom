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
    build_with(markup, css, &|_| {})
}

/// [`build`], with `setup` run on the parsed tree before the cascade (a
/// popover shown, a picker opened).
fn build_with(markup: &str, css: &str, setup: &dyn Fn(&mut TuiDom)) -> TuiDom {
    let mut dom = TuiDom::new();
    let root = dom.root();
    rdom_parser::parse_into(&mut dom, markup, root).unwrap();
    setup(&mut dom);
    let sheet = rdom_css::from_css_strict(css).unwrap();
    dom.cascade(&sheet);
    dom.layout_dom(VIEW);
    dom
}

/// Every element's layout state, what `position-visibility` hides, the
/// positioned boxes' reach into it as a scroll container
/// (`positioned_overflow`), where its scroll-driven timelines stand and its
/// clamp point; and the root's anonymous block boxes (C16G-ORACLE-GAPS).
fn snapshot(dom: &TuiDom) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![dom.root()];
    while let Some(id) = stack.pop() {
        if let Some(ext) = dom.node(id).ext() {
            let hidden = crate::render::layout_pass::position_hidden(dom, id, None);
            let pseudo_hidden = [PseudoSlot::Before, PseudoSlot::After]
                .map(|s| crate::render::layout_pass::position_hidden(dom, id, Some(s)));
            let reach = crate::render::layout_pass::positioned_overflow::reach_of(dom, id);
            let clamp = crate::render::layout_pass::line_clamp::clamp_point(dom, id);
            let timelines: Vec<_> = ext
                .computed
                .iter()
                .flat_map(|c| c.motion.animation_timeline.iter())
                .map(|t| crate::runtime::animation::resolve_timeline(dom, id, t))
                .collect();
            out.push(format!(
                "{id:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?} {hidden} {pseudo_hidden:?} {reach:?} {timelines:?} {clamp:?}",
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
    // The document root's anonymous block boxes, kept for the initial
    // containing block.
    let root = crate::render::box_tree::icb::anonymous_blocks(dom, dom.root());
    out.push(format!("root boxes {root:?}"));
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
    agrees_with(markup, css, &|_| {}, steps, expect);
}

/// [`agrees`], each copy set up by `setup` before its cascade.
fn agrees_with(
    markup: &str,
    css: &str,
    setup: &dyn Fn(&mut TuiDom),
    steps: &[&[(&str, i32, i32)]],
    expect: Update,
) {
    let mut a = build_with(markup, css, setup);
    let mut b = build_with(markup, css, setup);
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

/// C16G-ANCHOR-AFTER-MOVE (CSS Anchor Positioning 1 §2, CSS Position 3
/// §3.4, CSS 2.1 §9.4.3): boxes anchored to a sticky header and to a
/// relatively moved inline box are placed against where those moved — by
/// a scroll update as by a layout.
#[test]
fn anchored_boxes_follow_sticky_and_relative_anchors() {
    let markup = r#"<div id="s"><h1>H</h1><p>a <span id="r">rel</span></p><p>b</p><p>c</p><p>d</p><p>e</p><p>f</p><p>g</p><p>h</p></div><div id="t">T</div><div id="u">U</div>"#;
    let css = "#s { overflow-y: auto; height: 5; width: 20 } p, h1 { margin: 0 } \
               h1 { position: sticky; top: 0; anchor-name: --h } \
               #r { position: relative; left: 2; anchor-name: --r } \
               #t { position: absolute; position-anchor: --h; top: anchor(bottom); left: anchor(left) } \
               #u { position: absolute; top: anchor(--r bottom); left: anchor(--r right) }";
    agrees(
        markup,
        css,
        &[&[("s", 0, 2)], &[("s", 0, 4)], &[("s", 0, 0)]],
        Update::Scrolled,
    );
    let mut dom = build(markup, css);
    let at = |dom: &TuiDom, id: &str| {
        let r = dom
            .node(dom.get_element_by_id(id).unwrap())
            .layout_rect()
            .unwrap();
        (r.x, r.y)
    };
    assert_eq!(at(&dom, "u"), (7, 2), "under the moved inline, laid out");
    scroll(&mut dom, "s", 0, 3);
    assert_eq!(update(&mut dom, VIEW), Update::Scrolled);
    assert_eq!(at(&dom, "t"), (0, 1), "under the stuck header, scrolled");
    assert_eq!(at(&dom, "u"), (7, -1), "under the moved inline, scrolled");
}

/// C16G-ANCHOR-AFTER-MOVE: a box stretched from a sticky anchor to its
/// containing block's bottom is laid out at the size the stuck anchor
/// gives it — phase 2 runs again with that rect — and a scroll that
/// changes the size lays out.
#[test]
fn a_box_sized_by_a_sticky_anchor_is_laid_out_at_that_size() {
    let markup = r#"<div id="s"><p>x</p><h1>H</h1><p>a</p><p>b</p><p>c</p><p>d</p><p>e</p><p>f</p></div><div id="t"><p id="in">in</p></div>"#;
    let css = "#s { overflow-y: auto; height: 5; width: 20 } p, h1 { margin: 0 } \
               h1 { position: sticky; top: 0; anchor-name: --h } \
               #t { position: absolute; top: anchor(--h bottom); bottom: 0; left: 0; width: 5 } \
               #in { height: 100% }";
    let rect = |dom: &TuiDom, id: &str| {
        let r = dom
            .node(dom.get_element_by_id(id).unwrap())
            .layout_rect()
            .unwrap();
        (r.x, r.y, r.width, r.height)
    };
    let mut dom = build(markup, css);
    assert_eq!(rect(&dom, "t"), (0, 2, 5, 10));
    scroll(&mut dom, "s", 0, 3);
    dom.layout_dom(VIEW);
    assert_eq!(rect(&dom, "t"), (0, 1, 5, 11), "from the stuck header down");
    assert_eq!(
        rect(&dom, "in"),
        (0, 1, 5, 11),
        "its content laid out at that size"
    );
    agrees(markup, css, &[&[("s", 0, 3)]], Update::LaidOut);
}

// ─── C16G-ORACLE-GAPS: the cases the Phase 16 gate found unpinned ──────

/// Sticky headings inside a multi-column container (CSS Multi-column 1 §3,
/// CSS Position 3 §3.4): each sticks within its column's share of the
/// container as a layout sticks it.
#[test]
fn sticky_inside_columns_moves_as_laid_out() {
    agrees(
        r#"<div id="s"><div class="mc"><h2>a</h2><p>1</p><p>2</p><p>3</p><p>4</p><h2>b</h2><p>5</p><p>6</p><p>7</p><p>8</p></div><p>x</p><p>y</p><p>z</p><p>w</p></div>"#,
        "#s { overflow-y: auto; height: 4; width: 30 } p, h2 { margin: 0 } \
         .mc { columns: 2 } h2 { position: sticky; top: 0 }",
        &[
            &[("s", 0, 1)],
            &[("s", 0, 3)],
            &[("s", 0, 6)],
            &[("s", 0, 0)],
        ],
        Update::Scrolled,
    );
}

/// Relatively positioned inline boxes (CSS 2.1 §9.4.3, ACID-FIX-5) — text
/// across lines, a nested bold box and an atom inside — move as laid out.
#[test]
fn relative_inline_boxes_move_as_laid_out() {
    agrees(
        r#"<div id="s"><p>a <span class="r">moved <b>bold</b> and <span class="ib">atom</span></span> text that wraps over the lines</p><p>b <span class="up">up</span></p><p>c</p><p>d</p><p>e</p><p>f</p></div>"#,
        "#s { overflow-y: auto; height: 4; width: 16 } p { margin: 0 } \
         .r { position: relative; top: 1; left: 2 } .up { position: relative; top: -1 } \
         .ib { display: inline-block; width: 4 }",
        &[&[("s", 0, 1)], &[("s", 0, 3)], &[("s", 0, 0)]],
        Update::Scrolled,
    );
}

/// The top layer (HTML §6.12, CSS Position 4 §3): a shown popover with
/// `auto` insets sits at its static position in a scroller — which moves
/// with the scroll while its containing block, the viewport, does not.
#[test]
fn a_top_layer_popover_at_its_static_position_moves_as_laid_out() {
    agrees_with(
        r#"<div id="s"><p>a</p><p>b</p><div popover id="pop">pop</div><p>c</p><p>d</p><p>e</p><p>f</p><p>g</p></div>"#,
        "#s { overflow-y: auto; height: 4; width: 20 } p { margin: 0 } \
         #pop { inset: auto; margin: 0; border: none; padding: 0 }",
        &|dom| {
            let pop = dom.get_element_by_id("pop").unwrap();
            crate::runtime::builtins::popover::show_popover(dom, pop).unwrap();
        },
        &[&[("s", 0, 1)], &[("s", 0, 3)], &[("s", 0, 0)]],
        Update::Scrolled,
    );
}

/// An open drop-down `<select>` (HTML's `::picker(select)`,
/// C15G-SELECT-FLIP) near the screen's bottom: its picker flips above the
/// field, and back once a scroll gives it room below, as laid out.
#[test]
fn an_open_select_picker_flips_as_laid_out() {
    agrees_with(
        r#"<div class="pad"></div><div id="s"><p>a</p><p>b</p><p>c</p><p>d</p><p>e</p><p>f</p><select id="sel"><option>one</option><option>two</option><option>three</option></select><p>g</p><p>h</p><p>i</p><p>j</p></div>"#,
        ".pad { height: 4 } #s { overflow-y: auto; height: 8; width: 20 } p { margin: 0 }",
        &|dom| {
            let sel = dom.get_element_by_id("sel").unwrap();
            crate::runtime::builtins::select::open(dom, sel);
        },
        &[&[("s", 0, 3)], &[("s", 0, 0)], &[("s", 0, 1)]],
        Update::Scrolled,
    );
}

/// An anchor in one scroller and the box anchored to it in another (CSS
/// Anchor Positioning 1 §2–§3): each scroll moves only its own content, and
/// the box follows its anchor.
#[test]
fn an_anchor_and_its_box_in_two_scrollers_move_as_laid_out() {
    agrees(
        r#"<div id="w"><div id="a"><p>1</p><p id="anc">anchor</p><p>3</p><p>4</p><p>5</p><p>6</p></div><div id="b"><p>x</p><div id="t">tip</div><p>y</p><p>z</p><p>v</p><p>u</p></div></div>"#,
        "#w { position: relative } #a, #b { overflow-y: auto; height: 4; width: 20 } p { margin: 0 } \
         #anc { anchor-name: --x } \
         #t { position: absolute; position-anchor: --x; top: anchor(bottom); left: anchor(right) }",
        &[
            &[("a", 0, 1)],
            &[("b", 0, 2)],
            &[("a", 0, 2), ("b", 0, 1)],
            &[("a", 0, 0), ("b", 0, 0)],
        ],
        Update::Scrolled,
    );
}

/// Nested scrollers each with a sticky header (CSS Position 3 §3.4: each
/// sticks in its nearest scrollport), scrolled one at a time and together.
#[test]
fn nested_scrollers_with_sticky_move_as_laid_out() {
    agrees(
        r#"<div id="o"><h1>outer</h1><p>a</p><div id="i"><h2>inner</h2><p>1</p><p>2</p><p>3</p><p>4</p><p>5</p><p>6</p></div><p>b</p><p>c</p><p>d</p><p>e</p><p>f</p></div>"#,
        "#o { overflow: auto; height: 6; width: 30 } #i { overflow: auto; height: 3; width: 20 } \
         p, h1, h2 { margin: 0 } h1, h2 { position: sticky; top: 0 }",
        &[
            &[("o", 0, 2), ("i", 0, 2)],
            &[("i", 0, 4)],
            &[("o", 0, 5)],
            &[("o", 0, 0), ("i", 0, 0)],
        ],
        Update::Scrolled,
    );
}

/// Scroll-driven timelines (Scroll-driven Animations 1 §2–§3) read the
/// scroll offsets, the scrollport and the subject's box: `scroll()` and
/// `view()` stand where they stand after a layout.
#[test]
fn scroll_timelines_stand_as_laid_out() {
    agrees(
        r#"<div id="s"><p class="sc">a</p><p>b</p><p>c</p><p class="v">view</p><p>d</p><p>e</p><p>f</p><p>g</p></div>"#,
        "@keyframes k { to { color: red } } #s { overflow-y: auto; height: 3; width: 20 } p { margin: 0 } \
         .sc { animation: k 1s linear; animation-timeline: scroll() } \
         .v { animation: k 1s linear; animation-timeline: view() }",
        &[
            &[("s", 0, 1)],
            &[("s", 0, 3)],
            &[("s", 0, 5)],
            &[("s", 0, 0)],
        ],
        Update::Scrolled,
    );
}

/// Inline text at the document root (its anonymous block boxes are the
/// initial containing block's, kept apart) and a line-clamp container in a
/// scroller (its clamp point kept from the scrolled content top).
#[test]
fn root_lines_and_clamp_points_move_as_laid_out() {
    agrees(
        r#"root text <b>bold</b><div id="s"><div class="lc">one two three four five six seven eight nine</div><p>a</p><p>b</p><p>c</p><p>d</p></div>more root text"#,
        "#s { overflow-y: auto; height: 3; width: 10 } p { margin: 0 } .lc { line-clamp: 2 }",
        &[&[("s", 0, 1)], &[("s", 0, 3)], &[("s", 0, 0)]],
        Update::Scrolled,
    );
}

/// The fixtures above hold what they claim: the picker flipped and then
/// not, the popover in the top layer moving with the scroll, the
/// timelines active, the multi-column headings stuck.
#[test]
fn the_gap_fixtures_exercise_their_cases() {
    let y = |dom: &TuiDom, id: NodeId| dom.node(id).layout_rect().unwrap().y;
    let mut dom = build_with(
        r#"<div class="pad"></div><div id="s"><p>a</p><p>b</p><p>c</p><p>d</p><p>e</p><p>f</p><select id="sel"><option>one</option><option>two</option><option>three</option></select><p>g</p><p>h</p><p>i</p><p>j</p></div>"#,
        ".pad { height: 4 } #s { overflow-y: auto; height: 8; width: 20 } p { margin: 0 }",
        &|dom| {
            let sel = dom.get_element_by_id("sel").unwrap();
            crate::runtime::builtins::select::open(dom, sel);
        },
    );
    let sel = dom.get_element_by_id("sel").unwrap();
    let first = dom.node(sel).first_element_child().unwrap().id();
    assert!(y(&dom, first) < y(&dom, sel), "flipped above the field");
    scroll(&mut dom, "s", 0, 3);
    assert_eq!(update(&mut dom, VIEW), Update::Scrolled);
    assert!(y(&dom, first) >= y(&dom, sel), "below the field again");

    let mut dom = build_with(
        r#"<div id="s"><p>a</p><p>b</p><div popover id="pop">pop</div><p>c</p><p>d</p><p>e</p><p>f</p><p>g</p></div>"#,
        "#s { overflow-y: auto; height: 4; width: 20 } p { margin: 0 } \
         #pop { inset: auto; margin: 0; border: none; padding: 0 }",
        &|dom| {
            let pop = dom.get_element_by_id("pop").unwrap();
            crate::runtime::builtins::popover::show_popover(dom, pop).unwrap();
        },
    );
    let pop = dom.get_element_by_id("pop").unwrap();
    assert!(dom.is_in_top_layer(pop));
    assert_eq!(y(&dom, pop), 2);
    scroll(&mut dom, "s", 0, 1);
    assert_eq!(update(&mut dom, VIEW), Update::Scrolled);
    assert_eq!(y(&dom, pop), 1, "at its static position, scrolled");

    let mut dom = build(
        r#"<div id="s"><p class="sc">a</p><p>b</p><p>c</p><p class="v">view</p><p>d</p><p>e</p><p>f</p><p>g</p></div>"#,
        "@keyframes k { to { color: red } } #s { overflow-y: auto; height: 3; width: 20 } p { margin: 0 } \
         .sc { animation: k 1s linear; animation-timeline: scroll() } \
         .v { animation: k 1s linear; animation-timeline: view() }",
    );
    scroll(&mut dom, "s", 0, 3);
    update(&mut dom, VIEW);
    let progress = snapshot(&dom)
        .iter()
        .filter(|l| l.contains("Progress"))
        .count();
    assert_eq!(progress, 2, "both timelines active");

    let mut dom = build(
        r#"<div id="s"><div class="mc"><h2 id="h">a</h2><p>1</p><p>2</p><p>3</p><p>4</p><h2>b</h2><p>5</p><p>6</p><p>7</p><p>8</p></div><p>x</p><p>y</p><p>z</p><p>w</p></div>"#,
        "#s { overflow-y: auto; height: 4; width: 30 } p, h2 { margin: 0 } \
         .mc { columns: 2 } h2 { position: sticky; top: 0 }",
    );
    scroll(&mut dom, "s", 0, 1);
    update(&mut dom, VIEW);
    let h = dom.get_element_by_id("h").unwrap();
    assert_eq!(y(&dom, h), 0, "stuck at the scrollport's top");
}
