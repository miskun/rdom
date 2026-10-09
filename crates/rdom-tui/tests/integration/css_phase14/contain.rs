//! `contain` (CSS Containment 2 §2–§3, Containment 3) and `will-change`
//! (CSS Will Change 1 §2–§3): each containment type's effect on layout,
//! paint and generated content.

use super::*;

fn rect(dom: &TuiDom, id: &str) -> (i32, i32, u16, u16) {
    let r = dom.node(by_id(dom, id)).layout_rect().expect("laid out");
    (r.x, r.y, r.width, r.height)
}

/// §3.1: size containment — `contain: size` (and `strict`) size the box
/// as if empty; `inline-size` the inline axis only (Containment 3).
#[test]
fn size_containment() {
    let mut dom = doc(r#"<div id="c">hello world</div>"#);
    styled(&mut dom, "#c { contain: size; float: left }", 30, 5);
    assert_eq!((rect(&dom, "c").2, rect(&dom, "c").3), (0, 0));
    let mut dom = doc(r#"<div id="c">hello world</div>"#);
    styled(
        &mut dom,
        "#c { contain: strict; contain-intrinsic-size: 4 2 }",
        30,
        5,
    );
    assert_eq!(rect(&dom, "c").3, 2);
    let mut dom = doc(r#"<div id="c">hello world</div>"#);
    styled(
        &mut dom,
        "#c { contain: inline-size layout; float: left }",
        30,
        5,
    );
    assert_eq!(rect(&dom, "c").2, 0);
}

/// §3.2 layout containment: an independent formatting context — floats
/// inside count for its height, margins do not collapse through it — and
/// the containing block of its absolutely and fixed positioned
/// descendants.
#[test]
fn layout_containment() {
    let markup = r#"<div id="c"><div id="f">f</div></div><p id="after">a</p>"#;
    let mut dom = doc(markup);
    styled(
        &mut dom,
        "#c { contain: layout } #f { float: left; height: 3 } p { margin: 0 }",
        30,
        8,
    );
    assert_eq!(rect(&dom, "c").3, 3, "the float is contained");
    let markup = r#"<p>x</p><div id="c"><span id="a">a</span><span id="fx">f</span></div>"#;
    let mut dom = doc(markup);
    styled(
        &mut dom,
        "p { margin: 0 } #c { contain: layout; margin-left: 5; height: 3 }
         #a { position: absolute; top: 1; left: 1 }
         #fx { position: fixed; top: 0; left: 0 }",
        30,
        8,
    );
    assert_eq!((rect(&dom, "a").0, rect(&dom, "a").1), (6, 2));
    assert_eq!(
        (rect(&dom, "fx").0, rect(&dom, "fx").1),
        (5, 1),
        "fixed too"
    );
    let markup = r#"<div id="c"><p id="in">x</p></div>"#;
    let mut dom = doc(markup);
    styled(
        &mut dom,
        "#c { contain: layout } #in { margin: 2 0 }",
        30,
        8,
    );
    assert_eq!(rect(&dom, "c").1, 0, "the child's margin stays inside");
    assert_eq!(rect(&dom, "in").1, 2);
}

/// §3.4 paint containment: the content is clipped to the padding box,
/// and the box contains positioned descendants.
#[test]
fn paint_containment_clips() {
    let mut dom = doc(r#"<div id="c"><div id="wide">0123456789</div></div>"#);
    let buf = paint(
        &mut dom,
        "#c { contain: paint; width: 4; height: 1 } #wide { width: 10 }",
        20,
        3,
    );
    let row: String = (0..12)
        .map(|x| buf.cell(x, 0).unwrap().symbol().to_string())
        .collect();
    assert_eq!(row.trim_end(), "0123");
}

/// Layout and paint containment make a stacking context (§3.2, §3.4): a
/// negative `z-index` child paints above the container's background
/// (inside a `<body>`, so `#c`'s background is its own, not the canvas).
#[test]
fn containment_makes_a_stacking_context() {
    let css = |contain: &str| {
        format!(
            "#c {{ {contain}; background: blue; height: 1 }}
             #k {{ position: relative; z-index: -1; background: red; width: 1 }}"
        )
    };
    let markup = r#"<body><div id="c"><div id="k"> </div></div></body>"#;
    let mut dom = doc(markup);
    let buf = paint(&mut dom, &css("contain: paint"), 10, 2);
    assert_eq!(buf.cell(0, 0).unwrap().bg, RED);
    let mut dom = doc(markup);
    let buf = paint(&mut dom, &css("contain: none"), 10, 2);
    assert_eq!(
        buf.cell(0, 0).unwrap().bg,
        BLUE,
        "no context: the child is beneath"
    );
}

/// §3.3 style containment: an increment inside of a counter from outside
/// creates a new counter there, and none of it reaches outside.
#[test]
fn style_containment_scopes_counters() {
    let markup = r#"<p></p><div id="c"><p></p><p></p></div><p id="out"></p>"#;
    let css = |contain: &str| {
        format!(
            "p {{ margin: 0; counter-increment: n }} p::before {{ content: counter(n) }} #c {{ contain: {contain} }}"
        )
    };
    let mut dom = doc(markup);
    let buf = paint(&mut dom, &css("style"), 10, 4);
    let col = |buf: &Buffer| {
        (0..4)
            .map(|y| buf.cell(0, y).unwrap().symbol().to_string())
            .collect::<Vec<_>>()
    };
    assert_eq!(col(&buf), ["1", "1", "2", "2"]);
    let mut dom = doc(markup);
    let buf = paint(&mut dom, &css("none"), 10, 4);
    assert_eq!(col(&buf), ["1", "2", "3", "4"]);
}

/// CSS Will Change 1 §3: a property whose non-initial value makes a
/// stacking context makes one when named, and one that makes a containing
/// block for positioned boxes makes one.
#[test]
fn will_change_makes_stacking_contexts_and_containing_blocks() {
    let markup = r#"<body><div id="c"><div id="k"> </div></div></body>"#;
    let mut dom = doc(markup);
    let buf = paint(
        &mut dom,
        "#c { will-change: opacity; background: blue; height: 1 }
         #k { position: relative; z-index: -1; background: red; width: 1 }",
        10,
        2,
    );
    assert_eq!(buf.cell(0, 0).unwrap().bg, RED);
    let mut dom = doc(r#"<p>x</p><div id="c"><span id="a">a</span></div>"#);
    styled(
        &mut dom,
        "p { margin: 0 } #c { will-change: transform; margin-left: 4 } #a { position: absolute; top: 0; left: 1 }",
        30,
        5,
    );
    assert_eq!((rect(&dom, "a").0, rect(&dom, "a").1), (5, 1));
    let mut dom = doc(r#"<p>x</p><div id="c"><span id="a">a</span></div>"#);
    styled(
        &mut dom,
        "p { margin: 0 } #c { will-change: scroll-position, color; margin-left: 4 } #a { position: absolute; top: 0; left: 1 }",
        30,
        5,
    );
    assert_eq!(
        (rect(&dom, "a").0, rect(&dom, "a").1),
        (1, 0),
        "inert: the viewport"
    );
}
