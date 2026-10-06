//! C10-LEGACY-COLON — Selectors 4 §15: the CSS 2.1 spellings `:before`,
//! `:after`, `:first-line` and `:first-letter` are pseudo-elements.

use super::{paint_tree, text_el};

/// Selectors 4 §15: `p:before` / `p:after` generate content exactly as
/// `p::before` / `p::after` do.
#[test]
fn single_colon_before_and_after_generate_content() {
    let rows = paint_tree(
        r#".p:before { content: "<" } .p:after { content: ">" }"#,
        8,
        1,
        |dom, root| {
            text_el(dom, root, "div", "p", "ab");
        },
    );
    assert_eq!(rows, vec!["<ab>    "]);
}

/// Selectors 4 §15 maps `:first-line` / `:first-letter` to the
/// pseudo-elements; the sheet parses strictly (no warning). Until
/// C10-FIRST lays them out, they style nothing — the text paints as if
/// the rules were absent (DIVERGENCES §3).
#[test]
fn single_colon_first_line_and_first_letter_parse() {
    let rows = paint_tree(
        r#".p:first-line { color: red } .p:first-letter { color: blue }
           .p::first-line { color: red } .p::first-letter { color: blue }"#,
        4,
        1,
        |dom, root| {
            text_el(dom, root, "div", "p", "ab");
        },
    );
    assert_eq!(rows, vec!["ab  "]);
}
