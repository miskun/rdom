//! C11-MODAL-POPOVER — the `popover` attribute through the UA sheet and
//! `:popover-open` (HTML §6.12, its rendering rules; Selectors 4).

use rdom_tui::runtime::builtins::popover;
use rdom_tui::{NodeId, TuiDom, TuiNodeExt};

use super::{RED, app, app_fg};
use crate::css_phase5::rows;

fn node(dom: &mut TuiDom, parent: NodeId, tag: &str, attrs: &[(&str, &str)], text: &str) -> NodeId {
    let e = dom.create_element(tag);
    for (k, v) in attrs {
        dom.set_attribute(e, k, v).unwrap();
    }
    if !text.is_empty() {
        let t = dom.create_text_node(text);
        dom.append_child(e, t).unwrap();
    }
    dom.append_child(parent, e).unwrap();
    e
}

/// HTML's rendering rules for popovers: `[popover]` is `display: none`
/// until its popover shows; a showing popover is fixed, inset 0 with
/// auto margins and a fit-content size — centred in the viewport — with
/// a solid border. `:popover-open` matches it while it shows.
#[test]
fn a_popover_is_hidden_until_shown_then_centred() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    node(&mut dom, root, "p", &[], "page");
    let p = node(&mut dom, root, "div", &[("popover", "")], "hi");
    let mut app = app(dom, ":popover-open { color: red }");
    assert_eq!(
        app.dom().node(p).computed().unwrap().display,
        rdom_tui::layout::Display::None
    );
    popover::show_popover(app.dom_mut(), p).unwrap();
    app.advance(0).unwrap();
    assert_eq!(app_fg(&app, p), RED);
    let r = app.dom().node(p).layout_rect().unwrap();
    assert_eq!(
        (r.x, r.y, r.width, r.height),
        (8, 3, 4, 3),
        "a bordered `hi`, centred in 20 × 10"
    );
    use rdom_tui::PaintExt;
    let area = rdom_tui::render::Rect::new(0, 0, 20, 10);
    let mut buf = rdom_tui::render::Buffer::empty(area);
    app.dom().paint_dom(&mut buf, area);
    assert_eq!(rows(&buf, 20, 10)[4], "        │hi│        ");
    popover::hide_popover(app.dom_mut(), p).unwrap();
    app.advance(0).unwrap();
    assert_eq!(
        app.dom().node(p).computed().unwrap().display,
        rdom_tui::layout::Display::None
    );
    assert_ne!(app_fg(&app, p), RED);
}
