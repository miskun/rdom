//! M3 demo — CSS transitions in rdom-tui, keyboard-driven.
//!
//! Four cards demonstrating four CSS transition timing functions
//! (linear / ease-in / ease-out / ease-in-out). Each card animates
//! its `border-color` from `--dim` to its own active color, using
//! its own duration and timing function. Press the matching number
//! key to **pulse** the card — `.active` is added, the forward
//! transition fires, and after the transition duration `setTimeout`
//! removes `.active` so the back transition fires. Single keypress
//! shows the whole timing curve end-to-end.
//!
//! Sticking to `border-color` keeps the demo focused on the timing
//! curves without the contrast hassle of changing fg/bg behind
//! text. The fg/bg of each card stay constant; only the border
//! pulses.
//!
//! Keyboard-driven on purpose: hover-driven transitions require a
//! terminal that emits mouse-motion events (mode 1003), which some
//! terminals — including macOS Terminal.app — don't reliably do.
//! Number-key pulses work everywhere, and they sidestep the
//! `FOCUS-1` issue (UA `:focus` invert on large block-level
//! focusables) since the cards aren't focusable at all.
//!
//! The animation engine ticks at the App's animation_frame_rate
//! (default ~60fps) only while transitions are in flight; idle
//! falls back to the regular tick_rate.
//!
//! Controls:
//!   1 — pulse linear (200ms forward + 200ms back)
//!   2 — pulse ease-in (300ms forward + 300ms back)
//!   3 — pulse ease-out (400ms forward + 400ms back)
//!   4 — pulse ease-in-out (600ms forward + 600ms back)
//!   Ctrl-C — exit
//!
//! Run with: `cargo run --example transitions_demo -p rdom-css`

use std::io;

use rdom_parser::parse_into;
use rdom_tui::ListenerOptions;
use rdom_tui::prelude::*;
use rdom_tui::runtime::timers::TuiTimers;
use rdom_tui::{extend_from_style_tags, seed_inline_styles};

const TEMPLATE: &str = r#"
<screen>
  <style>
    :root {
      --bg: #1f2123;
      --ink: #e0e0e0;
      --accent: #3d90ce;
      --warn: #c08040;
      --grass: #4a7a4a;
      --dim: #707070;
    }

    screen {
      display: block;
      padding: 1 2;
      gap: 1;
      background-color: var(--bg);
      color: var(--ink);
    }

    title {
      font-weight: bold;
      color: var(--accent);
      height: 1;
    }

    hint {
      color: var(--dim);
      height: 1;
    }

    card-row {
      display: block;
      gap: 2;
      flex-direction: row;
    }

    card {
      display: block;
      width: 1fr;
      padding: 1 2;
      border: rounded;
      border-color: var(--dim);
      background-color: var(--bg);
      color: var(--ink);
    }

    /* Each card transitions its `border-color` to a distinct
       active color, using its own timing function and duration.
       Sticking to border-color keeps the demo focused on the
       timing curves without the contrast hassle of changing
       fg/bg behind text. Press 1/2/3/4 to pulse. */
    card.linear {
      transition: border-color 200ms linear;
    }
    card.linear.active {
      border-color: var(--accent);
    }

    card.ease-in {
      transition: border-color 300ms ease-in;
    }
    card.ease-in.active {
      border-color: var(--warn);
    }

    card.ease-out {
      transition: border-color 400ms ease-out;
    }
    card.ease-out.active {
      border-color: var(--grass);
    }

    card.ease-in-out {
      transition: border-color 600ms ease-in-out;
    }
    card.ease-in-out.active {
      border-color: var(--accent);
    }

    label {
      font-weight: bold;
    }
  </style>

  <title>rdom M3 — CSS transitions demo</title>
  <hint>Press 1 / 2 / 3 / 4 to pulse each card (forward + back). Ctrl-C to exit.</hint>

  <card-row>
    <card class="linear">
      <label>1. linear</label>
      <hint>border-color over 200ms</hint>
    </card>
    <card class="ease-in">
      <label>2. ease-in</label>
      <hint>border-color over 300ms</hint>
    </card>
    <card class="ease-out">
      <label>3. ease-out</label>
      <hint>border-color over 400ms</hint>
    </card>
  </card-row>

  <card class="ease-in-out">
    <label>4. ease-in-out</label>
    <hint>border-color over 600ms</hint>
  </card>
</screen>
"#;

fn main() -> io::Result<()> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    parse_into(&mut dom, TEMPLATE, root).expect("template parses");

    let mut sheet = Stylesheet::new();
    let style_warnings = extend_from_style_tags(&dom, &mut sheet);
    let inline_warnings = seed_inline_styles(&mut dom);

    if !style_warnings.is_empty() {
        eprintln!("warnings from <style>: {style_warnings:?}");
    }
    if !inline_warnings.is_empty() {
        eprintln!("warnings from inline style: {inline_warnings:?}");
    }

    let sheet = sheet
        .rule(
            "screen",
            TuiStyle::new()
                .direction(Direction::Column)
                .width(Size::Flex(1))
                .height(Size::Flex(1)),
        )
        .unwrap();

    // Resolve the four cards by class — each is the unique element
    // with that class in this template.
    fn find_by_class(
        dom: &TuiDom,
        id: rdom_core::NodeId,
        class: &str,
    ) -> Option<rdom_core::NodeId> {
        let n = dom.node(id);
        if n.class_list().contains(class) {
            return Some(id);
        }
        for c in n.child_nodes() {
            if let Some(found) = find_by_class(dom, c.id(), class) {
                return Some(found);
            }
        }
        None
    }
    let linear = find_by_class(&dom, root, "linear").expect("linear card present");
    let ease_in = find_by_class(&dom, root, "ease-in").expect("ease-in card present");
    let ease_out = find_by_class(&dom, root, "ease-out").expect("ease-out card present");
    let ease_in_out = find_by_class(&dom, root, "ease-in-out").expect("ease-in-out card present");

    // Root-level keydown listener: number keys pulse the matching
    // card. Adds `.active` immediately (forward animation fires),
    // then schedules a removal after the transition duration so
    // the back animation fires from a clean end-state. Single
    // keypress shows the timing curve end-to-end.
    dom.add_event_listener(root, "keydown", ListenerOptions::default(), move |ctx| {
        let Some(kbd) = ctx.event.detail.as_keyboard() else {
            return;
        };
        let (target, duration_ms) = match kbd.key.as_str() {
            "1" => (linear, 200u32),
            "2" => (ease_in, 300u32),
            "3" => (ease_out, 400u32),
            "4" => (ease_in_out, 600u32),
            _ => return,
        };
        let _ = ctx.dom.add_class(target, "active");
        ctx.set_timeout(
            move |tctx| {
                let _ = tctx.dom.remove_class(target, "active");
            },
            duration_ms,
        );
    })
    .unwrap();

    App::new(dom, sheet)?.run()
}
