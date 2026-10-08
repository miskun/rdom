//! C10-DETAILS-CONTENT — HTML §4.11.1 / §15.5.20 and CSS Pseudo-Elements
//! 4: a `<details>` element's content — everything but its first
//! `<summary>` child — is slotted into `::details-content`, which hides it
//! while the element is closed (`content-visibility: hidden` in the UA
//! sheet) and which its content inherits from.

use rdom_tui::prelude::*;

use super::{el, paint_tree, text_el};

/// `<details [open]><summary>S</summary>loose<p>para</p></details>`.
fn details(dom: &mut TuiDom, root: NodeId, open: bool) -> NodeId {
    let d = el(dom, root, "details", "");
    if open {
        dom.set_attribute(d, "open", "").unwrap();
    }
    text_el(dom, d, "summary", "", "S");
    let t = dom.create_text_node("loose");
    dom.append_child(d, t).unwrap();
    text_el(dom, d, "p", "", "para");
    d
}

/// HTML §15.5.20: a closed `<details>` renders its summary alone — its
/// whole content slot is hidden, text directly inside it included (it
/// used to show: only element children were hidden).
#[test]
fn a_closed_details_hides_its_whole_content() {
    let rows = paint_tree("", 8, 3, |dom, root| {
        details(dom, root, false);
    });
    assert_eq!(rows, ["▸ S     ", "        ", "        "]);
    let rows = paint_tree("", 8, 3, |dom, root| {
        details(dom, root, true);
    });
    assert_eq!(rows, ["▾ S     ", "loose   ", "para    "]);
}

/// The content slot is the content's parent for inheritance: a color set
/// on `::details-content` reaches the content, not the summary.
#[test]
fn the_content_inherits_from_details_content() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let d = details(&mut dom, root, true);
    let buf = super::paint(&mut dom, "details::details-content { color: red }", 8, 3);
    let red = rdom_tui::Color::Rgb(255, 0, 0);
    assert_eq!(buf.cell(0, 2).unwrap().fg, red, "the <p> in the slot");
    assert_ne!(
        buf.cell(2, 0).unwrap().fg,
        red,
        "the summary is not slotted"
    );
    assert!(
        dom.node(d)
            .ext()
            .unwrap()
            .computed_details_content()
            .is_some()
    );
}

/// `display: none` on the slot hides the content of an open `<details>`
/// too; a second `<summary>` is content, hidden while closed.
#[test]
fn the_slot_decides_what_shows() {
    let rows = paint_tree(
        "details::details-content { display: none }",
        8,
        3,
        |dom, root| {
            details(dom, root, true);
        },
    );
    assert_eq!(rows, ["▾ S     ", "        ", "        "]);
    let rows = paint_tree("", 8, 3, |dom, root| {
        let d = details(dom, root, false);
        text_el(dom, d, "summary", "", "T");
    });
    assert_eq!(rows, ["▸ S     ", "        ", "        "]);
}

// ── C10G-DETAILS-CONTENT-BOX: the slot is a box ─────────────────────

/// `<details open>` holding only `<p>` rows `texts`, after a summary `S`,
/// followed by a `<p>after</p>` sibling.
fn rows_details(dom: &mut TuiDom, root: NodeId, texts: &[&str]) -> NodeId {
    let d = el(dom, root, "details", "");
    dom.set_attribute(d, "open", "").unwrap();
    text_el(dom, d, "summary", "", "S");
    for t in texts {
        text_el(dom, d, "p", "", t);
    }
    text_el(dom, root, "p", "", "after");
    d
}

/// HTML §15.5.20 (the second slot is a block box holding the details
/// element's content) / CSS Pseudo-Elements 4 `::details-content`: the
/// slot's border, padding and background draw around the content, below
/// the summary — not around the summary.
#[test]
fn the_slot_is_a_block_box_around_the_content() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    details(&mut dom, root, true);
    let buf = super::paint(
        &mut dom,
        "details::details-content { border: solid; padding: 0 1; background-color: blue }",
        10,
        5,
    );
    assert_eq!(
        super::rows(&buf, 10, 5),
        [
            "▾ S       ",
            "┌────────┐",
            "│ loose  │",
            "│ para   │",
            "└────────┘"
        ]
    );
    let blue = rdom_tui::Color::Rgb(0, 0, 255);
    assert_eq!(buf.cell(1, 2).unwrap().bg, blue, "the slot's padding");
    assert_eq!(buf.cell(7, 3).unwrap().bg, blue, "behind the content");
    assert_ne!(buf.cell(3, 0).unwrap().bg, blue, "the summary is outside");
}

