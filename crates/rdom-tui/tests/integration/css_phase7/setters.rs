//! C7G-GRID-SETTERS — a grid built only through the `TuiNodeMutExt` node
//! setters (CSS Grid 2): `display: grid`, the track lists, the named
//! areas, `gap`, and each placement form — an area name, the four-line
//! `grid-area`, `grid-row` / `grid-column` — reach the cascade and lay
//! out as the same CSS would.

use super::rect;
use rdom_tui::prelude::*;
use rdom_tui::render::Rect;

/// A `div` with the text `x`, appended to `parent`.
fn item(dom: &mut TuiDom, parent: NodeId) -> NodeId {
    let id = dom.create_element("div");
    let t = dom.create_text_node("x");
    dom.append_child(id, t).unwrap();
    dom.append_child(parent, id).unwrap();
    id
}

/// §7.2–§7.3 and §8.3–§8.4 through node setters, at 20 × 7: columns
/// `6 1fr`, rows `auto 1fr auto`, `gap: 1`, areas `"head head" "nav
/// main" "foot foot"`. The columns are 6 and 13 (20 less 6 and the gap),
/// the rows 1, 3 and 1 (7 less the two auto rows and two gaps, §11.7);
/// each item fills its area (`normal` stretches, §10.3–§10.4).
#[test]
fn node_setters_drive_grid_layout() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let page = dom.create_element("div");
    dom.append_child(root, page).unwrap();
    dom.node_mut(page)
        .set_grid()
        .set_height(7u16)
        .set_grid_template_columns(vec![TrackSize::cells(6), TrackSize::fr(1.0)])
        .set_grid_template_rows(vec![TrackSize::AUTO, TrackSize::fr(1.0), TrackSize::AUTO])
        .set_grid_template_areas(
            GridTemplateAreas::new(["head head", "nav main", "foot foot"]).unwrap(),
        )
        .set_gap(1);
    let head = item(&mut dom, page);
    dom.node_mut(head).set_grid_area_named("head");
    let nav = item(&mut dom, page);
    dom.node_mut(nav).set_grid_area(
        GridLine::line(2),
        GridLine::line(1),
        GridLine::span(1),
        GridLine::nth_named(1, "main-start"),
    );
    let main = item(&mut dom, page);
    dom.node_mut(main)
        .set_grid_row(GridLine::named("main"), GridLine::Auto)
        .set_grid_column(GridLine::line(2), GridLine::line(-1));
    let foot = item(&mut dom, page);
    dom.node_mut(foot).set_grid_area_named("foot");

    let sheet = Stylesheet::bare();
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 20, 7));

    let at = |id| {
        let r = rect(&dom, id);
        (r.x, r.y, r.width, r.height)
    };
    assert_eq!(at(head), (0, 0, 20, 1));
    assert_eq!(at(nav), (0, 2, 6, 3));
    assert_eq!(at(main), (7, 2, 13, 3));
    assert_eq!(at(foot), (0, 6, 20, 1));
}

/// CSS Display 3 §2.1: `set_grid` is `display: grid` (block outside, grid
/// inside), `set_inline_grid` is `display: inline-grid`.
#[test]
fn display_grid_node_setters_compute_their_display() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let (g, i) = (dom.create_element("div"), dom.create_element("span"));
    dom.append_child(root, g).unwrap();
    dom.append_child(g, i).unwrap();
    dom.node_mut(g).set_grid();
    dom.node_mut(i).set_inline_grid();
    dom.cascade(&Stylesheet::bare());
    let c = dom.node(g).computed().unwrap();
    assert_eq!((c.display, c.flow), (Display::Block, Flow::Grid));
    // A grid item is blockified (§2.7): `inline-grid` computes to `grid`.
    let c = dom.node(i).computed().unwrap();
    assert_eq!((c.display, c.flow), (Display::Block, Flow::Grid));
}
