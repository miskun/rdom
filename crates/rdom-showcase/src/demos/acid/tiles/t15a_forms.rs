//! Tile 15a — form controls (HTML §4.10, §15.5; CSS UI 4 §5–§7;
//! Selectors 4 §14).
//!
//! The UA chrome of a text field, a styled `::placeholder`, a
//! `field-sizing: content` field, a textarea with its resize grip, a
//! closed drop-down `<select>`; checkboxes and radios (checked,
//! unchecked, indeterminate) and a progress bar under a form's
//! `accent-color`, beside a meter it does not tint; buttons, a value-less
//! submit, and `appearance: none` on a button and a progress bar;
//! outlines (`solid`, `double` with an offset, a negative offset over the
//! box's content, `auto`, one clipped by an `overflow: hidden` parent and
//! one around an inline span over the text beside it); a `<fieldset
//! disabled>`'s control; the form states — `:invalid`, `:read-only`,
//! `:read-write`, `:out-of-range` / `:in-range` on `number`, `date` and a
//! reversed `time` range, `:default`, `:indeterminate` on an unchecked
//! radio group, and `:user-valid` / `:user-invalid` rules that do not
//! match before any interaction. The top layer is tile 15b's.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "15a",
    title: "Form controls",
    class: "acid-t15a",
    page: 3,
    x: 61,
    y: 1,
    w: 58,
    h: 22,
    markup: r#"
<div class="band"><input class="w8" value="abc"><input class="w8 ph" placeholder="name"><input class="fs" value="ab"><textarea class="ta">hi</textarea><select class="s"><option>One</option><option selected>Two</option></select></div>
<form class="band ac"><div><input type="checkbox" checked> <input type="checkbox"> <input type="checkbox" indeterminate> <input type="radio" name="r" checked> <input type="radio" name="r"></div><progress value="1" max="4"></progress><meter value="0.5"></meter></form>
<div class="band"><button>OK</button><input type="submit"><button class="na">OK</button><progress class="na" value="1"></progress></div>
<div class="band ol"><b class="o1">ab</b><b class="o2">cd</b><div class="o3">1234 5678 90ab</div><b class="o4">gh</b><div class="clip"><b class="o5">xy</b></div><div class="tx">aa <span class="o6">bb</span> cc</div></div>
<div class="band"><fieldset disabled><legend>L</legend><input class="w4 di" value="d"></fieldset><input class="w4 rq" required><input class="w4 ro" readonly value="r"><div class="ce" contenteditable>e</div><input class="w4 nr" type="number" min="1" max="5" value="9"><input class="w4 ni" type="number" min="1" max="5" value="3"></div>
<div class="band"><form class="df"><button>A</button> <button>B</button></form><span><input type="radio" name="q" class="iq"><input type="radio" name="q" class="iq"></span><input class="w12 dr" type="date" min="2026-01-01" max="2026-12-31" value="2025-06-01"><input class="w7 tr" type="time" min="22:00" max="02:00" value="23:00"></div>
"#,
    css: r#"
.acid-t15a .band { display: flex; gap: 1; align-items: flex-start; margin-bottom: 1; }
.acid-t15a .w4 { width: 4; }
.acid-t15a .w7 { width: 7; }
.acid-t15a .w8 { width: 8; }
.acid-t15a .w12 { width: 12; }
.acid-t15a .ph::placeholder { color: rgb(0, 160, 0); }
.acid-t15a .fs { field-sizing: content; }
.acid-t15a .ta { width: 8; height: 3; }
.acid-t15a .s { width: 10; }
.acid-t15a .ac { accent-color: rgb(255, 0, 255); }
.acid-t15a progress, .acid-t15a meter { width: 8; }
.acid-t15a .na { appearance: none; }
.acid-t15a progress.na { width: 6; }
.acid-t15a .ol { padding: 2 2; gap: 4; }
.acid-t15a .o1 { outline: solid; }
.acid-t15a .o2 { outline: double; outline-offset: 1; }
.acid-t15a .o3 { width: 4; outline: solid; outline-offset: -1; }
.acid-t15a .o4 { outline: auto; }
.acid-t15a .clip { width: 3; height: 3; overflow: hidden; }
.acid-t15a .o5 { display: block; width: 2; margin: 1 0 0 1; outline: solid; }
.acid-t15a .o6 { outline: solid; }
.acid-t15a b { font-weight: normal; }
.acid-t15a .rq:invalid { background-color: rgb(128, 0, 0); }
.acid-t15a .rq:user-invalid { background-color: rgb(255, 0, 255); }
.acid-t15a .ro:read-only { background-color: rgb(0, 0, 128); }
.acid-t15a .di:disabled { background-color: rgb(96, 96, 96); }
.acid-t15a .ce:read-write { color: rgb(0, 160, 0); }
.acid-t15a .nr:out-of-range, .acid-t15a .dr:out-of-range { background-color: rgb(128, 128, 0); }
.acid-t15a .ni:in-range, .acid-t15a .tr:in-range { background-color: rgb(0, 128, 128); }
.acid-t15a .ni:user-valid { background-color: rgb(255, 0, 255); }
.acid-t15a .df :default { color: rgb(255, 255, 0); }
.acid-t15a .iq:indeterminate { color: rgb(255, 165, 0); }
"#,
    late_css: "",
    setup: None,
    script: None,
};
