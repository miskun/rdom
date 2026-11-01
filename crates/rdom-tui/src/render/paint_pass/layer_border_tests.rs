//! Borders across paint layers: a later stacking context covers the
//! borders beneath it (CSS 2.1 Appendix E — stacking contexts paint
//! atomically, in order), and inside one stacking context borders of
//! unrelated boxes take the paint order, while a collapse group keeps
//! CSS Tables 3 §11.5 conflict resolution. See DIVERGENCES
//! "A stacking context covers the borders beneath it".

use crate::prelude::*;

// ── Helpers ──────────────────────────────────────────────────────

const RED: Color = Color::Rgb(255, 0, 0);
const BLUE: Color = Color::Rgb(0, 0, 255);
const WHITE: Color = Color::Rgb(255, 255, 255);

fn render(html: &str, css: &str, w: u16, h: u16) -> Buffer {
    let mut dom = TuiDom::new();
    let root = dom.root();
    rdom_parser::parse_into(&mut dom, html, root).expect("markup parses");
    let sheet = rdom_css::from_css_strict(css).expect("css parses");
    let viewport = Rect::new(0, 0, w, h);
    dom.cascade(&sheet);
    dom.layout_dom(viewport);
    let mut buf = Buffer::empty(viewport);
    dom.paint_dom(&mut buf, viewport);
    buf
}

