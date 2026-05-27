//! Pins that overflow clipping, scrollbar paint, sticky scrollport,
//! and the runtime/CSSOM scroll math all use the CSS Overflow 3 §3
//! scrollport — the padding-box — not the layout-side `content_layout`
//! rect which can be widened into the border ring by M5.5b under
//! `border-collapse: collapse`.
//!
//! The visible regression these tests catch: a `<details>` (or any
//! `overflow: auto` element) with `border-top: solid` under
//! `border-collapse: collapse` would let scrolled child bg and the
//! scrollbar track both paint *into* the border-top row. CSS Overflow
//! 3 §3 places the scrollport at the padding-box — *always*, no
//! collapse exception — so the border row is outside the clip.

use rdom_tui::accessors::{TuiAccessors, TuiAccessorsMut};
use rdom_tui::render::{Buffer, Rect};
use rdom_tui::{CascadeExt, LayoutExt, NodeId, PaintExt, TuiDom, TuiNodeExt};

/// Build a minimal `parent > <pre>` tree where the parent has
/// `border-top: solid`, `overflow-y: auto`, optional `border-collapse:
/// collapse`, and a `<pre>` body taller than the parent's content
/// area. Returns `(dom, parent_id, child_id)`.
fn scroll_bleed_fixture(collapse: bool) -> (TuiDom, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("parent_el");
    let child = dom.create_element("child_el");
    dom.append_child(root, parent).unwrap();
    dom.append_child(parent, child).unwrap();

    let collapse_rule = if collapse {
        "border-collapse: collapse;"
    } else {
        ""
    };
    let css = format!(
        r#"
        parent_el {{
            display: block;
            width: 20;
            height: 8;
            border-top: solid;
            border-color: rgb(100, 100, 100);
            overflow-y: auto;
            background-color: rgb(20, 20, 20);
            {collapse_rule}
        }}
        child_el {{
            display: block;
            width: 20;
            height: 20;
            background-color: rgb(60, 60, 60);
        }}
        "#
    );
    let sheet = rdom_css::from_css(&css);
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 30, 12));
    (dom, parent, child)
}

fn paint(dom: &TuiDom) -> Buffer {
    let viewport = Rect::new(0, 0, 30, 12);
    let mut buf = Buffer::empty(viewport);
    dom.paint_dom(&mut buf, viewport);
    buf
}

/// Re-run cascade + layout after a scroll-state mutation (scroll is
/// applied at layout time as `cursor -= scroll_y`).
fn relayout(dom: &mut TuiDom, css_collapse: bool) {
    let collapse_rule = if css_collapse {
        "border-collapse: collapse;"
    } else {
        ""
    };
    let css = format!(
        r#"
        parent_el {{
            display: block;
            width: 20;
            height: 8;
            border-top: solid;
            border-color: rgb(100, 100, 100);
            overflow-y: auto;
            background-color: rgb(20, 20, 20);
            {collapse_rule}
        }}
        child_el {{
            display: block;
            width: 20;
            height: 20;
            background-color: rgb(60, 60, 60);
        }}
        "#
    );
    let sheet = rdom_css::from_css(&css);
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 30, 12));
}

/// True if the rendered cell at (x, y) is the border glyph `─`.
fn is_border_top_glyph(buf: &Buffer, x: u16, y: u16) -> bool {
    buf.cell(x, y).map(|c| c.symbol() == "─").unwrap_or(false)
}

// ── Test 4: the bleed regression ────────────────────────────────────

