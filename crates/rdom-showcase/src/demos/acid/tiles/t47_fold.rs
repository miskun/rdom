//! Tile 47 — content just above the fold (CSS Containment 2 §4.4,
//! CSS Values 4 §6.1.2; ACID-INTERACTIVE I19).
//!
//! A paragraph `40vw` wide, so a narrower terminal wraps it into more
//! lines, and under it, on the page's last row, a `content-visibility:
//! auto` block whose `contentvisibilityautostatechange` listener logs
//! `t` / `f` (`skipped`) on row 0.

use rdom_tui::{ListenerOptions, NodeId, TuiDom};

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "47",
    title: "Above the fold",
    class: "acid-t47",
    page: 6,
    x: 1,
    y: 18,
    w: 58,
    h: 32,
    markup: r#"
<div class="lg">log:</div><div class="sp"></div><p class="tx">ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab ab</p><div class="cv">cv</div>
"#,
    css: r#"
.acid-t47 .sp { height: 27; }
.acid-t47 .tx { width: 40vw; margin: 0; }
.acid-t47 .cv { content-visibility: auto; contain-intrinsic-size: auto 1; }
"#,
    late_css: "",
    setup: Some(listen),
    script: None,
};

/// The `content-visibility: auto` block's state log.
fn listen(dom: &mut TuiDom, tile: NodeId) {
    let find = |dom: &TuiDom, sel: &str| {
        dom.query_selector_in(tile, sel)
            .expect("a valid selector")
            .expect("the tile holds it")
    };
    let (cv, lg) = (find(dom, ".cv"), find(dom, ".lg"));
    dom.add_event_listener(
        cv,
        "contentvisibilityautostatechange",
        ListenerOptions::default(),
        move |ctx| {
            let skipped = ctx
                .event
                .detail
                .as_content_visibility_auto_state_change()
                .expect("a content-visibility event");
            let text = ctx.dom.text_content(lg) + if skipped { "t" } else { "f" };
            ctx.dom.set_text_content(lg, &text).unwrap();
        },
    )
    .unwrap();
}
