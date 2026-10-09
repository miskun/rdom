//! Tile 36 — form state under input (HTML §4.10.5, §4.10.21, §4.16.3;
//! Selectors 4 §14; ACID-INTERACTIVE I4, I5).
//!
//! Row 0: a `required` empty field in a form, a sibling whose `::after`
//! reads the field's `:valid` / `:invalid`, and a label coloured by the
//! form's own `:valid` / `:invalid`. Row 1: a checkbox and a radio group
//! `g` inside a form, and a radio of the same name outside it — another
//! group, its form owner being none — both radios checked, `:checked`
//! green.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "36",
    title: "Form state",
    class: "acid-t36",
    page: 10,
    x: 81,
    y: 1,
    w: 38,
    h: 2,
    markup: r#"
<form class="f1"><input class="rq" required><span class="st"></span><span class="lab">form</span></form>
<div class="tg"><form class="f2"><input type="checkbox" class="cb"><input type="radio" name="g" class="r1" checked><input type="radio" name="g" class="r2"></form><input type="radio" name="g" class="r3" checked></div>
"#,
    css: r#"
.acid-t36 .f1, .acid-t36 .tg { display: flex; gap: 1; }
.acid-t36 .tg { gap: 0; }
.acid-t36 .rq { width: 3; color: rgb(255, 255, 255); }
.acid-t36 .rq:invalid + .st::after { content: "bad"; color: rgb(255, 0, 0); }
.acid-t36 .rq:valid + .st::after { content: "ok"; color: rgb(0, 160, 0); }
.acid-t36 .f1:invalid .lab { color: rgb(255, 0, 0); }
.acid-t36 .f1:valid .lab { color: rgb(0, 160, 0); }
.acid-t36 .tg :checked { color: rgb(0, 160, 0); }
"#,
    late_css: "",
    setup: None,
    script: None,
};
