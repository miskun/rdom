//! Tile 4 — selectors (Selectors 4 §5–§16, HTML §4.16).
//!
//! One probe per item: a rule that should match paints its probe green, a
//! rule that should not paints it red, so every word is green or the
//! tile's default colour. Rows: combinators (with a backtracking `>`),
//! attribute operators and case flags, structural pseudo-classes with
//! `:root` / `:scope`, the `:nth-*` family, logical combinations and link
//! states, language and directionality, `:has()`; a row for step I13's
//! `:has(:hover)` and `:has(:checked)`.

use super::super::Tile;

pub const TILE: Tile = Tile {
    id: "4",
    title: "Selectors",
    class: "acid-t4",
    page: 1,
    x: 1,
    y: 8,
    w: 58,
    h: 8,
    markup: r##"
<div><span class="d1"><a><time>desc</time></a></span> <span class="d2"><time>child</time></span> <span class="d3"><a><time>!child</time></a></span> <span class="d8"><data><time><data><a>deep</a></data></time></data></span> <a class="d4a">.</a><a class="d4b">adj</a> <a class="d5a">.</a>t<a class="d5b">adj-text</a> <a class="d6a">.</a><data>.</data><a class="d6b">sib</a> <a class="d7a">.</a><data>.</data><a class="d7b">!adj</a></div>
<div><a class="a1" data-k="ab">eq</a> <a class="a2" data-k="x ab y">word</a> <a class="a3" data-k="ab-cd">dash</a> <a class="a4" data-k="abc">pre</a> <a class="a5" data-k="cab">suf</a> <a class="a6" data-k="xaby">sub</a> <a class="a7" data-k="AB">i-flag</a> <a class="a8" type="A">type-ci</a> <a class="a9" type="A">!s-flag</a> <a class="a10" data-k="AB">!case</a></div>
<div><span class="s1"><a>f</a><a>m</a><a>l</a></span> <span class="s2"><a>only</a></span> <span class="s3"><a>x</a><a>!only</a></span> <a class="e1"></a> <a class="e2"><!-- c --></a> <a class="e3">x</a> <a class="rt">root</a> <a class="sc">scope</a></div>
<div><span class="n1"><a>1</a><a>2</a><a>3</a><a>4</a><a>5</a></span> <span class="n2"><a class="x">a</a><a>b</a><a class="x">c</a><a class="x">d</a><a>e</a><a class="x">f</a></span> <span class="n3"><a>1</a><a>2</a><a>3</a><a>4</a></span> <span class="n4"><a>a</a><data>b</data><a>c</a><data>d</data><a>e</a></span> <span class="n5"><a>a</a><data>b</data><a>c</a><time>d</time></span></div>
<div><a class="l1">is</a> <a class="l2">where</a> <a class="l3">not</a> <a class="l4">!not</a> <a class="l5">not-cx</a> <a class="k1" href="#a">link</a> <a class="k2" href="#b">!visited</a> <a class="k3" href="#c">any</a> <a class="k4">!any</a> <a class="k5" href="#d">list</a></div>
<div><span lang="de-CH"><a class="g1">de</a> <a class="g2">ch</a> <a class="g3">!fr</a></span> <span dir="auto" class="h1">שלום</span> <span class="h2" style="direction: rtl">!rtl-css</span> <span class="h3" dir="rtl">rtl</span></div>
<div><span class="x1"><a><a class="err">e</a></a>has</span> <span class="x2"><a class="sel">child</a></span> <span class="x3"><data><a class="sel">!child</a></data></span> <a class="x4">adj</a><a class="note">.</a> <a class="x5">sib</a><data>.</data><aside><a class="warn">w</a></aside> <span class="x6"><data><time class="dp">no-img</time></data></span> <span class="x7"><data class="img">!no-img</data></span></div>
<div><span class="y1"><a>hover</a></span> <form class="y2"><input type="checkbox">chk</form></div>
"##,
    css: r#"
.acid-t4 aside, .acid-t4 form { display: inline; }
.acid-t4 .d1 time { color: rgb(0, 160, 0); }
.acid-t4 .d2 > time { color: rgb(0, 160, 0); }
.acid-t4 .d3 > time { color: rgb(192, 0, 0); }
.acid-t4 .d8 > data a { color: rgb(0, 160, 0); }
.acid-t4 .d4a + .d4b { color: rgb(0, 160, 0); }
.acid-t4 .d5a + .d5b { color: rgb(0, 160, 0); }
.acid-t4 .d6a ~ .d6b { color: rgb(0, 160, 0); }
.acid-t4 .d7a + .d7b { color: rgb(192, 0, 0); }
.acid-t4 .a1[data-k=ab] { color: rgb(0, 160, 0); }
.acid-t4 .a2[data-k~=ab] { color: rgb(0, 160, 0); }
.acid-t4 .a3[data-k|=ab] { color: rgb(0, 160, 0); }
.acid-t4 .a4[data-k^=ab] { color: rgb(0, 160, 0); }
.acid-t4 .a5[data-k$=ab] { color: rgb(0, 160, 0); }
.acid-t4 .a6[data-k*=ab] { color: rgb(0, 160, 0); }
.acid-t4 .a7[data-k=ab i] { color: rgb(0, 160, 0); }
.acid-t4 .a8[type=a] { color: rgb(0, 160, 0); }
.acid-t4 .a9[type=a s] { color: rgb(192, 0, 0); }
.acid-t4 .a10[data-k=ab] { color: rgb(192, 0, 0); }
.acid-t4 .s1 > :first-child { color: rgb(0, 160, 0); }
.acid-t4 .s1 > :last-child { color: rgb(0, 160, 0); }
.acid-t4 .s2 > :only-child { color: rgb(0, 160, 0); }
.acid-t4 .s3 > :only-child { color: rgb(192, 0, 0); }
.acid-t4 .e1:empty::before { content: "empty"; color: rgb(0, 160, 0); }
.acid-t4 .e2:empty::before { content: "cmt"; color: rgb(0, 160, 0); }
.acid-t4 .e3:empty::before { content: "bad"; color: rgb(192, 0, 0); }
:root .acid-t4 .rt { color: rgb(0, 160, 0); }
:scope .acid-t4 .sc { color: rgb(0, 160, 0); }
.acid-t4 .n1 > :nth-child(2n+1) { color: rgb(0, 160, 0); }
.acid-t4 .n2 > :nth-child(even of .x) { color: rgb(0, 160, 0); }
.acid-t4 .n3 > :nth-last-child(-n+2) { color: rgb(0, 160, 0); }
.acid-t4 .n4 > a:nth-of-type(2) { color: rgb(0, 160, 0); }
.acid-t4 .n4 > data:nth-last-of-type(1) { color: rgb(0, 160, 0); }
.acid-t4 .n5 > a:first-of-type { color: rgb(0, 160, 0); }
.acid-t4 .n5 > a:last-of-type { color: rgb(0, 160, 0); }
.acid-t4 .n5 > time:only-of-type { color: rgb(0, 160, 0); }
.acid-t4 :is(.l1, .zz) { color: rgb(0, 160, 0); }
.acid-t4 :where(.l2) { color: rgb(0, 160, 0); }
.acid-t4 .l3:not(.zz) { color: rgb(0, 160, 0); }
.acid-t4 .l4:not(.l4) { color: rgb(192, 0, 0); }
.acid-t4 .l5:not(.zz .l5) { color: rgb(0, 160, 0); }
.acid-t4 .k1:link { color: rgb(0, 160, 0); }
.acid-t4 .k2 { color: rgb(0, 0, 192); }
.acid-t4 .k2:visited { color: rgb(192, 0, 0); }
.acid-t4 .k3:any-link { color: rgb(0, 160, 0); }
.acid-t4 .k4:any-link { color: rgb(192, 0, 0); }
.acid-t4 .k5:visited, .acid-t4 .k5:link { color: rgb(0, 160, 0); }
.acid-t4 .g1:lang(de) { color: rgb(0, 160, 0); }
.acid-t4 .g2:lang("*-CH") { color: rgb(0, 160, 0); }
.acid-t4 .g3:lang(fr) { color: rgb(192, 0, 0); }
.acid-t4 .h1:dir(rtl) { color: rgb(0, 160, 0); }
.acid-t4 .h2:dir(rtl) { color: rgb(192, 0, 0); }
.acid-t4 .h3:dir(rtl) { color: rgb(0, 160, 0); }
.acid-t4 .x1:has(.err) { color: rgb(0, 160, 0); }
.acid-t4 .x2:has(> a.sel) { color: rgb(0, 160, 0); }
.acid-t4 .x3:has(> a.sel) { color: rgb(192, 0, 0); }
.acid-t4 .x4:has(+ a.note) { color: rgb(0, 160, 0); }
.acid-t4 .x5:has(~ aside .warn) { color: rgb(0, 160, 0); }
.acid-t4 .x6:not(:has(.img)) { color: rgb(0, 160, 0); }
.acid-t4 .x7:not(:has(.img)) { color: rgb(192, 0, 0); }
.acid-t4 .y1:has(:hover) { color: rgb(0, 160, 0); }
.acid-t4 .y2:has(:checked) { color: rgb(0, 160, 0); }
"#,
    late_css: "",
    setup: None,
    script: None,
};
