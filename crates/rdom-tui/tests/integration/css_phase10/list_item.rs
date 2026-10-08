//! C10-LIST-ITEM — CSS Lists 3: `display: list-item` generates a
//! `::marker` (§3.1–§3.2) whose content `list-style-type` makes (§3.4),
//! placed by `list-style-position` (§3.5) and `marker-side` (§3.6).

use rdom_tui::{NodeId, TuiDom, TuiNodeExt};

use super::{el, lay_out, text_el};

/// The `::marker` text of `id`, `None` without a marker.
fn marker(dom: &TuiDom, id: NodeId) -> Option<String> {
    dom.node(id)
        .computed_marker()
        .and_then(|m| m.content.clone())
}

/// Lay out `css` over a `<ul>` / `<ol>` (`tag`) of items `texts` in a
/// `w` × `h` viewport: the dom and the items.
fn list(tag: &str, css: &str, texts: &[&str], w: u16, h: u16) -> (TuiDom, Vec<NodeId>) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let l = el(&mut dom, root, tag, "l");
    let items = texts
        .iter()
        .map(|t| text_el(&mut dom, l, "li", "", t))
        .collect();
    lay_out(&mut dom, css, w, h);
    (dom, items)
}

/// §3.1: a list item generates a `::marker`; §3.4: its content is the
/// `list-item` counter in `list-style-type`, between the counter style's
/// prefix and suffix — HTML's `ul` is `disc`, `ol` `decimal` (HTML
/// §15.3.8) — a string as written, nothing for `none`.
#[test]
fn list_style_type_makes_the_marker() {
    let (dom, items) = list("ol", "", &["a", "b"], 20, 4);
    assert_eq!(marker(&dom, items[0]).as_deref(), Some("1. "));
    assert_eq!(marker(&dom, items[1]).as_deref(), Some("2. "));
    let (dom, items) = list("ul", "", &["a"], 20, 4);
    assert_eq!(marker(&dom, items[0]).as_deref(), Some("• "));
    let (dom, items) = list(
        "ol",
        "li { list-style-type: upper-roman }",
        &["a", "b"],
        20,
        4,
    );
    assert_eq!(marker(&dom, items[1]).as_deref(), Some("II. "));
    let (dom, items) = list("ul", r#"li { list-style-type: "→ " }"#, &["a"], 20, 4);
    assert_eq!(marker(&dom, items[0]).as_deref(), Some("→ "));
    let (dom, items) = list("ul", "ul { list-style: none }", &["a"], 20, 4);
    assert_eq!(marker(&dom, items[0]), None);
}

/// HTML §15.3.8: a `ul` nested in a list is `circle`, one level deeper
/// `square`.
#[test]
fn nested_bullet_lists_change_their_bullet() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let ul = el(&mut dom, root, "ul", "");
    let li = text_el(&mut dom, ul, "li", "", "a");
    let ul2 = el(&mut dom, li, "ul", "");
    let li2 = text_el(&mut dom, ul2, "li", "", "b");
    let ul3 = el(&mut dom, li2, "ul", "");
    let li3 = text_el(&mut dom, ul3, "li", "", "c");
    lay_out(&mut dom, "", 20, 6);
    assert_eq!(marker(&dom, li).as_deref(), Some("• "));
    assert_eq!(marker(&dom, li2).as_deref(), Some("◦ "));
    assert_eq!(marker(&dom, li3).as_deref(), Some("▪ "));
}

/// §3.2: `::marker { content }` replaces the marker; `content: none`
/// removes it, `normal` is `list-style-type`'s. Properties outside the
/// marker's list do not apply (`padding`), `color` does.
#[test]
fn the_marker_pseudo_element() {
    let (dom, items) = list(
        "ol",
        r#"li::marker { content: "*" counter(list-item) ") "; padding: 3; color: red }
           li + li::marker { content: none }"#,
        &["a", "b"],
        20,
        4,
    );
    let m = dom.node(items[0]).computed_marker().unwrap();
    assert_eq!(m.content.as_deref(), Some("*1) "));
    assert_eq!(m.padding.top, rdom_tui::layout::PaddingValue::Cells(0));
    assert_eq!(m.fg, rdom_tui::Color::Rgb(255, 0, 0));
    assert_eq!(marker(&dom, items[1]), None);
}

/// §3.1: any `display: list-item` box has a marker — `disc` by default
/// (the initial `list-style-type`); an `li` made `display: block` has
/// none.
#[test]
fn display_list_item_generates_the_marker() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let d = text_el(&mut dom, root, "div", "d", "a");
    let l = el(&mut dom, root, "ul", "");
    let li = text_el(&mut dom, l, "li", "b", "b");
    lay_out(
        &mut dom,
        ".d { display: list-item } .b { display: block }",
        20,
        4,
    );
    assert_eq!(marker(&dom, d).as_deref(), Some("• "));
    assert_eq!(marker(&dom, li), None);
}

