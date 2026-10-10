//! I7 — rewrite a `<style>` element and an inline style through the
//! CSSOM (tile 38).
//!
//! Spec: HTML §4.2.6 — when a `<style>` element's child text changes,
//! the UA must update its style sheet: the old sheet is dropped and the
//! new text parsed in its place, at the same position in the document's
//! sheet order; CSSOM §6.7 — `setProperty()` / `removeProperty()` on an
//! element's `style` change its `style` attribute's declarations; HTML
//! §8.1.7.3 — the next rendering update restyles. CSS Cascade 4 §6.1
//! (important author declarations beat normal ones whatever their sheet
//! order; the `style` attribute's normal declarations beat every normal
//! sheet rule); DIVERGENCES §2 "An `App`'s own sheets cascade after the
//! document's `<style>` sheets" (the order the rewritten sheet keeps).
//!
//! Derivation, from tile 38's rest state:
//!
//! 1. The `<style>` text becomes `.a { green } .b { green !important }`
//!    and `inl`'s `style.setProperty("color", green)`: `sty` green (the
//!    sheet's only rule); `app` green — important beats the App sheet's
//!    normal blue though that sheet is later; `inl` green.
//! 2. `inl`'s `style.removeProperty("color")`: no rule colours `.c`, so
//!    it takes the tile's default; the others stay green.

use rdom_tui::TuiAccessorsMut;

use super::super::super::reference::Reference;
use super::super::session::{Session, Step};

pub static STEP: Step = Step {
    id: "I7",
    title: "Rewrite a <style> and an inline style",
    page: 10,
    spec: &["HTML §4.2.6; CSSOM §6.7; CSS Cascade 4 §6.1"],
    run,
    configure: None,
};

const SPEC: &[&str] = &[
    "HTML §4.2.6; CSSOM §6.7; CSS Cascade 4 §6.1",
    "DIVERGENCES §2 App sheets after <style> sheets",
];

static REWRITTEN: Reference = Reference {
    tile: "38",
    spec: SPEC,
    legend: &[('g', "fg #00a000")],
    grid: r#"
|sty app inl                           |
|ggg.ggg.ggg...........................|
"#,
};

static REMOVED: Reference = Reference {
    tile: "38",
    spec: SPEC,
    legend: &[('g', "fg #00a000")],
    grid: r#"
|sty app inl                           |
|ggg.ggg...............................|
"#,
};

const NEW_SHEET: &str = ".acid-t38 .a { color: rgb(0, 160, 0); } \
                         .acid-t38 .b { color: rgb(0, 160, 0) !important; }";

fn run(s: &mut Session) {
    let (st, c) = (s.find("38", ".st"), s.find("38", ".c"));
    s.script(|dom| {
        dom.set_text_content(st, NEW_SHEET).unwrap();
        let mut node = dom.node_mut(c);
        let mut style = node.style_mut().expect("an element");
        style.set_property("color", "rgb(0, 160, 0)").unwrap();
    });
    s.expect("the sheet and the inline style rewritten", &[&REWRITTEN]);
    s.script(|dom| {
        let mut node = dom.node_mut(c);
        let mut style = node.style_mut().expect("an element");
        style.remove_property("color").unwrap();
    });
    s.expect("the inline color removed", &[&REMOVED]);
}