/// CSS 2.1 §10.5 / CSS Overflow 3 §3: the slot takes its `height`, and
/// `overflow: hidden` clips the content past it; the details element's
/// next sibling follows the slot's box.
#[test]
fn the_slot_is_sized_and_clips_its_content() {
    let rows = paint_tree(
        "details::details-content { height: 1; overflow: hidden }",
        8,
        4,
        |dom, root| {
            rows_details(dom, root, &["one", "two"]);
        },
    );
    assert_eq!(rows, ["▾ S     ", "one     ", "after   ", "        "]);
    // Without the clip, `two` overflows over `after`'s row.
    let rows = paint_tree(
        "details::details-content { height: 1 }",
        8,
        4,
        |dom, root| {
            rows_details(dom, root, &["one", "two"]);
        },
    );
    assert_eq!(rows[1], "one     ");
    assert_eq!(rows[3], "        ");
}

/// HTML §15.5.20: a closed element's slot is `content-visibility:
/// hidden` — its content is skipped, its box stays: the padding and
/// background still draw, empty.
#[test]
fn a_closed_slot_keeps_its_box_without_content() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    details(&mut dom, root, false);
    text_el(&mut dom, root, "p", "", "after");
    let buf = super::paint(
        &mut dom,
        "details::details-content { padding-top: 1; background-color: blue }",
        8,
        3,
    );
    assert_eq!(
        super::rows(&buf, 8, 3),
        ["▸ S     ", "        ", "after   "]
    );
    let blue = rdom_tui::Color::Rgb(0, 0, 255);
    assert_eq!(buf.cell(4, 1).unwrap().bg, blue);
}

/// CSS Overflow 3 §3: an `overflow: auto` slot shorter than its content
/// is a scroll container — a scrollbar, and the wheel over it scrolls it.
#[test]
fn an_overflowing_slot_scrolls() {
    use crossterm::event::{
        Event as CtEvent, KeyModifiers, MouseEvent as CtMouseEvent, MouseEventKind,
    };
    let mut dom = TuiDom::new();
    let root = dom.root();
    rows_details(&mut dom, root, &["one", "two", "three"]);
    let sheet = rdom_css::from_css_strict(
        "details::details-content { height: 2; overflow-y: auto; scrollbar-width: none }",
    )
    .unwrap();
    let terminal = Terminal::new(TestBackend::new(8, 4)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.draw_if_dirty().unwrap();
    let area = Rect::new(0, 0, 8, 4);
    let painted = |app: &App<TestBackend>| {
        let mut buf = Buffer::empty(area);
        app.dom().paint_dom(&mut buf, area);
        super::rows(&buf, 8, 4)
    };
    assert_eq!(
        painted(&app),
        ["▾ S     ", "one     ", "two     ", "after   "]
    );
    app.handle_event(CtEvent::Mouse(CtMouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 1,
        row: 1,
        modifiers: KeyModifiers::empty(),
    }));
    app.draw_if_dirty().unwrap();
    assert_eq!(
        painted(&app),
        ["▾ S     ", "two     ", "three   ", "after   "]
    );
}

/// The slot is part of its element's box for the pointer (CSS
/// Pseudo-Elements 4 §2: a pseudo-element's box is hit as its
/// originating element): a point on its border or padding hits the
/// `<details>`; a point on its content hits that content, with the
/// `<details>` above it on the path; and the text in it takes the caret.
#[test]
fn the_slot_hits_as_its_element_and_its_text_takes_the_caret() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let d = details(&mut dom, root, true);
    let p = dom.node(d).last_child().unwrap().id();
    let loose = dom.node(d).child_nodes().nth(1).unwrap().id();
    super::lay_out(
        &mut dom,
        "details::details-content { border: solid; padding: 0 1 }",
        10,
        5,
    );
    assert_eq!(dom.hit_test(0, 2), Some(d), "the slot's border");
    assert_eq!(dom.hit_test(1, 3), Some(d), "the slot's padding");
    assert_eq!(dom.hit_test(3, 3), Some(p));
    let path = dom.hit_test_path(3, 3);
    assert_eq!(&path[path.len() - 2..], [d, p], "the slot is no node");
    let pos = dom
        .position_at(4, 2)
        .expect("the loose text takes the caret");
    assert_eq!((pos.node, pos.offset), (loose, 2));
}

/// CSS 2.1 §10.3.5 (shrink-to-fit) counts the slot's box: an
/// `inline-block` `<details>` is as wide as its slot's padding box.
#[test]
fn the_slots_box_counts_in_its_elements_intrinsic_width() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let d = details(&mut dom, root, true);
    super::lay_out(
        &mut dom,
        "details { display: inline-block } details::details-content { padding: 0 2 }",
        20,
        4,
    );
    assert_eq!(super::size(&dom, d), (9, 3), "`loose` + 2 + 2");
}

