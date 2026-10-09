//! C14-ASPECT-BLOCK — `aspect-ratio` on a block-level box in block flow
//! (CSS Sizing 4 §5.1): with a definite height, the automatic width is
//! the transferred size (not the stretch fit); with an automatic height,
//! the height is transferred from the width — floored, unless the box is
//! a scroll container, at its content (the automatic minimum size).

use super::*;

fn rect(dom: &TuiDom, id: &str) -> (u16, u16) {
    let r = dom.node(by_id(dom, id)).layout_rect().expect("laid out");
    (r.width, r.height)
}

/// §5.1: a definite block size transfers through the ratio to the
/// automatic inline size — Chrome gives `height: 9; aspect-ratio: 16/9`
/// a width of 16.
#[test]
fn a_definite_height_gives_the_width() {
    let mut dom = doc(r#"<div id="a"></div>"#);
    styled(&mut dom, "#a { height: 9; aspect-ratio: 16/9 }", 40, 20);
    assert_eq!(rect(&dom, "a"), (16, 9));
    // The ratio sizes the content box under `content-box`: padding adds.
    let mut dom = doc(r#"<div id="a"></div>"#);
    styled(
        &mut dom,
        "#a { height: 4; padding: 0 1; aspect-ratio: 2 }",
        40,
        20,
    );
    assert_eq!(rect(&dom, "a"), (10, 4));
    // A percentage height of a definite containing block is definite.
    let mut dom = doc(r#"<div id="a"></div>"#);
    styled(&mut dom, "#a { height: 50%; aspect-ratio: 2 }", 40, 10);
    assert_eq!(rect(&dom, "a"), (10, 5));
    // `auto` margins center the transferred width.
    let mut dom = doc(r#"<div id="a"></div>"#);
    styled(
        &mut dom,
        "#a { height: 2; aspect-ratio: 5; margin: 0 auto }",
        20,
        5,
    );
    let r = dom.node(by_id(&dom, "a")).layout_rect().unwrap();
    assert_eq!((r.x, r.width), (5, 10));
}

/// §5.1: an automatic height transfers from the width; the automatic
/// minimum height is the content's, except in a scroll container.
#[test]
fn an_automatic_height_follows_the_width() {
    let mut dom = doc(r#"<div id="a"></div>"#);
    styled(&mut dom, "#a { aspect-ratio: 2 }", 40, 30);
    assert_eq!(rect(&dom, "a"), (40, 20));
    let markup = r#"<div id="a">one two three</div>"#;
    let mut dom = doc(markup);
    styled(&mut dom, "#a { width: 5; aspect-ratio: 5 }", 40, 10);
    assert_eq!(
        rect(&dom, "a"),
        (5, 3),
        "three lines of content beat the ratio's 1"
    );
    let mut dom = doc(markup);
    styled(
        &mut dom,
        "#a { width: 5; aspect-ratio: 5; overflow: auto }",
        40,
        10,
    );
    assert_eq!(rect(&dom, "a").1, 1, "a scroll container keeps the ratio");
}
