//! Tile 1 — cascade order.
//!
//! Spec: CSS Cascade 4 §6.1 (the cascade sorting order: origin and
//! importance, then the style attribute, then specificity, then order of
//! appearance), §6.2 (the origins: UA below author; important author
//! declarations above normal ones), §4.2 (an invalid declaration is
//! ignored); CSS Cascade 5 §6.4 (cascade layers: unlayered normal styles
//! above every layer, important layered styles above unlayered important
//! ones); CSSOM §6.2 + `DIVERGENCES.md` §2 "An `App`'s own sheets cascade
//! after the document's `<style>` sheets".
//!
//! Derivation. Green `#00a000` is every contest's winner, red `#c00000`
//! every loser, so a wrong winner shows as a red word (or, for `ua`, as
//! the UA's black on yellow). The words sit in four one-line `div`s; the
//! `<style>` element is `display: none` and the white space between the
//! blocks makes no line box (CSS 2.1 §9.2.1.1, §16.6.1), so the rows are
//! the four `div`s, then the tile's empty fifth row.
//!
//! - `ua`: `mark` is black on yellow in the UA sheet (HTML §15.3.4); an
//!   author rule beats any normal UA rule (§6.2): green on the default
//!   background (`background-color: transparent` paints nothing).
//! - `late-sheet`: equal specificity in the main sheet (red) and the
//!   later-pushed one (green): the later sheet's declaration is later in
//!   the order of appearance (§6.1 "Order of Appearance").
//! - `style-el`: equal specificity in the `<style>` element (red) and the
//!   App's main sheet (green): the App's sheets come after the document's
//!   (DIVERGENCES §2), so green.
//! - `style-spec`: the `<style>` element's `span.c-style2` (0,2,1) beats the
//!   main sheet's `.c-style2` (0,2,0) whatever the order: green.
//! - `inline`: a style attribute's declaration (green) beats every normal
//!   rule declaration, even the 0,4,0 one (§6.1 "Element-Attached
//!   Styles").
//! - `imp-sheet`: an important sheet declaration (green) beats a normal
//!   style attribute (red) — importance comes first (§6.1 "Origin and
//!   Importance").
//! - `imp-inline`: important in both; the style attribute's (green) wins
//!   (§6.1 "Element-Attached Styles").
//! - `order`: two equal-specificity rules in one sheet: the later (green).
//! - `imp-main`: the main sheet's important declaration (green) beats the
//!   later sheet's normal one.
//! - `spec-main`: the main sheet's `span.c-spec` (0,2,1) beats the later
//!   sheet's `.c-spec` (0,2,0): specificity before order.
//! - `invalid`: `color: not-a-color` is invalid and ignored at parse time
//!   (Syntax 3 / Cascade 4 §4.2), so the valid green before it stands.
//! - `unlayered`: an unlayered normal rule (green, 0,2,0) beats a layered
//!   one with more specificity (red, 0,4,1) — Cascade 5 §6.4.
//! - `imp-layer`: important in layer `base` (green) beats important
//!   unlayered (red) — the layer order reverses for important styles.
//! - `layer-id`: an id selector in a layer (red) still loses to the
//!   unlayered class (green).
//!
//! The spaces between words are blank cells in the tile's red, which a
//! blank does not show (the reference format compares no foreground on
//! a blank), so they are `.`.

use super::super::reference::Reference;

pub const REF: Reference = Reference {
    tile: "1",
    spec: &[
        "CSS Cascade 4 §6.1, §6.2, §4.2",
        "CSS Cascade 5 §6.4",
        "DIVERGENCES §2 App sheets after <style> sheets",
    ],
    legend: &[('g', "fg #00a000")],
    grid: r#"
|ua late-sheet style-el style-spec     |
|gg.gggggggggg.gggggggg.gggggggggg.....|
|inline imp-sheet imp-inline           |
|gggggg.ggggggggg.gggggggggg...........|
|order imp-main spec-main invalid      |
|ggggg.gggggggg.ggggggggg.ggggggg......|
|unlayered imp-layer layer-id          |
|ggggggggg.ggggggggg.gggggggg..........|
|                                      |
|......................................|
"#,
};
