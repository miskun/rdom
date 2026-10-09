//! Tile 9a — counters and generated content.
//!
//! Spec: CSS Generated Content 3 §1 (`content` strings, `attr()`; the alt
//! text after `/` is not rendered), §2 (quotes: `open-quote` /
//! `close-quote` take the pair at the current depth and move it; a
//! `close-quote` at depth 0 inserts nothing; `quotes: auto` by the content
//! language); CSS Lists 3 §4 (`counter-reset` makes a new counter, one
//! made by a previous sibling replaced rather than nested; the order
//! reset → increment → set; `counter()` / `counters()`); CSS Counter Styles
//! 3 §3 (the systems: `additive` `upper-roman` / `hebrew`, `alphabetic`
//! `lower-greek`, `numeric` `cjk-decimal`, `cyclic` `disc`), §3.1.4
//! (`negative`), §3.1.6 (`pad`: shortened by the negative sign's length),
//! §3.1.5 (`range`), §3.1.7 (`fallback`; a looping chain uses `decimal`),
//! §6 (`symbols()`); CSS Pseudo-Elements 4 §4 (`::before` / `::after` are
//! boxes: a `display: block` one stands on its own line, an inline one
//! wraps with its line), §2 (the legacy single-colon `:before`); CSS 2.1
//! Appendix E with CSS Position 3 (a negative `z-index` box paints below
//! its stacking context's in-flow content; a positive one stays inside its
//! own context; a relatively positioned box keeps its place in the line and
//! paints after the in-flow content); HTML §15.5.20 (a `summary` is a
//! `list-item` with an inside `disclosure-closed` marker, `disclosure-open`
//! when its `details` is open, otherwise unstyled; a closed `details` renders
//! nothing of its content slot, `::details-content`); DIVERGENCES §2
//! "`quotes: auto` knows a subset of languages", "A `<details>`'s content
//! slot is a box of the box tree".
//!
//! Derivation (bands are flex rows, a row apart, their items a cell apart
//! at the band's top row):
//!
//! - Row 0: `<` `attr(data-x)` `:` `mid` `>` → `<ATTR:mid>`; `★alt` (the
//!   `/ "star"` alt text is not drawn); `S:one` (`:before`). The
//!   `::before:hover` rule does not apply: nothing is hovered.
//! - Rows 2–6, x 0: `.ca` and each nested `.lv` reset `n`, each `.it`
//!   increments it: `counters(n, ".")` reads every instance outward-in —
//!   `1 A`, `1.1 B`, `1.2 C`, `1.2.1 D`, one block per row.
//! - x 10: the first `.sl` resets `m`; `x` 1, `y` 2, `w` incremented to 3
//!   then set to 7, `v` 8. The second `.sl`'s reset replaces the first
//!   list's `m` — made by its previous sibling — so `z` reads `1`, not
//!   `1.1` or `9`.
//! - x 16: 12 as `upper-roman` `XII`, `lower-greek` `μ` (the 12th letter),
//!   `hebrew` `יב` (10 + 2), `cjk-decimal` `一二` (two two-cell digits),
//!   `disc` `•` (`counter()` adds no suffix).
//! - x 32: `acid-x` extends `decimal` with `pad: 3 "0"`, `negative: "("
//!   ")"`, `range: -5 50`, `fallback: acid-y`. 7 → `007`. −3 → `(3)`: the
//!   pad of 3 is shortened by the sign's two graphemes to 1, which `3`
//!   meets. 99 is outside `acid-x`'s range: its fallback `acid-y` (range
//!   1–5) cannot take it either and falls back to `acid-x` — a loop, so
//!   `decimal`: `99`. `symbols(cyclic "a" "b")` at 3 → `a`.
//! - Row 8, x 0: `.qq`'s quotes are `« » ‹ ›`: `.q1` opens at depth 0
//!   (`«`), `.q2` at depth 1 (`‹`), they close `›` then `»`. The `q`s set
//!   `quotes: auto` and each opens at depth 0 by its language: German
//!   `„x“`, French `«y»` (no inner spaces), Japanese `「z」` (two-cell
//!   brackets). `.cq`'s `close-quote` at depth 0 draws nothing: `w`.
//! - x 23: `display: block` puts the `::before` `BLOCK` on its own line
//!   above `text`.
//! - x 30, 9 wide: `aa [bb cc] dd` (the span's `::before` / `::after`)
//!   breaks after `[bb` — `aa [bb cc]` is 10 — so `cc]` starts row 9.
//! - Row 11, x 0: `.host` is positioned with `z-index: auto` — no stacking
//!   context — so its `::after` (`z-index: -1`, navy, red `____` at the
//!   host's top-left) paints in the tile's context below every in-flow
//!   box: the host's `HOST` is drawn over it, keeping its navy.
//! - x 9: `.h2` (`z-index: 1`) holds an `::after` with `z-index: 5` at
//!   `left: 2` — teal `aaaa` over `.h2`'s `AA` from x 11; `.sib`
//!   (`z-index: 2`, purple, 2 wide) is moved up a row and 4 right, onto
//!   x 13–14: its context is above `.h2`'s whole context, so `SS` covers
//!   the `::after` there whatever its own 5. Row 12, where `.sib` lays
//!   out, stays empty.
//! - x 20: `.rb::before` `R` keeps its cell at x 20 and is drawn 2 right,
//!   at x 22, over `y` (positioned boxes paint after in-flow text): `xRz`
//!   from x 21.
//! - Row 14, x 0: the closed `details` shows only its summary, `▸ closed`
//!   — plain: HTML gives `summary` no weight or colour. x 21: the open one
//!   shows `▾ open`, then its content slot, green — `shown` (text directly
//!   inside `details` included) and `para`.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "9a",
    spec: &[
        "CSS Generated Content 3 §1, §2",
        "CSS Lists 3 §4; CSS Counter Styles 3 §3, §6",
        "CSS Pseudo-Elements 4 §2, §4; CSS 2.1 Appendix E",
        "HTML §15.5.20",
        "DIVERGENCES §2 quotes: auto, details content slot",
    ],
    legend: &[
        ('n', "bg #000080"),
        ('t', "bg #008080"),
        ('p', "bg #800080"),
        ('g', "fg #00a000"),
    ],
    grid: r#"
|<ATTR:mid> ★alt S:one                                     |
|..........................................................|
|                                                          |
|..........................................................|
|1 A       1 x   XII μ יב 一二 • 007 (3) 99 a              |
|..........................................................|
|1.1 B     2 y                                             |
|..........................................................|
|1.2 C     7 w                                             |
|..........................................................|
|1.2.1 D   8 v                                             |
|..........................................................|
|          1 z                                             |
|..........................................................|
|                                                          |
|..........................................................|
|«a‹b›» „x“ «y» 「z」 w BLOCK  aa [bb                      |
|..........................................................|
|                       text   cc] dd                      |
|..........................................................|
|                                                          |
|..........................................................|
|HOST     AAaaSS      xRz                                  |
|nnnn.......ttpp...........................................|
|                                                          |
|..........................................................|
|                                                          |
|..........................................................|
|▸ closed             ▾ open                               |
|..........................................................|
|                     shown                                |
|.....................ggggg................................|
|                     para                                 |
|.....................gggg.................................|
"#,
};
