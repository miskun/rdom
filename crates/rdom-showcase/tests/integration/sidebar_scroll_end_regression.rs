//! Regression: scrolling the sidebar to its end must leave no empty
//! row between the last item and the bottom border. Caused by
//! `record_scroll_content_size` overcounting by the `top_inset` that
//! `collapse_parent_edge_insets` adds under M5.5b (shifts the first
//! child off the parent's top border). The overcount inflated
//! `scroll_content_height` by 1, which inflated `max_scroll` by 1,
//! which let the user scroll past the last content row.

use rdom_showcase::{DEMOS, ShowcaseState, build_shell, mount_demo, shell::base_stylesheet};
use rdom_tui::node::TuiNodeExt;
use rdom_tui::render::{Buffer, Rect};
use rdom_tui::{CascadeExt, LayoutExt, PaintExt, TuiDom};

#[test]
fn sidebar_scrolled_to_end_has_no_gap_before_bottom_border() {
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);
    let mut state = ShowcaseState::from_handles(&handles);
    mount_demo(&mut state, &mut dom, 0);

    let base = base_stylesheet();
    let mut sheets = vec![base];
    for d in DEMOS {
        sheets.push(d.stylesheet());
    }
    let refs: Vec<&_> = sheets.iter().collect();

    let viewport = Rect::new(0, 0, 80, 20);
    dom.cascade_all(&refs);
    dom.layout_dom(viewport);

    let sidebar = handles.sidebar;
    let ext = dom.node(sidebar).tui_ext().unwrap();
    let pb_h = ext.layout.height.saturating_sub(2); // border top + bottom
    let max_scroll = ext.scroll_content_height.saturating_sub(pb_h as usize);

    if let Some(ext) = dom.node_mut(sidebar).ext_mut() {
        ext.scroll_y = max_scroll;
    }
    dom.cascade_all(&refs);
    dom.layout_dom(viewport);

    let mut buf = Buffer::empty(viewport);
    dom.paint_dom(&mut buf, viewport);

    // Sidebar's bottom border lives at sidebar.outer.bottom - 1.
    let sb = dom.node(sidebar).tui_ext().unwrap().layout;
    let border_row = (sb.y + sb.height as i32 - 1) as u16;

    // The row JUST ABOVE the bottom border must contain a sidebar
    // item (• something or ▾ Category), not be blank. Probe the first
    // few interior columns — we just need to confirm SOMETHING painted.
    let row_above = border_row - 1;
    let any_glyph = (1..(sb.width - 1)).any(|x| {
        let cell = buf.cell(x, row_above);
        cell.map(|c| c.symbol() != " " && c.symbol() != "")
            .unwrap_or(false)
    });
    assert!(
        any_glyph,
        "row above bottom border (y={row_above}) must hold sidebar content \
         after scrolling to max; if blank, `record_scroll_content_size` is \
         likely overcounting by the M5.5b top_inset again"
    );
}
