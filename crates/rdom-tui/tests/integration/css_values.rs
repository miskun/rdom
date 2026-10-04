//! CSS-COMPLETE Phase 2 — values, units and math functions, end to end:
//! a sheet parsed by `rdom_css`, cascaded and laid out by `rdom-tui`.
//! One section per item (`C2-*`); each test cites the spec text that
//! fixes the expected value.

use rdom_tui::render::Rect;
use rdom_tui::style::cascade::computed_of;
use rdom_tui::{CascadeExt, LayoutExt, LayoutRect, NodeId, TuiDom, TuiNodeExt, Viewport};

fn el(dom: &mut TuiDom, parent: NodeId, class: &str) -> NodeId {
    let id = dom.create_element("div");
    if !class.is_empty() {
        dom.set_attribute(id, "class", class).unwrap();
    }
    dom.append_child(parent, id).unwrap();
    id
}

/// Cascade `css` (strict: no warning allowed) and lay out at `cols` × `rows`.
fn lay_out(dom: &mut TuiDom, css: &str, cols: u16, rows: u16) {
    let sheet = rdom_css::from_css_strict(css).expect("sheet parses without warnings");
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, cols, rows));
}

fn rect(dom: &TuiDom, id: NodeId) -> LayoutRect {
    dom.node(id).layout_rect().expect("laid out")
}

fn content(dom: &TuiDom, id: NodeId) -> LayoutRect {
    dom.node(id).content_layout_rect().expect("laid out")
}

// ── C2-PERCENT ───────────────────────────────────────────────────────

/// CSS Box 3 §4.2 / CSS 2.1 §8.4: a `padding` percentage "refers to the
/// logical width of the containing block" — on all four sides, so the
/// vertical padding of a 40 × 30 containing block is 10% of 40.
#[test]
fn percent_padding_resolves_against_containing_block_width_on_every_side() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let cb = el(&mut dom, root, "cb");
    let child = el(&mut dom, cb, "in");
    lay_out(
        &mut dom,
        ".cb { width: 40; height: 30 } .in { padding: 10%; height: 20 }",
        80,
        40,
    );
    let (outer, inner) = (rect(&dom, child), content(&dom, child));
    assert_eq!(inner.x - outer.x, 4, "padding-left = 10% of 40");
    assert_eq!(
        inner.y - outer.y,
        4,
        "padding-top = 10% of the width 40, not of the height"
    );
    assert_eq!(outer.width - inner.width, 8, "left + right");
    assert_eq!(
        outer.height - inner.height,
        8,
        "top + bottom, against the width"
    );
}

/// The same rule in a flex item's intrinsic (content-based) size: the
/// horizontal padding that narrows the text's wrap width is a share of
/// the containing block's width (40), not of the item's own (30). With
/// 10 + 10 padding the 14-column text wraps onto two rows inside 10.
#[test]
fn percent_padding_in_intrinsic_size_uses_the_containing_block_width() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let col = el(&mut dom, root, "col");
    let item = el(&mut dom, col, "in");
    let text = dom.create_text_node("aaaa bbbb cccc");
    dom.append_child(item, text).unwrap();
    lay_out(
        &mut dom,
        ".col { display: flex; flex-direction: column; width: 40; height: 20 }
         .in { width: 30; padding: 0 25% }",
        80,
        40,
    );
    assert_eq!(content(&dom, item).width, 10, "30 - 2 × 25% of 40");
    assert_eq!(rect(&dom, item).height, 2, "two rows at a wrap width of 10");
}

/// CSS Box 3 §3.2 / CSS 2.1 §8.3: margin percentages refer to the
/// containing block's width, vertical margins included.
#[test]
fn percent_margin_resolves_against_containing_block_width() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let cb = el(&mut dom, root, "cb");
    let child = el(&mut dom, cb, "in");
    lay_out(
        &mut dom,
        ".cb { width: 40; height: 30 } .in { margin: 10% 0 0 5%; height: 2 }",
        80,
        40,
    );
    let (parent, r) = (rect(&dom, cb), rect(&dom, child));
    assert_eq!(r.y - parent.y, 4, "margin-top = 10% of the width 40");
    assert_eq!(r.x - parent.x, 2, "margin-left = 5% of 40");
    assert_eq!(r.width, 38, "auto width fills what the margin leaves");
}

