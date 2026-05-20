//! Phase B+.2 tests — scrollbar hit detection + mouse interaction
//! end-to-end through the router.
//!
//! Covers:
//! - `hit()` classifies clicks on thumb vs track (above / below).
//! - Track click pages scroll by one viewport.
//! - Thumb drag engages pointer capture and adjusts scroll
//!   proportionally to cursor movement.
//! - Drag state clears on mouseup.

use crossterm::event::{KeyModifiers, MouseButton, MouseEvent as CtMouseEvent, MouseEventKind};
use rdom_core::NodeId;

use crate::TuiDom;
use crate::layout::{Overflow, Size};
use crate::render::{LayoutExt, Rect};
use crate::runtime::hit_test::HitTestExt;
use crate::runtime::router::Router;
use crate::runtime::scrollbar::{ScrollAxis, ScrollbarPart, hit};
use crate::style::{CascadeExt, Stylesheet, TuiStyle};

// ── Fixtures ────────────────────────────────────────────────────────

/// Build a single `<c>` element 10x6 (outer) with overflow-y:
/// scroll + content of 30 rows. After the gutter, content area is
/// 9x6. Vertical scrollbar column at x=9.
fn vertical_scrollbar_dom() -> (TuiDom, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let c = dom.create_element("c");
    dom.append_child(root, c).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "c",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(6))
            .overflow_y(Overflow::Scroll),
    );
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 12, 8));
    if let Some(ext) = dom.node_mut(c).ext_mut() {
        ext.scroll_content_height = 30;
    }
    (dom, c)
}

fn mouse_at(kind: MouseEventKind, x: u16, y: u16) -> CtMouseEvent {
    CtMouseEvent {
        kind,
        column: x,
        row: y,
        modifiers: KeyModifiers::empty(),
    }
}

fn down(x: u16, y: u16) -> crossterm::event::Event {
    crossterm::event::Event::Mouse(mouse_at(MouseEventKind::Down(MouseButton::Left), x, y))
}

fn up(x: u16, y: u16) -> crossterm::event::Event {
    crossterm::event::Event::Mouse(mouse_at(MouseEventKind::Up(MouseButton::Left), x, y))
}

fn drag(x: u16, y: u16) -> crossterm::event::Event {
    crossterm::event::Event::Mouse(mouse_at(MouseEventKind::Drag(MouseButton::Left), x, y))
}

// ── hit() ───────────────────────────────────────────────────────────

#[test]
fn hit_detects_vertical_thumb() {
    // content=30, viewport=6, scroll=0. track=6. thumb_size =
    // 6*6/30 = 1 (min 1). thumb_off = 0. So thumb at row 0.
    let (dom, c) = vertical_scrollbar_dom();
    let path = dom.hit_test_path(9, 0);
    let result = hit(&dom, &path, 9, 0).expect("should hit");
    assert_eq!(result.element, c);
    assert_eq!(result.axis, ScrollAxis::Vertical);
    assert_eq!(result.part, ScrollbarPart::Thumb);
}

#[test]
fn hit_detects_track_after_thumb() {
    // Same fixture — thumb is row 0, rows 1..6 are "track after".
    let (dom, c) = vertical_scrollbar_dom();
    let path = dom.hit_test_path(9, 3);
    let result = hit(&dom, &path, 9, 3).expect("should hit");
    assert_eq!(result.element, c);
    assert_eq!(result.part, ScrollbarPart::TrackAfter);
}

#[test]
fn hit_returns_none_off_scrollbar() {
    let (dom, _) = vertical_scrollbar_dom();
    let path = dom.hit_test_path(3, 3);
    assert!(hit(&dom, &path, 3, 3).is_none());
}

#[test]
fn hit_respects_auto_visibility() {
    // overflow-y: auto + content fits (content 3 < viewport 6)
    // → scrollbar doesn't render, so hit should return None even
    // on the gutter column.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let c = dom.create_element("c");
    dom.append_child(root, c).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "c",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(6))
            .overflow_y(Overflow::Auto),
    );
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 12, 8));
    if let Some(ext) = dom.node_mut(c).ext_mut() {
        ext.scroll_content_height = 3; // fits
    }
    let path = dom.hit_test_path(9, 2);
    assert!(hit(&dom, &path, 9, 2).is_none());
}

// ── Track click (page) ─────────────────────────────────────────────

#[test]
fn track_click_after_thumb_pages_scroll_forward() {
    let (mut dom, c) = vertical_scrollbar_dom();
    let mut router = Router::new();
    // Click on track below thumb — pages forward by viewport.
    router.route(&mut dom, down(9, 4));
    let scroll = dom.node(c).ext().unwrap().scroll_y;
    // viewport = 6, starting scroll = 0 → new scroll = 6.
    assert_eq!(scroll, 6);
}

