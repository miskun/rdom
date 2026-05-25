//! `setInterval` ticker — start/stop a counter that increments
//! every 200ms.
//!
//! Click "Start" → `ctx.set_interval(..., 200)` returns a
//! `TimerId`, stashed in shared state. Each tick the callback
//! increments a counter + writes the new value into a text
//! node. Click "Stop" → `clear_interval(id)`.
//!
//! Exercises the substrate's [`TuiTimers`] trait
//! (`set_interval` / `clear_interval`). The callback also
//! demonstrates mutation from within a timer tick (allowed —
//! unlike `MutationObserver` callbacks).

use std::cell::{Cell, RefCell};
use std::io;
use std::rc::Rc;

use rdom_tui::runtime::timers::{TimerId, TuiTimers};
use rdom_tui::{App, ListenerOptions, NodeId, Stylesheet, TuiDom};

use crate::{Category, Demo, Source};

pub const MARKUP: &str = r#"<div class="interval-demo">
  <h1>setInterval ticker</h1>
  <p>200ms cadence. Click Start/Stop to toggle.</p>
  <button class="toggle-btn">Start</button>
  <p class="value-line">Counter: <span class="value">0</span></p>
</div>"#;

pub const CSS: &str = r#"
.interval-demo {
  flex: 1;
  flex-direction: column;
  padding: 1 2;
  gap: 1;
}
.interval-demo h1 {
  height: 1;
  color: rgb(180, 220, 255);
  font-weight: bold;
}
.interval-demo p {
  height: 1;
}
.interval-demo .value-line .value {
  color: rgb(220, 230, 255);
  font-weight: bold;
}
"#;

pub fn build(dom: &mut TuiDom) -> NodeId {
    let root = dom.create_element("div");
    dom.set_attribute(root, "class", "interval-demo").unwrap();

    let h1 = dom.create_element("h1");
    let h1_t = dom.create_text_node("setInterval ticker");
    dom.append_child(h1, h1_t).unwrap();
    dom.append_child(root, h1).unwrap();

    let p = dom.create_element("p");
    let p_t = dom.create_text_node("200ms cadence. Click Start/Stop to toggle.");
    dom.append_child(p, p_t).unwrap();
    dom.append_child(root, p).unwrap();

    let btn = dom.create_element("button");
    dom.set_attribute(btn, "class", "toggle-btn").unwrap();
    let btn_text = dom.create_text_node("Start");
    dom.append_child(btn, btn_text).unwrap();
    dom.append_child(root, btn).unwrap();

    // Counter line: "Counter: 0".
    let value_line = dom.create_element("p");
    dom.set_attribute(value_line, "class", "value-line")
        .unwrap();
    let prefix = dom.create_text_node("Counter: ");
    dom.append_child(value_line, prefix).unwrap();
    let value = dom.create_element("span");
    dom.set_attribute(value, "class", "value").unwrap();
    let value_text = dom.create_text_node("0");
    dom.append_child(value, value_text).unwrap();
    dom.append_child(value_line, value).unwrap();
    dom.append_child(root, value_line).unwrap();

    // Shared state. `count` is mutated on every tick; `running_id`
    // holds the active interval's TimerId (or None when stopped).
    let count = Rc::new(Cell::new(0u64));
    let running_id: Rc<RefCell<Option<TimerId>>> = Rc::new(RefCell::new(None));

    let count_for_click = Rc::clone(&count);
    let running_for_click = Rc::clone(&running_id);
    dom.add_event_listener(btn, "click", ListenerOptions::default(), move |ctx| {
        // Toggle: if running, stop. If stopped, start.
        let was_running = running_for_click.borrow().is_some();
        if was_running {
            // Stop.
            let id = running_for_click.borrow_mut().take().unwrap();
            ctx.clear_interval(id);
            // Update button label.
            let _ = ctx.dom.clear_children(btn);
            let new_text = ctx.dom.create_text_node("Start");
            let _ = ctx.dom.append_child(btn, new_text);
            return;
        }

        // Start. Schedule a recurring callback at 200ms cadence
        // that increments + rewrites the value's text. Returns
        // `true` to keep firing.
        let count_for_tick = Rc::clone(&count_for_click);
        let id = ctx.set_interval(
            move |tick_ctx: &mut rdom_tui::runtime::timers::TimerCtx<'_>| {
                let n = count_for_tick.get() + 1;
                count_for_tick.set(n);
                let _ = tick_ctx.dom.clear_children(value);
                let new_text = tick_ctx.dom.create_text_node(&n.to_string());
                let _ = tick_ctx.dom.append_child(value, new_text);
                true // keep firing
            },
            200,
        );
        *running_for_click.borrow_mut() = Some(id);
        // Update button label.
        let _ = ctx.dom.clear_children(btn);
        let new_text = ctx.dom.create_text_node("Stop");
        let _ = ctx.dom.append_child(btn, new_text);
    })
    .unwrap();

    root
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

pub struct IntervalCounter;

impl Demo for IntervalCounter {
    fn slug(&self) -> &'static str {
        "animations/interval-counter"
    }

    fn title(&self) -> &'static str {
        "setInterval ticker"
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
