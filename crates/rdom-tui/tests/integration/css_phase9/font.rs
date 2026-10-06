//! C9-FONT — the font properties (CSS Fonts 4): `font-weight` (numeric,
//! `bolder` / `lighter`) and `font-style` (`oblique`) drawn as SGR bold
//! and italic; the `font` shorthand, which also resets `line-height`; the
//! other font properties parsed, cascaded and inert.

use super::{el, lay_out, paint, rect, rows, text_block};
use rdom_tui::prelude::*;

/// The modifiers of the first cell of `.b`'s text, styled `decl`.
fn mods(decl: &str) -> Modifier {
    let (mut dom, _, _) = text_block("ab");
    let buf = paint(&mut dom, &format!(".b {{ {decl} }}"), 2, 1);
    buf.cell(0, 0).expect("in the buffer").modifier
}

/// §2.2: a numeric weight; the terminal's bold from 600 ("Semi Bold"),
/// lighter weights drawn normal (DIVERGENCES §2).
#[test]
fn a_numeric_weight_is_bold_from_600() {
    for (weight, bold) in [
        ("100", false),
        ("300", false),
        ("500", false),
        ("600", true),
        ("700", true),
        ("1000", true),
        ("bold", true),
        ("normal", false),
    ] {
        assert_eq!(
            mods(&format!("font-weight: {weight}")).contains(Modifier::BOLD),
            bold,
            "font-weight: {weight}"
        );
    }
}

/// §2.2, "Determining the relative weight": `bolder` and `lighter` step
/// from the parent's weight by the spec's table.
#[test]
fn bolder_and_lighter_are_relative_to_the_parent() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", "p");
    let mut spans = Vec::new();
    for (class, text) in [("a", "a"), ("b", "b"), ("c", "c")] {
        let s = el(&mut dom, p, "span", class);
        let t = dom.create_text_node(text);
        dom.append_child(s, t).unwrap();
        spans.push(s);
    }
    // 400 → bolder 700 (bold); 700 → lighter 400; 300 → bolder 400.
    let css = ".a { font-weight: bolder } \
               .b { font-weight: 700 } .b { font-weight: lighter } \
               .p .c { font-weight: 300 }";
    let buf = paint(&mut dom, css, 3, 1);
    assert!(buf.cell(0, 0).unwrap().modifier.contains(Modifier::BOLD));
    assert!(!buf.cell(1, 0).unwrap().modifier.contains(Modifier::BOLD));
    assert!(!buf.cell(2, 0).unwrap().modifier.contains(Modifier::BOLD));

    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", "p");
    let s = el(&mut dom, p, "span", "s");
    let t = dom.create_text_node("x");
    dom.append_child(s, t).unwrap();
    let buf = paint(
        &mut dom,
        ".p { font-weight: 800 } .s { font-weight: lighter }",
        1,
        1,
    );
    assert!(
        buf.cell(0, 0).unwrap().modifier.contains(Modifier::BOLD),
        "lighter than 800 is 700"
    );
}

/// §2.4: `oblique` (with an angle or none) is drawn italic; `oblique
/// 0deg` is upright.
#[test]
fn oblique_is_italic() {
    assert!(mods("font-style: oblique").contains(Modifier::ITALIC));
    assert!(mods("font-style: oblique 10deg").contains(Modifier::ITALIC));
    assert!(mods("font-style: oblique -20deg").contains(Modifier::ITALIC));
    assert!(!mods("font-style: oblique 0deg").contains(Modifier::ITALIC));
    assert!(mods("font-style: italic").contains(Modifier::ITALIC));
}

/// §3.7: the `font` shorthand sets weight and style, and resets
/// `line-height` — to `normal` when it omits it.
#[test]
fn the_font_shorthand_sets_weight_style_and_line_height() {
    let m = mods("font: italic small-caps bold condensed 16px/2 \"Fira Code\", monospace");
    assert!(m.contains(Modifier::BOLD | Modifier::ITALIC));
    let (mut dom, b, _) = text_block("aa bb");
    lay_out(
        &mut dom,
        ".b { width: 2; line-height: 3; font: 12px monospace }",
        2,
        8,
    );
    assert_eq!(rect(&dom, b).height, 2, "the shorthand reset line-height");
    let (mut dom, b, _) = text_block("aa bb");
    lay_out(&mut dom, ".b { width: 2; font: 600 1em/2 serif }", 2, 8);
    assert_eq!(rect(&dom, b).height, 4);
    let buf = paint(&mut dom, ".b { width: 2; font: menu }", 2, 2);
    assert_eq!(rows(&buf, 2, 2), ["aa", "bb"]);
}

/// The inert font properties parse (a strict sheet takes them) and
/// round-trip through the CSSOM.
#[test]
fn the_inert_font_properties_parse_and_round_trip() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let b = el(&mut dom, root, "div", "b");
    let css = ".b { font-size: 1.2em; font-family: \"Fira Code\", monospace; \
               font-stretch: semi-condensed; font-variant: small-caps; font-width: 80% }";
    lay_out(&mut dom, css, 4, 1);
    dom.node_mut(b)
        .style_mut()
        .expect("an element")
        .set_property("font", "bold 14px/1.5 sans-serif")
        .unwrap();
    let style = dom.node(b).style().expect("an element");
    assert_eq!(style.get_property_value("font-weight"), "bold");
    assert_eq!(style.get_property_value("font-size"), "14px");
    assert_eq!(style.get_property_value("line-height"), "1.5");
    assert_eq!(style.get_property_value("font-family"), "sans-serif");
    assert_eq!(
        style.get_property_value("font"),
        "bold 14px / 1.5 sans-serif"
    );
}