/// CSS 2.1 §8.3.1: a content child's top margin collapses through a slot
/// with no border or padding — the slot sits where the margin puts it — and
/// stays inside one with a top border.
#[test]
fn margins_collapse_through_the_slot_as_through_a_block() {
    let rows = paint_tree("p { margin: 1 0 0 0 }", 8, 4, |dom, root| {
        rows_details(dom, root, &["one"]);
    });
    assert_eq!(rows, ["▾ S     ", "        ", "one     ", "        "]);
    let rows = paint_tree(
        "details > p { margin: 1 0 0 0 } details::details-content { border-top: solid }",
        8,
        5,
        |dom, root| {
            rows_details(dom, root, &["one"]);
        },
    );
    assert_eq!(
        rows,
        ["▾ S     ", "────────", "        ", "one     ", "after   "]
    );
}

/// CSS Transitions 1 §3: the slot's transitions run as an element's; the
/// events fire on the `<details>` with `pseudoElement`
/// `"::details-content"` (§6.1).
#[test]
fn the_slots_transitions_run_and_fire_on_its_element() {
    use std::cell::RefCell;
    use std::rc::Rc;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let d = details(&mut dom, root, false);
    let seen: Rc<RefCell<Vec<Option<String>>>> = Rc::default();
    let log = seen.clone();
    dom.add_event_listener(
        d,
        "transitionstart",
        rdom_core::ListenerOptions::default(),
        move |ev| {
            if let Some(t) = ev.event.detail.as_transition() {
                log.borrow_mut().push(t.pseudo_element.clone());
            }
        },
    )
    .unwrap();
    let sheet = rdom_css::from_css_strict(
        "details::details-content { padding-top: 1; background-color: blue; \
         transition: background-color 10s linear } \
         details[open]::details-content { background-color: red }",
    )
    .unwrap();
    let terminal = Terminal::new(TestBackend::new(8, 4)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.draw_if_dirty().unwrap();
    app.dom_mut().set_attribute(d, "open", "").unwrap();
    app.draw_if_dirty().unwrap();
    app.advance(0).unwrap();
    let area = Rect::new(0, 0, 8, 4);
    let mut buf = Buffer::empty(area);
    app.dom().paint_dom(&mut buf, area);
    let bg = buf.cell(4, 1).unwrap().bg;
    assert_ne!(bg, rdom_tui::Color::Rgb(255, 0, 0), "still transitioning");
    assert_eq!(*seen.borrow(), [Some("::details-content".to_string())]);
}

/// HTML §6.6.3 (sequential focus navigation follows the tree): content in
/// the slot keeps its place in the tab order — after the summary, before
/// what follows the `<details>` — and its text puts the caret where it is
/// painted, inside the slot's border and padding.
#[test]
fn the_slots_content_keeps_its_tab_order_and_caret() {
    use crossterm::event::{Event as CtEvent, KeyCode, KeyEvent, KeyModifiers};
    let mut dom = TuiDom::new();
    let root = dom.root();
    let d = details(&mut dom, root, true);
    let loose = dom.node(d).child_nodes().nth(1).unwrap().id();
    let inner = text_el(&mut dom, d, "button", "", "in");
    let outer = text_el(&mut dom, root, "button", "", "out");
    let sheet =
        rdom_css::from_css_strict("details::details-content { border: solid; padding: 0 1 }")
            .unwrap();
    let terminal = Terminal::new(TestBackend::new(12, 8)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.draw_if_dirty().unwrap();
    let summary = app.dom().node(d).first_child().unwrap().id();
    let mut order = Vec::new();
    for _ in 0..3 {
        app.handle_event(CtEvent::Key(KeyEvent::new(
            KeyCode::Tab,
            KeyModifiers::empty(),
        )));
        app.draw_if_dirty().unwrap();
        order.push(app.dom().focused());
    }
    assert_eq!(order, [Some(summary), Some(inner), Some(outer)]);
    let cell = rdom_tui::render::inline::cell_of_position(app.dom(), Position::new(loose, 2));
    assert_eq!(
        cell,
        Some((4, 2)),
        "`loose` at (2, 2), after the border and padding"
    );
}

/// The slot's box lives as long as its element: dropping the `<details>`
/// from the arena reclaims it at the next cascade (no node outlives the
/// element it was the box of).
#[test]
fn a_dropped_details_takes_its_slots_box_with_it() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let before = dom.len();
    let d = details(&mut dom, root, true);
    super::lay_out(&mut dom, "", 8, 4);
    assert!(dom.len() > before);
    dom.drop_subtree(d).unwrap();
    super::lay_out(&mut dom, "", 8, 4);
    assert_eq!(
        dom.len(),
        before,
        "the details, its content and its slot's box"
    );
}

// ── C11G-DETAILS-PARENT: box-parent climbs through the slot ──────────

/// `<details open><summary>s</summary><span>x</span></details>`; the
/// span's id.
fn details_span(dom: &mut TuiDom, root: NodeId) -> NodeId {
    let d = el(dom, root, "details", "");
    dom.set_attribute(d, "open", "").unwrap();
    text_el(dom, d, "summary", "", "s");
    text_el(dom, d, "span", "", "x")
}

/// The pseudo-element hit test finds a slotted element's `::before` on
/// the line boxes of the `::details-content` box that holds them — its
/// parent in the box tree — so `span::before:hover` can match (Selectors 4
/// §3.6.3; architect N1).
#[test]
fn a_slotted_elements_before_is_hit() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let span = details_span(&mut dom, root);
    let buf = super::paint(&mut dom, "span::before { content: '*' }", 8, 3);
    assert_eq!(
        buf.cell(0, 1).unwrap().symbol(),
        "*",
        "{:?}",
        super::rows(&buf, 8, 3)
    );
    assert_eq!(
        dom.hit_test_pseudo(0, 1),
        Some((span, rdom_tui::ext::PseudoSlot::Before))
    );
}

