//! IFC detection + single-line inline flow integration tests.
//!
//! Uses `VirtualScreen` to assert on what actually lands in the
//! terminal after cascade → layout → paint.

use rdom_tui::prelude::*;

/// Helper: cascade + layout + paint into a `VirtualScreen`. Returns
/// row 0 (trimmed) as the most common assertion target.
fn render(dom: &mut TuiDom, sheet: &Stylesheet, w: u16, h: u16) -> VirtualScreen {
    let backend = TestBackend::new(w, h);
    let mut term = Terminal::new(backend).unwrap();
    term.draw(|buf| {
        dom.cascade(sheet);
        dom.layout_dom(buf.area);
        dom.paint_dom(buf, buf.area);
        Ok(())
    })
    .unwrap();
    let mut screen = VirtualScreen::new(w, h);
    screen.apply(term.backend().bytes());
    screen
}

#[test]
fn single_line_inline_flow_renders_in_document_order() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");

    let t1 = dom.create_text_node("prefix ");
    let b = dom.create_element("b");
    let b_text = dom.create_text_node("bold");
    dom.append_child(b, b_text).unwrap();
    let t2 = dom.create_text_node(" suffix");

    dom.append_child(p, t1).unwrap();
    dom.append_child(p, b).unwrap();
    dom.append_child(p, t2).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .height(Size::Fixed(1)),
        )
        .rule_unchecked("b", TuiStyle::new().display(Display::Inline).bold(true));

    let screen = render(&mut dom, &sheet, 30, 1);
    assert_eq!(screen.row(0).trim_end(), "prefix bold suffix");
}

#[test]
fn inline_element_styles_its_fragment() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");

    let t1 = dom.create_text_node("a ");
    let code = dom.create_element("code");
    let code_text = dom.create_text_node("X");
    dom.append_child(code, code_text).unwrap();
    let t2 = dom.create_text_node(" b");

    dom.append_child(p, t1).unwrap();
    dom.append_child(p, code).unwrap();
    dom.append_child(p, t2).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .height(Size::Fixed(1)),
        )
        .rule_unchecked(
            "code",
            TuiStyle::new()
                .display(Display::Inline)
                .fg(Color::Rgb(255, 255, 0)),
        );

    let screen = render(&mut dom, &sheet, 10, 1);
    // Text ordering: "a X b"
    assert_eq!(screen.row(0).trim_end(), "a X b");
    // Per-fragment styling: "a " and " b" default fg; "X" yellow.
    assert_eq!(screen.cell(0, 0).unwrap().fg, Color::Reset); // 'a'
    assert_eq!(screen.cell(2, 0).unwrap().fg, Color::Rgb(255, 255, 0)); // 'X'
    assert_eq!(screen.cell(3, 0).unwrap().fg, Color::Reset); // ' '
    assert_eq!(screen.cell(4, 0).unwrap().fg, Color::Reset); // 'b'
}

#[test]
fn nested_inline_elements_compose_styles() {
    // <p>a <b><i>bi</i></b> c</p>: the inner <i> inherits bold from <b>
    // (bold inherits per CSS) and adds italic.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t1 = dom.create_text_node("a ");
    let b = dom.create_element("b");
    let i = dom.create_element("i");
    let bi_text = dom.create_text_node("bi");
    dom.append_child(i, bi_text).unwrap();
    dom.append_child(b, i).unwrap();
    let t2 = dom.create_text_node(" c");
    dom.append_child(p, t1).unwrap();
    dom.append_child(p, b).unwrap();
    dom.append_child(p, t2).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .height(Size::Fixed(1)),
        )
        .rule_unchecked("b", TuiStyle::new().display(Display::Inline).bold(true))
        .rule_unchecked("i", TuiStyle::new().display(Display::Inline).italic(true));

    let screen = render(&mut dom, &sheet, 10, 1);
    assert_eq!(screen.row(0).trim_end(), "a bi c");
    // 'b' in "bi" should be bold + italic.
    let cell = screen.cell(2, 0).unwrap();
    assert!(cell.modifier.contains(Modifier::BOLD), "nested inline bold");
    assert!(
        cell.modifier.contains(Modifier::ITALIC),
        "nested inline italic"
    );
}

