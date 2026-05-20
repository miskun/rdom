//! `<canvas>` — raw cell-buffer escape hatch for custom rendering.
//!
//! ## Contract (adapted from HTML)
//!
//! HTML's `<canvas>` is a 2D drawing surface with a
//! `getContext('2d')` API — apps call imperative methods
//! (`fillRect`, `fillText`, `beginPath` / `lineTo` / `stroke`, etc.)
//! to build up a bitmap. The rdom-tui equivalent replaces
//! "pixels and ImageData" with "cells and `Style`" and the imperative
//! context with a [`RenderContext`] handle bounded to the canvas's
//! paint rect.
//!
//! ## API sketch
//!
//! ```no_run
//! # use rdom_tui::runtime::builtins::canvas;
//! # use rdom_tui::render::Style;
//! # use rdom_tui::style::Color;
//! # use rdom_tui::TuiDom;
//! # let mut dom = TuiDom::new();
//! # let canvas_id = dom.create_element("canvas");
//! canvas::set_paint(&mut dom, canvas_id, |_dom, ctx| {
//!     ctx.fill(Style::new().bg(Color::Rgb(169, 169, 169)));
//!     ctx.rect(2, 1, 10, 3, Style::new().bg(Color::Rgb(0, 0, 255)));
//!     ctx.text(3, 2, "Hello", Style::new().fg(Color::Rgb(255, 255, 255)));
//! });
//! ```
//!
//! The callback is invoked every paint pass. Apps redraw from
//! scratch (same model as HTML `<canvas>` — the bitmap is not
//! preserved between frames). `RenderContext` clips silently for
//! out-of-bounds writes.
//!
//! ## Fallback content
//!
//! HTML canvas renders its DOM children when the feature is
//! unavailable (old browsers, JS disabled). rdom-tui mirrors this:
//! `<canvas>` with NO registered paint callback falls through to
//! the normal paint pass, so text children + CSS work normally.
//! Apps can place a fallback `<p>loading…</p>` inside the canvas
//! until their render function is ready.
//!
//! ## v1 deliberate simplifications
//!
//! - No stroke / path API (`moveTo` / `lineTo` / `stroke`) —
//!   `rect` / `set` / `text` cover most TUI needs; line drawing
//!   helpers (box-drawing characters, `h_line`, `v_line`) land
//!   in a follow-up.
//! - No save/restore state stacks — apps pass `Style` per call.
//! - No sixel / kitty graphics protocol — later.
//! - The callback cannot mutate the DOM during paint (same
//!   constraint as `MutationObserver::observe`). Apps that need
//!   reactive drawing call `AppHandle::needs_redraw()` when state
//!   changes; the next paint invokes the callback with the new
//!   state.

use std::fmt;
use std::rc::Rc;

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::render::{Buffer, Rect, Style};

/// Callback signature: receives a readonly `&Dom` (for reading
/// element state — attributes, selection, focus, etc.) and the
/// `&mut RenderContext` bounded to this canvas's paint rect.
pub type PaintFn = dyn Fn(&Dom<TuiExt>, &mut RenderContext<'_>);

/// Type-erased, shared, cloneable wrapper for the canvas paint
/// callback. Stored on `TuiExt.canvas_paint`. PartialEq compares
/// by pointer identity (equal iff both sides hold the same Rc
/// instance) — good enough for the change-detection dirty tracker.
#[derive(Clone)]
pub struct CanvasPaint(Rc<PaintFn>);

impl CanvasPaint {
    /// Wrap a callback. `pub(crate)` — external consumers register
    /// callbacks via [`set_paint`], which internally constructs
    /// the `CanvasPaint`. The type itself is `pub` so it can flow
    /// through `TuiExt.canvas_paint` reads, but it's opaque to
    /// external code (no public construction, no public invocation).
    pub(crate) fn new<F>(f: F) -> Self
    where
        F: Fn(&Dom<TuiExt>, &mut RenderContext<'_>) + 'static,
    {
        CanvasPaint(Rc::new(f))
    }

    /// Invoke the stored callback. Internal — paint pass use.
    pub(crate) fn call(&self, dom: &Dom<TuiExt>, ctx: &mut RenderContext<'_>) {
        (self.0)(dom, ctx);
    }
}

impl fmt::Debug for CanvasPaint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CanvasPaint(…)")
    }
}

