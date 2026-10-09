//! C14G-CONTAIN-FIDELITY — CSS Containment 2 in its corners: skipped
//! contents keep no geometry (§4), containment applies only to the boxes
//! §3.1–§3.4 name, a `fixed` box a contained element contains is clipped
//! and moved with it (§3.2, §3.4), a remembered size goes with
//! `contain-intrinsic-size: auto` (CSS Sizing 4 §6.1), and an aspect ratio
//! transfers the clamped size (CSS Sizing 4 §5.1).

use super::*;

fn rect(dom: &TuiDom, id: &str) -> rdom_tui::layout::LayoutRect {
    dom.node(by_id(dom, id)).layout_rect().expect("laid out")
}

fn row(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width)
        .map(|x| buf.cell(x, y).unwrap().symbol().to_string())
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// §4: skipped contents are not laid out — their rects read zero once the
/// element starts skipping (they kept the rects they had while rendered,
/// which `layout_rect()`, `scroll_into_view` and view timelines read).
#[test]
fn skipped_contents_keep_no_geometry() {
    let mut dom = doc(
        r#"<div id="c"><p id="p">x</p></div><details id="d" open><summary>S</summary><p id="q">y</p></details>"#,
    );
    styled(&mut dom, "p { margin: 0 }", 20, 6);
    assert_ne!(rect(&dom, "p").width, 0);
    assert_ne!(rect(&dom, "q").width, 0);
    let d = by_id(&dom, "d");
    dom.remove_attribute(d, "open").unwrap();
    styled(
        &mut dom,
        "p { margin: 0 } #c { content-visibility: hidden }",
        20,
        6,
    );
    assert_eq!((rect(&dom, "p").width, rect(&dom, "p").height), (0, 0));
    assert_eq!((rect(&dom, "q").width, rect(&dom, "q").height), (0, 0));
}

/// §3.1: size containment has no effect on a box whose inner display is
/// `table` or an internal table box — `table { contain: size }` keeps its
/// columns (it was 0 wide), `td { contain: size }` its content.
#[test]
fn size_containment_skips_tables_and_their_parts() {
    let mut dom = doc(r#"<table id="t"><tr><td id="d">abcdef</td></tr></table>"#);
    styled(
        &mut dom,
        "table { contain: size } td { contain: size; padding: 0 }",
        20,
        4,
    );
    assert!(rect(&dom, "t").width >= 6, "{:?}", rect(&dom, "t"));
    assert!(rect(&dom, "d").width >= 6, "{:?}", rect(&dom, "d"));
}

/// §4: `content-visibility` applies only where size containment can — a
/// non-atomic inline `span { content-visibility: hidden }` still shows its
/// text (it hid it).
#[test]
fn content_visibility_does_not_apply_to_an_inline() {
    let mut dom = doc(r#"<p>a<span id="s">bc</span>d</p>"#);
    let buf = paint(
        &mut dom,
        "p { margin: 0 } span { content-visibility: hidden }",
        10,
        2,
    );
    assert_eq!(row(&buf, 0), "abcd");
}

/// §3.4: paint containment has no effect on an internal table box other
/// than a cell — `tr { contain: paint }` does not clip.
#[test]
fn paint_containment_skips_a_table_row() {
    let mut dom = doc(r#"<table><tr id="r"><td id="d">ab</td></tr></table><p>z</p>"#);
    let css = "tr { contain: paint } td { padding: 0 } p { margin: 0 } \
               #d::after { content: 'X'; position: absolute; top: 2; left: 0 }";
    let buf = paint(&mut dom, css, 10, 4);
    // The abspos `X` is contained by the row only if paint containment
    // applied (§3.4 makes it a containing block); it did not, so it
    // takes the viewport's origin.
    assert_eq!(&row(&buf, 2)[..1], "X");
}

/// §3.4 / CSS Position 3 §2.1: a `fixed` box inside paint containment is
/// placed in the contained element and clipped by it (its clip was the
/// viewport's, painting it past the element).
#[test]
fn a_fixed_box_in_paint_containment_is_clipped_by_it() {
    let mut dom = doc(r#"<div id="c"><div id="f">F</div></div>"#);
    let buf = paint(
        &mut dom,
        "#c { contain: paint; height: 2; width: 10 } #f { position: fixed; top: 3; left: 0 }",
        20,
        6,
    );
    assert_eq!(rect(&dom, "f").y, 3, "placed in the container");
    assert_eq!(row(&buf, 3), "", "clipped by it");
}

/// §3.2: a `fixed` box a contained element contains moves with it when a
/// sticky ancestor sticks (it stayed where the viewport would hold it).
#[test]
fn a_fixed_box_in_layout_containment_moves_with_a_sticky_ancestor() {
    let mut dom = doc(
        r#"<div id="sc"><div id="pad"></div><div id="s"><div id="c"><div id="f">F</div></div></div><div id="tail"></div></div>"#,
    );
    let css = "#sc { height: 3; overflow: auto } #pad { height: 2 } #tail { height: 10 } \
               #s { position: sticky; top: 0 } #c { contain: layout } \
               #f { position: fixed; top: 0; left: 0 }";
    styled(&mut dom, css, 20, 6);
    let sc = by_id(&dom, "sc");
    rdom_tui::TuiAccessorsMut::scroll_to(&mut dom.node_mut(sc), 0, 4).unwrap();
    dom.layout_dom(Rect::new(0, 0, 20, 6));
    assert_eq!(rect(&dom, "s").y, 0, "stuck");
    assert_eq!(rect(&dom, "f").y, rect(&dom, "c").y, "with its container");
}

/// CSS Sizing 4 §6.1: a last remembered size is kept while
/// `contain-intrinsic-size` holds `auto`; once it does not, the size is
/// forgotten — back on `auto`, the length applies until the element
/// renders again.
#[test]
fn a_remembered_size_goes_with_auto() {
    let markup = r#"<div id="c"><p>a</p><p>b</p><p>c</p></div>"#;
    let mut dom = doc(markup);
    let base = "p { margin: 0 }";
    styled(
        &mut dom,
        &format!("{base} #c {{ contain-intrinsic-size: auto 1 }}"),
        20,
        8,
    );
    styled(
        &mut dom,
        &format!("{base} #c {{ contain-intrinsic-size: 1 }}"),
        20,
        8,
    );
    styled(
        &mut dom,
        &format!("{base} #c {{ contain-intrinsic-size: auto 1; content-visibility: hidden }}"),
        20,
        8,
    );
    assert_eq!(rect(&dom, "c").height, 1, "the remembered 3 was dropped");
}

/// CSS Sizing 4 §5.1: the size transferred through the ratio is the
/// clamped one — `height: 9; max-height: 5; aspect-ratio: 2` is 10 wide
/// (it was 18), `height: 2; min-height: 4` 8.
#[test]
fn an_aspect_ratio_transfers_the_clamped_height() {
    let mut dom = doc(r#"<div id="a"></div><div id="b"></div>"#);
    styled(
        &mut dom,
        "#a { height: 9; max-height: 5; aspect-ratio: 2 } #b { height: 2; min-height: 4; aspect-ratio: 2 }",
        40,
        20,
    );
    assert_eq!((rect(&dom, "a").width, rect(&dom, "a").height), (10, 5));
    assert_eq!((rect(&dom, "b").width, rect(&dom, "b").height), (8, 4));
}