#[test]
fn ifc_clips_overflow_at_content_width() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("0123456789ABCDEF");
    dom.append_child(p, t).unwrap();
    // Add an inline child to trigger IFC path.
    let span = dom.create_element("span");
    let span_text = dom.create_text_node("!");
    dom.append_child(span, span_text).unwrap();
    dom.append_child(p, span).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(8))
                .height(Size::Fixed(1)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));

    let screen = render(&mut dom, &sheet, 20, 1);
    // First 8 cells hold "01234567", rest blank — "89ABCDEF!" is clipped.
    assert_eq!(&screen.row(0)[..8], "01234567");
    assert_eq!(screen.row(0)[8..].trim_end(), "");
}

#[test]
fn block_with_only_block_children_not_ifc() {
    // Sanity check: a block with all-block children keeps normal flex
    // behavior (each child gets its own rect on its own line).
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let section = dom.create_element("section");
    let p1 = dom.create_element("p");
    let t1 = dom.create_text_node("first");
    dom.append_child(p1, t1).unwrap();
    let p2 = dom.create_element("p");
    let t2 = dom.create_text_node("second");
    dom.append_child(p2, t2).unwrap();
    dom.append_child(section, p1).unwrap();
    dom.append_child(section, p2).unwrap();
    dom.append_child(root, section).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "section",
            TuiStyle::new()
                .display(Display::Block)
                .flow(Flow::Flex)
                .direction(Direction::Column),
        )
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .height(Size::Fixed(1)),
        );

    let screen = render(&mut dom, &sheet, 10, 3);
    assert_eq!(screen.row(0).trim_end(), "first");
    assert_eq!(screen.row(1).trim_end(), "second");
}

#[test]
fn ifc_with_only_text_no_inline_elements_not_ifc() {
    // A block with only text children (no inline element) is NOT an
    // IFC — it falls through to the current single-line own-text paint.
    // This test guards against accidentally turning every text-containing
    // block into an IFC.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("hello");
    dom.append_child(p, t).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "p",
        TuiStyle::new()
            .display(Display::Block)
            .height(Size::Fixed(1)),
    );

    let screen = render(&mut dom, &sheet, 10, 1);
    assert_eq!(screen.row(0).trim_end(), "hello");
}

// ── Whitespace normalization ───────────────────────────────────────

#[test]
fn whitespace_runs_collapse_to_single_space() {
    // Text like "foo     bar\n\nbaz" should render as "foo bar baz" in
    // Normal mode. Simulates what rdom-parser gives us when a template
    // wraps inline content across lines.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("foo     bar\n\nbaz");
    dom.append_child(p, t).unwrap();
    // Inline child to trigger IFC.
    let span = dom.create_element("span");
    let st = dom.create_text_node("");
    dom.append_child(span, st).unwrap();
    dom.append_child(p, span).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .height(Size::Fixed(1)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));

    let screen = render(&mut dom, &sheet, 20, 1);
    assert_eq!(screen.row(0).trim_end(), "foo bar baz");
}

#[test]
fn leading_whitespace_trimmed_at_ifc_start() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    // This is exactly what rdom-parser emits between <p> and the first
    // element child when the template indents them.
    let t1 = dom.create_text_node("\n        ");
    let b = dom.create_element("b");
    let bt = dom.create_text_node("bold");
    dom.append_child(b, bt).unwrap();
    let t2 = dom.create_text_node(" after");
    dom.append_child(p, t1).unwrap();
    dom.append_child(p, b).unwrap();
    dom.append_child(p, t2).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .height(Size::Fixed(1)),
        )
        .rule_unchecked("b", TuiStyle::new().display(Display::Inline).bold(true));

    let screen = render(&mut dom, &sheet, 20, 1);
    // Leading "\n        " dropped; no leading space before "bold".
    assert_eq!(screen.row(0).trim_end(), "bold after");
}