impl PartialEq for CanvasPaint {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for CanvasPaint {}

/// Register a paint callback on a `<canvas>` element. Replaces
/// any previously-registered callback. No-op on non-canvas
/// elements (silent — matches the forgiving builder-chain style).
pub fn set_paint<F>(dom: &mut crate::TuiDom, canvas: NodeId, f: F)
where
    F: Fn(&Dom<TuiExt>, &mut RenderContext<'_>) + 'static,
{
    let paint = CanvasPaint::new(f);
    if let Some(ext) = dom.node_mut(canvas).ext_mut() {
        ext.canvas_paint = Some(paint);
    }
}

/// Remove the paint callback from a `<canvas>`. Subsequent paints
/// fall through to the normal paint pass (renders fallback text
/// children per HTML's unsupported-canvas behavior).
pub fn clear_paint(dom: &mut crate::TuiDom, canvas: NodeId) {
    if let Some(ext) = dom.node_mut(canvas).ext_mut() {
        ext.canvas_paint = None;
    }
}

/// True when the element has a registered paint callback.
pub fn has_paint(dom: &crate::TuiDom, canvas: NodeId) -> bool {
    dom.node(canvas)
        .ext()
        .map(|e| e.canvas_paint.is_some())
        .unwrap_or(false)
}

// ── RenderContext ──────────────────────────────────────────────────

/// Bounded handle to the cell buffer for painting a single
/// `<canvas>`. The app's paint callback uses canvas-local
/// coordinates (`(0, 0)` is the top-left of the canvas's content
/// rect); `RenderContext` translates to buffer coords and clips
/// silently against both the canvas bounds AND the current paint
/// clip (so canvases partially off-screen don't stomp on chrome).
pub struct RenderContext<'a> {
    buffer: &'a mut Buffer,
    /// Canvas's content-rect top-left in buffer coords. May be
    /// outside the buffer when the canvas is partially off-screen;
    /// clipping handles the negative case.
    origin_x: i32,
    origin_y: i32,
    /// Canvas's full content size in cells. Apps see this as
    /// `width()` / `height()` regardless of how much is actually
    /// visible.
    width: u16,
    height: u16,
    /// Paint clip in buffer coords — writes outside are dropped.
    clip: Rect,
}

impl<'a> RenderContext<'a> {
    /// Construct a render context for paint-pass use. Internal.
    pub(crate) fn new(
        buffer: &'a mut Buffer,
        origin_x: i32,
        origin_y: i32,
        width: u16,
        height: u16,
        clip: Rect,
    ) -> Self {
        Self {
            buffer,
            origin_x,
            origin_y,
            width,
            height,
            clip,
        }
    }

    /// Canvas width in cells. Apps use `0..width()` as the x range.
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Canvas height in cells. Apps use `0..height()` as the y range.
    pub fn height(&self) -> u16 {
        self.height
    }

    /// Set a single cell at canvas-local `(x, y)`. Silent no-op
    /// when out of bounds (either past the canvas edge or past
    /// the paint clip).
    pub fn set(&mut self, x: u16, y: u16, ch: char, style: Style) {
        if x >= self.width || y >= self.height {
            return;
        }
        let ax = self.origin_x + x as i32;
        let ay = self.origin_y + y as i32;
        if !self.in_clip(ax, ay) {
            return;
        }
        self.buffer.set_char(ax as u16, ay as u16, ch, style);
    }

    /// Write a string starting at `(x, y)`. Unicode-width-aware
    /// (wide glyphs take 2 cells). Silently clips at the canvas's
    /// right edge.
    pub fn text(&mut self, x: u16, y: u16, s: &str, style: Style) {
        if y >= self.height || x >= self.width {
            return;
        }
        let ax = self.origin_x + x as i32;
        let ay = self.origin_y + y as i32;
        // Canvas-local max width (right-edge of canvas minus x).
        let canvas_budget = self.width - x;
        // Clip-local max width (right-edge of clip minus ax).
        let clip_right = self.clip.right() as i32;
        let clip_budget = (clip_right - ax).max(0) as u16;
        let max = canvas_budget.min(clip_budget);
        if max == 0 {
            return;
        }
        // Skip if above or below the clip band.
        if ay < self.clip.y as i32 || ay >= self.clip.bottom() as i32 {
            return;
        }
        // Skip if starting x is left of the clip; non-trivial to
        // partial-render so just drop when out of left edge.
        if ax < self.clip.x as i32 {
            return;
        }
        self.buffer.set_stringn(ax as u16, ay as u16, s, max, style);
    }

    /// Fill a rectangle with `style` (background fill + space
    /// character). Silent on out-of-bounds.
    pub fn rect(&mut self, x: u16, y: u16, w: u16, h: u16, style: Style) {
        let x_end = x.saturating_add(w).min(self.width);
        let y_end = y.saturating_add(h).min(self.height);
        for yy in y..y_end {
            for xx in x..x_end {
                self.set(xx, yy, ' ', style);
            }
        }
    }

    /// Fill the whole canvas with `style`. Convenience over
    /// `rect(0, 0, width(), height(), style)`.
    pub fn fill(&mut self, style: Style) {
        self.rect(0, 0, self.width, self.height, style);
    }

    /// Clear the whole canvas to a blank (space + default style).
    /// Equivalent to `fill(Style::default())`.
    pub fn clear(&mut self) {
        self.fill(Style::new());
    }

    /// Slice off a sub-rectangle of this canvas as its own
    /// `RenderContext`. The returned context uses local coords
    /// — `(0, 0)` is the sub-rect's top-left, `width()` /
    /// `height()` return the sub-rect dimensions.
    ///
    /// Useful when an app wants to delegate part of its canvas
    /// to a sub-renderer — e.g. a virtualized-list canvas
    /// passing a row-sized slice to a per-row callback so the
    /// callback can paint at local `(0, 0)` without doing its
    /// own origin math.
    ///
    /// Clamps: if `(x, y, w, h)` extends beyond this canvas,
    /// the sub-context is shrunk to fit. The underlying paint
    /// clip carries through unchanged (still honored by writes).
    pub fn sub<'b>(&'b mut self, x: u16, y: u16, w: u16, h: u16) -> RenderContext<'b> {
        let clamped_w = self.width.saturating_sub(x).min(w);
        let clamped_h = self.height.saturating_sub(y).min(h);
        RenderContext::new(
            &mut *self.buffer,
            self.origin_x + x as i32,
            self.origin_y + y as i32,
            clamped_w,
            clamped_h,
            self.clip,
        )
    }

    // ── Clip arithmetic ────────────────────────────────────────────

    fn in_clip(&self, x: i32, y: i32) -> bool {
        x >= self.clip.x as i32
            && x < self.clip.right() as i32
            && y >= self.clip.y as i32
            && y < self.clip.bottom() as i32
    }
}

#[cfg(test)]
mod tests;
