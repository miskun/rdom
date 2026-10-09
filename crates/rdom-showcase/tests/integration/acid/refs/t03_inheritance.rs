//! Tile 3 — inheritance and keywords.
//!
//! Spec: CSS Cascade 4 §3.1 (inherited properties take the parent's
//! computed value), §7.3.1 `initial`, §7.3.2 `inherit`, §7.3.3 `unset`
//! (inherit for an inherited property, initial otherwise); CSS Color 4
//! §3 (`color` inherits, its initial value `CanvasText` — the terminal's
//! default foreground); CSS Backgrounds 3 §3.2 (`background-color` does
//! not inherit, initial `transparent`); CSS Fonts 4 §2.2, §3.7 (the font
//! weight and style inherit; `bold` and `italic` draw as SGR bold and
//! italic, DIVERGENCES §2 "The font is drawn as SGR bold and italic");
//! CSS Variables 1 §2 (custom properties inherit), §3 (`var()` and its
//! fallback), §3.1 (a `var()` naming a missing property with no fallback
//! makes the declaration invalid at computed-value time, so the property
//! behaves as `unset`); CSS Overflow 3 §2 (`overflow: visible` content
//! paints outside its box).
//!
//! Derivation, row by row (green `#00a000`, blue `#0000c0`, navy
//! `#000080`):
//!
//! 0. `.i-col` is green: `color` and the `span`'s `kid` inherit it.
//!    `em.i-init { color: initial }` — `CanvasText`, the default
//!    foreground — keeps the UA's italic. `.i-unset` declares red and then
//!    `unset` in one block: the later valid declaration wins (Cascade 4
//!    §6.1), and `unset` on the inherited `color` inherits: green.
//!    `.i-inh`'s parent `.i-mid` is blue; red then `inherit`: blue.
//! 1. `.i-font` is bold, italic and green; `bold` and the `span`'s `kid`
//!    inherit all three. `.i-norm` sets the weight to `initial` (`normal`)
//!    and the style to `normal`: green, plain.
//! 2. (to 7.) Three `.i-bg` boxes, navy, one row tall with a one-row bottom
//!    margin; each holds the text `parent` (its anonymous block's line, the
//!    whole row navy — the block is the tile's 38 cells wide) and a block
//!    child on the next row, overflowing the parent's one row. Painted over
//!    no parent background, the child row shows only the child's own:
//!    `no-inherit` sets none — `background-color` does not inherit —
//!    default; `inherit` takes the parent's navy across its 38 cells;
//!    `unset` (after a red in the same block) is `initial`, `transparent`:
//!    default. The margins put the boxes at rows 2, 4 and 6 (their
//!    margins do not collapse with anything between them but each other's
//!    zero top margins).
//! 8. `.i-var` defines `--c: green` and is blue itself. `var` reads
//!    `var(--c)`: green (inherited custom property). `fallback` reads
//!    `var(--nope, green)`: green. `missing` declares red, then
//!    `var(--nope)`: the later declaration wins at parse time and is
//!    invalid at computed-value time, so `color` is `unset` — inherited
//!    from `.i-var`: blue. `nested` reads `var(--nope, var(--c))`: green.
//!    `deep` reads `--c` two levels below its definition: green.
//!
//! Blanks between words show nothing but their background (`.`). The
//! static stage cannot show `user-select: unset` (ACID.md); stage 2 does.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "3",
    spec: &[
        "CSS Cascade 4 §3.1, §7.3",
        "CSS Variables 1 §2, §3, §3.1",
        "CSS Backgrounds 3 §3.2",
        "CSS Fonts 4 §2.2, §3.7",
    ],
    legend: &[
        ('g', "fg #00a000"),
        ('b', "fg #0000c0"),
        ('i', "italic"),
        ('B', "fg #00a000 bold italic"),
        ('n', "bg #000080"),
    ],
    grid: r#"
|color kid initial unset inherit       |
|ggggg.ggg.iiiiiii.ggggg.bbbbbbb.......|
|bold kid normal                       |
|BBBB.BBB.gggggg.......................|
|parent                                |
|nnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnn|
|no-inherit                            |
|......................................|
|parent                                |
|nnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnn|
|inherit                               |
|nnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnn|
|parent                                |
|nnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnn|
|unset                                 |
|......................................|
|var fallback missing nested deep      |
|ggg.gggggggg.bbbbbbb.gggggg.gggg......|
"#,
};
