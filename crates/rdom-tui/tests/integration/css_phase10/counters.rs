//! C10-COUNTERS — CSS Lists 3 §4 (counters) and CSS Counter Styles 3 §6
//! (the predefined counter styles), through `counter()` / `counters()`
//! in generated content.

use super::{el, paint_tree, text_el};

/// CSS Counter Styles 3 §6: `counter(c, <style>)` writes the counter in
/// any predefined style — numeric, alphabetic, additive, symbolic,
/// fixed — and `none` writes nothing.
#[test]
fn counter_writes_every_predefined_style() {
    let styles = [
        ("decimal-leading-zero", "07"),
        ("lower-greek", "η"),
        ("upper-roman", "VII"),
        ("hebrew", "ז"),
        ("armenian", "Է"),
        ("georgian", "ზ"),
        ("cjk-decimal", "七"),
        ("arabic-indic", "٧"),
        ("disc", "•"),
        ("cjk-earthly-branch", "午"),
        ("none", ""),
    ];
    let mut css = String::from(".l { counter-reset: c 7 } .i { display: block }");
    for (i, (style, _)) in styles.iter().enumerate() {
        css.push_str(&format!(
            ".s{i}::before {{ content: counter(c, {style}) \"|\" }}"
        ));
    }
    let rows = paint_tree(&css, 6, styles.len() as u16, |dom, root| {
        let l = el(dom, root, "div", "l");
        for i in 0..styles.len() {
            text_el(dom, l, "div", &format!("i s{i}"), "");
        }
    });
    for (row, (style, want)) in rows.iter().zip(styles) {
        assert!(
            row.starts_with(&format!("{want}|")),
            "{style}: {row:?} should start with {want:?}|"
        );
    }
}

/// CSS Counter Styles 3 §6.3: `disclosure-closed` points to the inline
/// end — `▸` left to right, `◂` right to left.
#[test]
fn disclosure_closed_follows_the_direction() {
    let css = r#".r { direction: rtl } .i::before { content: counter(c, disclosure-closed) }"#;
    let rows = paint_tree(css, 4, 2, |dom, root| {
        text_el(dom, root, "div", "i", "a");
        text_el(dom, root, "div", "i r", "a");
    });
    assert_eq!(rows[0], "▸a  ");
    assert!(rows[1].contains('◂'), "{:?}", rows[1]);
}