#[test]
fn overflow_auto_scroll_does_not_paint_over_border_top() {
    // The headline bug (user screenshot #3). Parent has border-top +
    // overflow-auto + border-collapse. Child has its own bg, taller
    // than the parent so scrolling shifts it up. Per CSS Overflow 3
    // §3, scrolled content is clipped at the padding-box edge —
    // never paints over the border row.
    //
    // Before fix: `children_clip` read `content_layout`, which under
    // M5.5b includes the border-top row. `paint_border` writes the
    // `─` glyph but NOT cell.bg (see paint_pass/mod.rs:311-315), so
    // the symbol survives — but the scrolled child's `fill_bg` then
    // overwrites the cell's BG with the child's color. The visible
    // bleed is in the cell's background color, not the glyph.
    let (mut dom, parent, _child) = scroll_bleed_fixture(true);

    // Capture the border-row bg color before any scroll. This is
    // the parent's bg (rgb(20,20,20) per the fixture CSS).
    let buf0 = paint(&dom);
    let pre_scroll_bg = buf0.cell(5, 0).map(|c| c.bg);
    assert!(
        pre_scroll_bg.is_some(),
        "pre-scroll: row 0 col 5 must have a cell"
    );

    // Scroll down by 3 rows.
    if let Some(ext) = dom.node_mut(parent).ext_mut() {
        ext.scroll_y = 3;
    }
    relayout(&mut dom, true);
    let buf1 = paint(&dom);

    // The bleed: border-row cell.bg must STAY the parent's bg, NOT
    // be overwritten by the child's bg.
    for x in 0..20 {
        assert!(
            is_border_top_glyph(&buf1, x, 0),
            "post-scroll: row 0 col {x} must still paint border-top glyph"
        );
        let bg = buf1.cell(x, 0).map(|c| c.bg);
        assert_eq!(
            bg, pre_scroll_bg,
            "post-scroll: row 0 col {x} bg ({bg:?}) must equal \
             pre-scroll bg ({pre_scroll_bg:?}). CSS Overflow 3 §3 \
             clips scrolled content (including bg) at the padding-box \
             edge; the border row is outside that clip."
        );
    }
}

// ── Test 5: scrollbar gutter excludes border row ────────────────────

#[test]
fn scrollbar_gutter_excludes_border_row_under_collapse() {
    // The companion bug (user's screenshot #2): the scrollbar track
    // overpainted into the border-top row because `paint_scrollbars`
    // read `content_layout` for the track y-range. Under M5.5b that
    // included the border row.
    //
    // CSS Overflow 3 §3 places the gutter inside the padding-box.
    // After fix, the track starts at row 1 (padding-box.y), not row 0.
    //
    // Detection: scrollbar paint sets a distinctive bg color. The
    // pre-scroll border row at the scrollbar column must keep its
    // border-row bg, NOT pick up the scrollbar's bg.
    let (dom, parent, _child) = scroll_bleed_fixture(true);

    let ext = dom.node(parent).tui_ext().expect("parent ext");
    let content = ext.content_layout;
    // CSS Overflow 3 §3 classic-platform model: the scrollbar lives
    // in the reserved gutter at content.right (= content.x +
    // content.width). The two-pass layout shrinks `content.width` by
    // 1 when overflow is detected, so the gutter sits in its own
    // dedicated column — no overlay over content.
    let track_x = (content.x + content.width as i32) as u16;

    let buf = paint(&dom);

    // (track_x, 0) is the border-top row at the scrollbar's x column.
    // Symbol MUST be the border glyph — paint_scrollbars must not
    // overwrite it.
    let border_cell = buf.cell(track_x, 0);
    let border_sym = border_cell.map(|c| c.symbol());
    assert_eq!(
        border_sym,
        Some("─"),
        "scrollbar must not overpaint border-top row at (col {track_x}, row 0); \
         got symbol {border_sym:?}. CSS Overflow 3 §3: gutter inside padding-box."
    );

    // Confirm scrollbar paint actually fired somewhere in the
    // padding-box rows (y >= 1), so this test isn't a no-op when the
    // scrollbar is simply absent.
    //
    // UA stylesheet (post-palette-refresh): `*::scrollbar-thumb`
    // ships fg only (no bg), so the thumb's signature is the `┃`
    // glyph itself appearing in the padding-box rows.
    let mut found_thumb_glyph = false;
    for y in 1..8 {
        if buf.cell(track_x, y).map(|c| c.symbol()) == Some("┃") {
            found_thumb_glyph = true;
            break;
        }
    }
    assert!(
        found_thumb_glyph,
        "scrollbar thumb (`┃` per UA stylesheet) should appear at col \
         {track_x} somewhere in rows 1..8 — if it doesn't, this test \
         isn't actually exercising scrollbar paint and the border-row \
         assertion above is vacuous"
    );
}

