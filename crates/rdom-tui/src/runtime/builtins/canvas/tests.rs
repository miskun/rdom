//! `<canvas>` + RenderContext tests.

use std::cell::Cell;
use std::rc::Rc;

use rdom_core::NodeId;

use crate::TuiDom;
use crate::layout::Size;
use crate::render::{Buffer, LayoutExt, PaintExt, Rect, Style};
use crate::runtime::builtins::canvas;
use crate::style::{CascadeExt, Color, Stylesheet, TuiStyle};

/// Cascade → layout → paint into a fresh `Buffer`. Drives a
/// single paint pass synchronously, so callbacks registered on
/// canvases fire once.
fn pipeline(dom: &mut TuiDom, sheet: &Stylesheet, viewport: Rect) -> Buffer {
    dom.cascade(sheet);
    dom.layout_dom(viewport);
    let mut buf = Buffer::empty(viewport);
    dom.paint_dom(&mut buf, viewport);
    buf
}

/// Read row `y` as a string, skipping spacer cells (wide-glyph
/// secondary halves). Mirrors the helper in `render/paint_pass/tests.rs`.
fn row(buf: &Buffer, y: u16) -> String {
    let mut s = String::new();
    for x in buf.area.x..buf.area.right() {
        if let Some(c) = buf.cell(x, y) {
            if c.is_spacer() {
                continue;
            }
            s.push_str(c.symbol());
        }
    }
    s
}

fn canvas_fixture(width: u16, height: u16) -> (TuiDom, NodeId, Stylesheet) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let cv = dom.create_element("canvas");
    dom.append_child(root, cv).unwrap();
    let sheet = Stylesheet::new().rule_unchecked(
        "canvas",
        TuiStyle::new()
            .width(Size::Fixed(width))
            .height(Size::Fixed(height)),
    );
    (dom, cv, sheet)
}

// ── Registration ──────────────────────────────────────────────────

#[test]
fn set_paint_stores_callback_on_canvas_element() {
    let (mut dom, cv, _sheet) = canvas_fixture(10, 3);
    assert!(!canvas::has_paint(&dom, cv));
    canvas::set_paint(&mut dom, cv, |_dom, _ctx| {});
    assert!(canvas::has_paint(&dom, cv));
}

#[test]
fn clear_paint_removes_callback() {
    let (mut dom, cv, _sheet) = canvas_fixture(10, 3);
    canvas::set_paint(&mut dom, cv, |_dom, _ctx| {});
    canvas::clear_paint(&mut dom, cv);
    assert!(!canvas::has_paint(&dom, cv));
}

#[test]
fn set_paint_replaces_previous_callback() {
    let (mut dom, cv, sheet) = canvas_fixture(10, 3);
    let first = Rc::new(Cell::new(0u32));
    let f1 = first.clone();
    canvas::set_paint(&mut dom, cv, move |_, _| {
        f1.set(f1.get() + 1);
    });

    let second = Rc::new(Cell::new(0u32));
    let f2 = second.clone();
    canvas::set_paint(&mut dom, cv, move |_, _| {
        f2.set(f2.get() + 1);
    });

    let _ = pipeline(&mut dom, &sheet, Rect::new(0, 0, 20, 5));
    assert_eq!(first.get(), 0);
    assert_eq!(second.get(), 1);
}

// ── Paint invocation ──────────────────────────────────────────────

#[test]
fn registered_paint_callback_fires_on_paint_pass() {
    let (mut dom, cv, sheet) = canvas_fixture(10, 3);
    let fired = Rc::new(Cell::new(0u32));
    let f = fired.clone();
    canvas::set_paint(&mut dom, cv, move |_, _| {
        f.set(f.get() + 1);
    });
    let _ = pipeline(&mut dom, &sheet, Rect::new(0, 0, 20, 5));
    assert_eq!(fired.get(), 1);
}

#[test]
fn paint_callback_can_fill_canvas_with_bg_style() {
    let (mut dom, cv, sheet) = canvas_fixture(5, 2);
    canvas::set_paint(&mut dom, cv, |_, ctx| {
        ctx.fill(Style::new().bg(Color::Rgb(255, 0, 0)));
    });
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 3));
    // Top-left of canvas is at (0, 0) in buffer coords. Check the
    // cell has the Red background.
    let cell = buf.cell(0, 0).unwrap();
    assert_eq!(cell.bg, Color::Rgb(255, 0, 0));
}

#[test]
fn paint_callback_writes_text_at_canvas_local_coords() {
    let (mut dom, cv, sheet) = canvas_fixture(20, 2);
    canvas::set_paint(&mut dom, cv, |_, ctx| {
        ctx.text(2, 0, "hello", Style::new());
    });
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 20, 2));
    // Canvas origin is (0, 0). Text written at canvas x=2 lands
    // at buffer x=2.
    assert_eq!(row(&buf, 0).trim_end(), "  hello");
}

#[test]
fn out_of_bounds_writes_are_silent() {
    let (mut dom, cv, sheet) = canvas_fixture(5, 2);
    canvas::set_paint(&mut dom, cv, |_, ctx| {
        // All well past the canvas edges — should NOT panic.
        ctx.set(100, 100, 'X', Style::new());
        ctx.text(50, 0, "oversized", Style::new());
        ctx.rect(0, 10, 999, 999, Style::new().bg(Color::Rgb(255, 255, 0)));
    });
    let _ = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 3));
}