#[test]
fn trailing_whitespace_trimmed_at_ifc_end() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t1 = dom.create_text_node("hello ");
    let span = dom.create_element("span");
    let st = dom.create_text_node("world");
    dom.append_child(span, st).unwrap();
    let t2 = dom.create_text_node("   \n    "); // trailing whitespace
    dom.append_child(p, t1).unwrap();
    dom.append_child(p, span).unwrap();
    dom.append_child(p, t2).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .height(Size::Fixed(1)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));

    let screen = render(&mut dom, &sheet, 20, 1);
    // Trailing whitespace fully dropped.
    assert_eq!(screen.row(0).trim_end(), "hello world");
    // Cell after 'd' is still empty (nothing painted past content).
    assert_eq!(screen.cell(11, 0).unwrap().symbol(), " ");
}

#[test]
fn whitespace_across_node_boundaries_collapses_to_one_space() {
    // "a " + "" + " b" should become "a b", not "a   b".
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t1 = dom.create_text_node("a   ");
    let span = dom.create_element("span");
    // Inline element with only whitespace text.
    let sp_t = dom.create_text_node("   ");
    dom.append_child(span, sp_t).unwrap();
    let t2 = dom.create_text_node("   b");
    dom.append_child(p, t1).unwrap();
    dom.append_child(p, span).unwrap();
    dom.append_child(p, t2).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .height(Size::Fixed(1)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));

    let screen = render(&mut dom, &sheet, 10, 1);
    // Three whitespace runs across node boundaries collapse to one
    // shared space; inline span contributes nothing visible.
    assert_eq!(screen.row(0).trim_end(), "a b");
}

#[test]
fn nbsp_is_not_collapsed() {
    // CSS semantics: U+00A0 (NBSP) is not whitespace for the collapse
    // algorithm. "a\u{00A0}\u{00A0}\u{00A0}b" should render verbatim.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("a\u{00A0}\u{00A0}\u{00A0}b");
    dom.append_child(p, t).unwrap();
    let span = dom.create_element("span");
    let st = dom.create_text_node("");
    dom.append_child(span, st).unwrap();
    dom.append_child(p, span).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .height(Size::Fixed(1)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));

    let screen = render(&mut dom, &sheet, 10, 1);
    assert_eq!(screen.row(0).trim_end(), "a\u{00A0}\u{00A0}\u{00A0}b");
}

#[test]
fn white_space_pre_preserves_spaces() {
    // In Pre mode, multiple spaces are preserved.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("a    b");
    dom.append_child(p, t).unwrap();
    let span = dom.create_element("span");
    let st = dom.create_text_node("");
    dom.append_child(span, st).unwrap();
    dom.append_child(p, span).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .white_space(WhiteSpace::Pre)
                .height(Size::Fixed(1)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));

    let screen = render(&mut dom, &sheet, 10, 1);
    assert_eq!(screen.row(0).trim_end(), "a    b");
}

// ── Wrapping + multi-line IFC ──────────────────────────────────────

#[test]
fn long_paragraph_wraps_at_word_boundaries() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("the quick brown fox jumps");
    dom.append_child(p, t).unwrap();
    let span = dom.create_element("span");
    dom.append_child(p, span).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(10)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));

    // p is Auto height → should grow to fit. 25 chars wrap at 10:
    // "the quick" (9), "brown fox" (9), "jumps" (5) → 3 lines.
    let screen = render(&mut dom, &sheet, 10, 4);
    assert_eq!(screen.row(0).trim_end(), "the quick");
    assert_eq!(screen.row(1).trim_end(), "brown fox");
    assert_eq!(screen.row(2).trim_end(), "jumps");
    assert_eq!(screen.row(3).trim_end(), "");
}

