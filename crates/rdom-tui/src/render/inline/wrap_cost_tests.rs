//! C9-TEXT-WRAP: what `text-wrap-style` costs, counted in graphemes fed to
//! the packer (`packer::GRAPHEMES`, replays included) — `balance` is
//! bounded by the text's length times about log2 of the line width, never
//! quadratic in the text or in its number of groups.

use super::super::packer::GRAPHEMES;
use crate::render::Rect;
use crate::{CascadeExt, LayoutExt, TuiDom};

/// The graphemes fed to lay out a 20-cell block of `text` styled `decl`
/// (the text's own count subtracted out: the multiple of it).
fn fed(decl: &str, text: &str) -> usize {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = dom.create_element("div");
    dom.set_attribute(b, "class", "b").unwrap();
    dom.append_child(root, b).unwrap();
    let t = dom.create_text_node(text);
    dom.append_child(b, t).unwrap();
    let sheet = rdom_css::from_css_strict(&format!(".b {{ width: 20; {decl} }}")).unwrap();
    dom.cascade(&sheet);
    GRAPHEMES.with(|c| c.set(0));
    dom.layout_dom(Rect::new(0, 0, 20, 400));
    GRAPHEMES.with(|c| c.get())
}

/// A paragraph longer than six lines is packed once: no replay.
#[test]
fn a_long_paragraph_is_not_balanced() {
    let text = "word ".repeat(400);
    let auto = fed("", &text);
    assert_eq!(fed("text-wrap: balance", &text), auto);
}

/// A short paragraph: the greedy pack, one replay per bisection step of
/// the 20-cell width (⌈log2 20⌉ = 5) and the final one.
#[test]
fn a_short_paragraph_costs_log_width_replays() {
    let text = "aa bb cc dd ee ff gg hh ii jj kk";
    let auto = fed("", text);
    let balanced = fed("text-wrap: balance", text);
    assert!(balanced > auto, "it was balanced");
    assert!(balanced <= auto * (1 + 5 + 1), "{balanced} for {auto}");
}

/// Many short groups between forced breaks are bisected together: the
/// replays do not multiply by the number of groups.
#[test]
fn many_groups_are_balanced_together() {
    let text = "aa bb cc dd ee ff gg hh ii jj\n".repeat(60);
    let auto = fed("white-space: pre-line", &text);
    let balanced = fed("white-space: pre-line; text-wrap: balance", &text);
    assert!(balanced <= auto * (1 + 5 + 1), "{balanced} for {auto}");
}

/// `pretty` replays once at most.
#[test]
fn pretty_replays_once() {
    let text = format!("{}ddd", "aaa bbb ccc ".repeat(30));
    let auto = fed("", &text);
    assert!(fed("text-wrap: pretty", &text) <= auto * 2);
}
