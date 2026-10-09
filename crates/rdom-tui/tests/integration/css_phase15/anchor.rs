//! CSS Anchor Positioning 1 (C15-ANCHOR): the anchor names and the
//! default anchor (§2), `anchor()` / `anchor-size()` (§5), `position-area`
//! (§3.1), `anchor-center` (§3.4), the position fallbacks (§4) and
//! `position-visibility` (§5) — read from the computed style, the laid-out
//! rects and the painted cells.

use super::*;
use rdom_tui::Length;

const PAGE: &str = "body { margin: 0 } ";

fn computed(dom: &TuiDom, id: &str) -> rdom_tui::ComputedStyle {
    dom.node(by_id(dom, id))
        .computed()
        .cloned()
        .expect("cascaded")
}

/// §5.1, §5.2: an anchor function in a box that is not absolutely
/// positioned is invalid at computed-value time — its fallback stands in,
/// or the property takes its initial value.
#[test]
fn anchor_functions_outside_abspos_compute_to_their_fallbacks() {
    let mut dom = doc(r#"<body><div id="r"></div><div id="a"></div></body>"#);
    styled(
        &mut dom,
        &format!(
            "{PAGE} #r {{ position: relative; top: anchor(bottom); left: anchor(right, 3); width: anchor-size(width) }}
             #a {{ position: absolute; top: anchor(bottom) }}"
        ),
        20,
        5,
    );
    let r = computed(&dom, "r");
    assert_eq!(r.top, Length::Auto);
    assert_eq!(r.left.cells(0), Some(3));
    assert_eq!(r.width, rdom_tui::Size::Auto);
    let a = computed(&dom, "a");
    assert!(matches!(&a.top, Length::Calc(e) if e.contains_anchor()));
}

/// §4.1: a fallback naming a `@position-try` rule carries that rule's
/// declarations — the last of the name, by layer order.
#[test]
fn fallbacks_carry_their_position_try_rules() {
    let mut dom = doc(r#"<body><div id="a"></div></body>"#);
    styled(
        &mut dom,
        &format!(
            "{PAGE} @position-try --x {{ top: 1 }}
             @layer l {{ @position-try --x {{ top: 2 }} }}
             #a {{ position: absolute; position-try-fallbacks: --x, flip-block, --missing }}"
        ),
        20,
        5,
    );
    let a = computed(&dom, "a");
    let f = &a.anchor.position_try_fallbacks;
    assert_eq!(f.len(), 3);
    assert_eq!(
        f[0].declarations().and_then(|d| d.top.clone()),
        Some(rdom_tui::Value::Specified(Length::Cells(1))),
        "unlayered beats layered"
    );
    assert!(f[1].declarations().is_none() && f[2].declarations().is_none());
}

/// An anchor six cells wide at column 5, row 2, and the box `#p` styled
/// `css` (absolutely positioned against the viewport): `#p`'s rect.
fn placed(css: &str) -> (i32, i32, u16, u16) {
    let mut dom = doc(r#"<body><div id="t">anchor</div><div id="p">tip</div></body>"#);
    styled(
        &mut dom,
        &format!(
            "{PAGE} #t {{ anchor-name: --t; margin: 2 0 0 5; width: 6 }}
             #p {{ position: absolute; position-anchor: --t; {css} }}"
        ),
        20,
        8,
    );
    rect(&dom, "p")
}

/// §5.1: `anchor()` is the distance from the containing block's edge to
/// the anchor's edge — named or the default anchor, a side, a percentage,
/// `inside` / `outside` by the inset's own side.
#[test]
fn anchor_resolves_to_the_anchor_edges() {
    assert_eq!(
        placed("top: anchor(bottom); left: anchor(left)"),
        (5, 3, 3, 1)
    );
    assert_eq!(
        placed("bottom: anchor(top); left: anchor(--t right)"),
        (11, 1, 3, 1)
    );
    assert_eq!(
        placed("top: anchor(outside); left: anchor(50%)"),
        (8, 3, 3, 1)
    );
    assert_eq!(
        placed("top: anchor(inside); right: anchor(inside)"),
        (8, 2, 3, 1)
    );
    assert_eq!(
        placed("top: calc(anchor(bottom) + 1); left: anchor(center)"),
        (8, 4, 3, 1)
    );
}

/// §5.1, §5.2: without an acceptable anchor the function's fallback stands
/// in; without one the inset is `auto` (the box at its static position).
/// `anchor-size()` is the anchor's size.
#[test]
fn a_missing_anchor_takes_the_fallback_and_anchor_size_measures() {
    assert_eq!(
        placed("top: anchor(--none bottom, 6); left: 1"),
        (1, 6, 3, 1)
    );
    assert_eq!(placed("top: anchor(--none bottom); left: 1"), (1, 3, 3, 1));
    assert_eq!(
        placed("top: 0; left: 0; width: anchor-size(width); height: anchor-size(--t width)"),
        (0, 0, 6, 6)
    );
}

/// §3.1: `position-area` makes the box's containing block the area of the
/// 3 × 3 grid of the anchor's and the containing block's edges it names,
/// the box hugging the anchor there (§3.1.3) — across the anchor
/// `anchor-center`, beside it towards it.
#[test]
fn position_area_places_in_the_grid_around_the_anchor() {
    // Centred on the anchor's centre line (column 8), the leading space
    // rounded down, as `center`.
    assert_eq!(placed("position-area: bottom"), (6, 3, 3, 1));
    assert_eq!(placed("position-area: bottom span-right"), (5, 3, 3, 1));
    assert_eq!(placed("position-area: top left"), (2, 1, 3, 1));
    assert_eq!(placed("position-area: right"), (11, 2, 3, 1));
    assert_eq!(placed("position-area: center"), (6, 2, 3, 1));
    // `span-left` spans the left and centre columns: the box at its end.
    assert_eq!(placed("position-area: span-all span-left"), (8, 2, 3, 1));
}

/// §3.4: `anchor-center` centres the box on its default anchor, in the
/// largest part of its inset-modified containing block centred on it.
#[test]
fn anchor_center_centres_on_the_anchor() {
    assert_eq!(
        placed("top: anchor(bottom); justify-self: anchor-center"),
        (6, 3, 3, 1)
    );
    // An anchor at the left edge: the centred containing block is as wide
    // as the anchor allows.
    let mut dom = doc(r#"<body><div id="t">ab</div><div id="p">wide</div></body>"#);
    styled(
        &mut dom,
        &format!(
            "{PAGE} #t {{ anchor-name: --t; width: 2 }}
             #p {{ position: absolute; position-anchor: --t; top: anchor(bottom); justify-self: anchor-center }}"
        ),
        20,
        4,
    );
    assert_eq!(rect(&dom, "p"), (-1, 1, 4, 1));
}

/// §2.4: an anchor is acceptable only when laid out before the box is
/// placed (a positioned anchor later in tree order is not), and an
/// `anchor-scope` keeps a name to its subtree (§2.2).
#[test]
fn only_acceptable_anchors_anchor() {
    let mut dom = doc(
        r#"<body><div id="p">tip</div><div id="late">late</div><div id="s"><div id="in">in</div></div><div id="q">tip</div></body>"#,
    );
    styled(
        &mut dom,
        &format!(
            "{PAGE} #late {{ position: absolute; top: 5; left: 5; anchor-name: --late }}
             #p {{ position: absolute; top: anchor(--late bottom, 1); left: 0 }}
             #s {{ anchor-scope: --in }} #in {{ anchor-name: --in; margin-left: 9 }}
             #q {{ position: absolute; top: 3; left: anchor(--in left, 2) }}"
        ),
        20,
        8,
    );
    assert_eq!(
        rect(&dom, "p").1,
        1,
        "the later positioned anchor is not used"
    );
    assert_eq!(
        rect(&dom, "q").0,
        2,
        "the scoped name is not visible outside"
    );
}

/// HTML §6.12, Anchor Positioning 1 §2.3: a popover's implicit anchor is
/// its invoker — `position-area` places it there with no anchor name.
#[test]
fn a_popover_is_anchored_to_its_invoker() {
    use rdom_tui::runtime::builtins::popover;
    let mut dom = doc(
        r#"<body><p>x</p><button id="b" popovertarget="m">Menu</button><div id="m" popover>Copy</div></body>"#,
    );
    let (b, m) = (by_id(&dom, "b"), by_id(&dom, "m"));
    popover::show_popover_from(&mut dom, m, Some(b)).unwrap();
    styled(
        &mut dom,
        &format!("{PAGE} [popover] {{ position-area: bottom span-right }}"),
        30,
        10,
    );
    let (bx, by, _, bh) = rect(&dom, "b");
    let (mx, my, _, _) = rect(&dom, "m");
    assert_eq!((mx, my), (bx, by + i32::from(bh)));
}

/// §3 (scroll): an anchor inside a scroll container is read where the
/// scroll puts it — the anchored box follows a scroll.
#[test]
fn an_anchored_box_follows_its_scrolled_anchor() {
    let mut dom = doc(
        r#"<body><div id="sc"><p>1</p><p>2</p><p id="t">t</p><p>4</p><p>5</p></div><div id="p">tip</div></body>"#,
    );
    let css = format!(
        "{PAGE} #sc {{ height: 3; overflow: auto }} p {{ margin: 0 }}
         #t {{ anchor-name: --t }}
         #p {{ position: fixed; position-anchor: --t; top: anchor(bottom); left: 5 }}"
    );
    styled(&mut dom, &css, 20, 8);
    assert_eq!(rect(&dom, "p").1, 3);
    let sc = by_id(&dom, "sc");
    dom.node_mut(sc).ext_mut().unwrap().scroll_y = 2;
    dom.layout_dom(Rect::new(0, 0, 20, 8));
    assert_eq!(rect(&dom, "p").1, 1);
}

/// An anchor at column 5, row `row` (six wide) in a 20 × 8 viewport and
/// the box `#p` (three wide, `extra` styles) styled `css` — its rect.
fn tried(row: u16, css: &str) -> (i32, i32, u16, u16) {
    let mut dom = doc(r#"<body><div id="t">anchor</div><div id="p">tip</div></body>"#);
    styled(
        &mut dom,
        &format!(
            "{PAGE} #t {{ anchor-name: --t; margin: {row} 0 0 5; width: 6 }}
             #p {{ position: absolute; position-anchor: --t; {css} }}"
        ),
        20,
        8,
    );
    rect(&dom, "p")
}

/// §4.1, §4.3: when the base position overflows, the first fallback that
/// fits is used — `flip-block` swaps the block-axis insets and the anchor
/// sides they name; a fitting base is kept.
#[test]
fn flip_block_tries_the_other_side() {
    let css =
        "top: anchor(bottom); left: anchor(left); height: 3; position-try-fallbacks: flip-block";
    assert_eq!(tried(1, css), (5, 2, 3, 3), "the base fits below");
    assert_eq!(tried(6, css), (5, 3, 3, 3), "flipped above");
    // `flip-inline` on the inline axis; `flip-start` swaps the axes (and
    // the sizes): an anchor at column 12, row 2.
    let at12 = |css: &str| {
        let mut dom = doc(r#"<body><div id="t">anchor</div><div id="p">tip</div></body>"#);
        styled(
            &mut dom,
            &format!(
                "{PAGE} #t {{ anchor-name: --t; margin: 2 0 0 12; width: 6 }}
                 #p {{ position: absolute; position-anchor: --t; {css} }}"
            ),
            20,
            20,
        );
        rect(&dom, "p")
    };
    let css =
        "left: anchor(right); top: anchor(top); width: 4; position-try-fallbacks: flip-inline";
    assert_eq!(at12(css), (8, 2, 4, 1));
    // `flip-start`: `top: anchor(bottom)`, `left: anchor(left)`, `height:
    // 10`.
    let css =
        "left: anchor(right); top: anchor(top); width: 10; position-try-fallbacks: flip-start";
    assert_eq!(at12(css), (12, 3, 3, 10));
}

/// §4.1: a `@position-try` rule's declarations replace the box's, and a
/// `position-area` entry tries that area; one naming no rule is skipped.
#[test]
fn position_try_rules_and_areas_are_options() {
    let rule = "@position-try --above { top: auto; bottom: anchor(top) } ";
    let css = format!(
        "top: anchor(bottom); left: 0; height: 3; position-try-fallbacks: --missing, --above }} {rule} #q {{ color: red"
    );
    assert_eq!(tried(6, &css).1, 3);
    assert_eq!(
        tried(
            6,
            "position-area: bottom; height: 3; position-try-fallbacks: top"
        )
        .1,
        3
    );
}

/// §4.2: `position-try-order` tries the options with the largest
/// inset-modified containing block on its axis first.
#[test]
fn position_try_order_prefers_room() {
    let css = "top: anchor(bottom); left: 0; position-try-fallbacks: flip-block; position-try-order: most-height";
    // The anchor at row 2: below it 5 rows, above it 2 — below fits and is
    // larger.
    assert_eq!(tried(2, css).1, 3);
    // At row 5: above (5 rows) is larger than below (2): above, though
    // the base fits too.
    assert_eq!(tried(5, css).1, 4);
}

/// §5: `position-visibility` hides an anchored box — its default anchor
/// scrolled out of view (`anchors-visible`, the initial value), a
/// referenced anchor missing (`anchors-valid`), or every option
/// overflowing (`no-overflow`); `always` never does. A hidden box paints
/// nothing and is not hit.
#[test]
fn position_visibility_hides_anchored_boxes() {
    use rdom_tui::HitTestExt;
    let html = r#"<body><div id="sc"><p id="t">t</p><p>2</p><p>3</p><p>4</p></div><div id="p">TIP</div></body>"#;
    let css = |extra: &str| {
        format!(
            "{PAGE} #sc {{ height: 2; width: 5; overflow: auto }} p {{ margin: 0 }} #t {{ anchor-name: --t }}
             #p {{ position: fixed; position-anchor: --t; top: 5; left: anchor(right); {extra} }}"
        )
    };
    let shows = |extra: &str, scroll: i32| {
        let mut dom = doc(html);
        styled(&mut dom, &css(extra), 20, 8);
        let sc = by_id(&dom, "sc");
        dom.node_mut(sc).ext_mut().unwrap().scroll_y = scroll;
        let buf = paint(&mut dom, &css(extra), 20, 8);
        let tip = row(&buf, 5).contains("TIP");
        let (x, y, _, _) = rect(&dom, "p");
        let hit = dom.hit_test(x as u16, y as u16) == Some(by_id(&dom, "p"));
        assert_eq!(tip, hit, "{extra} {scroll}: painted and hit agree");
        tip
    };
    assert!(shows("", 0));
    assert!(!shows("", 3), "the anchor scrolled out of view");
    assert!(shows("position-visibility: always", 3));
    assert!(!shows(
        "position-visibility: anchors-valid; top: anchor(--none bottom)",
        0
    ));
    assert!(!shows("position-visibility: no-overflow; width: 30", 0));
    assert!(shows("position-visibility: always; width: 30", 0));
}