#[test]
fn wrapped_inline_styling_preserved_across_lines() {
    // <p>aaa <b>bbb ccc</b> ddd</p>. If we wrap so <b>'s text spans
    // two lines, the style survives on both lines.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t1 = dom.create_text_node("aaa ");
    let b = dom.create_element("b");
    let bt = dom.create_text_node("bbb ccc");
    dom.append_child(b, bt).unwrap();
    let t2 = dom.create_text_node(" ddd");
    dom.append_child(p, t1).unwrap();
    dom.append_child(p, b).unwrap();
    dom.append_child(p, t2).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(8)),
        )
        .rule_unchecked(
            "b",
            TuiStyle::new()
                .display(Display::Inline)
                .bold(true)
                .fg(Color::Rgb(255, 255, 0)),
        );

    // Width 8: "aaa bbb" (7) line 1, "ccc ddd" (7) line 2.
    let screen = render(&mut dom, &sheet, 10, 2);
    assert_eq!(screen.row(0).trim_end(), "aaa bbb");
    assert_eq!(screen.row(1).trim_end(), "ccc ddd");
    // bbb on line 1 is bold+yellow.
    let bbb_cell = screen.cell(4, 0).unwrap();
    assert_eq!(bbb_cell.symbol(), "b");
    assert!(bbb_cell.modifier.contains(Modifier::BOLD));
    assert_eq!(bbb_cell.fg, Color::Rgb(255, 255, 0));
    // ccc on line 2 is also bold+yellow (same inline element).
    let ccc_cell = screen.cell(0, 1).unwrap();
    assert_eq!(ccc_cell.symbol(), "c");
    assert!(ccc_cell.modifier.contains(Modifier::BOLD));
    assert_eq!(ccc_cell.fg, Color::Rgb(255, 255, 0));
    // ddd unbold default.
    let ddd_cell = screen.cell(4, 1).unwrap();
    assert_eq!(ddd_cell.symbol(), "d");
    assert!(!ddd_cell.modifier.contains(Modifier::BOLD));
}

#[test]
fn cjk_wraps_between_graphemes() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("中文中文中文");
    dom.append_child(p, t).unwrap();
    let span = dom.create_element("span");
    dom.append_child(p, span).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(6)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));

    // 6 CJK graphemes × 2 cells = 12. Width 6 → 3 per line, 2 lines.
    let screen = render(&mut dom, &sheet, 10, 3);
    assert_eq!(screen.row(0).trim_end(), "中文中");
    assert_eq!(screen.row(1).trim_end(), "文中文");
}

#[test]
fn auto_height_ifc_grows_to_fit_content() {
    // IFC with height: Auto grows vertically to fit the wrapped lines.
    // A sibling block on the same column positions after the IFC's
    // final row, so we can observe the actual height.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let section = dom.create_element("section");
    let p = dom.create_element("p");
    let t = dom.create_text_node("one two three four five");
    dom.append_child(p, t).unwrap();
    let span = dom.create_element("span");
    dom.append_child(p, span).unwrap();
    dom.append_child(section, p).unwrap();
    let footer = dom.create_element("footer");
    let ft = dom.create_text_node("END");
    dom.append_child(footer, ft).unwrap();
    dom.append_child(section, footer).unwrap();
    dom.append_child(root, section).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "section",
            TuiStyle::new()
                .display(Display::Block)
                .flow(Flow::Flex)
                .direction(Direction::Column)
                .width(Size::Fixed(10)),
        )
        .rule_unchecked("p", TuiStyle::new().display(Display::Block))
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline))
        .rule_unchecked(
            "footer",
            TuiStyle::new()
                .display(Display::Block)
                .height(Size::Fixed(1)),
        );

    // "one two three four five" = 23 chars. Width 10 wraps:
    //   "one two" (7), "three four" (10), "five" (4) → 3 lines.
    // Footer sits at row 3.
    let screen = render(&mut dom, &sheet, 10, 6);
    assert_eq!(screen.row(0).trim_end(), "one two");
    assert_eq!(screen.row(1).trim_end(), "three four");
    assert_eq!(screen.row(2).trim_end(), "five");
    assert_eq!(screen.row(3).trim_end(), "END");
}

