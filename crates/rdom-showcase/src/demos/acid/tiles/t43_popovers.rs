//! Tile 43 — popover light dismiss and close requests (HTML §6.12
//! `popover`, §4.11.4 `dialog`; ACID-INTERACTIVE I15).
//!
//! A `popovertarget` button for an auto popover holding an `[autofocus]`
//! field and a button for a nested auto popover; a button for a manual
//! popover; a line of plain text to click outside on; two closed modal
//! dialogs a script opens one above the other. The popovers sit at fixed
//! cells of the page (`inset: auto` and their own `top` / `left`).

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "43",
    title: "Popover light dismiss",
    class: "acid-t43",
    page: 12,
    x: 1,
    y: 1,
    w: 58,
    h: 10,
    markup: r#"
<button popovertarget="po43" class="ob">open</button> <button popovertarget="pm43" class="mb">man</button>
<div popover id="po43" class="po"><input autofocus class="af"><button popovertarget="pn43" class="nb">n</button></div>
<div popover id="pn43" class="pn">inner</div>
<div popover="manual" id="pm43" class="pm">manual</div>
<div class="out">outside</div>
<dialog class="d1">one</dialog><dialog class="d2">two</dialog>
"#,
    css: r#"
.acid-t43 [popover] { inset: auto; margin: 0; padding: 0; }
.acid-t43 .po { top: 3; left: 1; }
.acid-t43 .po:popover-open { display: flex; gap: 1; }
.acid-t43 .pn { top: 3; left: 16; }
.acid-t43 .pm { top: 7; left: 1; }
.acid-t43 .af { width: 3; color: rgb(255, 255, 255); }
.acid-t43 .out { position: absolute; top: 9; left: 0; }
"#,
    late_css: "",
    setup: None,
    script: None,
};