/// CSS Lists 3 §3.2's UA sheet: `::marker { text-transform: none }` —
/// an uppercased item keeps a lowercase alphabetic marker.
#[test]
fn the_marker_is_not_transformed() {
    let (dom, items) = list(
        "ol",
        "li { list-style-type: lower-alpha; text-transform: uppercase }",
        &["a"],
        20,
        4,
    );
    assert_eq!(marker(&dom, items[0]).as_deref(), Some("a. "));
}

// ── Laying the marker out (part 2) ──────────────────────────────────

use super::{paint, paint_tree, rows};

/// A `tag` list of items `texts` under the root, painted with `css` in a
/// `w` × `h` viewport: the rows.
fn list_rows(tag: &str, css: &str, texts: &[&str], w: u16, h: u16) -> Vec<String> {
    paint_tree(css, w, h, |dom, root| {
        let l = el(dom, root, tag, "l");
        for t in texts {
            text_el(dom, l, "li", "", t);
        }
    })
}

/// §3.5 `outside` (the initial value): the marker hangs outside the list
/// item's box, its end at the item's inline-start border edge, on the
/// row of the item's first line — in the list's padding, which HTML
/// gives the list (`padding-inline-start`, four cells: C10G-MARKER-CLIP).
#[test]
fn outside_markers_hang_in_the_lists_padding() {
    assert_eq!(list_rows("ul", "", &["a", "b"], 6, 2), ["  • a ", "  • b "]);
    assert_eq!(list_rows("ol", "", &["a", "b"], 6, 2), [" 1. a ", " 2. b "]);
}

/// §3.5: an outside marker is no inline content — the item's lines start
/// at its content edge, so a wrapped line lines up under the first
/// line's text, not under the marker.
#[test]
fn a_wrapped_item_hangs_its_marker() {
    let rows = list_rows("ul", ".l { width: 4 }", &["aaa bbb"], 8, 2);
    assert_eq!(rows, ["  • aaa ", "    bbb "]);
}

/// §3.5 `inside`: the marker is the first inline box of the item's first
/// line, and wraps with it.
#[test]
fn inside_markers_are_the_first_inline_box() {
    let rows = list_rows(
        "ul",
        ".l { width: 6 } li { list-style-position: inside }",
        &["aaa bbb"],
        10,
        2,
    );
    assert_eq!(rows, ["    • aaa ", "    bbb   "]);
}

/// §3.1: the marker sits on the list item's first line box even when
/// that line belongs to a block descendant — `<li><p>Step</p></li>`
/// reads "1. Step"; a second paragraph has no marker.
#[test]
fn a_marker_rides_a_descendants_first_line() {
    let build = |dom: &mut TuiDom, root: NodeId| {
        let ol = el(dom, root, "ol", "");
        let li = el(dom, ol, "li", "");
        text_el(dom, li, "p", "", "Step");
        text_el(dom, li, "p", "", "More");
    };
    assert_eq!(paint_tree("", 9, 2, build), [" 1. Step ", "    More "]);
    let inside = "li { list-style-position: inside }";
    assert_eq!(
        paint_tree(inside, 11, 2, build),
        ["    1. Step", "    More   "]
    );
}

/// CSS 2.1 §9.2.1.1 for every host, `li` included: a `::before` before a
/// block child is an anonymous block box of its own — the list item's
/// first line, which the marker rides.
#[test]
fn an_items_before_is_an_ordinary_pseudo_element() {
    let rows = paint_tree(r#"li::before { content: ">" }"#, 5, 2, |dom, root| {
        let ul = el(dom, root, "ul", "");
        let li = el(dom, ul, "li", "");
        text_el(dom, li, "p", "", "x");
    });
    assert_eq!(rows, ["  • >", "    x"]);
}

/// Nested lists: both markers ride the inner item's line, each beside its
/// own item (§3.5).
#[test]
fn nested_markers_share_the_inner_line() {
    let rows = paint_tree("", 9, 1, |dom, root| {
        let ul = el(dom, root, "ul", "");
        let li = el(dom, ul, "li", "");
        let ul2 = el(dom, li, "ul", "");
        text_el(dom, ul2, "li", "", "x");
    });
    assert_eq!(rows, ["  •   ◦ x"]);
}

/// §3.1: an empty list item still shows its marker — it makes the item's
/// one line.
#[test]
fn an_empty_item_shows_its_marker() {
    assert_eq!(list_rows("ul", "", &["", "b"], 6, 2), ["  •   ", "  • b "]);
}