// ── Test 6: parent-child border sharing under collapse ──────────────

#[test]
fn first_child_with_matching_border_still_shares_under_collapse() {
    // Architect's regression concern: when the FIRST child has its
    // OWN border-top matching the parent's, M5.5b lets child.outer.y
    // = parent.outer.y (the border row). Does clipping at padding-box
    // wipe out the child's border?
    //
    // Answer under BORDER-MODEL-1's CSS Tables 3 §11.5 conflict
    // resolution: the per-direction joiner picks the highest-rank
    // contribution at the shared cell. Both parent and child
    // contribute Solid + same border-color, so the cell renders
    // the corresponding glyph (matched colors blend trivially).
    // The fix neither helps nor hurts this case visually; pin
    // the behavior.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("parent_el");
    let child = dom.create_element("child_el");
    dom.append_child(root, parent).unwrap();
    dom.append_child(parent, child).unwrap();

    let css = r#"
        parent_el {
            display: flex;
            flex-direction: column;
            width: 20;
            height: 10;
            border: solid;
            border-color: rgb(100, 100, 100);
            border-collapse: collapse;
        }
        child_el {
            width: 20;
            height: 5;
            border-top: solid;
            border-color: rgb(200, 50, 50);
        }
    "#;
    let sheet = rdom_css::from_css(css);
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 30, 12));

    // The shared border row exists at row 0 (parent's border-top
    // shared with child's border-top under collapse). Glyph must
    // still render `─` after paint.
    let viewport = Rect::new(0, 0, 30, 12);
    let mut buf = Buffer::empty(viewport);
    dom.paint_dom(&mut buf, viewport);

    for x in 1..19 {
        assert!(
            buf.cell(x, 0).map(|c| c.symbol() == "─").unwrap_or(false),
            "shared parent-child border-top must render at row 0 col {x}; \
             got {:?}",
            buf.cell(x, 0).map(|c| c.symbol())
        );
    }
}

// ── Test 7: overflow: visible unaffected ────────────────────────────

#[test]
fn overflow_visible_unchanged_by_padding_box_fix() {
    // When overflow is Visible, there's no clipping. padding_box is
    // computed but unused. Behavior must be identical to today.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("parent_el");
    dom.append_child(root, parent).unwrap();

    let css = r#"
        parent_el {
            display: block;
            width: 20;
            height: 5;
            border-top: solid;
            border-color: rgb(100, 100, 100);
            border-collapse: collapse;
            overflow: visible;
        }
    "#;
    let sheet = rdom_css::from_css(css);
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 30, 12));

    // No assertion beyond "doesn't panic and lays out". The fix
    // doesn't reach into the overflow:visible code path; this just
    // pins that we didn't accidentally regress it.
    let ext = dom.node(parent).tui_ext().expect("parent laid out");
    assert_eq!(ext.layout.height, 5);
}

// ── Test 9: horizontal axis equivalent ──────────────────────────────

