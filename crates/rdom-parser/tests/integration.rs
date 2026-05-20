//! Integration tests — real-world templates, property tests, edge cases.

use rdom_core::Dom;
use rdom_parser::{ParseError, parse, parse_into};

fn p(s: &str) -> (Dom<()>, Vec<rdom_core::NodeId>) {
    parse(s).unwrap()
}

// ─── Canonical round-trip corpus ─────────────────────────────────────

fn round_trip(src: &str) {
    let (dom, ids) = parse::<()>(src).unwrap();
    let out = if ids.len() == 1 {
        dom.outer_markup(ids[0])
    } else {
        dom.inner_markup(dom.root())
    };
    assert_eq!(out, src, "round-trip mismatch");
}

#[test]
fn round_trip_empty_div() {
    round_trip("<div></div>");
}

#[test]
fn round_trip_nested() {
    round_trip("<div><p><span></span></p></div>");
}

#[test]
fn round_trip_attrs_alphabetic() {
    // Attrs serialize in alphabetical order, so input must match.
    round_trip(r#"<div data-x="1" id="main"></div>"#);
}

#[test]
fn round_trip_boolean_attr() {
    round_trip("<input disabled/>");
}

#[test]
fn round_trip_void() {
    round_trip("<br/>");
    round_trip("<hr/>");
    round_trip("<img/>");
}

#[test]
fn round_trip_entity_text() {
    round_trip("<p>&amp;</p>");
    round_trip("<p>&lt;</p>");
    round_trip("<p>&gt;</p>");
    // Note: parser handles &quot;/&apos; but outer_markup only escapes
    // `& < >` in text (" and ' don't need escaping outside attributes).
    // So we don't round-trip those.
}

#[test]
fn round_trip_mixed_content() {
    round_trip("<p>before <b>mid</b> after</p>");
}

#[test]
fn round_trip_comment() {
    round_trip("<!-- note -->");
}

#[test]
fn round_trip_classes() {
    // Class order is alphabetic in outer_markup, so input order must match.
    round_trip(r#"<div class="a b c"></div>"#);
}

#[test]
fn round_trip_deep_tree() {
    round_trip("<html><body><h1>Hi</h1><p>Para <em>emph</em> rest.</p></body></html>");
}

// ─── Entity edge cases ───────────────────────────────────────────────

#[test]
fn entity_at_text_boundary() {
    let (dom, ids) = p("<p>&amp;abc</p>");
    assert_eq!(
        dom.node(ids[0]).first_child().unwrap().node_value(),
        Some("&abc")
    );
}

#[test]
fn entity_at_end_of_text() {
    let (dom, ids) = p("<p>abc&amp;</p>");
    assert_eq!(
        dom.node(ids[0]).first_child().unwrap().node_value(),
        Some("abc&")
    );
}

#[test]
fn multiple_entities_in_row() {
    let (dom, ids) = p("<p>&amp;&lt;&gt;</p>");
    assert_eq!(
        dom.node(ids[0]).first_child().unwrap().node_value(),
        Some("&<>")
    );
}

#[test]
fn bare_ampersand_preserved() {
    let (dom, ids) = p("<p>a & b</p>");
    assert_eq!(
        dom.node(ids[0]).first_child().unwrap().node_value(),
        Some("a & b")
    );
}

#[test]
fn numeric_entity_emoji() {
    // U+1F600 = 😀
    let (dom, ids) = p("<p>&#128512;</p>");
    assert_eq!(
        dom.node(ids[0]).first_child().unwrap().node_value(),
        Some("😀")
    );
}

#[test]
fn numeric_entity_hex_emoji() {
    let (dom, ids) = p("<p>&#x1F600;</p>");
    assert_eq!(
        dom.node(ids[0]).first_child().unwrap().node_value(),
        Some("😀")
    );
}

// ─── Complex attribute scenarios ─────────────────────────────────────

#[test]
fn attr_with_special_chars() {
    let (dom, ids) = p(r#"<a href="https://example.com/?q=x&amp;y=1"></a>"#);
    assert_eq!(
        dom.node(ids[0]).get_attribute("href"),
        Some("https://example.com/?q=x&y=1")
    );
}

#[test]
fn attr_empty_quoted() {
    let (dom, ids) = p(r#"<div data-x=""></div>"#);
    assert_eq!(dom.node(ids[0]).get_attribute("data-x"), Some(""));
    assert!(dom.node(ids[0]).has_attribute("data-x"));
}

#[test]
fn attr_value_with_equals() {
    let (dom, ids) = p(r#"<div data="a=b=c"></div>"#);
    assert_eq!(dom.node(ids[0]).get_attribute("data"), Some("a=b=c"));
}

#[test]
fn attr_value_with_slash() {
    let (dom, ids) = p(r#"<div path="a/b/c"></div>"#);
    assert_eq!(dom.node(ids[0]).get_attribute("path"), Some("a/b/c"));
}

#[test]
fn multiple_classes_preserved() {
    let (dom, ids) = p(r#"<div class="alpha beta gamma"></div>"#);
    for c in &["alpha", "beta", "gamma"] {
        assert!(dom.node(ids[0]).has_class(c));
    }
}

// ─── Error positions ─────────────────────────────────────────────────

#[test]
fn error_line_col_multi_line() {
    let err = parse::<()>("<div>\n  <span></p>").unwrap_err();
    assert_eq!(err.line, 2);
}

#[test]
fn error_first_line_col() {
    let err = parse::<()>("<div x=").unwrap_err();
    assert_eq!(err.line, 1);
}

#[test]
fn display_format_reads_natural() {
    let err =
        ParseError::new("expected `>`", 5, 12, 100).with_hint("missing closing angle bracket");
    let s = format!("{err}");
    assert!(s.contains("line 5"));
    assert!(s.contains("col 12"));
    assert!(s.contains("hint"));
}

// ─── Large / stress ──────────────────────────────────────────────────

#[test]
fn deep_nesting() {
    // 50 levels of nesting.
    let mut open = String::new();
    let mut close = String::new();
    for _ in 0..50 {
        open.push_str("<x>");
        close.push_str("</x>");
    }
    let src = format!("{open}{close}");
    let (dom, ids) = parse::<()>(&src).unwrap();
    // Walk to depth 50.
    let mut cur = ids[0];
    for _ in 0..49 {
        cur = dom.node(cur).first_child().unwrap().id();
    }
    assert_eq!(dom.node(cur).tag_name(), Some("x"));
}

#[test]
fn many_siblings() {
    let src: String = (0..200).map(|_| "<a></a>").collect();
    let (_dom, ids) = parse::<()>(&src).unwrap();
    assert_eq!(ids.len(), 200);
}

// ─── parse_into under existing tree ──────────────────────────────────

#[test]
fn parse_into_preserves_existing_children() {
    let mut dom: Dom<()> = Dom::new();
    let root = dom.root();
    let existing = dom.create_element("existing");
    dom.append_child(root, existing).unwrap();

    let ids = parse_into(&mut dom, "<new></new>", root).unwrap();
    assert_eq!(ids.len(), 1);
    // Root now has 2 children: existing + new.
    assert_eq!(dom.node(root).child_nodes().count(), 2);
}

#[test]
fn parse_into_returns_top_level_only() {
    let mut dom: Dom<()> = Dom::new();
    let mount = dom.create_element("body");
    let root = dom.root();
    dom.append_child(root, mount).unwrap();

    let ids = parse_into(&mut dom, "<outer><inner></inner></outer>", mount).unwrap();
    assert_eq!(ids.len(), 1, "only the <outer> is top-level");
    assert_eq!(dom.node(ids[0]).tag_name(), Some("outer"));
}

// ─── Generic Ext parameter ───────────────────────────────────────────

#[test]
fn parse_with_unit_ext() {
    let _: (Dom<()>, _) = parse("<a></a>").unwrap();
}

#[test]
fn parse_with_tui_like_ext() {
    // Emulate what a rdom-tui user would do — Default Ext type.
    #[derive(Debug, Default, Clone, PartialEq)]
    struct MyExt {
        hovered: bool,
    }

    let (_dom, _ids): (Dom<MyExt>, _) = parse("<button disabled>Click</button>").unwrap();
}

// ─── Realistic snippets ──────────────────────────────────────────────

#[test]
fn snippet_list() {
    let src = r#"<ul><li>A</li><li>B</li><li>C</li></ul>"#;
    let (dom, ids) = p(src);
    let ul = ids[0];
    assert_eq!(dom.node(ul).child_element_count(), 3);
}

#[test]
fn snippet_form() {
    let src = r#"
        <form id="login">
            <input type="text" name="user" placeholder="Username"/>
            <input type="password" name="pass"/>
            <button type="submit">Log in</button>
        </form>
    "#;
    let (dom, ids) = parse::<()>(src).unwrap();
    let form = ids
        .iter()
        .find(|&&id| dom.node(id).tag_name() == Some("form"))
        .copied()
        .unwrap();
    assert_eq!(dom.node(form).get_attribute("id"), Some("login"));
    // Count inputs + button.
    let inputs: usize = dom
        .node(form)
        .child_nodes()
        .filter(|c| c.tag_name() == Some("input"))
        .count();
    assert_eq!(inputs, 2);
}

#[test]
fn snippet_tree_item() {
    let src = r#"<tree-item expanded="true" name="Folder"><span>item-abc123</span></tree-item>"#;
    let (dom, ids) = p(src);
    let item = ids[0];
    assert_eq!(dom.node(item).tag_name(), Some("tree-item"));
    assert_eq!(dom.node(item).get_attribute("expanded"), Some("true"));
    assert_eq!(dom.node(item).get_attribute("name"), Some("Folder"));
}

// ─── Unicode content ─────────────────────────────────────────────────

#[test]
fn cjk_content_preserved() {
    let (dom, ids) = p("<p>中文</p>");
    assert_eq!(
        dom.node(ids[0]).first_child().unwrap().node_value(),
        Some("中文")
    );
}

#[test]
fn emoji_zwj_in_text() {
    let (dom, ids) = p("<p>👨‍👩‍👧</p>");
    let c = dom.node(ids[0]).first_child().unwrap();
    assert_eq!(c.node_value(), Some("👨\u{200D}👩\u{200D}👧"));
}

#[test]
fn combining_marks_preserved() {
    let (dom, ids) = p("<p>e\u{0301}</p>");
    assert_eq!(
        dom.node(ids[0]).first_child().unwrap().node_value(),
        Some("e\u{0301}")
    );
}

// ─── Adjacent text / element alternation ─────────────────────────────

#[test]
fn many_alternating_text_elements() {
    let (_dom, ids) = p("a<b>B</b>c<d>D</d>e<f>F</f>g");
    // Top-level: text 'a', element b, text 'c', element d, text 'e',
    // element f, text 'g'. 7 nodes.
    assert_eq!(ids.len(), 7);
}

// ─── Comment edge cases ──────────────────────────────────────────────

#[test]
fn empty_comment() {
    let (dom, ids) = p("<!---->");
    let c = dom.node(ids[0]);
    assert_eq!(c.data(), Some(""));
}

#[test]
fn comment_with_dashes() {
    let (dom, ids) = p("<!-- this - is - fine -->");
    let c = dom.node(ids[0]);
    assert_eq!(c.data(), Some(" this - is - fine "));
}

// ─── Text after void ─────────────────────────────────────────────────

#[test]
fn text_follows_void_element() {
    let (dom, ids) = p("<br>hello");
    assert_eq!(ids.len(), 2);
    assert_eq!(dom.node(ids[0]).tag_name(), Some("br"));
    assert_eq!(dom.node(ids[1]).node_value(), Some("hello"));
}

// ─── Type-safety assertion via rdom-tui (smoke only) ─────────────────

#[test]
fn parser_works_with_generic_ext() {
    // This ensures parse<Ext> works with any Default Ext.
    #[derive(Default, Clone)]
    struct E;
    let _: (Dom<E>, Vec<_>) = parse::<E>("<x/>").unwrap();
}
