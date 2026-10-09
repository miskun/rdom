//! Tile 2 — specificity.
//!
//! Spec: Selectors 4 §17 (calculating a selector's specificity: ids,
//! then classes / attributes / pseudo-classes, then types; `:is()`,
//! `:not()` and `:has()` take their most specific argument, `:where()`
//! zero, `:nth-child(An+B of S)` a pseudo-class plus its most specific
//! argument; a selector list's specificity is each selector's own — the
//! one that matched); CSS Cascade 4 §6.1 (specificity before order of
//! appearance).
//!
//! Derivation. In each contest the green rule comes first and the red one
//! later, so only a higher specificity lets green win; every word is
//! green (`#00a000`). Specificities below include the `.acid-t2` scope
//! class, `(a, b, c)` = ids, classes, types.
//!
//! - `type<class`: `.k1` (0,2,0) beats `data` (0,1,1).
//! - `id>class`: `#k2` (1,1,0) beats `.k2.k2.k2.k2` (0,5,0).
//! - `where:0`: `time` (0,1,1) beats `:where(#k3)` (0,1,0).
//! - `is:max`: `:is(#k4, output)` (1,1,0) beats `output.k4.k4` (0,3,1).
//! - `not:arg`: `.k5:not(#nope)` (1,2,0) beats `a.k5.k5` (0,3,1).
//! - `list:branch`: `a.k6` (0,2,1) beats the list `#nope, .k6`, whose
//!   matching selector is (0,2,0) — not the list's highest, (1,1,0).
//! - `attr=class`: `[data-k7]` and `.k7` are both (0,2,0); the later,
//!   green, wins on order — an attribute weighs as a class.
//! - `nth-of`: `:nth-child(1 of #k8)` (1,2,0) beats `a.k8.k8` (0,3,1).
//!
//! The `<a>` elements have no `href`, so no UA link style applies.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "2",
    spec: &["Selectors 4 §17", "CSS Cascade 4 §6.1"],
    legend: &[('g', "fg #00a000")],
    grid: r#"
|type<class id>class where:0           |
|gggggggggg.gggggggg.ggggggg...........|
|is:max not:arg list:branch            |
|gggggg.ggggggg.ggggggggggg............|
|attr=class nth-of                     |
|gggggggggg.gggggg.....................|
"#,
};