#[test]
fn horizontal_axis_does_not_paint_over_border_left() {
    // Mirror of test 4 on the x-axis: border-left + overflow-x: auto +
    // collapse must clip scrolled content at the padding-box's left
    // edge.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("parent_el");
    let child = dom.create_element("child_el");
    dom.append_child(root, parent).unwrap();
    dom.append_child(parent, child).unwrap();

    let css = r#"
        parent_el {
            display: block;
            width: 12;
            height: 6;
            border-left: solid;
            border-color: rgb(100, 100, 100);
            overflow-x: auto;
            border-collapse: collapse;
            background-color: rgb(20, 20, 20);
        }
        child_el {
            display: block;
            width: 24;
            height: 6;
            background-color: rgb(60, 60, 60);
        }
    "#;
    let sheet = rdom_css::from_css(css);
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 30, 12));

    if let Some(ext) = dom.node_mut(parent).ext_mut() {
        ext.scroll_x = 5;
    }
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 30, 12));

    let viewport = Rect::new(0, 0, 30, 12);
    let mut buf = Buffer::empty(viewport);
    dom.paint_dom(&mut buf, viewport);

    // Column 0 is the parent's border-left. Must paint `│` glyphs at
    // every row inside the parent, even after horizontal scroll.
    for y in 0..6 {
        let cell = buf.cell(0, y).map(|c| c.symbol());
        assert_eq!(
            cell,
            Some("│"),
            "post-scroll: col 0 row {y} must still paint border-left; got {cell:?}"
        );
    }
}

// ── Test 11: scrollTop clamp uses padding-box, not content-box ──────

#[test]
fn scroll_top_clamp_uses_padding_box_viewport() {
    // `element.scrollTop = N` clamps to [0, scroll_content_height -
    // viewport]. CSSOM View defines viewport as the scrollport =
    // padding-box. The previous behavior used `content_layout.height`
    // which under M5.5b can include the border row; after fix the
    // max-scroll is 1 cell larger (because viewport is 1 cell smaller).
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("parent_el");
    let child = dom.create_element("child_el");
    dom.append_child(root, parent).unwrap();
    dom.append_child(parent, child).unwrap();

    let css = r#"
        parent_el {
            display: block;
            width: 20;
            height: 6;
            border-top: solid;
            border-color: rgb(100, 100, 100);
            overflow-y: auto;
            border-collapse: collapse;
        }
        child_el {
            display: block;
            width: 20;
            height: 20;
        }
    "#;
    let sheet = rdom_css::from_css(css);
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 30, 12));

    let ext = dom.node(parent).tui_ext().unwrap();
    let scroll_content_h = ext.scroll_content_height;
    let outer_h = ext.layout.height as usize;
    // padding_box height = outer_h - 1 (border-top eats 1 row).
    let padding_box_h = outer_h - 1;
    let expected_max = scroll_content_h.saturating_sub(padding_box_h);

    // Try to scroll past the end; clamp should land at expected_max.
    dom.node_mut(parent).set_scroll_top(i32::MAX).unwrap();
    let actual = dom.node(parent).scroll_top().unwrap_or(0) as usize;
    assert_eq!(
        actual, expected_max,
        "scrollTop max = scroll_content_height ({scroll_content_h}) - \
         padding_box_height ({padding_box_h}) = {expected_max}; got {actual}"
    );
}

// ── Two-pass auto-gutter (classic scrollbar) ────────────────────────

#[test]
fn overflow_auto_no_overflow_does_not_reserve_gutter() {
    // CSS Overflow 3: `scrollbar-gutter: auto` (default) means
    // "classic if platform is classic, overlay if platform is overlay."
    // TUI is structurally classic (no cell-overlay possible). The
    // classic semantic is: scrollbar consumes space ONLY when present.
    // No overflow → no scrollbar → content uses every cell.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("parent_el");
    let child = dom.create_element("child_el");
    dom.append_child(root, parent).unwrap();
    dom.append_child(parent, child).unwrap();

    let css = r#"
        parent_el {
            display: block;
            width: 20;
            height: 10;
            overflow-y: auto;
        }
        child_el {
            display: block;
            width: 20;
            height: 5;
        }
    "#;
    let sheet = rdom_css::from_css(css);
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 30, 12));

    let ext = dom.node(parent).tui_ext().unwrap();
    // No overflow → no gutter → content_layout uses full inner width
    // (= outer width here since no border / padding).
    assert_eq!(
        ext.content_layout.width, 20,
        "no overflow on Auto axis → no gutter reserved, content uses \
         every cell (CSS Overflow 3 classic-platform behavior)"
    );
}

