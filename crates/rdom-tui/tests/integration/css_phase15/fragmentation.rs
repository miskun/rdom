//! C15G-FRAGMENT-GAPS — CSS Fragmentation 3 in a multi-column container:
//! class C break points (§4.1) and floats at a break (§3.1, CSS 2.1 §9.5).

use super::*;

const PAGE: &str = "body { margin: 0 } p { margin: 0 } ";

/// §4.1: "Class C: between the content edge of a block container box and
/// the outer edges of its child content … if there is a (non-zero) gap
/// between them." Ten lines in a 14-row box leave a gap of four rows below
/// the last; a break there lets the first 12-row column take all ten lines
/// (the last class A break is after the ninth).
#[test]
fn a_class_c_break_falls_between_the_last_child_and_the_content_edge() {
    let paras: String = (1..=10)
        .map(|k| format!(r#"<p id="p{k}">l{k}</p>"#))
        .collect();
    let mut dom = doc(&format!(
        r#"<body><div id="m"><div id="d">{paras}</div></div></body>"#
    ));
    styled(
        &mut dom,
        &format!(
            "{PAGE} #m {{ column-count: 2; column-gap: 1; width: 11; height: 12; column-fill: auto }}
             #d {{ height: 14 }}"
        ),
        11,
        12,
    );
    assert_eq!(
        rect(&dom, "p10"),
        (0, 9, 5, 1),
        "the tenth line in the first column"
    );
}

/// CSS 2.1 §9.5 with Fragmentation 3 §4.1: rdom keeps a float whole
/// (monolithic), so a break never falls inside its rows while a break
/// before it fits — the float moves to the next column with the content
/// after the break, rather than hanging below its column.
#[test]
fn a_float_at_a_break_moves_to_the_next_column() {
    let mut dom = doc(
        r#"<body><div id="m"><p>a</p><p>b</p><div id="f">F</div><p>c</p><p>d</p><p>e</p></div></body>"#,
    );
    styled(
        &mut dom,
        &format!(
            "{PAGE} #m {{ column-count: 2; column-gap: 1; width: 11; height: 4; column-fill: auto }}
             #f {{ float: left; width: 2; height: 3 }}"
        ),
        11,
        4,
    );
    assert_eq!(
        rect(&dom, "f"),
        (6, 0, 2, 3),
        "at the top of the second column"
    );
}
