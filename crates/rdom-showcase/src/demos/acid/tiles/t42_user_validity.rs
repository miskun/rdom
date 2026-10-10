//! Tile 42 — user validity (HTML §4.16.3 `:user-valid` /
//! `:user-invalid`, §4.10.21, §4.10.22; ACID-INTERACTIVE I14).
//!
//! A form with a `pattern` field, a `required` checkbox, a `required`
//! field showing a placeholder, a submit and a reset button, a label the
//! form's `:has(:user-invalid)` colours, and a log the pattern field's
//! `change` (`c`) and `blur` (`b`) listeners write. The `:user-*` states
//! colour the pattern field's text, the checkbox and the placeholder —
//! not a background, which a focused field's UA tint would hide.

use rdom_tui::{ListenerOptions, NodeId, TuiDom};

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "42",
    title: "User validity",
    class: "acid-t42",
    page: 11,
    x: 41,
    y: 1,
    w: 48,
    h: 1,
    markup: r#"
<form class="uv"><input class="pt" pattern="[0-9]+"><input type="checkbox" class="cb" required><input class="rq" required placeholder="rq"><button class="sb">Go</button><button type="reset" class="rs">R</button><span class="fl">form</span><span class="lg">log:</span></form>
"#,
    css: r#"
.acid-t42 .uv { display: flex; gap: 1; }
.acid-t42 .pt, .acid-t42 .rq { width: 3; }
.acid-t42 .pt { color: rgb(255, 255, 255); }
.acid-t42 .pt:user-invalid, .acid-t42 .cb:user-invalid { color: rgb(255, 0, 0); }
.acid-t42 .pt:user-valid, .acid-t42 .cb:user-valid { color: rgb(0, 160, 0); }
.acid-t42 .rq::placeholder { color: rgb(128, 128, 128); }
.acid-t42 .rq:user-invalid::placeholder { color: rgb(255, 0, 0); }
.acid-t42 .uv:has(:user-invalid) .fl { color: rgb(255, 0, 0); }
"#,
    late_css: "",
    setup: Some(listen),
    script: None,
};

/// The pattern field's `change` / `blur` log.
fn listen(dom: &mut TuiDom, tile: NodeId) {
    let find = |dom: &TuiDom, sel: &str| {
        dom.query_selector_in(tile, sel)
            .expect("a valid selector")
            .expect("the tile holds it")
    };
    let (pt, lg) = (find(dom, ".pt"), find(dom, ".lg"));
    for (ty, letter) in [("change", "c"), ("blur", "b")] {
        dom.add_event_listener(pt, ty, ListenerOptions::default(), move |ctx| {
            let text = ctx.dom.text_content(lg) + letter;
            ctx.dom.set_text_content(lg, &text).unwrap();
        })
        .unwrap();
    }
}