/// CSS Position 3 §3.1: inset percentages refer to the containing
/// block's size on the matching axis (`top` / `bottom` its height,
/// `left` / `right` its width). The `inset` shorthand takes the same
/// values.
#[test]
fn percent_insets_resolve_against_the_containing_block() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let cb = el(&mut dom, root, "cb");
    let a = el(&mut dom, cb, "a");
    let b = el(&mut dom, cb, "b");
    let r = el(&mut dom, cb, "r");
    lay_out(
        &mut dom,
        ".cb { position: relative; width: 40; height: 20 }
         .a { position: absolute; top: 50%; left: 25%; width: 2; height: 2 }
         .b { position: absolute; inset: 10% auto auto calc(50% + 1); width: 2; height: 2 }
         .r { position: relative; top: 10%; left: -10%; height: 1 }",
        80,
        40,
    );
    let p = rect(&dom, cb);
    assert_eq!((rect(&dom, a).x - p.x, rect(&dom, a).y - p.y), (10, 10));
    assert_eq!((rect(&dom, b).x - p.x, rect(&dom, b).y - p.y), (21, 2));
    assert_eq!((rect(&dom, r).x - p.x, rect(&dom, r).y - p.y), (-4, 2));
}

/// CSS Sizing 3 §5.2: `min-*` / `max-*` percentages resolve against the
/// containing block's size on the same axis, and clamp the used size.
#[test]
fn percent_min_and_max_sizes_clamp_against_the_containing_block() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let cb = el(&mut dom, root, "cb");
    let a = el(&mut dom, cb, "a");
    let b = el(&mut dom, cb, "b");
    let c = el(&mut dom, cb, "c");
    let d = el(&mut dom, cb, "d");
    let row = el(&mut dom, root, "row");
    let f = el(&mut dom, row, "f");
    let g = el(&mut dom, row, "g");
    lay_out(
        &mut dom,
        ".cb { width: 40; height: 20 }
         .a { width: 5; min-width: 50%; height: 1 }
         .b { width: 30; max-width: 25%; height: 1 }
         .c { height: 1; min-height: calc(25% + 2) }
         .d { height: 18; max-height: 25% }
         .row { display: flex; flex-direction: row; width: 40; height: 3 }
         .f { width: 30; max-width: 50% }
         .g { width: 2; min-height: 50%; max-height: 1 }",
        80,
        40,
    );
    assert_eq!(rect(&dom, a).width, 20, "min-width: 50% of 40");
    assert_eq!(rect(&dom, b).width, 10, "max-width: 25% of 40");
    assert_eq!(rect(&dom, c).height, 7, "min-height: 25% of 20, plus 2");
    assert_eq!(rect(&dom, d).height, 5, "max-height: 25% of 20");
    assert_eq!(
        rect(&dom, f).width,
        20,
        "flex item, main axis: max-width 50% of 40"
    );
    assert_eq!(
        rect(&dom, g).height,
        2,
        "flex item, cross axis: min-height 50% of 3, over max-height"
    );
}

/// CSS Color 4 §11.1: `opacity: <opacity-value>` is `<number> |
/// <percentage>`; a percentage is the number divided by 100.
#[test]
fn percent_opacity_is_the_fraction() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let a = el(&mut dom, root, "a");
    lay_out(&mut dom, ".a { opacity: 50% }", 10, 5);
    assert_eq!(computed_of(&dom, a).opacity, 0.5);
}

// ── C2-NUMBER ────────────────────────────────────────────────────────

/// Lay out `items` (one class each) in a 40-column flex row under `css`
/// and return their widths.
fn row_widths(css: &str, items: &[&str]) -> Vec<u16> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let row = el(&mut dom, root, "row");
    let ids: Vec<NodeId> = items.iter().map(|c| el(&mut dom, row, c)).collect();
    lay_out(
        &mut dom,
        &format!(".row {{ display: flex; flex-direction: row; width: 40; height: 1 }} {css}"),
        80,
        10,
    );
    ids.iter().map(|&id| rect(&dom, id).width).collect()
}

