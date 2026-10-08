//! C11-ATTR-FLAGS — Selectors 4 §6.3 attribute selector case flags,
//! through a stylesheet.

use rdom_tui::{Color, TuiDom, TuiNodeExt};

use super::{UNSTYLED, cascade, el, fg};

/// Selectors 4 §6.3: `i` folds ASCII case where the attribute is
/// case-sensitive, `s` tells `a` from `A` on `type`, which HTML §4.16.2
/// otherwise compares ASCII case-insensitively.
#[test]
fn a_sheets_case_flags_decide_the_value_comparison() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let a = el(&mut dom, root, "ol", "");
    dom.set_attribute(a, "type", "a").unwrap();
    dom.set_attribute(a, "data-k", "Ab").unwrap();
    let upper = el(&mut dom, root, "ol", "");
    dom.set_attribute(upper, "type", "A").unwrap();
    cascade(
        &mut dom,
        "ol[type=a s] { color: red } [data-k=ab i] { background-color: blue }",
    );
    assert_eq!(fg(&dom, a), Color::Rgb(255, 0, 0));
    assert_eq!(fg(&dom, upper), UNSTYLED, "`s`: `A` is not `a`");
    let bg = |id| dom.node(id).computed().expect("cascaded").bg;
    assert_eq!(bg(a), Color::Rgb(0, 0, 255));
    assert_eq!(bg(upper), Color::TRANSPARENT);
}