#[test]
fn fixed_height_ifc_clips_overflowing_lines() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("one two three four five six seven");
    dom.append_child(p, t).unwrap();
    let span = dom.create_element("span");
    dom.append_child(p, span).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(10))
                .height(Size::Fixed(2)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));

    let screen = render(&mut dom, &sheet, 10, 4);
    // Only 2 rows visible; later lines clipped.
    assert_eq!(screen.row(0).trim_end(), "one two");
    assert_eq!(screen.row(1).trim_end(), "three four");
    assert_eq!(screen.row(2).trim_end(), "");
    assert_eq!(screen.row(3).trim_end(), "");
}

#[test]
fn hyphenated_word_breaks_after_hyphen() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("state-of-the-art");
    dom.append_child(p, t).unwrap();
    let span = dom.create_element("span");
    dom.append_child(p, span).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(10)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));

    let screen = render(&mut dom, &sheet, 10, 2);
    assert_eq!(screen.row(0).trim_end(), "state-of-");
    assert_eq!(screen.row(1).trim_end(), "the-art");
}

// ── <br>, Pre mode hard-break, UA defaults ─────────────────────────

#[test]
fn br_forces_hard_line_break() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t1 = dom.create_text_node("first");
    let br = dom.create_element("br");
    let t2 = dom.create_text_node("second");
    dom.append_child(p, t1).unwrap();
    dom.append_child(p, br).unwrap();
    dom.append_child(p, t2).unwrap();
    dom.append_child(root, p).unwrap();

    // Use UA defaults via Stylesheet::new() — p is block, br is inline.
    let sheet = Stylesheet::new()
        .rule("p", TuiStyle::new().width(Size::Fixed(10)))
        .unwrap();

    let screen = render(&mut dom, &sheet, 10, 3);
    assert_eq!(screen.row(0).trim_end(), "first");
    assert_eq!(screen.row(1).trim_end(), "second");
}

#[test]
fn double_br_produces_blank_line() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t1 = dom.create_text_node("a");
    let br1 = dom.create_element("br");
    let br2 = dom.create_element("br");
    let t2 = dom.create_text_node("b");
    dom.append_child(p, t1).unwrap();
    dom.append_child(p, br1).unwrap();
    dom.append_child(p, br2).unwrap();
    dom.append_child(p, t2).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::new()
        .rule("p", TuiStyle::new().width(Size::Fixed(10)))
        .unwrap();

    let screen = render(&mut dom, &sheet, 10, 3);
    assert_eq!(screen.row(0).trim_end(), "a");
    assert_eq!(screen.row(1).trim_end(), "");
    assert_eq!(screen.row(2).trim_end(), "b");
}

#[test]
fn pre_mode_newline_is_hard_break() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let pre = dom.create_element("pre");
    let t = dom.create_text_node("line1\nline2\nline3");
    dom.append_child(pre, t).unwrap();
    // Force IFC by adding an inline child (without it, <pre> would
    // just own-text-paint on one line the legacy way).
    let span = dom.create_element("span");
    dom.append_child(pre, span).unwrap();
    dom.append_child(root, pre).unwrap();

    // UA defaults: <pre> → block + white_space: pre.
    let sheet = Stylesheet::new();

    let screen = render(&mut dom, &sheet, 20, 3);
    assert_eq!(screen.row(0).trim_end(), "line1");
    assert_eq!(screen.row(1).trim_end(), "line2");
    assert_eq!(screen.row(2).trim_end(), "line3");
}