/// CSS Flexbox §7.1: `<flex-grow>` and `<flex-shrink>` are `<number
/// [0,∞]>` — fractions included — and §9.7 step 4.b: when the
/// unfrozen items' flex factors sum to less than one, they share only
/// that fraction of the free space.
#[test]
fn fractional_flex_grow_shares_by_weight() {
    assert_eq!(
        row_widths(".a { flex: 1.5 } .b { flex: 0.5 }", &["a", "b"]),
        [30, 10]
    );
    assert_eq!(
        row_widths(".a { flex: 0.5 }", &["a"]),
        [20],
        "a sum below 1 takes that share"
    );
    assert_eq!(
        row_widths(".a { flex: 0.25 } .b { flex: 0.25 }", &["a", "b"]),
        [10, 10]
    );
    assert_eq!(
        row_widths(".a { width: 1.5fr } .b { width: 0.5fr }", &["a", "b"]),
        [30, 10]
    );
}

/// CSS Flexbox §9.7 step 4.c / 4.d: negative free space is shared in
/// proportion to `flex-shrink × flex base size`, fractions included;
/// with factors summing below one only that fraction of the overflow
/// is taken away.
#[test]
fn fractional_flex_shrink_scales_by_base_size() {
    assert_eq!(
        row_widths(
            ".a { width: 40; flex-shrink: 0.5 } .b { width: 40; flex-shrink: 1.5 }",
            &["a", "b"]
        ),
        [30, 10],
        "40 of overflow, shrunk 1 : 3"
    );
    assert_eq!(
        row_widths(
            ".a { width: 40; flex-shrink: 0.25 } .b { width: 40; flex-shrink: 0.25 }",
            &["a", "b"]
        ),
        [30, 30],
        "factors sum to 0.5: half the overflow is taken"
    );
}

// ── C2-MINMAX ────────────────────────────────────────────────────────

/// Lay out `items` (one class each) as blocks in a 40 × 20 containing
/// block under `css` and return their border-box rects.
fn block_rects(css: &str, items: &[&str]) -> Vec<LayoutRect> {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let cb = el(&mut dom, root, "cb");
    let ids: Vec<NodeId> = items.iter().map(|c| el(&mut dom, cb, c)).collect();
    lay_out(
        &mut dom,
        &format!(".cb {{ position: relative; width: 40; height: 20 }} {css}"),
        80,
        30,
    );
    ids.iter().map(|&id| rect(&dom, id)).collect()
}

/// CSS Values 4 §10.2: `min()` / `max()` take the smallest / largest
/// of their comma-separated calculations, `clamp(MIN, VAL, MAX)` is
/// `max(MIN, min(VAL, MAX))` (MIN wins a conflict) with `none` for an
/// absent bound; percentages resolve at layout like any calculation,
/// and the functions nest inside `calc()` and each other.
#[test]
fn min_max_clamp_resolve_with_percentages_at_layout() {
    let r = block_rects(
        ".a { width: min(50%, 30); height: 1 }
         .b { width: max(25%, 15); height: 1 }
         .c { width: clamp(10, 50% + 5, 22); height: 1 }
         .d { width: clamp(30, 10, 20); height: 1 }
         .e { width: clamp(none, 90%, 20); height: 1 }
         .f { width: calc(min(10%, 3) * 2 + max(1, 2)); height: 1 }
         .g { width: MAX(3, 7); height: min(5, 10%, 30) }
         .h { position: absolute; left: max(-5, 10%); top: clamp(1, 50%, 3); width: 1; height: 1 }",
        &["a", "b", "c", "d", "e", "f", "g", "h"],
    );
    let w: Vec<u16> = r.iter().map(|r| r.width).collect();
    assert_eq!(w[..7], [20, 15, 22, 30, 20, 8, 7]);
    assert_eq!(r[6].height, 2, "min(5, 10% of 20, 30)");
    assert_eq!(
        (r[7].x, r[7].y),
        (4, 3),
        "insets: max(-5, 10% of 40), clamp(1, 10, 3)"
    );
}