/// The marker's text is its own: CSS Lists 3's UA `::marker {
/// text-transform: none }` keeps it lowercase under an uppercased item;
/// `letter-spacing` (inherited, CSS Text 3 §9.2) spaces it as the item's
/// text — the marker hangs as wide as it is packed.
#[test]
fn markers_with_text_transform_and_letter_spacing() {
    let rows = list_rows(
        "ol",
        "ol { list-style-type: lower-alpha } li { text-transform: uppercase }",
        &["xy"],
        7,
        1,
    );
    assert_eq!(rows, [" a. XY "]);
    let rows = list_rows(
        "ul",
        ".l { padding-left: 4 } li { letter-spacing: 1 }",
        &["ab"],
        8,
        1,
    );
    assert_eq!(rows, [" •  a b "]);
}

/// §3.5 / §3.6: under `direction: rtl` the item's inline-start edge is
/// its right one, so an outside marker hangs on the right — `marker-side:
/// match-self` — written in visual order (` •`); `match-parent` hangs
/// it on the parent's start side instead.
#[test]
fn right_to_left_markers_hang_on_the_right() {
    let rows = list_rows("ul", ".l { direction: rtl; width: 4 }", &["a"], 6, 1);
    assert_eq!(rows, ["   a •"]);
    let rows = list_rows(
        "ul",
        "li { direction: rtl; marker-side: match-parent }",
        &["a"],
        6,
        1,
    );
    assert_eq!(rows, ["  •  a"]);
}

/// §3.2: `::marker` takes `color`; the marker sits outside the item's
/// box, so the item's background does not reach under it.
#[test]
fn the_marker_is_styled_and_outside_the_items_background() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let ul = el(&mut dom, root, "ul", "");
    text_el(&mut dom, ul, "li", "", "a");
    let buf = paint(
        &mut dom,
        "li::marker { color: red } li { background-color: blue }",
        6,
        1,
    );
    assert_eq!(rows(&buf, 6, 1), ["  • a "]);
    let marker = buf.cell(2, 0).unwrap();
    assert_eq!(marker.fg, rdom_tui::Color::Rgb(255, 0, 0));
    assert_ne!(marker.bg, rdom_tui::Color::Rgb(0, 0, 255));
    assert_eq!(buf.cell(4, 0).unwrap().bg, rdom_tui::Color::Rgb(0, 0, 255));
}

/// A marker riding a descendant's line counts in the intrinsic width as
/// it is packed (C9G-MISC-CORRECTNESS's leftover): an inline-block list
/// whose item's first line is a paragraph with an inside, letter-spaced
/// marker is as wide as that line — `•`, a space and `a b` with a cell
/// after each but the last, plus the list's four cells of padding.
#[test]
fn a_riding_marker_is_measured_through_the_packer() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let c = el(&mut dom, root, "div", "c");
    let ul = el(&mut dom, c, "ul", "");
    let li = el(&mut dom, ul, "li", "");
    // A paragraph whose first line is anonymous inline content before a
    // block child: measured child by child, its line's markers added.
    let p = text_el(&mut dom, li, "p", "", "ab");
    text_el(&mut dom, p, "div", "", "c");
    lay_out(
        &mut dom,
        ".c { display: inline-block } li { list-style-position: inside; letter-spacing: 1 }",
        20,
        3,
    );
    assert_eq!(super::size(&dom, c).0, 4 + 7);
}

/// §3.1: any `display: list-item` box hangs a marker; HTML's `li` is
/// one, and an author `display: list-item` element too.
#[test]
fn a_list_item_div_hangs_a_marker() {
    let rows = paint_tree(
        ".d { display: list-item; margin-left: 2 }",
        4,
        1,
        |dom, root| {
            text_el(dom, root, "div", "d", "a");
        },
    );
    assert_eq!(rows, ["• a "]);
}

// ── C10G-MARKER-CLIP: the list's padding holds the marker ──────────────

/// HTML §15.3.8 gives `ul`, `ol` and `menu` a `padding-inline-start` of
/// 40px — 2.5em at the 16px default font, room for about five digits —
/// so a list at the page's edge shows "10. " whole; rdom's is four cells.
/// An outside marker wider than that (CSS Lists 3 §3.5: "III. " is five)
/// overflows the list's box, as in a browser: it shows in the margin
/// beside the list, and only the viewport or a clipping ancestor cuts it.
#[test]
fn the_lists_padding_holds_ten_and_overflow_is_clipped_only_by_the_viewport() {
    let ten = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"];
    let rows = list_rows("ol", "", &ten, 7, 10);
    assert_eq!(rows[0], " 1. a  ");
    assert_eq!(rows[9], "10. j  ");
    let roman = "ol { list-style-type: upper-roman }";
    let three = ["a", "b", "c"];
    // At the viewport's edge: its first cell is off the screen.
    assert_eq!(list_rows("ol", roman, &three, 7, 3)[2], "II. c  ");
    // Beside a margin: it overflows the list into it, whole.
    let rows = list_rows(
        "ol",
        &format!("{roman} .l {{ margin-left: 2 }}"),
        &three,
        8,
        3,
    );
    assert_eq!(rows, ["   I. a ", "  II. b ", " III. c "]);
    // In a clipping ancestor: cut at its edge.
    let rows = paint_tree(
        &format!("{roman} .c {{ overflow: hidden; margin-left: 2 }}"),
        8,
        3,
        |dom, root| {
            let c = el(dom, root, "div", "c");
            let l = el(dom, c, "ol", "");
            for t in three {
                text_el(dom, l, "li", "", t);
            }
        },
    );
    assert_eq!(rows[2], "  II. c ");
}

