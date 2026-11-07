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

// ── phase 2 review: borders through layers, clips, painters ──

/// A page of `markup` styled by `css` in a 30 × 8 viewport.
fn page_of(markup: &str, css: &str) -> Page {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let host = dom.create_element("div");
    dom.append_child(root, host).unwrap();
    rdom_parser::parse_into(&mut dom, markup, host).unwrap();
    let area = Rect::new(0, 0, 30, 8);
    dom.cascade(&rdom_css::from_css(css));
    dom.layout_dom(area);
    Page { dom, area }
}

/// Every join's `joined` names exactly `expected` at the box's corner.
fn corner_join(p: &Page, x: u16, y: u16) -> Vec<NodeId> {
    let (_, trace) = traced(p);
    let top = trace.top_at(x, y).expect("a border glyph");
    assert_eq!(top.kind, OpKind::Join, "{top:?}");
    top.joined.clone()
}

#[test]
fn a_border_inside_a_group_is_its_elements_not_the_groups() {
    let p = page_of(
        r#"<div class="g"><div class="b">x</div></div>"#,
        ".g { opacity: 0.5; } .b { border: solid; width: 6; height: 3; }",
    );
    assert_eq!(corner_join(&p, 0, 0), [node(&p.dom, ".b")]);
    let (_, trace) = traced(&p);
    let painter = trace.painter_at(1, 1).expect("the x");
    assert_eq!(
        (painter.node, painter.part),
        (Some(node(&p.dom, ".b")), PaintPart::Text)
    );
    assert_eq!(painter.depth, 1, "painted in the group's layer");
}

#[test]
fn a_border_in_nested_groups_is_still_its_elements() {
    let p = page_of(
        r#"<div class="g"><div class="h"><div class="b">x</div></div></div>"#,
        ".g { opacity: 0.8; } .h { opacity: 0.6; } .b { border: solid; width: 6; height: 3; }",
    );
    assert_eq!(corner_join(&p, 0, 0), [node(&p.dom, ".b")]);
    let (_, trace) = traced(&p);
    let painter = trace.painter_at(1, 1).unwrap();
    assert_eq!((painter.node, painter.depth), (Some(node(&p.dom, ".b")), 2));
}

#[test]
fn a_translucent_backdrop_tints_borders_without_taking_them() {
    let mut p = page_of(
        r#"<div class="b">page</div><dialog class="d">hi</dialog>"#,
        ".b { border: solid; width: 8; height: 3; } dialog::backdrop { background-color: rgba(0, 0, 0, 0.5); }",
    );
    let d = node(&p.dom, ".d");
    crate::runtime::builtins::dialog::show_modal(&mut p.dom, d).unwrap();
    p.dom.cascade(&rdom_css::from_css(
        ".b { border: solid; width: 8; height: 3; } dialog::backdrop { background-color: rgba(0, 0, 0, 0.5); }",
    ));
    p.dom.layout_dom(p.area);
    let (buf, trace) = traced(&p);
    assert_eq!(buf, untraced(&p));
    assert_eq!(
        corner_join(&p, 0, 0),
        [node(&p.dom, ".b")],
        "the tint recolors, the box keeps it"
    );
    assert!(
        trace
            .ops_at(0, 0)
            .any(|op| op.part == PaintPart::Backdrop && op.node == Some(d)),
        "the backdrop's tint is recorded on the cell"
    );
}

#[test]
fn a_clip_path_puts_the_backdrop_back_and_says_so() {
    let p = page_of(
        r#"<div class="o"><div class="c">clipped</div></div>"#,
        ".o { border: solid; width: 20; height: 6; } .c { border: solid; width: 12; height: 4; clip-path: inset(1); }",
    );
    let (buf, trace) = traced(&p);
    assert_eq!(buf, untraced(&p));
    let c = node(&p.dom, ".c");
    // The clipped element's own border ring (row 1, cols 1–12) is clipped
    // away: the frame shows the outer box's inside there.
    for x in 1..13 {
        let painter = trace.painter_at(x, 1);
        assert!(
            painter.is_none_or(|op| op.node != Some(c) || op.part == PaintPart::Group),
            "cell ({x}, 1) is not the clipped element's: {painter:?}"
        );
        if let Some(op) = painter {
            assert_eq!(
                &op.cell,
                buf.cell(x, 1).unwrap(),
                "the painter's cell is the frame's"
            );
        }
    }
    assert_eq!(corner_join(&p, 0, 0), [node(&p.dom, ".o")]);
}

#[test]
fn a_wide_glyphs_two_cells_are_its_texts() {
    let p = page_of(r#"<p class="w">中a</p>"#, "");
    let (buf, trace) = traced(&p);
    let w = node(&p.dom, ".w");
    for x in 0..3 {
        let top = trace.top_at(x, 0).unwrap();
        assert_eq!((top.node, top.part), (Some(w), PaintPart::Text), "cell {x}");
        assert_eq!(&top.cell, buf.cell(x, 0).unwrap());
    }
    assert!(buf.cell(1, 0).unwrap().is_spacer());
}

#[test]
fn a_region_inside_a_layer_records_only_its_cells_through_the_layer() {
    let p = page_of(
        r#"<div class="g"><p class="t">ghost text</p></div>"#,
        ".g { opacity: 0.5; }",
    );
    let region = Rect::new(2, 0, 2, 1);
    let (buf, trace) = trace_paint(&p.dom, p.area, region);
    assert_eq!(buf, untraced(&p));
    assert!(trace.ops().iter().all(|op| region.contains(op.x, op.y)));
    let painter = trace.painter_at(2, 0).unwrap();
    assert_eq!((painter.node, painter.depth), (Some(node(&p.dom, ".t")), 1));
}

#[test]
fn an_inline_box_has_no_content_box_of_its_own() {
    let p = page();
    let bold = super::box_model(&p.dom, node(&p.dom, ".bold")).expect("a rendered inline box");
    assert_eq!(bold.content, None);
    assert!(bold.border.width > 0);
}

// ── style traces (phase 3) ──

/// A page of `markup` cascaded under `css` read from "app.css".
fn styled(markup: &str, css: &str) -> (TuiDom, crate::style::Stylesheet) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let host = dom.create_element("div");
    dom.append_child(root, host).unwrap();
    rdom_parser::parse_into(&mut dom, markup, host).unwrap();
    crate::seed_inline_styles(&mut dom);
    let sheet = rdom_css::from_css_at(css, "app.css");
    dom.cascade(&sheet);
    (dom, sheet)
}