// ── C2-STEPPED ───────────────────────────────────────────────────────

/// CSS Values 4 §10.3: `round(<rounding-strategy>?, A, B?)` rounds A to
/// a multiple of B (1 when omitted) — `nearest` (ties toward +∞), `up`,
/// `down`, `to-zero`; `mod()` takes B's sign, `rem()` A's. §10.7:
/// `abs()` and `sign()`. Percentages resolve at layout first.
#[test]
fn stepped_and_sign_functions_resolve_at_layout() {
    let r = block_rects(
        ".a { width: round(down, 50% + 3, 4); height: 1 }
         .b { width: round(up, 21, 4); height: 1 }
         .c { width: round(2.5); height: 1 }
         .d { width: round(to-zero, -50%, 7); height: round(nearest, 10%, 3) }
         .e { width: mod(-7, 5); height: 1 }
         .f { width: abs(-30%); height: 1 }
         .g { width: calc((1 + sign(50% - 10)) * 3); height: calc(sign(-2) + 2) }
         .h { position: absolute; left: rem(-7, 5); top: mod(7, -5); width: 1; height: 1 }",
        &["a", "b", "c", "d", "e", "f", "g", "h"],
    );
    let w: Vec<u16> = r.iter().map(|r| r.width).collect();
    assert_eq!(w[..7], [20, 24, 3, 0, 3, 12, 6]);
    assert_eq!(r[3].height, 3, "round(nearest, 2, 3): the nearer multiple");
    assert_eq!(r[6].height, 1, "sign(-2) + 2");
    let cb = r[0].y; // `a` sits at the containing block's top edge
    assert_eq!((r[7].x, r[7].y - cb), (-2, -3), "rem: A's sign; mod: B's");
}

// ── C2-TRIG ──────────────────────────────────────────────────────────

/// CSS Values 4 §10.4 – §10.6: trigonometric (`sin()` … `atan2()`, a
/// `<number>` argument in radians, the inverse functions returning an
/// `<angle>`) and exponential functions (`pow()`, `sqrt()`, `hypot()`,
/// `log()`, `exp()`); §10.7.1 the constants `e`, `pi`, `infinity`,
/// `-infinity` and `NaN`. The value becomes whole cells where it
/// becomes a length; a top-level NaN is 0 and infinity clamps (§10.9).
#[test]
fn trig_exponential_functions_and_constants() {
    let r = block_rects(
        ".a { width: calc(sin(pi / 6) * 20); height: calc(cos(0) * 7) }
         .b { width: calc(tan(atan(2)) * 5); height: calc(sin(atan2(1, 1)) * sin(atan2(1, 1)) * 10) }
         .c { width: pow(2, 3); height: sqrt(81) }
         .d { width: hypot(30%, 16); height: calc(log(e) * 5) }
         .e { width: calc(log(8, 2) + exp(0)); height: min(infinity, 4) }
         .f { width: max(-infinity, 3); height: 1 }
         .g { width: calc(NaN); height: max(nan, 5) }",
        &["a", "b", "c", "d", "e", "f", "g"],
    );
    let wh: Vec<(u16, u16)> = r.iter().map(|r| (r.width, r.height)).collect();
    assert_eq!(
        wh,
        [(10, 7), (10, 5), (8, 9), (20, 5), (4, 4), (3, 1), (0, 0)]
    );
}

// ── C2-CH ────────────────────────────────────────────────────────────

/// CSS Values 4 §6.1.1: `ch` is the advance of the "0" glyph — exactly
/// one column on a monospaced character grid. It works wherever a
/// length does, fractions rounding onto the grid where the value
/// becomes a length.
#[test]
fn ch_is_one_column() {
    let r = block_rects(
        ".a { width: 10ch; height: 1 }
         .b { width: calc(50% - 2.5ch); height: 1 }
         .c { width: 4CH; margin-left: -1.5ch; height: 1 }
         .d { position: absolute; left: 2ch; top: 0; width: max(3ch, 10%); height: 1 }",
        &["a", "b", "c", "d"],
    );
    assert_eq!(r[0].width, 10);
    assert_eq!(r[1].width, 18, "20 - 2.5 = 17.5, ties to even");
    assert_eq!((r[2].width, r[2].x), (4, -2), "-1.5ch rounds to -2");
    assert_eq!((r[3].x, r[3].width), (2, 4));
}