#[test]
fn no_callback_falls_through_to_normal_paint() {
    // <canvas> with no registered callback renders its children
    // (HTML fallback-content behavior). Text child should appear.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let cv = dom.create_element("canvas");
    let t = dom.create_text_node("fallback");
    dom.append_child(cv, t).unwrap();
    dom.append_child(root, cv).unwrap();
    let sheet = Stylesheet::new().rule_unchecked(
        "canvas",
        TuiStyle::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(1)),
    );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 20, 1));
    assert!(row(&buf, 0).contains("fallback"));
}

#[test]
fn paint_callback_overrides_fallback_children() {
    // With both a callback AND fallback children, the callback
    // wins — the children don't paint.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let cv = dom.create_element("canvas");
    let t = dom.create_text_node("fallback");
    dom.append_child(cv, t).unwrap();
    dom.append_child(root, cv).unwrap();
    canvas::set_paint(&mut dom, cv, |_, ctx| {
        ctx.text(0, 0, "custom", Style::new());
    });
    let sheet = Stylesheet::new().rule_unchecked(
        "canvas",
        TuiStyle::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(1)),
    );
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 20, 1));
    let r = row(&buf, 0);
    assert!(r.contains("custom"), "got: {:?}", r);
    assert!(
        !r.contains("fallback"),
        "fallback should not render, got: {:?}",
        r
    );
}

// ── RenderContext semantics ───────────────────────────────────────

#[test]
fn render_context_reports_canvas_dimensions() {
    let (mut dom, cv, sheet) = canvas_fixture(7, 3);
    let sizes = Rc::new(Cell::new((0u16, 0u16)));
    let s = sizes.clone();
    canvas::set_paint(&mut dom, cv, move |_, ctx| {
        s.set((ctx.width(), ctx.height()));
    });
    let _ = pipeline(&mut dom, &sheet, Rect::new(0, 0, 20, 5));
    assert_eq!(sizes.get(), (7, 3));
}

#[test]
fn rect_fills_only_within_canvas_bounds() {
    let (mut dom, cv, sheet) = canvas_fixture(4, 2);
    canvas::set_paint(&mut dom, cv, |_, ctx| {
        // Request rect that overflows: (0,0,10,10) on a 4×2 canvas.
        ctx.rect(0, 0, 10, 10, Style::new().bg(Color::Rgb(255, 255, 0)));
    });
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 4));
    // Cells inside canvas (0..4, 0..2): Yellow bg.
    for y in 0..2 {
        for x in 0..4 {
            assert_eq!(buf.cell(x, y).unwrap().bg, Color::Rgb(255, 255, 0));
        }
    }
    // Cells outside canvas: default bg (Color::Reset — nothing
    // painted there).
    assert_eq!(buf.cell(5, 0).unwrap().bg, Color::Reset);
    assert_eq!(buf.cell(0, 3).unwrap().bg, Color::Reset);
}

#[test]
fn set_writes_single_cell_character() {
    let (mut dom, cv, sheet) = canvas_fixture(5, 1);
    canvas::set_paint(&mut dom, cv, |_, ctx| {
        ctx.set(2, 0, '★', Style::new().fg(Color::Rgb(255, 255, 0)));
    });
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 5, 1));
    assert_eq!(buf.cell(2, 0).unwrap().symbol(), "\u{2605}");
}

#[test]
fn sub_context_exposes_slice_at_local_origin() {
    let (mut dom, cv, sheet) = canvas_fixture(10, 4);
    canvas::set_paint(&mut dom, cv, |_, ctx| {
        // Carve out a 5×2 sub-rect at (3, 1) and paint (0,0) in
        // its local coords — that lands at buffer (3, 1).
        let mut sub = ctx.sub(3, 1, 5, 2);
        assert_eq!(sub.width(), 5);
        assert_eq!(sub.height(), 2);
        sub.set(0, 0, 'X', Style::new());
    });
    let buf = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 4));
    assert_eq!(buf.cell(3, 1).unwrap().symbol(), "X");
}

#[test]
fn sub_context_clamps_oversized_rect_to_parent_bounds() {
    let (mut dom, cv, sheet) = canvas_fixture(5, 2);
    canvas::set_paint(&mut dom, cv, |_, ctx| {
        // Request (0, 0, 100, 100) on a 5×2 canvas — clamps to 5×2.
        let sub = ctx.sub(0, 0, 100, 100);
        assert_eq!(sub.width(), 5);
        assert_eq!(sub.height(), 2);
    });
    let _ = pipeline(&mut dom, &sheet, Rect::new(0, 0, 10, 3));
}

// ── CanvasPaint PartialEq ─────────────────────────────────────────

#[test]
fn canvas_paint_equality_is_pointer_identity() {
    let paint = canvas::CanvasPaint::new(|_, _| {});
    let same = paint.clone();
    assert_eq!(paint, same);

    let different = canvas::CanvasPaint::new(|_, _| {});
    assert_ne!(paint, different);
}

// ── UA stylesheet baked-in ────────────────────────────────────────

#[test]
fn canvas_ua_rule_is_baked_in() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let cv = dom.create_element("canvas");
    dom.append_child(root, cv).unwrap();
    dom.cascade(&Stylesheet::new());
    use crate::node::TuiNodeExt;
    let computed = dom.node(cv).computed().cloned().unwrap();
    assert_eq!(computed.width, Size::Fixed(40));
    assert_eq!(computed.height, Size::Fixed(10));
}
