//! The paint trace's contract: a traced paint is the frame's paint, every
//! cell's last operation is what the cell shows, every write is
//! attributed, layers nest and blend, and borders say whose they are.

use rdom_core::NodeId;

use super::{OpKind, PaintPart, PaintTrace, trace_paint};
use crate::render::{Buffer, Cell, PaintExt, Rect};
use crate::style::{CascadeExt, Color};
use crate::{LayoutExt, TuiDom};

/// A page exercising the paint parts: a bordered panel with a background,
/// a paragraph with a styled span and a `::before`, a translucent banner,
/// a half-opaque group and an outlined button.
const MARKUP: &str = r#"<div class="panel"><p class="para">Hi <b class="bold">there</b></p><div class="banner">over</div><div class="group"><span class="g">ghost</span></div><button class="btn">Go</button></div>"#;

const CSS: &str = "
.panel { border: solid; background: #102030; color: #c0c0c0; width: 30; height: 9; }
.para::before { content: '» '; color: #ffcc00; }
.bold { color: #ff0000; font-weight: bold; }
.banner { background: rgba(255, 255, 255, 0.5); }
.group { opacity: 0.5; color: #00ff00; }
.btn { outline: solid #ff00ff; margin-left: 2; margin-top: 1; }
";

struct Page {
    dom: TuiDom,
    area: Rect,
}

fn page() -> Page {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let host = dom.create_element("div");
    dom.append_child(root, host).unwrap();
    rdom_parser::parse_into(&mut dom, MARKUP, host).expect("the test markup parses");
    let area = Rect::new(0, 0, 40, 12);
    let sheet = rdom_css::from_css(CSS);
    dom.cascade(&sheet);
    dom.layout_dom(area);
    Page { dom, area }
}

fn node(dom: &TuiDom, selector: &str) -> NodeId {
    dom.query_selector(selector)
        .expect("the element is on the page")
        .id()
}

fn untraced(p: &Page) -> Buffer {
    let mut buf = Buffer::empty(p.area);
    p.dom.paint_dom(&mut buf, p.area);
    buf
}

fn traced(p: &Page) -> (Buffer, PaintTrace) {
    trace_paint(&p.dom, p.area, p.area)
}

/// Where `text` starts on the screen.
fn find(buf: &Buffer, text: &str) -> (u16, u16) {
    let first = text.chars().next().unwrap().to_string();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            let row: String = (x..buf.area.width)
                .map(|cx| buf.cell(cx, y).unwrap().symbol().to_string())
                .collect();
            if buf.cell(x, y).unwrap().symbol() == first && row.starts_with(text) {
                return (x, y);
            }
        }
    }
    let screen: Vec<String> = (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf.cell(x, y).unwrap().symbol().to_string())
                .collect()
        })
        .collect();
    panic!("{text:?} is not on screen:\n{}", screen.join("\n"))
}

#[test]
fn a_traced_paint_is_the_frames_paint() {
    let p = page();
    let (buf, _) = traced(&p);
    assert_eq!(buf, untraced(&p));
    assert_eq!(buf.border_dirs, untraced(&p).border_dirs);
}

#[test]
fn every_cells_last_operation_is_what_it_shows() {
    let p = page();
    let (buf, trace) = traced(&p);
    for y in 0..p.area.height {
        for x in 0..p.area.width {
            let shown = buf.cell(x, y).unwrap();
            match trace.top_at(x, y) {
                Some(op) => assert_eq!(&op.cell, shown, "cell ({x}, {y}): {op:?}"),
                None => assert_eq!(
                    shown,
                    &Cell::EMPTY,
                    "cell ({x}, {y}) changed with no operation"
                ),
            }
        }
    }
}

#[test]
fn every_write_is_attributed_to_an_element() {
    let p = page();
    let (_, trace) = traced(&p);
    for op in trace.ops() {
        assert_ne!(op.part, PaintPart::Unattributed, "{op:?}");
        if op.part != PaintPart::BorderJoin {
            assert!(op.node.is_some(), "{op:?}");
        }
    }
}

#[test]
fn text_is_its_elements_and_the_background_its_boxs() {
    let p = page();
    let (buf, trace) = traced(&p);
    let (x, y) = find(&buf, "there");
    let top = trace.top_at(x, y).unwrap();
    assert_eq!(
        (top.node, top.part),
        (Some(node(&p.dom, ".bold")), PaintPart::Text)
    );
    let fill = trace
        .ops_at(x, y)
        .find(|op| op.part == PaintPart::Background)
        .expect("the panel's background is under the text");
    assert_eq!(fill.node, Some(node(&p.dom, ".panel")));
    let (bx, by) = find(&buf, "»");
    let before = trace.top_at(bx, by).unwrap();
    assert_eq!(
        before.part,
        PaintPart::Generated(crate::ext::StyleSlot::Before)
    );
    assert_eq!(before.node, Some(node(&p.dom, ".para")));
}

#[test]
fn a_border_glyph_is_the_joiners_from_the_boxs_contributions() {
    let p = page();
    let (_, trace) = traced(&p);
    let panel = node(&p.dom, ".panel");
    // The panel's top-left corner.
    let top = trace.top_at(0, 0).unwrap();
    assert_eq!(top.kind, OpKind::Join);
    assert_eq!(top.part, PaintPart::BorderJoin);
    assert_eq!(top.joined, [panel]);
    assert!(
        trace
            .ops_at(0, 0)
            .any(|op| op.kind == OpKind::Border && op.node == Some(panel)),
        "the contribution is recorded"
    );
}

#[test]
fn a_translucent_background_is_a_blend() {
    let p = page();
    let (buf, trace) = traced(&p);
    let (x, y) = find(&buf, "over");
    let banner = node(&p.dom, ".banner");
    let blend = trace
        .ops_at(x, y)
        .find(|op| matches!(op.kind, OpKind::Blend { .. }) && op.depth == 0)
        .expect("the banner's background blends");
    assert_eq!(blend.node, Some(banner));
    let OpKind::Blend { alpha } = blend.kind else {
        unreachable!()
    };
    assert!((alpha - 0.5).abs() < 0.01, "{alpha}");
    assert!(
        trace
            .ops_at(x, y)
            .any(|op| op.depth == 1 && op.node == Some(banner)),
        "its opaque fill in the layer precedes the blend"
    );
}

#[test]
fn a_group_paints_in_a_layer_and_blends_back() {
    let p = page();
    let (buf, trace) = traced(&p);
    let (x, y) = find(&buf, "ghost");
    let ops: Vec<_> = trace.ops_at(x, y).collect();
    let g = node(&p.dom, ".g");
    let inner = ops
        .iter()
        .position(|op| op.depth >= 1 && op.node == Some(g) && op.part == PaintPart::Text)
        .expect("the span's text is painted in the group's layer");
    let blend = ops
        .iter()
        .position(|op| op.depth == 0 && matches!(op.kind, OpKind::Blend { .. }))
        .expect("and composited back");
    assert!(inner < blend);
    assert_ne!(
        ops[blend].cell.fg,
        Color::Rgb(0, 255, 0),
        "blended, not the opaque color"
    );
}

#[test]
fn an_outline_is_its_elements() {
    let p = page();
    let (_, trace) = traced(&p);
    let btn = node(&p.dom, ".btn");
    assert!(
        trace
            .ops()
            .iter()
            .any(|op| op.part == PaintPart::Outline && op.node == Some(btn))
    );
}

#[test]
fn a_region_records_only_its_cells() {
    let p = page();
    let region = Rect::new(2, 1, 5, 2);
    let (buf, trace) = trace_paint(&p.dom, p.area, region);
    assert_eq!(buf, untraced(&p), "the paint is whole");
    assert!(!trace.ops().is_empty());
    assert!(trace.ops().iter().all(|op| region.contains(op.x, op.y)));
    assert_eq!(trace.region(), region);
}

#[test]
fn a_box_model_reads_the_boxes_the_frame_painted() {
    let p = page();
    let panel = super::box_model(&p.dom, node(&p.dom, ".panel")).expect("the panel has a box");
    assert_eq!(panel.border, crate::layout::LayoutRect::new(0, 0, 32, 11));
    assert_eq!(panel.padding, crate::layout::LayoutRect::new(1, 1, 30, 9));
    assert_eq!(
        panel.content,
        Some(crate::layout::LayoutRect::new(1, 1, 30, 9))
    );
    assert!(panel.scroll.is_none(), "not a scroll container");
    let text = p
        .dom
        .node(node(&p.dom, ".bold"))
        .first_child()
        .unwrap()
        .id();
    assert!(
        super::box_model(&p.dom, text).is_none(),
        "a text node has no box of its own"
    );
}

#[test]
fn a_scroll_containers_box_model_has_its_scroll_state() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let host = dom.create_element("div");
    dom.append_child(root, host).unwrap();
    rdom_parser::parse_into(
        &mut dom,
        "<div class=s><p>1</p><p>2</p><p>3</p><p>4</p></div>",
        host,
    )
    .unwrap();
    let area = Rect::new(0, 0, 10, 6);
    dom.cascade(&rdom_css::from_css(".s { height: 2; overflow-y: scroll; }"));
    dom.layout_dom(area);
    let s = super::box_model(&dom, node(&dom, ".s")).unwrap();
    let scroll = s.scroll.expect("a scroll container");
    assert_eq!(scroll.offset, (0, 0));
    assert_eq!(scroll.size.1, 4, "four rows of content");
    assert_eq!(scroll.port.height, 2);
}
