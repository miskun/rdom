//! Tile 35 — focus by keyboard and by pointer (Selectors 4 §13.2, HTML
//! §6.6, CSS UI 4 §5; ACID-INTERACTIVE I3).
//!
//! A button whose author `:focus-visible { outline: auto }` rule rings it
//! when the keyboard focused it, beside an empty text field, whose focus
//! is always evident; a second row's button that, in its `click`
//! listener, shows a `hidden` panel and focuses the panel's field —
//! focused at once though it was `display: none` until that listener ran
//! (C12-FOCUS-FLUSH) — and writes whether it got the focus.

use rdom_tui::{ListenerOptions, NodeId, TuiAccessorsMut, TuiDom};

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "35",
    title: "Focus: Tab vs click",
    class: "acid-t35",
    page: 10,
    x: 41,
    y: 1,
    w: 38,
    h: 4,
    markup: r#"
<div class="fr"><button class="fb">go</button><input class="tf"></div>
<div class="sw"><button class="show">show</button><div class="pn" hidden><input class="pi"></div><span class="fl"></span></div>
"#,
    css: r#"
.acid-t35 .fr { display: flex; gap: 2; padding: 1; }
.acid-t35 .fb:focus-visible { outline: auto; }
.acid-t35 .tf, .acid-t35 .pi { width: 3; color: rgb(255, 255, 0); }
.acid-t35 .sw { display: flex; gap: 1; }
"#,
    late_css: "",
    setup: Some(listen),
    script: None,
};

/// The second button's listener: unhide the panel, focus its field and
/// log whether the field holds the focus right after `focus()`.
fn listen(dom: &mut TuiDom, tile: NodeId) {
    let find = |dom: &TuiDom, sel: &str| {
        dom.query_selector_in(tile, sel)
            .expect("a valid selector")
            .expect("the tile holds it")
    };
    let (show, pn, pi, fl) = (
        find(dom, ".show"),
        find(dom, ".pn"),
        find(dom, ".pi"),
        find(dom, ".fl"),
    );
    dom.add_event_listener(show, "click", ListenerOptions::default(), move |ctx| {
        ctx.dom.remove_attribute(pn, "hidden").unwrap();
        ctx.dom.node_mut(pi).focus();
        let got = ctx.dom.focused() == Some(pi);
        ctx.dom
            .set_text_content(fl, if got { "ok" } else { "no" })
            .unwrap();
    })
    .unwrap();
}