// ── C10G-MARKER-HIT: an outside marker is hit as its item ──────────────

/// CSS Lists 3 §3.5 makes an outside marker a box of its list item, hung
/// outside the item's principal box (CSS Pseudo 4 §2: a pseudo-element's
/// box is part of its originating element's), and browsers hit-test it to
/// the item: a click on a bullet targets the `li`, not the list whose
/// padding it hangs in, and `hit_test_pseudo` names the marker — for a
/// marker on the item's own line, on a descendant's line, and for both
/// markers nested items put on one line. Beside a marker the list is hit.
#[test]
fn an_outside_marker_is_hit_as_its_item() {
    use rdom_tui::HitTestExt;
    use rdom_tui::ext::PseudoSlot;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let ul = el(&mut dom, root, "ul", "");
    let own = text_el(&mut dom, ul, "li", "", "a");
    let ol = el(&mut dom, root, "ol", "");
    let riding = el(&mut dom, ol, "li", "");
    text_el(&mut dom, riding, "p", "", "Step");
    let outer = el(&mut dom, root, "ul", "");
    let outer_li = el(&mut dom, outer, "li", "");
    let inner = el(&mut dom, outer_li, "ul", "");
    let inner_li = text_el(&mut dom, inner, "li", "", "x");
    lay_out(&mut dom, "", 12, 3);
    for (x, y, item) in [
        (2, 0, own),
        (1, 1, riding),
        (2, 2, outer_li),
        (6, 2, inner_li),
    ] {
        assert_eq!(dom.hit_test(x, y), Some(item), "({x}, {y})");
        assert_eq!(
            dom.hit_test_pseudo(x, y),
            Some((item, PseudoSlot::Marker)),
            "({x}, {y})"
        );
    }
    assert_eq!(dom.hit_test(0, 0), Some(ul));
    assert!(dom.hit_test_path(1, 1).contains(&ol));
}

/// HTML §15.3.8 maps `<ol type>` and `<li type>` to `list-style-type` —
/// `1`, `a`, `A`, `i`, `I`, compared case-sensitively (the UA rules'
/// `s` flag) — and `<ul type>` / `<li type>` `none`, `disc`, `circle`,
/// `square`, case-insensitively. rdom applies them as presentational hints
/// (C10G-MARKER-HIT), so an author rule beats them.
#[test]
fn the_type_attribute_sets_the_list_style_type() {
    let first_marker = |tag: &str, list_type: Option<&str>, item_type: Option<&str>, css: &str| {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let l = el(&mut dom, root, tag, "");
        if let Some(t) = list_type {
            dom.set_attribute(l, "type", t).unwrap();
        }
        let li = text_el(&mut dom, l, "li", "", "x");
        if let Some(t) = item_type {
            dom.set_attribute(li, "type", t).unwrap();
        }
        lay_out(&mut dom, css, 10, 1);
        marker(&dom, li)
    };
    for (list_type, want) in [
        ("1", "1. "),
        ("a", "a. "),
        ("A", "A. "),
        ("i", "i. "),
        ("I", "I. "),
        ("x", "1. "),
        ("disc", "1. "),
    ] {
        let got = first_marker("ol", Some(list_type), None, "");
        assert_eq!(got.as_deref(), Some(want), "<ol type={list_type}>");
    }
    for (list_type, want) in [
        ("circle", Some("◦ ")),
        ("SQUARE", Some("▪ ")),
        ("Disc", Some("• ")),
        ("a", Some("• ")),
        ("none", None),
    ] {
        let got = first_marker("ul", Some(list_type), None, "");
        assert_eq!(got.as_deref(), want, "<ul type={list_type}>");
    }
    let got = first_marker("ol", Some("a"), Some("I"), "");
    assert_eq!(got.as_deref(), Some("I. "), "<li type> over <ol type>");
    let got = first_marker("ul", None, Some("square"), "");
    assert_eq!(got.as_deref(), Some("▪ "), "<li type> in a ul");
    let got = first_marker("ol", Some("a"), None, "ol { list-style-type: decimal }");
    assert_eq!(got.as_deref(), Some("1. "), "an author rule beats the hint");
}
