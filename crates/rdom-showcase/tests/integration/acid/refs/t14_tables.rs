//! Tile 14 — tables and border collapse.
//!
//! Spec: CSS 2.1 §17.2.1 (anonymous table objects: consecutive
//! `table-row` boxes get an anonymous `table`), §17.4 / CSS Tables 3
//! (`caption-side`), §17.5.1 (a row's background under its cells),
//! §17.5.2.1 (fixed layout: the columns from the first row, the table's
//! width), §17.5.2.2 (auto layout: a column as wide as its widest cell, a
//! spanning cell's need spread over its columns; the extra width of a
//! wider table spread by max-content), §17.5.3 (row heights; a cell's
//! `vertical-align`), §17.6.1 (separated borders, `border-spacing` also
//! between the cells and the table's edges; `empty-cells: hide` draws no
//! border or background around an empty cell), §17.6.2 / CSS Tables 3
//! §11.5 (collapsed borders: `hidden` suppresses every border at its
//! place; then the wider wins; then the style, `double` above `solid`,
//! `none` below all; then a cell's beats the table's; between two cells
//! the left / upper one); Selectors 4 §16 (`||` and `:nth-col()` match a
//! cell in every column it spans); HTML §15.3.8 (the UA's `th` bold and
//! centred unless the parent's `text-align` is not the initial one,
//! `caption` centred, `vertical-align: middle` on the row groups); CSS
//! Overflow 3 §2 (`overflow: hidden` on a cell clips). DIVERGENCES §1 (a
//! border is one cell, its width a glyph weight; junctions resolve by
//! §11.5 and take the dominant side's set where Unicode has no mixed
//! glyph — a double axis with a single arm — and the mixed light / heavy
//! glyphs otherwise), §2 (the table formatting context in whole cells:
//! cells keep a one-cell inline padding, `border-spacing` 0 by default,
//! a collapsed line exists wherever any box has a border there, `hidden`
//! included; `middle` takes the upper of two middle rows).
//!
//! Derivation (band 1 at row 0: `.cc` x 0, `.sp` x 11, `.cs` x 25, `.an`
//! x 39 — flex items 2 apart; band 2 at row 7: `.va` x 0, `.fw` x 25,
//! `.fx` x 41):
//!
//! - `.cc`, collapsed, 2 × 2, the table `solid` red, the cells `solid` in
//!   the text colour; lines at x 0, 4, 8 and rows 0, 2, 4. Top: over `a`
//!   the cell ties the table and wins (default `─`); over `b`, whose top
//!   is `none`, the table's red `─`. The `double` `b` beats `solid` on
//!   its left (x 4) and right (x 8) and bottom (`═`). `c`'s `hidden` left
//!   suppresses the table's left border on row 3: a blank cell — and,
//!   §17.6.2 putting a cell's padding edge half its collapsed border in
//!   from the grid line, `c`'s border there is none (0 wide), so its
//!   padding starts on the line: the line is a whole cell that `c`
//!   covers (DIVERGENCES §2, `hidden` included), so `c` is at x 1, a cell
//!   left of `a` — whose one-cell border's half takes the whole line. `d`'s
//!   `thick` bottom is wider than the table's: heavy `━`. Junctions: (4,
//!   0) single across, double down — `╥`; (8, 0) `╖`; (0, 2) up and right
//!   only — `└`; (4, 2) double up and right, single down and left — no
//!   mixed glyph, the dominant double set: `╬`; (8, 2) likewise `╣`;
//!   (0, 4) only the bottom line's run east: `─`; (4, 4) light up and
//!   left, heavy right: `┶`; (8, 4) light up, heavy left: `┙`. Every
//!   junction's winner is a cell's border: the default colour.
//! - `.sp`, separated, `border-spacing: 1 0`, cells `padding: 0`: columns
//!   3 (`r`), 3 (`a`), 2 (the empty cell, its borders still laid out) —
//!   `wide` (6) spans 3 + 1 + 2 = 6, so nothing grows; the table is
//!   1 + 3 + 1 + 3 + 1 + 2 + 1 = 12. The empty cell is hidden. `r` spans
//!   both 3-row rows: 6 rows, its text `middle` in 4 content rows — 3
//!   free, the upper middle — row 2.
//! - `.cs`, `text-align: right`, the caption at the bottom: columns 4
//!   each (two letters + padding). The `th`s are bold and right-aligned
//!   (the parent's `text-align` is not the initial `start`); the `td`s
//!   inherit `right`. `col.x || td`: `bb` and the spanning `ef` (it spans
//!   column 2) red; `td:nth-col(3)`: `cc`'s cell and `ef`'s (it spans
//!   column 3) green, padding included; `ef` right-aligned in its 6 content
//!   cells. The caption keeps its UA `center`: (12 − 3) / 2 rounded down —
//!   x 4.
//! - `.an`: two `table-row` divs in an anonymous table; the cells share
//!   columns 4 (`ab`, `d`) and 5 (`c`, `efg`).
//! - `.va`: four columns of 5 and a last of 3 (`1`, `2`, `3` are one
//!   cell each, so its max-content is 1); the caption centred over 23
//!   ((23 − 3) / 2 = x 10). The `th`s bold, centred in their content
//!   cells (x + 1 in the first four, `E` alone at x 21). The second row is 3
//!   rows tall (`1`/`2`/`3`): `top` on its first row, `mid` its second
//!   (2 free rows, 1 above), `bot` its third, `bas` — the only `baseline`
//!   cell, so the row's baseline is its first line — its first. In the
//!   `tbody` that row is child 1, the `z` row child 2: navy across the 23
//!   cells, and nothing else striped.
//! - `.fw` (14 wide) holds a `width: 100%` table with a border: border-box
//!   (HTML's UA), so its right border is the box's last column; its two
//!   cells (3 each at max-content) share the 12 inner cells by max-content
//!   — 6 and 6: `a` at 2, `b` at 8.
//! - `.fx`, fixed, 12 wide, cells `padding: 0`: the first row makes the
//!   columns — `width: 4`, then the rest, 8 — so `longer` on the second
//!   row does not widen the first column and is clipped to `long` by its
//!   cell's `overflow: hidden`; `y` and `z` at x 4.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "14",
    spec: &[
        "CSS 2.1 §17.2.1, §17.4–§17.6; CSS Tables 3 §11.5",
        "Selectors 4 §16; HTML §15.3.8",
        "DIVERGENCES §1 border one cell, corner; §2 table formatting context",
    ],
    legend: &[
        ('r', "fg #ff0000"),
        ('B', "bold"),
        ('g', "bg #008000"),
        ('R', "fg #ff0000 bg #008000"),
        ('n', "bg #000080"),
    ],
    grid: r#"
|┌───╥───╖   ┌─┐ ┌─┐        A   B   C    ab  c             |
|.....rrr...................B...B...B......................|
|│ a ║ b ║   │ │ │a│       aa  bb  cc    d   efg           |
|..............................rr.gggg.....................|
|└───╬═══╣   │r│ └─┘       dd      ef                      |
|.............................gggggRRg.....................|
| c  │ d │   │ │ ┌────┐       cap                          |
|..........................................................|
|────┶━━━┙   │ │ │wide│                                    |
|..........................................................|
|            └─┘ └────┘                                    |
|..........................................................|
|                                                          |
|..........................................................|
|          cap            ┌────────────┐  x   y            |
|..........................................................|
|  A    B    C    D   E   │ a     b    │  longz            |
|..B....B....B....B...B....................................|
| top            bas  1   └────────────┘                   |
|..........................................................|
|      mid            2                                    |
|..........................................................|
|           bot       3                                    |
|..........................................................|
| z    z    z    z    z                                    |
|nnnnnnnnnnnnnnnnnnnnnnn...................................|
"#,
};
