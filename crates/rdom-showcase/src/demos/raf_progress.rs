//! `requestAnimationFrame` demo — a progress bar that fills smoothly
//! over 2 seconds, driven by rAF callbacks.
//!
//! Click "Start" → schedules the first rAF; each callback computes
//! the elapsed fraction (`(now - start) / 2000`), writes the new
//! width into the bar's inline style, and re-schedules the next rAF
//! if not yet at 100%. Click "Reset" → cancels any pending rAF and
//! zeros the bar.
//!
//! rAF callbacks receive a `DOMHighResTimeStamp`-equivalent
//! (`f64` ms since `App::new`); all rAFs that drain in the same
//! tick observe the same timestamp (browser-faithful per HTML
//! spec).

use std::cell::{Cell, RefCell};
use std::io;
use std::rc::Rc;

use rdom_tui::runtime::timers::{TimerId, TuiTimers};
use rdom_tui::{App, ListenerOptions, NodeId, Stylesheet, TuiDom};

use crate::{Category, Demo, Source};

const DURATION_MS: f64 = 2000.0;

pub const MARKUP: &str = r#"<div class="raf-demo">
  <h1>requestAnimationFrame</h1>
  <p>2-second smooth fill driven by per-frame rAF callbacks.</p>
  <div class="row">
    <button class="start-btn">Start</button>
    <button class="reset-btn">Reset</button>
  </div>
  <div class="track">
    <div class="bar" style="width: 0"></div>
  </div>
</div>"#;

pub const CSS: &str = r#"
.raf-demo {
  flex: 1;
  flex-direction: column;
  padding: 1 2;
  gap: 1;
}
.raf-demo h1 {
  height: 1;
  color: rgb(180, 220, 255);
  font-weight: bold;
}
.raf-demo .row {
  flex-direction: row;
  gap: 2;
  height: 1;
}
.raf-demo .track {
  height: 1;
  width: 50;
  border: solid;
  border-color: rgb(120, 130, 150);
}
.raf-demo .track .bar {
  height: 1;
  background: rgb(80, 160, 220);
}
"#;

pub fn build(dom: &mut TuiDom) -> NodeId {
    let root = dom.create_element("div");
    dom.set_attribute(root, "class", "raf-demo").unwrap();

    let h1 = dom.create_element("h1");
    let h1_t = dom.create_text_node("requestAnimationFrame");
    dom.append_child(h1, h1_t).unwrap();
    dom.append_child(root, h1).unwrap();

    let p = dom.create_element("p");
    let p_t = dom.create_text_node("2-second smooth fill driven by per-frame rAF callbacks.");
    dom.append_child(p, p_t).unwrap();
    dom.append_child(root, p).unwrap();

    let row = dom.create_element("div");
    dom.set_attribute(row, "class", "row").unwrap();
    let start_btn = dom.create_element("button");
    dom.set_attribute(start_btn, "class", "start-btn").unwrap();
    let start_t = dom.create_text_node("Start");
    dom.append_child(start_btn, start_t).unwrap();
    dom.append_child(row, start_btn).unwrap();
    let reset_btn = dom.create_element("button");
    dom.set_attribute(reset_btn, "class", "reset-btn").unwrap();
    let reset_t = dom.create_text_node("Reset");
    dom.append_child(reset_btn, reset_t).unwrap();
    dom.append_child(row, reset_btn).unwrap();
    dom.append_child(root, row).unwrap();

    let track = dom.create_element("div");
    dom.set_attribute(track, "class", "track").unwrap();
    let bar = dom.create_element("div");
    dom.set_attribute(bar, "class", "bar").unwrap();
    dom.set_attribute(bar, "style", "width: 0").unwrap();
    dom.append_child(track, bar).unwrap();
    dom.append_child(root, track).unwrap();

    // Shared state: animation start timestamp (set by Start) and
    // the most-recently-scheduled rAF id (cancellable by Reset).
    let started_at_ms: Rc<Cell<Option<f64>>> = Rc::new(Cell::new(None));
    let active_raf: Rc<RefCell<Option<TimerId>>> = Rc::new(RefCell::new(None));

    let started_for_start = Rc::clone(&started_at_ms);
    let active_for_start = Rc::clone(&active_raf);
    dom.add_event_listener(start_btn, "click", ListenerOptions::default(), move |ctx| {
        // Capture the start timestamp from the first rAF
        // (browsers do the same — the start is the timestamp
        // of the first frame that runs the animation).
        started_for_start.set(None);
        schedule_next(
            ctx,
            bar,
            Rc::clone(&started_for_start),
            Rc::clone(&active_for_start),
        );
    })
    .unwrap();

    let started_for_reset = Rc::clone(&started_at_ms);
    let active_for_reset = Rc::clone(&active_raf);
    dom.add_event_listener(reset_btn, "click", ListenerOptions::default(), move |ctx| {
        if let Some(id) = active_for_reset.borrow_mut().take() {
            ctx.cancel_animation_frame(id);
        }
        started_for_reset.set(None);
        ctx.dom
            .node_mut(bar)
            .set_attribute("style", "width: 0")
            .unwrap();
    })
    .unwrap();

    root
}