// ── C2-LH ────────────────────────────────────────────────────────────

/// CSS Values 4 §6.1.1: `lh` is the element's computed `line-height`,
/// `rlh` the root's. rdom's line is one row until `line-height` lands
/// (C9-LINE-HEIGHT), so both are one cell.
#[test]
fn lh_and_rlh_are_one_row() {
    let r = block_rects(
        ".a { height: 3lh; width: 2rlh }
         .b { height: calc(25% + 1.5LH); width: max(1rlh, 10%) }",
        &["a", "b"],
    );
    assert_eq!((r[0].height, r[0].width), (3, 2));
    assert_eq!(
        (r[1].height, r[1].width),
        (6, 4),
        "5 + 1.5 = 6.5, ties to even"
    );
}

// ── C2-VIEWPORT ──────────────────────────────────────────────────────

/// CSS Values 4 §6.1.2: a viewport-percentage length is 1% of the
/// initial containing block — the terminal: `vw` / `vi` of its columns,
/// `vh` / `vb` of its rows (horizontal-tb), `vmin` / `vmax` of the
/// smaller / larger. A terminal has one viewport size, so the small
/// (`sv*`), large (`lv*`) and dynamic (`dv*`) variants equal the plain
/// ones. They are absolute at computed-value time (§6.1.2), so a
/// percent-bearing expression keeps only its percentage for layout.
#[test]
fn viewport_units_are_percentages_of_the_terminal() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let cb = el(&mut dom, root, "cb");
    let names = ["a", "b", "c", "d", "e", "f"];
    let ids: Vec<NodeId> = names.iter().map(|c| el(&mut dom, cb, c)).collect();
    let sheet = rdom_css::from_css_strict(
        ".cb { width: 40; height: 30 }
         .a { width: 50vw; height: 20vh }
         .b { width: 10vmax; height: 10vmin }
         .c { width: 25vi; height: 50vb }
         .d { width: 50svw; height: 50LVH }
         .e { width: 10dvmax; height: calc(10svmin + 1) }
         .f { width: calc(100% - 25vw); height: min(5vh, 50%) }",
    )
    .unwrap();
    dom.cascade_all_in(&[&sheet], Viewport::new(80, 20));
    dom.layout_dom(Rect::new(0, 0, 80, 20));
    let wh: Vec<(u16, u16)> = ids
        .iter()
        .map(|&id| (rect(&dom, id).width, rect(&dom, id).height))
        .collect();
    assert_eq!(
        wh,
        [(40, 4), (8, 2), (20, 10), (40, 10), (8, 3), (20, 1)],
        "80 × 20 terminal; .f: 40 - 20, min(1, 15)"
    );
}

/// Viewport units follow the terminal: a resize re-resolves them on the
/// next frame (the `App` cascades the whole tree against the new size).
#[test]
fn viewport_units_follow_a_terminal_resize() {
    use crossterm::event::Event as CtEvent;
    use rdom_tui::{App, Terminal, TestBackend};
    let mut dom = TuiDom::new();
    let root = dom.root();
    let a = el(&mut dom, root, "a");
    let sheet = rdom_css::from_css_strict(".a { width: 50vw; height: 20vh }").unwrap();
    let terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.draw_if_dirty().unwrap();
    assert_eq!(
        (rect(app.dom(), a).width, rect(app.dom(), a).height),
        (40, 4)
    );
    app.terminal_mut().backend_mut().resize(40, 10);
    app.handle_event(CtEvent::Resize(40, 10));
    app.draw_if_dirty().unwrap();
    assert_eq!(
        (rect(app.dom(), a).width, rect(app.dom(), a).height),
        (20, 2)
    );
}
