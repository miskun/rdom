//! The example's footer (`--example acid`): a row of its own below the
//! 120 × 50 page — the page number and the pager's keys — and, when the
//! terminal cannot show the page and the row, a note saying so
//! (C16G-ACID-PAGER). It lives under the document's root, beside the
//! page, so no tile, label or gutter changes; the tests that compare
//! pages build none.

use rdom_tui::{NodeId, Stylesheet, TuiDom};

use super::{PAGE_HEIGHT, PAGE_WIDTH, page_count};

/// The footer's rules: fixed to the viewport's last row, the long keys
/// line when the terminal holds the page and the footer, the short one
/// with the note when it does not.
pub fn pager_stylesheet() -> Stylesheet {
    let css = format!(
        ".acid-pager {{ position: fixed; left: 0; right: 0; bottom: 0; height: 1;
           padding: 0 1; white-space: nowrap; overflow: clip; color: GrayText }}
         .acid-pager-small {{ display: none }}
         @media (width < {w}) or (height < {h}) {{
           .acid-pager-keys {{ display: none }}
           .acid-pager-small {{ display: inline }}
         }}",
        w = PAGE_WIDTH,
        h = PAGE_HEIGHT + 1,
    );
    rdom_css::parse(&css).stylesheet
}

/// Append the footer for `page` to the document's root.
pub fn add_pager_footer(dom: &mut TuiDom, page: u8) {
    let root = dom.root();
    let footer = dom.create_element("div");
    dom.set_attribute(footer, "class", "acid-pager").unwrap();
    dom.append_child(root, footer).unwrap();
    let span = |dom: &mut TuiDom, class: &str, text: &str| {
        let s = dom.create_element("span");
        dom.set_attribute(s, "class", class).unwrap();
        let t = dom.create_text_node(text);
        dom.append_child(s, t).unwrap();
        dom.append_child(footer, s).unwrap();
    };
    span(dom, "acid-pager-page", &page_label(page));
    span(
        dom,
        "acid-pager-keys",
        " · Ctrl+N next · Ctrl+P previous · Ctrl+C quit",
    );
    span(
        dom,
        "acid-pager-small",
        &format!(
            " · Ctrl+N / Ctrl+P · too small: the pages are {PAGE_WIDTH} × {PAGE_HEIGHT}, plus this row"
        ),
    );
}

/// Show `page` in the footer, when there is one.
pub(super) fn show_page(dom: &mut TuiDom, page: u8) {
    let root = dom.root();
    let label: Option<NodeId> = dom
        .query_selector_in(root, ".acid-pager-page")
        .expect("a valid selector");
    if let Some(label) = label {
        dom.set_text_content(label, &page_label(page)).unwrap();
    }
}

fn page_label(page: u8) -> String {
    format!("acid page {page}/{}", page_count())
}