/// Schedule the next rAF tick. Each tick advances the bar's width
/// based on elapsed time; chains the next tick until 100% or
/// cancellation.
fn schedule_next(
    ctx: &mut rdom_tui::TuiEventCtx<'_>,
    bar: NodeId,
    started_at_ms: Rc<Cell<Option<f64>>>,
    active_raf: Rc<RefCell<Option<TimerId>>>,
) {
    // Clone the Rcs so the closure captures one set and we keep
    // one set to write the returned id into.
    let started_in = Rc::clone(&started_at_ms);
    let active_in = Rc::clone(&active_raf);
    let id = ctx.request_animation_frame(move |tick_ctx, now_ms| {
        let start = match started_in.get() {
            Some(s) => s,
            None => {
                // First tick: anchor the start at this timestamp.
                started_in.set(Some(now_ms));
                now_ms
            }
        };
        let elapsed = now_ms - start;
        let fraction = (elapsed / DURATION_MS).clamp(0.0, 1.0);
        let width_cells = (fraction * 50.0).round() as i32;
        let style = format!("width: {width_cells}");
        let _ = tick_ctx.dom.node_mut(bar).set_attribute("style", &style);

        if fraction < 1.0 {
            // Chain the next frame.
            schedule_next_via_timer_ctx(
                tick_ctx,
                bar,
                Rc::clone(&started_in),
                Rc::clone(&active_in),
            );
        } else {
            // Done — clear the active id so Reset doesn't try to
            // cancel a finished raf (no harm if it does).
            active_in.borrow_mut().take();
        }
    });
    *active_raf.borrow_mut() = Some(id);
}

fn schedule_next_via_timer_ctx(
    tick_ctx: &mut rdom_tui::runtime::timers::TimerCtx<'_>,
    bar: NodeId,
    started_at_ms: Rc<Cell<Option<f64>>>,
    active_raf: Rc<RefCell<Option<TimerId>>>,
) {
    let started_inner = Rc::clone(&started_at_ms);
    let active_inner = Rc::clone(&active_raf);
    let id = tick_ctx.request_animation_frame(move |inner_ctx, now_ms| {
        let start = match started_inner.get() {
            Some(s) => s,
            None => {
                started_inner.set(Some(now_ms));
                now_ms
            }
        };
        let elapsed = now_ms - start;
        let fraction = (elapsed / DURATION_MS).clamp(0.0, 1.0);
        let width_cells = (fraction * 50.0).round() as i32;
        let style = format!("width: {width_cells}");
        let _ = inner_ctx.dom.node_mut(bar).set_attribute("style", &style);

        if fraction < 1.0 {
            schedule_next_via_timer_ctx(
                inner_ctx,
                bar,
                Rc::clone(&started_inner),
                Rc::clone(&active_inner),
            );
        } else {
            active_inner.borrow_mut().take();
        }
    });
    *active_raf.borrow_mut() = Some(id);
}

pub fn stylesheet() -> Stylesheet {
    rdom_css::from_css(CSS)
}

pub fn run_standalone() -> io::Result<()> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let demo_root = build(&mut dom);
    dom.append_child(root, demo_root).unwrap();
    App::new(dom, stylesheet())?.run()
}

pub struct RafProgress;

impl Demo for RafProgress {
    fn slug(&self) -> &'static str {
        "animations/raf-progress"
    }

    fn title(&self) -> &'static str {
        "requestAnimationFrame"
    }

    fn category(&self) -> Category {
        Category::Animations
    }

    fn build(&self, dom: &mut TuiDom) -> NodeId {
        build(dom)
    }

    fn stylesheet(&self) -> Stylesheet {
        stylesheet()
    }

    fn source(&self) -> Source {
        Source {
            markup: MARKUP,
            css: CSS,
        }
    }
}
