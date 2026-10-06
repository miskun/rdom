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

/// A list `.l` (resetting per `reset`) of `n` items `.i`, each showing
/// `counter(c)` in its `::before`, painted 6 wide: the rows.
fn list_rows(css: &str, n: usize) -> Vec<String> {
    paint_tree(
        &format!(".i {{ display: block }} .i::before {{ content: counter(c) \" \" }} {css}"),
        6,
        n as u16,
        |dom, root| {
            let l = el(dom, root, "div", "l");
            for k in 0..n {
                text_el(dom, l, "div", &format!("i i{k}"), "x");
            }
        },
    )
}

/// CSS Lists 3 §4.4: an element's counters are reset, then incremented,
/// then set — `counter-set` (§4.3) overrides the increment on the same
/// element without making a new counter, and the next item continues
/// from it.
#[test]
fn counter_set_sets_after_the_increment() {
    let css = ".l { counter-reset: c } .i { counter-increment: c } .i1 { counter-set: c 10 }";
    assert_eq!(list_rows(css, 3), vec!["1 x   ", "10 x  ", "11 x  "]);
}

/// §4.2: `reversed(<counter-name>)` creates a reversed counter; with no
/// integer its initial value is computed from the increments in its
/// scope (§4.2's algorithm: the negated increments, plus the last
/// non-zero one), so three items counting down end at 1.
#[test]
fn a_reversed_counter_counts_down_to_one() {
    let css = ".l { counter-reset: reversed(c) } .i { counter-increment: c -1 }";
    assert_eq!(list_rows(css, 3), vec!["3 x   ", "2 x   ", "1 x   "]);
    let css = ".l { counter-reset: reversed(c) 10 } .i { counter-increment: c -1 }";
    assert_eq!(list_rows(css, 3), vec!["9 x   ", "8 x   ", "7 x   "]);
}

/// §4.2's algorithm stops at the first element that sets the counter:
/// its set value plus the negated increments before it.
#[test]
fn a_reversed_counters_initial_value_stops_at_a_set() {
    let css = ".l { counter-reset: reversed(c) } .i { counter-increment: c -1 }
               .i1 { counter-set: c 5 }";
    assert_eq!(list_rows(css, 3), vec!["6 x   ", "5 x   ", "4 x   "]);
}

/// §4.5 ("instantiate a counter"): a counter instantiated by an element
/// replaces the same-named counter its previous sibling created, rather
/// than nesting in it — so a second list reads "1", not "N.1".
#[test]
fn a_siblings_reset_replaces_the_counter_rather_than_nesting() {
    let css = ".l { counter-reset: c; display: block } .i { display: block }
               .i::before { counter-increment: c; content: counters(c, \".\") \" \" }";
    let rows = paint_tree(css, 6, 3, |dom, root| {
        let a = el(dom, root, "div", "l");
        text_el(dom, a, "div", "i", "x");
        text_el(dom, a, "div", "i", "y");
        let b = el(dom, root, "div", "l");
        text_el(dom, b, "div", "i", "z");
    });
    assert_eq!(rows, vec!["1 x   ", "2 y   ", "1 z   "]);
}

/// §4.6: an element with `display: list-item` increments `list-item` by
/// one unless its `counter-increment` names `list-item` itself — a 0
/// there turns the implicit increment off.
#[test]
fn list_items_increment_the_list_item_counter_implicitly() {
    let css = ".l { counter-reset: list-item } .i { display: list-item }
               .i::before { content: counter(list-item) \" \" }
               .i2 { counter-increment: list-item 0 }";
    assert_eq!(list_rows(css, 3), vec!["1 x   ", "2 x   ", "2 x   "]);
}

/// The HTML list attributes, which HTML §15.3.8 maps to counter
/// properties: `<ol start>` (`counter-reset: list-item start-1`),
/// `<ol reversed>` (`reversed(list-item)`, start+1 with `start`) and
/// `<li value>` (`counter-set: list-item value`).
#[test]
fn ol_start_reversed_and_li_value_number_the_items() {
    let ol = |attrs: &[(&str, &str)], values: &[Option<&str>]| {
        let mut owned_rows = Vec::new();
        let rows = paint_tree("", 8, values.len() as u16, |dom, root| {
            let ol = el(dom, root, "ol", "");
            for (k, v) in attrs {
                dom.set_attribute(ol, k, v).unwrap();
            }
            for v in values {
                let li = text_el(dom, ol, "li", "", "x");
                if let Some(v) = v {
                    dom.set_attribute(li, "value", v).unwrap();
                }
            }
        });
        owned_rows.extend(rows.into_iter().map(|r| r.trim_end().to_string()));
        owned_rows
    };
    assert_eq!(
        ol(&[("start", "5")], &[None, None]),
        vec!["  5. x", "  6. x"]
    );
    assert_eq!(
        ol(&[("start", " -2xyz")], &[None, None]),
        vec!["  -2. x", "  -1. x"]
    );
    assert_eq!(
        ol(&[("reversed", "")], &[None, None, None]),
        vec!["  3. x", "  2. x", "  1. x"]
    );
    assert_eq!(
        ol(&[("reversed", ""), ("start", "10")], &[None, None]),
        vec!["  10. x", "  9. x"]
    );
    assert_eq!(
        ol(&[], &[None, Some("7"), None]),
        vec!["  1. x", "  7. x", "  8. x"]
    );
    assert_eq!(
        ol(&[("reversed", "")], &[None, Some("5"), None]),
        vec!["  6. x", "  5. x", "  4. x"]
    );
    assert_eq!(
        ol(&[("start", "x")], &[None]),
        vec!["  1. x"],
        "not an integer: no hint"
    );
}

/// HTML's attribute mapping is a presentational hint (CSS Cascade 4
/// §6.4.4: author origin, specificity zero, before every author rule):
/// an author `counter-reset` beats `<ol start>`.
#[test]
fn an_author_rule_beats_the_start_hint() {
    let rows = paint_tree("ol { counter-reset: list-item 100 }", 9, 1, |dom, root| {
        let ol = el(dom, root, "ol", "");
        dom.set_attribute(ol, "start", "5").unwrap();
        text_el(dom, ol, "li", "", "x");
    });
    assert_eq!(rows, vec!["  101. x "]);
}
