//! Tile 24 — filters, blending and clipping.
//!
//! Spec: Filter Effects 1 §5 (a filter applies to the element's rendering
//! as a group), §6 / §13 (the shorthand functions as color matrices in
//! sRGB: `invert(1)` C' = 1 − C; `grayscale(1)` every channel 0.2126 R +
//! 0.7152 G + 0.0722 B; `sepia(1)` R' = .393 R + .769 G + .189 B, G' =
//! .349 R + .686 G + .168 B, B' = .272 R + .534 G + .131 B;
//! `hue-rotate(θ)` the luminance-preserving rotation — at 180° R' =
//! −.574 R + 1.43 G + .144 B, G' = .426 R + .43 G + .144 B, B' = .426 R +
//! 1.43 G − .856 B; `contrast(c)` C' = c · C + (0.5 − c / 2); `opacity(a)`
//! as group opacity; `drop-shadow()` the element's alpha offset and
//! coloured, under it), Filter Effects 2 §3 (`backdrop-filter` filters
//! what lies behind the element, under its own background); Compositing 1
//! §5.2 (an `isolation: isolate` group is the backdrop of its members),
//! §10 (`multiply` Cb · Cs, `difference` |Cb − Cs|, `luminosity`
//! SetLum(Cb, Lum(Cs)) with Lum = .3 R + .59 G + .11 B and ClipColor),
//! §5.1 (source-over); CSS Masking 1 §5 (`clip-path` with `inset() round`,
//! `circle()`, `polygon()`). DIVERGENCES §2 "A filter maps the colors of
//! the cells its element paints" (rounded once to 8 bits; `drop-shadow()`
//! shades whole cells), "A blend mode blends the colors of the cells its
//! element paints with the backdrop's background" (inside a group, only
//! the cells the group painted are a backdrop), "A clip path clips whole
//! cells" (a cell paints when its centre is inside the shape), §1
//! "Images" (`mask` parsed and kept, drawing nothing).
//!
//! Derivation (rows 0, 2, 4 — a drop shadow takes no layout room; items 1
//! apart):
//!
//! - Row 0: `inv`, white on black, inverted: black on white;
//!   red through `grayscale` 0.2126 · 255 = 54.2 → `#363636`; `#646464`
//!   through `sepia` → 135.1, 120.3, 93.7 → `#87785e`; red through
//!   `hue-rotate(180deg)` → R clamped to 0, G and B .426 · 255 = 108.6 →
//!   `#006d6d`; `#c8c8c8` through `contrast(0.5)` → 0.5 · 200 + 63.75 =
//!   163.75 → `#a4a4a4`; `#c80000` at `opacity(0.5)` and at `opacity: 0.5`
//!   over the black canvas alike → `#640000`; the red badge (x 35–37) and
//!   its blue drop shadow one cell right and down (x 36–38, row 1); the
//!   masked box (x 39) drawn as it is, its text in the default colour.
//! - Row 2: the orange box (`padding: 0 1`) and its five 3-wide members:
//!   `multiply` grey `#808080` → (255 · 128, 165 · 128, 0) / 255 =
//!   `#805300`; `difference` blue → (255, 165, 255); `luminosity` blue —
//!   Lum(Cs) 0.11 set on orange (Lum 0.682) and clipped: (41.1, 26.6, 0) →
//!   `#291b00`; in an isolated group with no background the grey has
//!   nothing to blend with and stays `#808080`; in one with a green
//!   background it multiplies with it → `#008000`.
//! - Row 4: `abcdef` (the text outside the strip in the default colour:
//!   the tile is an isolated group — one of its members blends — painted
//!   through a layer at full opacity, which changes no colour; ACID-FIX-9) with a 3-wide strip over `bcd`: inverted behind it —
//!   the text (white as blended) black, the canvas white — then its
//!   `rgb(0 0 100 / 60%)` background over that: (102, 102, 162) under the
//!   letters, tinted 0.6 · (0, 0, 100) + 0.4 · black = `#00003c`. The
//!   10 × 6 teal box with `inset(0 round 3)`: a corner cell's centre (0.5,
//!   0.5) lies 3.54 from the arc's centre (3, 3), outside; its neighbours'
//!   2.92, inside — the four corner cells cut. The 6 × 4 navy box with
//!   `circle(2 at 3 2)`: rows 0 and 3 keep x 2–3, rows 1–2 x 1–4. The 6 × 3
//!   olive box with the triangle (0 0, 6 0, 0 3): row 0 keeps x 0–4, row 1
//!   x 0–2, row 2 x 0 (centre x / 6 + y / 3 < 1).

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "24",
    spec: &[
        "Filter Effects 1 §5, §6, §13; Filter Effects 2 §3",
        "Compositing and Blending 1 §5, §10; CSS Masking 1 §5",
        "DIVERGENCES §2 filter, blend, clip path; §1 images (mask)",
    ],
    legend: &[
        ('I', "fg #000000 bg #ffffff"),
        ('a', "bg #363636"),
        ('b', "bg #87785e"),
        ('c', "bg #006d6d"),
        ('d', "bg #a4a4a4"),
        ('e', "bg #640000"),
        ('r', "bg #c80000"),
        ('u', "bg #0000ff"),
        ('t', "bg #008080"),
        ('O', "bg #ffa500"),
        ('1', "bg #805300"),
        ('2', "bg #ffa5ff"),
        ('3', "bg #291b00"),
        ('4', "bg #808080"),
        ('5', "bg #008000"),
        ('B', "fg #00003c bg #6666a2"),
        ('n', "bg #000080"),
        ('o', "bg #808000"),
    ],
    grid: r#"
|inv                                    msk                |
|IIII.aaaa.bbbb.cccc.dddd.eeee.eeee.rrr.tttt...............|
|                                                          |
|....................................uuu...................|
|                                                          |
|O111O222O333O444O555O.....................................|
|                                                          |
|..........................................................|
|abcdef                                                    |
|.BBB....tttttttt....nn...ooooo............................|
|                                                          |
|.......tttttttttt..nnnn..ooo..............................|
|                                                          |
|.......tttttttttt..nnnn..o................................|
|                                                          |
|.......tttttttttt...nn....................................|
|                                                          |
|.......tttttttttt.........................................|
|                                                          |
|........tttttttt..........................................|
"#,
};
