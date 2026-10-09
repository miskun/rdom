//! Tile 21 — feature queries.
//!
//! Spec: CSS Conditional 3 §6 (`@supports (decl)` is true when the UA
//! parses the declaration; `not` / `and` / `or`), Conditional 4 §6
//! (`selector()`: true when the selector parses), Conditional 5 §5
//! (`font-tech()` / `font-format()`), Conditional 3 §6.1 (a
//! `<general-enclosed>` is false — so its `not` is true); CSS Nesting 1
//! (`@supports` inside a style rule), Conditional 3 §2 (inside `@media`),
//! CSS Cascade 5 §3 (`@import … supports()`: imported only when the
//! condition holds). DIVERGENCES §2 "No at-rule but … is evaluated"
//! (`@supports` answers for what rdom parses: a pixel geometry value is
//! not — `(width: 10px)` false; `font-tech()` / `font-format()` false;
//! a `<general-enclosed>` false).
//!
//! Derivation: green — `grid`, `notf`, `and` (both parse), `or` (`color:
//! red` parses), `has` (`:has(+ p)` parses), `nge`, `nest`, `inm`, `imp`;
//! default — `frob`, `wpx`, `sel` (`:frob` does not parse), `tech`,
//! `fmt`, `ge`, `impn`.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "21",
    spec: &[
        "CSS Conditional 3 §2, §6; Conditional 4 §6; Conditional 5 §5",
        "CSS Nesting 1; CSS Cascade 5 §3",
        "DIVERGENCES §2 at-rules evaluated, @supports",
    ],
    legend: &[('g', "fg #00a000")],
    grid: r#"
|grid frob wpx notf and or has sel tech fmt ge nge nest inm|
|gggg..........gggg.ggg.gg.ggg.................ggg.gggg.ggg|
|imp impn                                                  |
|ggg.......................................................|
"#,
};