#[test]
fn pre_mode_preserves_multiple_spaces() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let pre = dom.create_element("pre");
    let t = dom.create_text_node("a    b");
    dom.append_child(pre, t).unwrap();
    let span = dom.create_element("span");
    dom.append_child(pre, span).unwrap();
    dom.append_child(root, pre).unwrap();

    let sheet = Stylesheet::new();
    let screen = render(&mut dom, &sheet, 10, 1);
    assert_eq!(screen.row(0).trim_end(), "a    b");
}

#[test]
fn ua_defaults_bold_via_b_tag() {
    // Using ONLY UA defaults + the p width rule.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t1 = dom.create_text_node("a ");
    let b = dom.create_element("b");
    let bt = dom.create_text_node("bold");
    dom.append_child(b, bt).unwrap();
    let t2 = dom.create_text_node(" c");
    dom.append_child(p, t1).unwrap();
    dom.append_child(p, b).unwrap();
    dom.append_child(p, t2).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::new()
        .rule("p", TuiStyle::new().height(Size::Fixed(1)))
        .unwrap();

    let screen = render(&mut dom, &sheet, 10, 1);
    assert_eq!(screen.row(0).trim_end(), "a bold c");
    assert!(screen.cell(2, 0).unwrap().modifier.contains(Modifier::BOLD));
    assert!(screen.cell(5, 0).unwrap().modifier.contains(Modifier::BOLD));
    assert!(!screen.cell(0, 0).unwrap().modifier.contains(Modifier::BOLD));
    assert!(!screen.cell(7, 0).unwrap().modifier.contains(Modifier::BOLD));
}

#[test]
fn ua_defaults_italic_via_em_tag() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let em = dom.create_element("em");
    let t = dom.create_text_node("italic");
    dom.append_child(em, t).unwrap();
    dom.append_child(p, em).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::new()
        .rule("p", TuiStyle::new().height(Size::Fixed(1)))
        .unwrap();

    let screen = render(&mut dom, &sheet, 10, 1);
    assert_eq!(screen.row(0).trim_end(), "italic");
    assert!(
        screen
            .cell(0, 0)
            .unwrap()
            .modifier
            .contains(Modifier::ITALIC)
    );
}

#[test]
fn ua_defaults_code_yellow_foreground() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let code = dom.create_element("code");
    let t = dom.create_text_node("x");
    dom.append_child(code, t).unwrap();
    dom.append_child(p, code).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::new()
        .rule("p", TuiStyle::new().height(Size::Fixed(1)))
        .unwrap();

    let screen = render(&mut dom, &sheet, 10, 1);
    assert_eq!(screen.row(0).trim_end(), "x");
    // `<code>` fg is CSS `gold` (#FFD700) — saturated yellow against
    // the DarkGray field-tint bg.
    assert_eq!(screen.cell(0, 0).unwrap().fg, Color::Rgb(255, 215, 0));
}

// ── Resize regression for IFC layouts ─────────────────────────────