#[test]
fn overflow_auto_with_overflow_reserves_gutter_after_two_pass() {
    // The two-pass fix: when overflow IS detected on an Auto axis
    // without `scrollbar-gutter: stable`, the substrate redoes the
    // layout with the gutter forced on. Content reflows one cell
    // narrower; scrollbar lives in its dedicated column. No cell-
    // overlay (impossible in a TUI medium) and no bg "under" the
    // scrollbar.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("parent_el");
    let child = dom.create_element("child_el");
    dom.append_child(root, parent).unwrap();
    dom.append_child(parent, child).unwrap();

    // Child uses default block width (= fills parent) so we can
    // observe the pass-2 reflow. An explicit `width` on the child
    // would *not* shrink (CSS block-formula respects explicit width)
    // — that overflow on the cross axis is its own concern.
    let css = r#"
        parent_el {
            display: block;
            width: 20;
            height: 6;
            overflow-y: auto;
        }
        child_el {
            display: block;
            height: 20;
        }
    "#;
    let sheet = rdom_css::from_css(css);
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 30, 12));

    let ext = dom.node(parent).tui_ext().unwrap();
    // Overflow detected (child 20 tall vs viewport 6) → gutter
    // reserved → content width drops by 1.
    assert_eq!(
        ext.content_layout.width, 19,
        "overflow detected on Auto axis → gutter reserved in pass 2 \
         (CSS Overflow 3: 'classic scrollbars consume space when \
         present'); content width = outer.width - 1"
    );
    // Auto-width block child reflows to the narrower viewport.
    let child_w = dom.node(child).tui_ext().unwrap().layout.width;
    assert_eq!(
        child_w, 19,
        "auto-width block child reflowed to (parent content_layout.width \
         after gutter reservation); got {child_w}"
    );
}

#[test]
fn overflow_auto_horizontal_axis_reserves_gutter_when_overflow() {
    // Mirror of the vertical case on overflow-x.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("parent_el");
    let child = dom.create_element("child_el");
    dom.append_child(root, parent).unwrap();
    dom.append_child(parent, child).unwrap();

    let css = r#"
        parent_el {
            display: block;
            width: 12;
            height: 10;
            overflow-x: auto;
        }
        child_el {
            display: block;
            width: 50;
            height: 5;
        }
    "#;
    let sheet = rdom_css::from_css(css);
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 60, 12));

    let ext = dom.node(parent).tui_ext().unwrap();
    // Horizontal overflow → reserve 1 row of height for the
    // horizontal scrollbar gutter.
    assert_eq!(
        ext.content_layout.height, 9,
        "overflow detected on Auto horizontal axis → gutter reserved, \
         content height = outer.height - 1"
    );
}

// ── Overflow on non-collapse element still works (non-regression) ───

#[test]
fn overflow_auto_without_collapse_unchanged() {
    // The fix is independent of border-collapse. Pin that the non-
    // collapse case still works — padding-box equals content_layout
    // here, so behavior must be identical.
    let (mut dom, parent, _child) = scroll_bleed_fixture(false);

    if let Some(ext) = dom.node_mut(parent).ext_mut() {
        ext.scroll_y = 2;
    }
    relayout(&mut dom, false);
    let buf = paint(&dom);

    // No collapse → no shared border row → no M5.5b expansion. The
    // border row at y=0 should remain intact (today's behavior).
    for x in 0..20 {
        assert!(
            is_border_top_glyph(&buf, x, 0),
            "non-collapse, post-scroll: row 0 col {x} must paint border-top"
        );
    }
}