/// CSS Display 3 §2.7: the children of a flex container blockify, and a
/// `display: contents` box passes that on — climbing the box tree, so a
/// `details { display: flex }` whose `::details-content` is `contents`
/// makes its slotted content flex items.
#[test]
fn slotted_content_of_a_flex_details_through_a_contents_slot_blockifies() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let span = details_span(&mut dom, root);
    super::paint(
        &mut dom,
        "details { display: flex } details::details-content { display: contents }",
        8,
        3,
    );
    assert_eq!(
        dom.node(span).computed().unwrap().display,
        rdom_tui::layout::Display::Block
    );
}

/// A subtree restyle below a `<details>` that brings a positioned
/// `::before` flags the `::details-content` box above it too
/// (`tree_has_positioned_pseudo`), so the paint walk that skips unflagged
/// subtrees reaches it through the box.
#[test]
fn a_restyle_inside_details_flags_the_slot_box() {
    use rdom_tui::render::{Terminal, TestBackend};
    let mut dom = TuiDom::new();
    let root = dom.root();
    let span = details_span(&mut dom, root);
    let sheet = rdom_css::from_css_strict(
        ".on::before { content: '*'; position: absolute; left: 5; top: 2 }",
    )
    .unwrap();
    let terminal = Terminal::new(TestBackend::new(8, 3)).unwrap();
    let mut app = rdom_tui::App::with_backend(dom, sheet, terminal).unwrap();
    app.advance(0).unwrap();
    app.dom_mut().add_class(span, "on").unwrap();
    app.advance(0).unwrap();
    let area = rdom_tui::render::Rect::new(0, 0, 8, 3);
    let mut buf = rdom_tui::render::Buffer::empty(area);
    app.dom().paint_dom(&mut buf, area);
    assert_eq!(
        buf.cell(5, 2).unwrap().symbol(),
        "*",
        "{:?}",
        super::rows(&buf, 8, 3)
    );
}

/// A restyle that makes a flex `<details>`'s `::details-content` box
/// `display: contents` turns its slotted content into flex items — the
/// slot box's change reaches the children it holds, as an element's would
/// (C7G-MINOR's `items_changed`, through the box tree).
#[test]
fn a_restyled_contents_slot_reblockifies_its_content() {
    use rdom_tui::render::{Terminal, TestBackend};
    let mut dom = TuiDom::new();
    let root = dom.root();
    let span = details_span(&mut dom, root);
    let d = dom.node(span).parent_node().unwrap().id();
    let sheet = rdom_css::from_css_strict(
        "details { display: flex } details.c::details-content { display: contents }",
    )
    .unwrap();
    let terminal = Terminal::new(TestBackend::new(8, 3)).unwrap();
    let mut app = rdom_tui::App::with_backend(dom, sheet, terminal).unwrap();
    app.advance(0).unwrap();
    let display =
        |app: &rdom_tui::App<TestBackend>| app.dom().node(span).computed().unwrap().display;
    assert_eq!(display(&app), rdom_tui::layout::Display::Inline);
    app.dom_mut().add_class(d, "c").unwrap();
    app.advance(0).unwrap();
    assert_eq!(display(&app), rdom_tui::layout::Display::Block);
}