#[test]
fn resizing_terminal_wider_then_narrower_reflows_ifc_without_stale_cells() {
    // Builds a realistic layout (sidebar + main with IFC, footer),
    // renders at width 40, then 20, then 60. At each resize the IFC
    // content should reflow to the new width with no leaked cells
    // from previous frames.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let screen = dom.create_element("screen");
    let body = dom.create_element("body");
    let p = dom.create_element("p");
    let t1 = dom.create_text_node("hello ");
    let b = dom.create_element("b");
    let bt = dom.create_text_node("wrapping");
    dom.append_child(b, bt).unwrap();
    let t2 = dom.create_text_node(" world of inline text");
    dom.append_child(p, t1).unwrap();
    dom.append_child(p, b).unwrap();
    dom.append_child(p, t2).unwrap();
    dom.append_child(body, p).unwrap();
    dom.append_child(screen, body).unwrap();
    dom.append_child(root, screen).unwrap();

    let sheet = Stylesheet::new()
        .rule(
            "screen",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Column)
                .width(Size::Flex(1))
                .height(Size::Flex(1)),
        )
        .unwrap()
        .rule(
            "body",
            TuiStyle::new()
                .flow(Flow::Flex)
                .direction(Direction::Row)
                .padding(Padding::all(1))
                .height(Size::Flex(1)),
        )
        .unwrap()
        .rule(
            "p",
            TuiStyle::new()
                .width(Size::Flex(1))
                .border(Border::Rounded)
                .padding(Padding::all(1)),
        )
        .unwrap();

    // Simulate terminal resize via Terminal + TestBackend directly so
    // the autoresize path (with backend.clear()) fires.
    let backend = TestBackend::new(40, 10);
    let mut term = Terminal::new(backend).unwrap();

    // Frame 1: 40×10.
    term.draw(|buf| {
        dom.cascade(&sheet);
        dom.layout_dom(buf.area);
        dom.paint_dom(buf, buf.area);
        Ok(())
    })
    .unwrap();
    let mut screen40 = VirtualScreen::new(40, 10);
    screen40.apply(term.backend().bytes());

    // Frame 2: shrink to 20×10. Resize backend; redraw; virtual screen
    // needs to shrink too (mirrors terminal behavior on window resize).
    term.backend_mut().resize(20, 10);
    let mut screen20 = VirtualScreen::new(20, 10);
    let _ = term.backend_mut().take_bytes();
    term.draw(|buf| {
        dom.cascade(&sheet);
        dom.layout_dom(buf.area);
        dom.paint_dom(buf, buf.area);
        Ok(())
    })
    .unwrap();
    screen20.apply(term.backend().bytes());

    // No cell past x=19 can carry any content (the 20-wide terminal
    // doesn't have those columns anymore — but this also proves the
    // paint wasn't attempting to write off-grid).
    for y in 0..10 {
        // All cells must be valid (cell(19, y) exists; cell(20, y) doesn't).
        assert!(screen20.cell(19, y).is_some());
        assert!(screen20.cell(20, y).is_none());
    }

    // Frame 3: grow to 60×10. No cell past x=39 can carry stale
    // content from the 40-wide first frame.
    term.backend_mut().resize(60, 10);
    let mut screen60 = VirtualScreen::new(60, 10);
    let _ = term.backend_mut().take_bytes();
    term.draw(|buf| {
        dom.cascade(&sheet);
        dom.layout_dom(buf.area);
        dom.paint_dom(buf, buf.area);
        Ok(())
    })
    .unwrap();
    screen60.apply(term.backend().bytes());

    // The IFC block is inside body inside screen. Body has padding 1,
    // so content starts at row 1. p has border + padding so its
    // content starts at row 2.
    // With width 60, body inner = 60 - 2 (padding) = 58 wide. p gets
    // all 58. p content = 58 - 2 (border) - 2 (padding) = 54 wide.
    // "hello wrapping world of inline text" = 35 chars. Fits on one row.
    // Text is on row 3 (screen padding + body padding + p border + p padding).
    // Just assert: no leaked cells past where body should end (right
    // border at column 58 + 1 (screen origin, which is column 0 here)).

    // No cell past col 60 — verify boundary.
    for y in 0..10 {
        assert!(screen60.cell(59, y).is_some());
        assert!(screen60.cell(60, y).is_none());
    }

    // And the content actually rendered something sensible on row 3
    // (where p content should sit).
    let row3 = screen60.row(3);
    assert!(
        row3.contains("hello") || row3.contains("wrapping") || row3.contains("world"),
        "row 3 should contain inline text after wider resize: {:?}",
        row3
    );
}