fn trace_of(dom: &TuiDom, sheet: &crate::style::Stylesheet, selector: &str) -> super::StyleTrace {
    super::explain_style(dom, &[sheet], node(dom, selector)).expect("a styled element")
}

#[test]
fn the_more_specific_rule_wins_and_the_other_is_overridden() {
    let (dom, sheet) = styled(
        r#"<div class="a"><p class="t">x</p></div>"#,
        "p { color: blue; }\n.a p { color: red; }",
    );
    let t = trace_of(&dom, &sheet, ".t");
    let w = t.winner("color").unwrap();
    assert_eq!(w.rule.as_ref().unwrap().selector, ".a p");
    assert_eq!(w.origin, super::DeclarationOrigin::Author);
    let over = t.overridden("color");
    assert_eq!(over.len(), 1);
    assert_eq!(over[0].rule.as_ref().unwrap().selector, "p");
}

#[test]
fn important_inverts_origin_and_beats_specificity() {
    let (dom, sheet) = styled(
        r#"<p id="x" class="t" style="color: green">x</p>"#,
        "#x { color: red; }\np { color: blue !important; }",
    );
    let t = trace_of(&dom, &sheet, ".t");
    let w = t.winner("color").unwrap();
    assert!(w.important);
    assert_eq!(w.rule.as_ref().unwrap().selector, "p");
    let origins: Vec<_> = t.overridden("color").iter().map(|d| d.origin).collect();
    assert_eq!(
        origins,
        [
            super::DeclarationOrigin::Inline,
            super::DeclarationOrigin::Author
        ],
        "the inline style beat #x, and lost to !important"
    );
}

#[test]
fn an_unlayered_rule_beats_a_layered_one() {
    let (dom, sheet) = styled(
        r#"<p class="t">x</p>"#,
        "@layer base { .t { color: red; } }\np { color: blue; }",
    );
    let t = trace_of(&dom, &sheet, ".t");
    assert_eq!(
        t.winner("color").unwrap().rule.as_ref().unwrap().selector,
        "p"
    );
}

#[test]
fn a_var_is_traced_as_its_substituted_value_and_custom_properties_are_listed() {
    let (dom, sheet) = styled(
        r#"<div class="h"><p class="t">x</p></div>"#,
        ".h { --fg: #123456; }\n.t { color: var(--fg); }",
    );
    let t = trace_of(&dom, &sheet, ".t");
    let w = t.winner("color").unwrap();
    assert!(!w.value.contains("var("), "{w:?}");
    assert_eq!(w.rule.as_ref().unwrap().selector, ".t");
    let h = node(&dom, ".h");
    let r = super::explain_style(&dom, &[&sheet], h).unwrap();
    assert_eq!(r.winner("--fg").unwrap().value.trim(), "#123456");
    assert!(super::inherits("--fg") && super::inherits("color") && !super::inherits("padding-top"));
}

#[test]
fn a_shorthand_is_traced_as_its_longhands_and_rules_say_where_they_are() {
    let (dom, sheet) = styled(r#"<p class="t">x</p>"#, "\n\n.t {\n  padding: 1 2;\n}\n");
    let t = trace_of(&dom, &sheet, ".t");
    assert!(t.winner("padding").is_none(), "shorthands are not listed");
    let top = t.winner("padding-top").unwrap();
    let right = t.winner("padding-right").unwrap();
    assert_ne!(top.value, right.value);
    let rule = top.rule.as_ref().unwrap();
    assert_eq!(rule.source_name.as_deref(), Some("app.css"));
    assert_eq!(rule.location.map(|l| (l.line, l.column)), Some((3, 1)));
}

#[test]
fn a_ua_declaration_is_the_user_agents() {
    let (dom, sheet) = styled(r#"<button class="b">go</button>"#, "");
    let t = trace_of(&dom, &sheet, ".b");
    let ua: Vec<_> = t
        .declarations()
        .iter()
        .filter(|d| d.origin == super::DeclarationOrigin::UserAgent)
        .collect();
    assert!(!ua.is_empty(), "the UA styles a button");
    assert!(
        ua.iter()
            .all(|d| d.rule.as_ref().is_some_and(|r| r.location.is_none()))
    );
}

#[test]
fn a_rules_declaration_is_listed_once() {
    // A flow-relative declaration keeps the block's later declarations
    // for its direction-mapped replay: the rule holds `color` twice.
    let (dom, sheet) = styled(
        r#"<p class="t">x</p>"#,
        ".t { margin-inline-start: 1; color: red; }",
    );
    let t = trace_of(&dom, &sheet, ".t");
    let colors: Vec<_> = t
        .declarations()
        .iter()
        .filter(|d| d.property == "color")
        .collect();
    assert_eq!(colors.len(), 1, "{colors:?}");
    assert!(t.overridden("color").is_empty());
}