fn row(buf: &Buffer, y: u16) -> String {
    (buf.area.x..buf.area.right())
        .filter_map(|x| buf.cell(x, y))
        .filter(|c| !c.is_spacer())
        .map(|c| c.symbol().to_string())
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// One letter per cell of row `y`: `R` / `B` / `W` for a glyph in red,
/// blue or white, `.` for a blank, `?` for any other color.
fn colors(buf: &Buffer, y: u16) -> String {
    (buf.area.x..buf.area.right())
        .filter_map(|x| buf.cell(x, y))
        .filter(|c| !c.is_spacer())
        .map(|c| match c.fg {
            _ if c.symbol() == " " => '.',
            f if f == RED => 'R',
            f if f == BLUE => 'B',
            f if f == WHITE => 'W',
            _ => '?',
        })
        .collect::<String>()
        .trim_end_matches('.')
        .to_string()
}

fn screen(buf: &Buffer) -> String {
    (buf.area.y..buf.area.bottom())
        .map(|y| format!("{:<24}{}", row(buf, y), colors(buf, y)))
        .collect::<Vec<_>>()
        .join("\n")
}

// ── Tree guides paint in the tree's own layer ────────────────────

#[test]
fn a_higher_layer_covers_tree_guides() {
    // The guides are part of the tree's paint, so a fixed overlay above
    // the tree covers them like any other content of the tree.
    let buf = render(
        r#"<ul role="tree"><li role="treeitem" aria-expanded="true">root<ul role="group"><li role="treeitem">a</li><li role="treeitem">b</li><li role="treeitem">c</li></ul></li></ul><div class="ov">xxxxxxxx</div>"#,
        ".ov { position: fixed; left: 0; top: 1; width: 10; height: 2; background-color: blue; z-index: 100 }",
        14,
        5,
    );
    assert_eq!(row(&buf, 1), "xxxxxxxx", "{}", screen(&buf));
    assert_eq!(row(&buf, 2), "", "{}", screen(&buf));
    assert_eq!(row(&buf, 3), "└   c", "{}", screen(&buf));
}

// ── A stacking context covers the borders beneath it ─────────────

/// A collapsed flex container (white) with two collapsed children,
/// 22 × 8 at the origin, its children's shared edge at column 10.
const COLLAPSED: &str = r#"<div class="c"><div class="a"></div><div class="b"></div></div>"#;
const COLLAPSED_CSS: &str = ".c { display: flex; border-collapse: collapse; border: solid; color: white; width: 20; height: 6 }
     .a, .b { flex: 1; border: solid }";

#[test]
fn a_higher_z_index_overlay_covers_the_borders_it_crosses() {
    // The reported case: the overlay's whole ring is its own, in its own
    // color, over the container's top edge and across the children's
    // shared edge; nothing beneath joins it. Inside, where the overlay
    // paints nothing, the shared edge shows through.
    let buf = render(
        &format!(r#"{COLLAPSED}<div class="ov"></div>"#),
        &format!(
            "{COLLAPSED_CSS}
             .ov {{ position: absolute; left: 4; top: 0; width: 12; height: 3; border: solid; color: red; z-index: 10 }}"
        ),
        24,
        8,
    );
    let s = screen(&buf);
    assert_eq!(row(&buf, 0), "┌───┌────────────┐───┐", "{s}");
    assert_eq!(colors(&buf, 0), "WWWWRRRRRRRRRRRRRRWWWW", "{s}");
    assert_eq!(row(&buf, 1), "│   │     │      │   │", "{s}");
    assert_eq!(colors(&buf, 1), "W...R.....W......R...W", "{s}");
    assert_eq!(row(&buf, 4), "│   └────────────┘   │", "{s}");
    assert_eq!(colors(&buf, 4), "W...RRRRRRRRRRRRRR...W", "{s}");
    assert_eq!(row(&buf, 5), "│         │          │", "{s}");
}

#[test]
fn the_overlay_wins_wherever_it_sits_in_the_document() {
    // Stacking order, not document order: the same overlay first in the
    // document paints the same.
    let html = format!(r#"<div class="ov"></div>{COLLAPSED}"#);
    let css = format!(
        "{COLLAPSED_CSS}
         .ov {{ position: absolute; left: 4; top: 0; width: 12; height: 3; border: solid; color: red; z-index: 10 }}"
    );
    let buf = render(&html, &css, 24, 8);
    assert_eq!(row(&buf, 0), "┌───┌────────────┐───┐", "{}", screen(&buf));
    assert_eq!(
        colors(&buf, 0),
        "WWWWRRRRRRRRRRRRRRWWWW",
        "{}",
        screen(&buf)
    );
}

#[test]
fn an_overlays_rounded_corner_stays_rounded_over_a_line() {
    // Nothing beneath joins the overlay's corner, so it keeps its arc.
    let buf = render(
        &format!(r#"{COLLAPSED}<div class="ov"></div>"#),
        &format!(
            "{COLLAPSED_CSS}
             .ov {{ position: absolute; left: 4; top: 0; width: 12; height: 3; border: solid; border-radius: 1; color: red; z-index: 10 }}"
        ),
        24,
        8,
    );
    assert_eq!(row(&buf, 0), "┌───╭────────────╮───┐", "{}", screen(&buf));
}

#[test]
fn equal_z_index_boxes_stack_in_paint_order() {
    // Two `z-index: 1` boxes: the later one (blue) is on top. Its ring
    // covers the red ring where they cross; inside the blue box, which
    // paints no background, the red ring shows.
    let buf = render(
        r#"<div class="p"></div><div class="q"></div>"#,
        ".p, .q { position: absolute; z-index: 1; width: 10; height: 4; border: solid }
         .p { left: 0; top: 0; color: red }
         .q { left: 4; top: 2; color: blue }",
        16,
        8,
    );
    let s = screen(&buf);
    assert_eq!(row(&buf, 2), "│   ┌──────────┐", "{s}");
    assert_eq!(colors(&buf, 2), "R...BBBBBBBBBBBB", "{s}");
    assert_eq!(row(&buf, 3), "│   │      │   │", "{s}");
    assert_eq!(colors(&buf, 3), "R...B......R...B", "{s}");
    assert_eq!(row(&buf, 5), "└───│──────┘   │", "{s}");
    assert_eq!(colors(&buf, 5), "RRRRBRRRRRRR...B", "{s}");
}

#[test]
fn a_negative_z_index_box_stays_beneath_in_flow_borders() {
    // Appendix E: negative z-index paints before the in-flow content,
    // so the white box's ring covers the red one where they cross.
    let buf = render(
        r#"<div class="w"></div><div class="n"></div>"#,
        ".w { border: solid; color: white; width: 8; height: 2 }
         .n { position: absolute; left: 4; top: 1; width: 8; height: 4; border: solid; color: red; z-index: -1 }",
        16,
        7,
    );
    let s = screen(&buf);
    assert_eq!(row(&buf, 1), "│   ┌────│───┐", "{s}");
    assert_eq!(colors(&buf, 1), "W...RRRRRWRRRR", "{s}");
    assert_eq!(row(&buf, 3), "└────────┘   │", "{s}");
    assert_eq!(colors(&buf, 3), "WWWWWWWWWW...R", "{s}");
}

#[test]
fn an_opacity_group_covers_the_borders_beneath_it() {
    // `opacity < 1` makes a stacking context: the translucent box's top
    // edge is its own line over the inner box's top edge, not a junction
    // with it.
    let buf = render(
        r#"<div class="o"><div class="i"></div></div><div class="ov"></div>"#,
        ".o { border: solid; color: white; width: 20; height: 6 }
         .i { border: solid; height: 4 }
         .ov { position: absolute; left: 4; top: 1; width: 12; height: 3; border: solid; color: red; opacity: 0.75 }",
        22,
        8,
    );
    assert_eq!(row(&buf, 1), "│┌──┌────────────┐──┐│", "{}", screen(&buf));
    assert_eq!(row(&buf, 5), "││  └────────────┘  ││", "{}", screen(&buf));
}

#[test]
fn a_z_indexed_member_of_a_collapse_group_keeps_conflict_resolution() {
    // A collapse group resolves its shared edges as one (CSS Tables 3
    // §11.5), whatever stacking context a member forms: the thick blue
    // edge wins the shared column over the z-indexed red member's thin
    // one, though the red member paints later.
    let buf = render(
        r#"<div class="c"><div class="a"></div><div class="b"></div></div>"#,
        ".c { display: flex; border-collapse: collapse; width: 20; height: 4 }
         .a { position: relative; z-index: 1; flex: 1; border: solid; color: red }
         .b { flex: 1; border: thick solid; color: blue }",
        22,
        5,
    );
    let s = screen(&buf);
    let shared = (0..22u16)
        .find(|&x| buf.cell(x, 1).is_some_and(|c| c.symbol() == "┃"))
        .unwrap_or_else(|| panic!("no heavy shared edge:\n{s}"));
    assert_eq!(buf.cell(shared, 1).unwrap().fg, BLUE, "{s}");
    assert_eq!(buf.cell(shared, 2).unwrap().symbol(), "┃", "{s}");
}

// ── Inside a stacking context: collapse groups resolve, others stack ──

#[test]
fn a_later_box_in_the_same_context_takes_the_lines_it_paints() {
    // A positioned `z-index: auto` overlay paints after the in-flow
    // boxes in the same stacking context: its lines are its own (red)
    // where it crosses theirs, and the junctions still join every
    // line that meets there, in the color of the box painted last.
    let buf = render(
        &format!(r#"{COLLAPSED}<div class="ov"></div>"#),
        &format!(
            "{COLLAPSED_CSS}
             .ov {{ position: absolute; left: 4; top: 0; width: 12; height: 3; border: solid; color: red }}"
        ),
        24,
        8,
    );
    let s = screen(&buf);
    assert_eq!(row(&buf, 0), "┌───┬─────┬──────┬───┐", "{s}");
    assert_eq!(colors(&buf, 0), "WWWWRRRRRRRRRRRRRRWWWW", "{s}");
    assert_eq!(colors(&buf, 1), "W...R.....W......R...W", "{s}");
    assert_eq!(row(&buf, 4), "│   └─────┼──────┘   │", "{s}");
    assert_eq!(colors(&buf, 4), "W...RRRRRRRRRRRRRR...W", "{s}");
}

#[test]
fn an_overlapping_later_box_owns_the_shared_line() {
    // Two boxes overlapping by a row (a negative margin), neither in a
    // collapse group: the later (blue) paints over the earlier (red), so
    // the shared row is blue — even where the earlier border is heavier —
    // and its ends join the red sides above.
    let buf = render(
        r#"<div class="x"></div><div class="y"></div>"#,
        ".x { border: solid; color: red; width: 8; height: 1 }
         .y { border: solid; color: blue; width: 8; height: 1; margin-top: -1 }",
        12,
        5,
    );
    let s = screen(&buf);
    assert_eq!(row(&buf, 2), "├────────┤", "{s}");
    assert_eq!(colors(&buf, 2), "BBBBBBBBBB", "{s}");
    assert_eq!(colors(&buf, 1), "R........R", "{s}");

    let heavy = render(
        r#"<div class="x"></div><div class="y"></div>"#,
        ".x { border: thick solid; color: red; width: 8; height: 1 }
         .y { border: solid; color: blue; width: 8; height: 1; margin-top: -1 }",
        12,
        5,
    );
    let s = screen(&heavy);
    assert!(row(&heavy, 2).contains("────────"), "{s}");
    assert_eq!(colors(&heavy, 2), "BBBBBBBBBB", "{s}");
}

#[test]
fn a_collapse_group_still_resolves_its_shared_edges() {
    // CSS Tables 3 §11.5 inside the group, unchanged: the container's
    // thick red ring beats its children's thin blue edges on it, though
    // the children paint later; their shared edge stays theirs.
    let buf = render(
        COLLAPSED,
        ".c { display: flex; border-collapse: collapse; border: thick solid; color: red; width: 20; height: 6 }
         .a, .b { flex: 1; border: solid; color: blue }",
        24,
        8,
    );
    let s = screen(&buf);
    assert_eq!(colors(&buf, 0), "RRRRRRRRRRRRRRRRRRRRRR", "{s}");
    assert_eq!(buf.cell(1, 0).unwrap().symbol(), "━", "{s}");
    assert_eq!(buf.cell(10, 2).unwrap().symbol(), "│", "{s}");
    assert_eq!(buf.cell(10, 2).unwrap().fg, BLUE, "{s}");
}

#[test]
fn a_hidden_edge_does_not_erase_a_later_unrelated_border() {
    // `hidden` suppresses its collapse group's edge (§11.5 rule 1); a box
    // outside the group painted later over that edge still shows its own.
    let buf = render(
        &format!(r#"{COLLAPSED}<div class="ov"></div>"#),
        &format!(
            "{COLLAPSED_CSS}
             .a {{ border-top: hidden }}
             .ov {{ position: absolute; left: 2; top: 0; width: 4; height: 1; border: solid; color: red }}"
        ),
        24,
        8,
    );
    let s = screen(&buf);
    let top: String = (2..8u16)
        .map(|x| buf.cell(x, 0).unwrap().symbol().to_string())
        .collect();
    assert_eq!(top, "┌────┐", "{s}");
    assert_eq!(&colors(&buf, 0)[2..8], "RRRRRR", "{s}");
}

#[test]
fn removing_the_overlay_restores_the_borders_beneath() {
    // The next frame, painted into the same buffer once the overlay is
    // gone, draws the borders beneath as if it had never been there.
    let mut dom = TuiDom::new();
    let root = dom.root();
    rdom_parser::parse_into(
        &mut dom,
        &format!(r#"{COLLAPSED}<div class="ov"></div>"#),
        root,
    )
    .expect("markup parses");
    let sheet = rdom_css::from_css_strict(&format!(
        "{COLLAPSED_CSS}
         .ov {{ position: absolute; left: 4; top: 0; width: 12; height: 3; border: solid; color: red; z-index: 10 }}"
    ))
    .expect("css parses");
    let viewport = Rect::new(0, 0, 24, 8);
    let mut buf = Buffer::empty(viewport);
    dom.cascade(&sheet);
    dom.layout_dom(viewport);
    dom.paint_dom(&mut buf, viewport);
    assert_eq!(row(&buf, 0), "┌───┌────────────┐───┐", "{}", screen(&buf));

    let overlay = dom.query_selector(".ov").expect("overlay").id();
    dom.remove_child(root, overlay).unwrap();
    dom.cascade(&sheet);
    dom.layout_dom(viewport);
    buf.clear();
    dom.paint_dom(&mut buf, viewport);
    let without = render(COLLAPSED, COLLAPSED_CSS, 24, 8);
    assert_eq!(screen(&buf), screen(&without));
    assert_eq!(row(&buf, 0), "┌─────────┬──────────┐", "{}", screen(&buf));
}

// ── Out-of-flow children are not in a collapse group ─────────────

/// A collapsed frame (white), 22 × 12, with a header and a list sharing
/// its edges — junctions `├` / `┤` at row 5 — and an absolutely
/// positioned 1-column handle over its left edge, rows 1–10 (`.h`).
const FRAME: &str = r#"<main class="m"><div class="h"></div><header class="hd">Title</header><section class="ls">rows</section></main>"#;
const FRAME_CSS: &str = ".m { position: relative; display: flex; flex-direction: column; width: 20; height: 10; border: solid; color: white; border-collapse: collapse }
     .hd { height: 4; flex-shrink: 0; border: solid }
     .ls { flex: 1; border-left: solid; border-right: solid; border-bottom: solid }
     .h { position: absolute; left: -1; top: 0; bottom: 0; width: 1 }";

#[test]
fn an_absolutely_positioned_handle_draws_a_straight_line_over_the_edge() {
    // Out of flow, the handle shares no edge with the frame (CSS 2.1
    // §9.3.1; CSS Tables 3 §11.5 resolves the boxes that share grid
    // lines): its own straight line covers the frame's left edge, the
    // junction included; the frame's corners, outside it, stay the frame's.
    let buf = render(
        FRAME,
        &format!("{FRAME_CSS} .h {{ z-index: 10; border-left: solid red }}"),
        24,
        12,
    );
    let s = screen(&buf);
    let column: String = (0..12u16)
        .map(|y| buf.cell(0, y).unwrap().symbol().to_string())
        .collect();
    assert_eq!(column, "┌││││││││││└", "{s}");
    for y in 1..11u16 {
        assert_eq!(buf.cell(0, y).unwrap().fg, RED, "row {y}\n{s}");
    }
    assert_eq!(buf.cell(0, 0).unwrap().fg, WHITE, "{s}");
    assert_eq!(buf.cell(0, 11).unwrap().fg, WHITE, "{s}");
    assert_eq!(row(&buf, 5), "│────────────────────┤", "{s}");
}

#[test]
fn an_idle_handle_with_a_transparent_border_leaves_the_junctions_be() {
    // A handle that draws nothing until hovered covers nothing.
    let buf = render(
        FRAME,
        &format!("{FRAME_CSS} .h {{ z-index: 10; border-left: solid transparent }}"),
        24,
        12,
    );
    assert_eq!(row(&buf, 5), "├────────────────────┤", "{}", screen(&buf));
    assert_eq!(row(&buf, 0), "┌────────────────────┐", "{}", screen(&buf));
}

#[test]
fn a_recolored_collapsed_edge_keeps_its_junctions() {
    // The other design: recolor the shared edge itself — the frame's and
    // its in-flow children's left sides. Its junctions stay junctions; a
    // thick edge outweighs the thin separators, so they take its color
    // too (§11.5: the wider border wins; a thin one would leave each
    // junction to its box's horizontal side, DIVERGENCES "A border corner
    // takes one side's color").
    let buf = render(
        FRAME,
        &format!("{FRAME_CSS} .m, .m > .hd, .m > .ls {{ border-left: thick solid red }}"),
        24,
        12,
    );
    let s = screen(&buf);
    assert_eq!(buf.cell(0, 5).unwrap().symbol(), "┠", "{s}");
    for y in 0..12u16 {
        assert_eq!(buf.cell(0, y).unwrap().fg, RED, "row {y}\n{s}");
    }
}
