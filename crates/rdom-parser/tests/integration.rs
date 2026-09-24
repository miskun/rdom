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

// ── HARDENING-2026-09 R10: HTML tokenizer states the parser was missing ──

fn only_text(dom: &Dom<()>, el: rdom_core::NodeId) -> String {
    let kids: Vec<_> = dom.node(el).child_nodes().collect();
    assert_eq!(kids.len(), 1, "expected exactly one child under {el:?}");
    assert_eq!(kids[0].node_type(), rdom_core::NodeType::Text);
    kids[0].node_value().unwrap().to_string()
}

/// HTML §13.2.5.6 tag-open state: `<` not followed by an ASCII letter,
/// `/`, `!`, or `?` is emitted as a text character.
#[test]
fn less_than_before_non_letter_is_text() {
    let (dom, ids) = p("<p>a < b</p>");
    assert_eq!(only_text(&dom, ids[0]), "a < b");
    let (dom, ids) = p("<p>1<2 and 3 <= 4</p>");
    assert_eq!(only_text(&dom, ids[0]), "1<2 and 3 <= 4");
    let (dom, ids) = p("<p>lonely <</p>");
    assert_eq!(only_text(&dom, ids[0]), "lonely <");
}

/// `<style>` and `<script>` contents are RAWTEXT / script data: no
/// tags, no entity decoding, until the matching end tag.
#[test]
fn style_and_script_are_raw_text() {
    let css = r#"a::before { content: "<"; } b > c { color: red } .x { content: "&amp;" }"#;
    let (dom, ids) = p(&format!("<style>{css}</style><div></div>"));
    assert_eq!(dom.node(ids[0]).tag_name(), Some("style"));
    assert_eq!(only_text(&dom, ids[0]), css);
    assert_eq!(dom.node(ids[1]).tag_name(), Some("div"));

    let js = "if (a < b && c) { d = '</p>'; }";
    let (dom, ids) = p(&format!("<script>{js}</script>"));
    assert_eq!(only_text(&dom, ids[0]), js);
}

/// The end tag inside raw text is matched case-insensitively and only
/// for the same tag name.
#[test]
fn raw_text_ends_at_the_matching_end_tag_only() {
    let (dom, ids) = p("<style>x</styles> y</STYLE><b></b>");
    assert_eq!(only_text(&dom, ids[0]), "x</styles> y");
    assert_eq!(dom.node(ids[1]).tag_name(), Some("b"));
}

/// `<textarea>` and `<title>` are RCDATA: entities decode, tags are text.
#[test]
fn textarea_and_title_are_rcdata() {
    let (dom, ids) = p("<textarea>&lt;x&gt; <b>bold</b> &amp; more</textarea>");
    assert_eq!(only_text(&dom, ids[0]), "<x> <b>bold</b> & more");
    let (dom, ids) = p("<title>a &amp; b <i>c</i></title>");
    assert_eq!(only_text(&dom, ids[0]), "a & b <i>c</i>");
}

/// Common named character references decode; unknown names and
/// references without `;` stay literal; U+0000 and surrogates become
/// U+FFFD (HTML §13.2.5.80).
#[test]
fn named_and_numeric_character_references() {
    let (dom, ids) =
        p("<p>&copy; 2026 &mdash; &hellip; &laquo;x&raquo; &times; &euro;&nbsp;&trade;</p>");
    assert_eq!(
        only_text(&dom, ids[0]),
        "\u{A9} 2026 \u{2014} \u{2026} \u{AB}x\u{BB} \u{D7} \u{20AC}\u{A0}\u{2122}"
    );
    let (dom, ids) = p("<p>&bogus; &amp &#0; &#xD800; &#x1F600;</p>");
    assert_eq!(
        only_text(&dom, ids[0]),
        "&bogus; &amp \u{FFFD} \u{FFFD} \u{1F600}"
    );
}

/// A DOCTYPE (and any `<!…>` declaration) is consumed and produces no
/// node; it used to be a parse error with a misleading hint.
#[test]
fn doctype_is_skipped() {
    let (dom, ids) = p("<!DOCTYPE html><div>x</div>");
    assert_eq!(ids.len(), 1);
    assert_eq!(dom.node(ids[0]).tag_name(), Some("div"));
    // Whitespace after the declaration is ordinary text (rdom preserves
    // inter-element whitespace; collapsing is a layout concern).
    let (dom, ids) = p("<!doctype html>\n<div>x</div>");
    assert_eq!(ids.len(), 2);
    assert_eq!(dom.node(ids[1]).tag_name(), Some("div"));
}

