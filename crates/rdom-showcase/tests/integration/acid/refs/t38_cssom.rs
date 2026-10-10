//! Tile 38 — CSSOM rewrites, at rest (the state step I7 starts from).
//!
//! Spec: HTML §4.2.6 (a connected `<style>` element's text is a live
//! document sheet; the element itself is `display: none`, §15.3.1);
//! CSS Cascade 4 §6.1 (the `style` attribute's declarations above every
//! normal author rule; at equal specificity the later sheet wins);
//! DIVERGENCES §2 "An `App`'s own sheets cascade after the document's
//! `<style>` sheets".
//!
//! Derivation: `sty` red (the `<style>` element's rule alone); `app`
//! blue — the App's sheet, ordered after the `<style>` sheet, wins the
//! equal-specificity contest; `inl` red (its `style` attribute).

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "38",
    spec: &[
        "HTML §4.2.6, §15.3.1; CSS Cascade 4 §6.1",
        "DIVERGENCES §2 App sheets after <style> sheets",
    ],
    legend: &[('r', "fg #ff0000"), ('b', "fg #0000ff")],
    grid: r#"
|sty app inl                           |
|rrr.bbb.rrr...........................|
"#,
};