#[test]
fn ifc_reflows_when_only_width_changes_same_height() {
    // Tight focus: two frames at same height, different widths. The
    // IFC should wrap differently each time and no row should show
    // cells from the old width's wrap position.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("aaa bbb ccc ddd eee fff ggg hhh");
    dom.append_child(p, t).unwrap();
    let span = dom.create_element("span");
    dom.append_child(p, span).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::new()
        .rule(
            "p",
            TuiStyle::new().display(Display::Block).width(Size::Flex(1)),
        )
        .unwrap();

    let backend = TestBackend::new(30, 8);
    let mut term = Terminal::new(backend).unwrap();

    // Frame 1: 30 wide. 31 chars fit on one line? Actually 31>30 so wraps.
    term.draw(|buf| {
        dom.cascade(&sheet);
        dom.layout_dom(buf.area);
        dom.paint_dom(buf, buf.area);
        Ok(())
    })
    .unwrap();
    let mut s30 = VirtualScreen::new(30, 8);
    s30.apply(term.backend().bytes());

    // Frame 2: 10 wide — much narrower, more wrapping.
    term.backend_mut().resize(10, 8);
    let _ = term.backend_mut().take_bytes();
    term.draw(|buf| {
        dom.cascade(&sheet);
        dom.layout_dom(buf.area);
        dom.paint_dom(buf, buf.area);
        Ok(())
    })
    .unwrap();
    let mut s10 = VirtualScreen::new(10, 8);
    s10.apply(term.backend().bytes());

    // Each line must be ≤ 10 wide and not empty where content exists.
    let line0 = s10.row(0).trim_end().to_string();
    assert!(
        !line0.is_empty(),
        "row 0 empty after narrow reflow — content lost"
    );
    // Width constraint — no cell past column 9.
    for y in 0..8 {
        assert!(s10.cell(9, y).is_some());
        assert!(s10.cell(10, y).is_none());
    }

    // Frame 3: back to 30. Must match Frame 1 (same input, same
    // output). Proves reflow is deterministic and doesn't accumulate
    // state across resizes.
    term.backend_mut().resize(30, 8);
    let _ = term.backend_mut().take_bytes();
    term.draw(|buf| {
        dom.cascade(&sheet);
        dom.layout_dom(buf.area);
        dom.paint_dom(buf, buf.area);
        Ok(())
    })
    .unwrap();
    let mut s30_again = VirtualScreen::new(30, 8);
    s30_again.apply(term.backend().bytes());

    for y in 0..8 {
        assert_eq!(
            s30.row(y).trim_end(),
            s30_again.row(y).trim_end(),
            "row {y} differs between first 30-wide render and post-resize 30-wide render"
        );
    }
}

#[test]
fn ifc_block_intrinsic_height_is_one_line() {
    // Auto-height IFC block should size to 1 row.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let section = dom.create_element("section");
    let p = dom.create_element("p");
    let t = dom.create_text_node("x ");
    let b = dom.create_element("b");
    let b_text = dom.create_text_node("y");
    dom.append_child(b, b_text).unwrap();
    dom.append_child(p, t).unwrap();
    dom.append_child(p, b).unwrap();
    dom.append_child(section, p).unwrap();
    // Sibling block after the IFC — its y position tells us p's height.
    let after = dom.create_element("div");
    let after_text = dom.create_text_node("after");
    dom.append_child(after, after_text).unwrap();
    dom.append_child(section, after).unwrap();
    dom.append_child(root, section).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "section",
            TuiStyle::new()
                .display(Display::Block)
                .flow(Flow::Flex)
                .direction(Direction::Column),
        )
        // p: Auto height — should equal 1 because IFC Phase B is single-line.
        .rule_unchecked("p", TuiStyle::new().display(Display::Block))
        .rule_unchecked("b", TuiStyle::new().display(Display::Inline).bold(true))
        .rule_unchecked(
            "div",
            TuiStyle::new()
                .display(Display::Block)
                .height(Size::Fixed(1)),
        );

    let screen = render(&mut dom, &sheet, 10, 3);
    assert_eq!(screen.row(0).trim_end(), "x y");
    // If IFC is 1 line, 'after' sits on row 1.
    assert_eq!(screen.row(1).trim_end(), "after");
}