// ── Batch 4 review-gate follow-ups ───────────────────────────────────

/// HTML §13.2.5.6 `?` branch: `<?…>` is a bogus comment, consumed to
/// the next `>` and kept as a Comment node. It used to loop forever.
#[test]
fn processing_instruction_is_a_bogus_comment_not_a_hang() {
    let (dom, ids) = p("<?xml version='1.0'?><div></div>");
    assert_eq!(ids.len(), 2);
    assert_eq!(dom.node(ids[0]).node_type(), rdom_core::NodeType::Comment);
    assert_eq!(dom.node(ids[0]).node_value(), Some("?xml version='1.0'?"));
    assert_eq!(dom.node(ids[1]).tag_name(), Some("div"));
    let (dom, ids) = p("<p>a <?pi?> b</p>");
    let kids: Vec<_> = dom.node(ids[0]).child_nodes().collect();
    assert_eq!(kids.len(), 3);
    assert_eq!(kids[1].node_type(), rdom_core::NodeType::Comment);
}

/// Error positions stay correct after a multi-line raw-text body.
#[test]
fn error_line_is_tracked_through_raw_text_bodies() {
    let err = parse::<()>("<style>\n\n\n</style><p>").unwrap_err();
    assert_eq!(err.line, 4, "{err}");
}

/// Serializing a `<style>` / `<script>` must not escape its text, or the
/// round trip corrupts the CSS (`&gt;` is not a combinator); RCDATA
/// parents keep escaping so `<textarea>` text round-trips too.
#[test]
fn raw_text_and_rcdata_round_trip() {
    for src in [
        r#"<style>a > b { content: "<"; } c { x: "&amp;" }</style>"#,
        "<script>if (a < b && c) {}</script>",
        "<textarea>&lt;b&gt; &amp; x</textarea>",
        "<title>a &amp; b</title>",
    ] {
        let (dom, ids) = p(src);
        let out = dom.outer_markup(ids[0]);
        let (dom2, ids2) = p(&out);
        assert_eq!(
            dom.node(ids[0])
                .child_nodes()
                .next()
                .and_then(|t| t.node_value().map(str::to_string)),
            dom2.node(ids2[0])
                .child_nodes()
                .next()
                .and_then(|t| t.node_value().map(str::to_string)),
            "{src} → {out}"
        );
    }
    let (dom, ids) = p("<style>a > b {}</style>");
    assert_eq!(dom.outer_markup(ids[0]), "<style>a > b {}</style>");
}

/// A numeric reference that overflows `u32` is still "a number above
/// U+10FFFF" and decodes to U+FFFD (§13.2.5.80), not the literal `&`.
#[test]
fn overflowing_numeric_reference_is_replacement_character() {
    let (dom, ids) = p("<p>&#99999999999; &#x110000;</p>");
    assert_eq!(only_text(&dom, ids[0]), "\u{FFFD} \u{FFFD}");
}

/// The two reference scanners (text and RCDATA) agree: a `+` sign is not
/// part of a numeric reference in either.
#[test]
fn reference_scanners_agree_on_signs() {
    let (dom, ids) = p("<p>&#+65;</p>");
    assert_eq!(only_text(&dom, ids[0]), "&#+65;");
    let (dom, ids) = p("<textarea>&#+65;</textarea>");
    assert_eq!(only_text(&dom, ids[0]), "&#+65;");
}

/// A strict template parser does not silently truncate: a stray end tag
/// at the top level is an error, not the end of the input.
#[test]
fn stray_top_level_end_tag_is_an_error() {
    let err = parse::<()>("<p>x</p></b>garbage").unwrap_err();
    let text = format!("{err}");
    assert!(text.contains("closing tag"), "{text}");
}

/// HTML §13.2.6.4.7: a newline immediately after `<textarea>` is dropped.
#[test]
fn textarea_drops_a_leading_newline() {
    let (dom, ids) = p("<textarea>\nfoo\nbar</textarea>");
    assert_eq!(only_text(&dom, ids[0]), "foo\nbar");
    let (dom, ids) = p("<textarea>\n</textarea>");
    assert_eq!(dom.node(ids[0]).child_nodes().count(), 0);
}
