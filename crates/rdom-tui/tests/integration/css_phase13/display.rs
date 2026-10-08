//! The table `display` values in the cascade (C13-TFC; CSS Display 3
//! §2.4, §2.7; CSS 2.1 §9.7).

use rdom_tui::{CascadeExt, Display, Flow, TablePart, TuiNodeExt};

use super::{by_id, doc};

fn computed(dom: &rdom_tui::TuiDom, id: &str) -> (Display, Flow) {
    let c = dom.node(by_id(dom, id)).computed().cloned().unwrap();
    (c.display, c.flow)
}

/// CSS Display 3 §2.7: a flex or grid item is blockified — a
/// layout-internal box becomes `block`, `inline-table` becomes `table`.
#[test]
fn flex_and_grid_items_are_blockified() {
    let mut dom = doc(
        r#"<div class="f"><div id="a" class="cell"></div><div id="b" class="it"></div></div>
<div class="g"><div id="c" class="row"></div></div>"#,
    );
    let sheet = rdom_css::from_css_strict(
        ".f { display: flex } .g { display: grid } .cell { display: table-cell }
         .it { display: inline-table } .row { display: table-row }",
    )
    .unwrap();
    dom.cascade(&sheet);
    assert_eq!(computed(&dom, "a"), (Display::Block, Flow::Block));
    assert_eq!(computed(&dom, "b"), (Display::Block, Flow::Table));
    assert_eq!(computed(&dom, "c"), (Display::Block, Flow::Block));
}

/// CSS 2.1 §9.7: a float or an absolutely positioned box is no part of a
/// table — its table `display` computes to `block`.
#[test]
fn floats_and_absolute_boxes_are_blockified() {
    let mut dom = doc(r#"<div><div id="a"></div><div id="b"></div><div id="c"></div></div>"#);
    let sheet = rdom_css::from_css_strict(
        "#a { display: table-cell; float: left } #b { display: table-row; position: absolute }
         #c { display: table-caption }",
    )
    .unwrap();
    dom.cascade(&sheet);
    assert_eq!(computed(&dom, "a"), (Display::Block, Flow::Block));
    assert_eq!(computed(&dom, "b"), (Display::Block, Flow::Block));
    assert_eq!(
        computed(&dom, "c"),
        (Display::TablePart(TablePart::Caption), Flow::FlowRoot)
    );
}