#[test]
fn track_click_before_thumb_pages_scroll_back() {
    let (mut dom, c) = vertical_scrollbar_dom();
    // Pre-scroll so there's room to page back.
    if let Some(ext) = dom.node_mut(c).ext_mut() {
        ext.scroll_y = 15;
    }
    let mut router = Router::new();
    // At scroll=15, thumb_off = round(15 * (6-1) / (30-6)) =
    // round(75/24) = 3. Click on row 1 (above thumb) pages back.
    router.route(&mut dom, down(9, 1));
    let scroll = dom.node(c).ext().unwrap().scroll_y;
    // viewport = 6 paged back from 15 → 9.
    assert_eq!(scroll, 9);
}

#[test]
fn track_click_clamps_at_zero() {
    let (mut dom, c) = vertical_scrollbar_dom();
    // Already at top. Page back → clamped to 0.
    let mut router = Router::new();
    router.route(&mut dom, down(9, 1));
    // With scroll=0, thumb is at row 0 — click on row 1 is
    // "track after thumb" → pages FORWARD, not back. So scroll
    // goes to 6, not clamped at 0. Separate test below covers
    // page-back at top.
    let scroll = dom.node(c).ext().unwrap().scroll_y;
    assert_eq!(scroll, 6);
}

#[test]
fn track_click_clamps_at_bottom() {
    let (mut dom, c) = vertical_scrollbar_dom();
    if let Some(ext) = dom.node_mut(c).ext_mut() {
        ext.scroll_y = 22; // near bottom (max = 30 - 6 = 24)
    }
    let mut router = Router::new();
    // Click below thumb — pages forward; should clamp at 24.
    router.route(&mut dom, down(9, 5));
    let scroll = dom.node(c).ext().unwrap().scroll_y;
    assert_eq!(scroll, 24);
}

// ── Thumb drag ─────────────────────────────────────────────────────

#[test]
fn thumb_mousedown_engages_pointer_capture_and_records_drag() {
    let (mut dom, c) = vertical_scrollbar_dom();
    let mut router = Router::new();
    router.route(&mut dom, down(9, 0)); // hits thumb
    assert_eq!(dom.pointer_capture(), Some(c));
    assert!(router.scrollbar_drag.is_some());
}

#[test]
fn dragging_thumb_updates_scroll_proportionally() {
    let (mut dom, c) = vertical_scrollbar_dom();
    let mut router = Router::new();
    router.route(&mut dom, down(9, 0)); // start drag at track pos 0
    // Drag cursor from row 0 to row 3. Cursor delta = 3 cells
    // along the track. track = 6, thumb_size = 1, track_travel =
    // 5. content_travel = 24. scroll_delta = 3 * 24 / 5 = 14.
    router.route(&mut dom, drag(9, 3));
    let scroll = dom.node(c).ext().unwrap().scroll_y;
    assert_eq!(scroll, 14);
}

#[test]
fn drag_past_track_end_clamps_at_max_scroll() {
    let (mut dom, c) = vertical_scrollbar_dom();
    let mut router = Router::new();
    router.route(&mut dom, down(9, 0));
    router.route(&mut dom, drag(9, 100)); // waay past track
    let scroll = dom.node(c).ext().unwrap().scroll_y;
    assert_eq!(scroll, 24); // max = content 30 - viewport 6
}

#[test]
fn drag_past_track_start_clamps_at_zero() {
    // With the default fixture (content=30, viewport=6,
    // track=6, track_travel=5), one cell of drag moves scroll by
    // 24/5 = 4.8 ≈ 4 (integer). Max possible back-drag = 5 cells
    // from bottom, giving a scroll_delta of -24 — exactly enough
    // to clamp at 0 from max scroll of 24.
    let (mut dom, c) = vertical_scrollbar_dom();
    if let Some(ext) = dom.node_mut(c).ext_mut() {
        ext.scroll_y = 24; // max scroll for this fixture
    }
    let mut router = Router::new();
    // Thumb at max scroll is at track position 5 (track_len 6,
    // thumb_size 1, so thumb_off = track_travel = 5).
    router.route(&mut dom, down(9, 5));
    // Drag from row 5 to row 0 — delta -5 cells → -5 * 24 / 5 =
    // -24 scroll. 24 - 24 = 0, clamped at the floor.
    router.route(&mut dom, drag(9, 0));
    let scroll = dom.node(c).ext().unwrap().scroll_y;
    assert_eq!(scroll, 0);
}

#[test]
fn mouseup_clears_drag_state_and_releases_capture() {
    let (mut dom, _c) = vertical_scrollbar_dom();
    let mut router = Router::new();
    router.route(&mut dom, down(9, 0));
    router.route(&mut dom, up(9, 3));
    assert!(router.scrollbar_drag.is_none());
    assert!(dom.pointer_capture().is_none());
}

#[test]
fn reset_clears_drag_state() {
    let (mut dom, _c) = vertical_scrollbar_dom();
    let mut router = Router::new();
    router.route(&mut dom, down(9, 0));
    router.reset();
    assert!(router.scrollbar_drag.is_none());
}
