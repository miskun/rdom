//! `container-type` and size containment (CSS Conditional 5 §6.1, CSS
//! Containment 2 §3.1, CSS Containment 3 inline-size containment): a
//! `size` / `inline-size` query container's intrinsic size on those axes
//! is computed as if it had no content — `contain-intrinsic-size` (CSS
//! Sizing 4 §6.1) where given — so its layout never depends on what it
//! holds.

use super::*;

/// `id`'s laid-out border box `(x, y, width, height)`.
fn rect(dom: &TuiDom, id: &str) -> (i32, i32, u16, u16) {
    let r = dom.node(by_id(dom, id)).layout_rect().expect("laid out");
    (r.x, r.y, r.width, r.height)
}

const TEXT: &str = r#"<div id="c">hello world</div><p id="after">x</p>"#;

/// CSS Containment 3: inline-size containment — a shrink-to-fit box (a
/// float) sizes its width as if empty; `contain-intrinsic-width` gives
/// the size.
#[test]
fn inline_size_containment_ignores_the_content_width() {
    let mut dom = doc(TEXT);
    styled(
        &mut dom,
        "#c { float: left; container-type: inline-size }",
        30,
        5,
    );
    assert_eq!(rect(&dom, "c").2, 0);
    let mut dom = doc(TEXT);
    styled(
        &mut dom,
        "#c { float: left; container-type: inline-size; contain-intrinsic-width: 6; padding: 0 1 }",
        30,
        5,
    );
    assert_eq!(rect(&dom, "c").2, 8, "6 cells of content, 2 of padding");
}

/// Inline-size containment leaves the block axis alone: a block container
/// fills its containing block and grows with its (wrapped) content.
#[test]
fn inline_size_containment_keeps_the_block_size() {
    let mut dom = doc(TEXT);
    styled(
        &mut dom,
        "#c { container-type: inline-size; width: 5 }",
        30,
        5,
    );
    assert_eq!(rect(&dom, "c"), (0, 0, 5, 2), "two lines of text");
    assert_eq!(rect(&dom, "after").1, 2);
}

/// CSS Containment 2 §3.1: size containment on both axes — an `auto`
/// height is as if empty (the content still lays out, overflowing), and
/// `contain-intrinsic-height` gives it a size; a declared height stands.
#[test]
fn size_containment_ignores_the_content_height() {
    let mut dom = doc(TEXT);
    styled(&mut dom, "#c { container-type: size }", 30, 5);
    assert_eq!(rect(&dom, "c").3, 0);
    assert_eq!(
        rect(&dom, "after").1,
        0,
        "the next block starts where it ends"
    );
    let mut dom = doc(TEXT);
    styled(
        &mut dom,
        "#c { container-type: size; contain-intrinsic-size: 4 3 }",
        30,
        5,
    );
    assert_eq!(rect(&dom, "c").3, 3);
    let mut dom = doc(TEXT);
    styled(&mut dom, "#c { container-type: size; height: 2 }", 30, 5);
    assert_eq!(rect(&dom, "c").3, 2);
}

/// An inline block and a flex item size from their content's
/// contribution; under size containment that is nothing.
#[test]
fn contained_atoms_and_items_size_as_if_empty() {
    let mut dom = doc(r#"<span id="c">hello</span>"#);
    styled(
        &mut dom,
        "#c { display: inline-block; container-type: size; border: solid }",
        30,
        5,
    );
    assert_eq!(
        (rect(&dom, "c").2, rect(&dom, "c").3),
        (2, 2),
        "border only"
    );
    let mut dom = doc(r#"<div id="row"><div id="c">hello</div></div>"#);
    styled(
        &mut dom,
        "#row { display: flex } #c { flex: none; container-type: inline-size }",
        30,
        5,
    );
    assert_eq!(rect(&dom, "c").2, 0);
}

/// A `normal` (or `scroll-state`) container applies no containment.
#[test]
fn a_normal_container_applies_no_containment() {
    let mut dom = doc(TEXT);
    styled(
        &mut dom,
        "#c { float: left; container: card / scroll-state }",
        30,
        5,
    );
    assert_eq!(rect(&dom, "c").2, 11);
}
