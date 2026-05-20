//! `cssom::extend_from_style_tags` — `<style>` block extraction.
//! Walks a populated `TuiDom` for `<style>` elements, concatenates
//! each one's text-node children, parses the result via
//! `rdom_css::parse`, and merges the parsed rules + custom
//! properties into a `Stylesheet`. Free function the app calls
//! before `App::new`.

use rdom_style::Value;
use rdom_tui::{Color, Stylesheet, TuiColor, TuiDom, extend_from_style_tags};

fn build_dom_with_style(css: &str) -> TuiDom {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let style_el = dom.create_element("style");
    let text = dom.create_text_node(css);
    dom.append_child(style_el, text).unwrap();
    dom.append_child(root, style_el).unwrap();
    dom
}

#[test]
fn extracts_single_style_tag_rules() {
    let dom = build_dom_with_style("button { color: red; }");
    let mut sheet = Stylesheet::bare();
    let warnings = extend_from_style_tags(&dom, &mut sheet);
    assert!(warnings.is_empty(), "warnings: {warnings:?}");
    assert_eq!(sheet.rules().len(), 1);
    assert_eq!(sheet.rules()[0].source_text, "button");
    assert_eq!(
        sheet.rules()[0].style.fg,
        Some(Value::Specified(TuiColor::Literal(Color::Rgb(255, 0, 0))))
    );
}

#[test]
fn extracts_root_custom_properties_into_var_map() {
    let dom = build_dom_with_style(":root { --accent: #3d90ce; }");
    let mut sheet = Stylesheet::bare();
    let warnings = extend_from_style_tags(&dom, &mut sheet);
    assert!(warnings.is_empty());
    assert_eq!(sheet.var("accent"), Some("#3d90ce"));
}

#[test]
fn merges_with_existing_rules() {
    let dom = build_dom_with_style("p { color: blue; }");
    let mut sheet = Stylesheet::bare()
        .rule("h1", rdom_tui::TuiStyle::new())
        .unwrap();
    extend_from_style_tags(&dom, &mut sheet);
    let texts: Vec<_> = sheet
        .rules()
        .iter()
        .map(|r| r.source_text.as_str())
        .collect();
    // Existing rule preserved; parsed rule appended.
    assert_eq!(texts, vec!["h1", "p"]);
}

#[test]
fn multiple_style_tags_merge_in_document_order() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();

    let s1 = dom.create_element("style");
    let t1 = dom.create_text_node("a { color: red; }");
    dom.append_child(s1, t1).unwrap();
    dom.append_child(root, s1).unwrap();

    let s2 = dom.create_element("style");
    let t2 = dom.create_text_node("b { color: blue; }");
    dom.append_child(s2, t2).unwrap();
    dom.append_child(root, s2).unwrap();

    let mut sheet = Stylesheet::bare();
    extend_from_style_tags(&dom, &mut sheet);
    let texts: Vec<_> = sheet
        .rules()
        .iter()
        .map(|r| r.source_text.as_str())
        .collect();
    assert_eq!(texts, vec!["a", "b"]);
}

#[test]
fn no_style_tags_is_a_noop() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let mut sheet = Stylesheet::bare();
    let warnings = extend_from_style_tags(&dom, &mut sheet);
    assert!(warnings.is_empty());
    assert_eq!(sheet.rules().len(), 0);
}

#[test]
fn lenient_warnings_propagate_from_inner_parse() {
    let dom = build_dom_with_style("a { unknown-prop: 5; }");
    let mut sheet = Stylesheet::bare();
    let warnings = extend_from_style_tags(&dom, &mut sheet);
    // Rule still installed (lenient); warning is bubbled up.
    assert_eq!(sheet.rules().len(), 1);
    assert_eq!(warnings.len(), 1);
}
