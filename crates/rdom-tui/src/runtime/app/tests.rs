//! `App` tests — drive the event loop manually via `handle_event`
//! and `draw_if_dirty` against a `TestBackend`. Real `run` uses
//! crossterm polling which we can't easily stub in unit tests; the
//! surface exposed here is enough to exercise the App + loop
//! scenarios end-to-end.

use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MouseButton,
    MouseEvent as CtMouseEvent, MouseEventKind,
};
use rdom_core::{ListenerOptions, NodeId};
use std::cell::Cell;
use std::rc::Rc;

use crate::TuiDom;
use crate::layout::Size;
use crate::render::{Rect, Terminal, TestBackend};
use crate::runtime::app::{App, AppContext, ControlFlow};
use crate::style::{Color, Stylesheet, TuiStyle};

// ── Helpers ─────────────────────────────────────────────────────────

fn test_app(dom: TuiDom, sheet: Stylesheet, viewport: Rect) -> App<TestBackend> {
    let backend = TestBackend::new(viewport.width, viewport.height);
    let terminal = Terminal::new(backend).unwrap();
    App::with_backend(dom, sheet, terminal).unwrap()
}

fn ctrl_c() -> CtEvent {
    CtEvent::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL))
}

fn key(code: KeyCode) -> CtEvent {
    CtEvent::Key(KeyEvent::new(code, KeyModifiers::empty()))
}

/// Synthesize a `KeyEventKind::Release` event — used to drive the
/// `keyup` dispatch path. Terminals that don't enable the kitty
/// keyboard protocol won't actually send Release events; tests
/// construct them directly.
fn key_release(code: KeyCode) -> CtEvent {
    CtEvent::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Release,
        state: KeyEventState::empty(),
    })
}

fn click_at(x: u16, y: u16) -> Vec<CtEvent> {
    vec![
        CtEvent::Mouse(CtMouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: x,
            row: y,
            modifiers: KeyModifiers::empty(),
        }),
        CtEvent::Mouse(CtMouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: x,
            row: y,
            modifiers: KeyModifiers::empty(),
        }),
    ]
}

// ── Construction + initial state ────────────────────────────────────

#[test]
fn fresh_app_starts_with_redraw_requested() {
    let dom: TuiDom = TuiDom::new();
    let app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5));
    assert!(app.needs_redraw(), "initial paint must fire");
    assert!(!app.should_quit());
}

#[test]
fn draw_if_dirty_clears_the_flag() {
    let dom: TuiDom = TuiDom::new();
    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5));
    app.draw_if_dirty().unwrap();
    assert!(!app.needs_redraw());
}

#[test]
fn draw_if_dirty_no_op_when_not_dirty() {
    let dom: TuiDom = TuiDom::new();
    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5));
    app.draw_if_dirty().unwrap(); // initial
    let before_bytes = app.terminal().backend().bytes().to_vec();
    app.draw_if_dirty().unwrap();
    let after_bytes = app.terminal().backend().bytes().to_vec();
    assert_eq!(before_bytes, after_bytes, "second draw emitted nothing new");
}

// ── Ctrl-C exits ────────────────────────────────────────────────────

#[test]
fn ctrl_c_sets_should_quit() {
    let dom: TuiDom = TuiDom::new();
    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5));
    app.handle_event(ctrl_c());
    assert!(app.should_quit());
}

// ── Key routing to focused element ──────────────────────────────────

#[test]
fn key_dispatches_to_focused_element() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let btn = dom.create_element("button");
    dom.append_child(root, btn).unwrap();
    dom.set_focused(Some(btn));

    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    dom.add_event_listener(btn, "keydown", ListenerOptions::default(), move |_| {
        f.set(true);
    })
    .unwrap();

    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5));
    app.handle_event(key(KeyCode::Enter));
    assert!(fired.get(), "focused element received keydown");
}

#[test]
fn resize_dispatches_resize_event_on_root() {
    // M5 D4: terminal resize dispatches `resize` on the document
    // root. Coalesced per render step (one event per Resize signal,
    // not per cell delta).
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();

    let fired = Rc::new(Cell::new(0u32));
    let f = fired.clone();
    dom.add_event_listener(root, "resize", ListenerOptions::default(), move |_| {
        f.set(f.get() + 1);
    })
    .unwrap();

    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5));
    app.handle_event(CtEvent::Resize(40, 10));
    app.handle_event(CtEvent::Resize(60, 15));

    assert_eq!(
        fired.get(),
        2,
        "resize fires once per Resize event (no coalescing across events)"
    );
}

#[test]
fn key_release_dispatches_keyup_to_focused_element() {
    // M5 D1: Release events fire `keyup`, not `keydown`. The pairing
    // mirrors keydown/keyup dispatch in the DOM.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let btn = dom.create_element("button");
    dom.append_child(root, btn).unwrap();
    dom.set_focused(Some(btn));

    let keydown_fired = Rc::new(Cell::new(false));
    let keyup_fired = Rc::new(Cell::new(false));
    let kd = keydown_fired.clone();
    let ku = keyup_fired.clone();
    dom.add_event_listener(btn, "keydown", ListenerOptions::default(), move |_| {
        kd.set(true);
    })
    .unwrap();
    dom.add_event_listener(btn, "keyup", ListenerOptions::default(), move |_| {
        ku.set(true);
    })
    .unwrap();

    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5));
    app.handle_event(key_release(KeyCode::Enter));

    assert!(!keydown_fired.get(), "Release event must NOT fire keydown");
    assert!(
        keyup_fired.get(),
        "Release event fires keyup on the focused element"
    );
}

#[test]
fn key_release_does_not_run_default_actions() {
    // M5 D1: Default actions (tab focus traversal, editable key
    // handling, selection keyboard, etc.) run only on Press. A
    // Release event with KeyCode::Tab must NOT advance focus.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("button");
    let b = dom.create_element("button");
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();
    dom.set_attribute(a, "tabindex", "0").unwrap();
    dom.set_attribute(b, "tabindex", "0").unwrap();
    dom.set_focused(Some(a));

    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5));
    app.handle_event(key_release(KeyCode::Tab));

    assert_eq!(
        app.dom().focused(),
        Some(a),
        "Tab Release must not advance focus (only Press does)"
    );
}

#[test]
fn shift_f10_dispatches_contextmenu_on_focused_element() {
    // M5 D2: Shift+F10 is the keyboard equivalent of right-click —
    // browsers dispatch `contextmenu` on the focused element.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let btn = dom.create_element("button");
    dom.append_child(root, btn).unwrap();
    dom.set_focused(Some(btn));

    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    dom.add_event_listener(btn, "contextmenu", ListenerOptions::default(), move |_| {
        f.set(true);
    })
    .unwrap();

    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5));
    app.handle_event(CtEvent::Key(KeyEvent::new(
        KeyCode::F(10),
        KeyModifiers::SHIFT,
    )));

    assert!(
        fired.get(),
        "Shift+F10 fires contextmenu on focused element"
    );
}

#[test]
fn key_with_no_focus_dispatches_to_root() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();

    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    dom.add_event_listener(root, "keydown", ListenerOptions::default(), move |_| {
        f.set(true);
    })
    .unwrap();

    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5));
    app.handle_event(key(KeyCode::Char('x')));
    assert!(fired.get());
}

// ── Resize triggers redraw ──────────────────────────────────────────

#[test]
fn resize_event_marks_redraw() {
    let dom: TuiDom = TuiDom::new();
    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5));
    app.draw_if_dirty().unwrap(); // initial; clears flag
    assert!(!app.needs_redraw());

    app.handle_event(CtEvent::Resize(40, 10));
    assert!(app.needs_redraw(), "resize must request redraw");
}

// ── Mouse routing via App → Router ──────────────────────────────────

#[test]
fn click_through_app_reaches_handler() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let btn = dom.create_element("btn");
    dom.append_child(root, btn).unwrap();

    let clicks = Rc::new(Cell::new(0));
    let c = clicks.clone();
    dom.add_event_listener(btn, "click", ListenerOptions::default(), move |_| {
        c.set(c.get() + 1);
    })
    .unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "btn",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(3)),
    );
    let mut app = test_app(dom, sheet, Rect::new(0, 0, 20, 10));
    app.draw_if_dirty().unwrap(); // run cascade + layout so hit-test works

    for ev in click_at(3, 1) {
        app.handle_event(ev);
    }
    assert_eq!(clicks.get(), 1);
}

#[test]
fn mouse_hover_transition_sets_needs_redraw() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(3)),
    );
    let mut app = test_app(dom, sheet, Rect::new(0, 0, 20, 10));
    app.draw_if_dirty().unwrap();
    assert!(!app.needs_redraw());

    app.handle_event(CtEvent::Mouse(CtMouseEvent {
        kind: MouseEventKind::Moved,
        column: 3,
        row: 1,
        modifiers: KeyModifiers::empty(),
    }));
    assert!(app.needs_redraw(), "hover transition requests redraw");
}

// ── on_tick callback + ControlFlow ──────────────────────────────────

#[test]
fn on_tick_fires_and_can_request_quit() {
    let dom: TuiDom = TuiDom::new();
    let fired = Rc::new(Cell::new(0));
    let f = fired.clone();
    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5)).on_tick(
        move |_ctx: &mut AppContext<'_>| {
            f.set(f.get() + 1);
            if f.get() >= 2 {
                ControlFlow::Quit
            } else {
                ControlFlow::Continue
            }
        },
    );

    app.tick();
    assert_eq!(fired.get(), 1);
    assert!(!app.should_quit());

    app.tick();
    assert_eq!(fired.get(), 2);
    assert!(app.should_quit(), "ControlFlow::Quit propagates");
}

#[test]
fn on_tick_can_mutate_dom_through_context() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();

    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5)).on_tick(
        move |ctx: &mut AppContext<'_>| {
            // Create an attribute via the context's DOM handle. This
            // flows through MutationObserver → DirtyTracker so the
            // next paint re-cascades the affected subtree.
            let _ = ctx.dom.set_attribute(div, "data-touched", "1");
            ControlFlow::Continue
        },
    );

    app.tick();
    // DirtyTracker should have recorded the node as dirty.
    assert!(!app.dirty_roots_snapshot().is_empty());
}

#[test]
fn context_request_redraw_forces_paint_on_next_cycle() {
    let dom: TuiDom = TuiDom::new();
    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5)).on_tick(
        |ctx: &mut AppContext<'_>| {
            ctx.request_redraw();
            ControlFlow::Continue
        },
    );

    app.draw_if_dirty().unwrap();
    assert!(!app.needs_redraw());
    app.tick();
    assert!(app.needs_redraw());
}

#[test]
fn context_quit_sets_should_quit() {
    let dom: TuiDom = TuiDom::new();
    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5)).on_tick(
        |ctx: &mut AppContext<'_>| {
            ctx.quit();
            ControlFlow::Continue // even returning Continue, ctx.quit() wins
        },
    );
    app.tick();
    assert!(app.should_quit());
}

// ── Frame coalescing: many mutations → one paint ────────────────────

#[test]
fn multiple_mutations_coalesce_into_one_paint() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let mut children = Vec::new();
    for _ in 0..5 {
        let id = dom.create_element("span");
        dom.append_child(root, id).unwrap();
        children.push(id);
    }

    let child_refs: Vec<NodeId> = children.clone();
    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5)).on_tick(
        move |ctx: &mut AppContext<'_>| {
            // Mutate 5 element nodes in one tick. `set_attribute`
            // fires `AttributeChanged` which the DirtyTracker
            // observer marks dirty directly on each element —
            // pinning the "many mutations → one drain" contract.
            for &id in &child_refs {
                let _ = ctx.dom.set_attribute(id, "data-touched", "1");
            }
            ControlFlow::Continue
        },
    );

    app.draw_if_dirty().unwrap();
    // Coalescing signal: after `tick` queues 5 mutations, the
    // DirtyTracker's roots list holds N entries (at most 5, some
    // may dedupe via the ancestor check). After `draw_if_dirty`,
    // the tracker is drained to empty — proving one draw consumed
    // the whole batch. (We don't count emitted bytes: with BSU/
    // ESU removed, an empty diff emits zero bytes, breaking a
    // bytes-based heuristic.)
    app.tick();
    let roots_after_tick = app.dirty_roots_snapshot();
    assert!(
        !roots_after_tick.is_empty(),
        "tick should leave dirty roots from the 5 attribute mutations"
    );
    app.draw_if_dirty().unwrap();
    assert!(
        app.dirty_roots_snapshot().is_empty(),
        "draw_if_dirty must drain the dirty roots in one pass — coalescing 5 \
         mutations into one paint cycle"
    );
    // After the draw, flags reset.
    assert!(!app.needs_redraw());
}

// ── App::style_sheets() — D-M4-6 retirement ─────────────────────────

#[test]
fn style_sheets_returns_single_construction_sheet() {
    // App stores stylesheets in a `Vec<Stylesheet>`; v0.1.0 ships a
    // single-element vec built from the `Stylesheet` passed to
    // `App::new`/`App::with_backend`. The public accessor returns
    // the slice. Spec-name parity with `Document.styleSheets`.
    let dom: TuiDom = TuiDom::new();
    let sheet = Stylesheet::bare().rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
    let initial_rule_count = sheet.rules().len();
    let app = test_app(dom, sheet, Rect::new(0, 0, 20, 5));
    let sheets = app.style_sheets();
    assert_eq!(sheets.len(), 1);
    // The constructed sheet is preserved in slot 0; its rule
    // count round-trips.
    assert_eq!(sheets[0].rules().len(), initial_rule_count);
    assert_eq!(initial_rule_count, 1, "bare + one rule = 1 rule total");
}

#[test]
fn set_stylesheet_replaces_primary_in_place() {
    // `set_stylesheet` operates on index 0 of the vec. The vec
    // length stays at 1; the content is the replacement.
    let dom: TuiDom = TuiDom::new();
    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5));
    assert_eq!(app.style_sheets().len(), 1);
    assert_eq!(app.style_sheets()[0].rules().len(), 0, "bare = 0 rules");

    app.set_stylesheet(
        Stylesheet::bare().rule_unchecked("span", TuiStyle::new().fg(Color::Rgb(0, 0, 255))),
    );
    assert_eq!(app.style_sheets().len(), 1, "still single-element");
    assert_eq!(
        app.style_sheets()[0].rules().len(),
        1,
        "replacement sheet (1 rule) is now in slot 0"
    );
}

// ── Multi-slot stylesheet API (M1 D1) ───────────────────────────────

#[test]
fn push_stylesheet_appends_and_returns_distinct_ids() {
    // Construction puts one sheet in slot 0. Each push appends; each
    // push gets back a distinct StylesheetId.
    let dom: TuiDom = TuiDom::new();
    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5));
    assert_eq!(app.style_sheets().len(), 1, "construction sheet present");

    let id1 = app.push_stylesheet(Stylesheet::bare());
    let id2 = app.push_stylesheet(Stylesheet::bare());

    assert_eq!(app.style_sheets().len(), 3, "two pushes appended");
    assert_ne!(id1, id2, "each push gets a distinct id");
}

#[test]
fn remove_stylesheet_removes_only_that_slot() {
    // Push three sheets, remove the middle one, verify the other two
    // stay and order is preserved.
    let dom: TuiDom = TuiDom::new();
    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5));
    let id_a = app.push_stylesheet(
        Stylesheet::bare().rule_unchecked("a", TuiStyle::new().fg(Color::Rgb(1, 0, 0))),
    );
    let id_b = app.push_stylesheet(
        Stylesheet::bare().rule_unchecked("b", TuiStyle::new().fg(Color::Rgb(2, 0, 0))),
    );
    let _id_c = app.push_stylesheet(
        Stylesheet::bare().rule_unchecked("c", TuiStyle::new().fg(Color::Rgb(3, 0, 0))),
    );
    assert_eq!(app.style_sheets().len(), 4);

    app.remove_stylesheet(id_b);

    let sheets = app.style_sheets();
    assert_eq!(sheets.len(), 3, "one removed");
    // Slot 0 = construction (bare). Slot 1 = A. Slot 2 = C (shifted).
    assert_eq!(sheets[1].rules().len(), 1, "A still present at index 1");
    assert_eq!(sheets[2].rules().len(), 1, "C shifted to index 2");

    app.remove_stylesheet(id_a);
    assert_eq!(app.style_sheets().len(), 2);
}

#[test]
fn remove_stylesheet_unknown_id_is_noop() {
    // Removing a stale (already-removed) id is a no-op — no panic, no
    // state change. Same contract for an id from a different App.
    let dom: TuiDom = TuiDom::new();
    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5));
    let id = app.push_stylesheet(Stylesheet::bare());
    app.remove_stylesheet(id);
    let len_before = app.style_sheets().len();
    app.remove_stylesheet(id);
    assert_eq!(app.style_sheets().len(), len_before, "stale id no-op");
}

#[test]
fn later_pushed_sheet_wins_same_specificity() {
    // The cascade-order proof. Construction sheet: div → red.
    // Pushed sheet: div → blue. Same selector specificity; push
    // order is the tiebreaker — blue wins. Confirms that
    // additional sheets actually participate in the cascade, not
    // just sit on the side.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();

    let base = Stylesheet::bare().rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
    let mut app = test_app(dom, base, Rect::new(0, 0, 20, 5));
    let _blue = app.push_stylesheet(
        Stylesheet::bare().rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(0, 0, 255))),
    );
    app.draw_if_dirty().unwrap();

    let fg = crate::style::cascade::computed_of(app.dom(), div).fg;
    assert_eq!(
        fg,
        Color::Rgb(0, 0, 255),
        "later-pushed sheet wins same-specificity contest"
    );
}

#[test]
fn remove_stylesheet_triggers_recascade() {
    // Push a sheet that overrides the construction sheet, paint, then
    // remove the pushed sheet — the construction sheet's value must
    // come back. Proves removal actually invalidates the cascade.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();

    let base = Stylesheet::bare().rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(255, 0, 0)));
    let mut app = test_app(dom, base, Rect::new(0, 0, 20, 5));
    let blue = app.push_stylesheet(
        Stylesheet::bare().rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(0, 0, 255))),
    );
    app.draw_if_dirty().unwrap();
    assert_eq!(
        crate::style::cascade::computed_of(app.dom(), div).fg,
        Color::Rgb(0, 0, 255),
        "blue while pushed"
    );

    app.remove_stylesheet(blue);
    app.draw_if_dirty().unwrap();
    assert_eq!(
        crate::style::cascade::computed_of(app.dom(), div).fg,
        Color::Rgb(255, 0, 0),
        "back to base red after removal"
    );
}

#[test]
fn stylesheet_mutation_forces_full_cascade_even_when_dom_is_already_dirty() {
    // The contract: push/remove/set_stylesheet "force a full re-cascade
    // on the next paint." Latent v0.1.0 bug — invalidate_cascade
    // peeked instead of draining the dirty tracker, so when the DOM
    // had a dirty subtree at the time of stylesheet mutation, the
    // next paint did a *partial* cascade rooted at the dirty subtree
    // and skipped every other element. Elements outside the dirty
    // subtree kept stale computed styles from the previous sheet
    // stack.
    //
    // Repro: two siblings <a> and <b>; mutate only <b> after the
    // initial cascade; then remove the sheet that gave <a> its
    // color. If the bug is live, <a>'s computed.fg stays at the
    // pre-removal red. If the contract holds, <a> re-cascades to
    // initial.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();

    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5));
    let colors = app.push_stylesheet(
        Stylesheet::bare()
            .rule_unchecked("a", TuiStyle::new().fg(Color::Rgb(255, 0, 0)))
            .rule_unchecked("b", TuiStyle::new().fg(Color::Rgb(0, 255, 0))),
    );
    app.draw_if_dirty().unwrap();
    assert_eq!(
        crate::style::cascade::computed_of(app.dom(), a).fg,
        Color::Rgb(255, 0, 0),
        "<a> picks up red from the pushed sheet"
    );

    // Dirty only <b>. <a> is *not* in the tracker's dirty set.
    app.dom_mut().set_attribute(b, "class", "x").unwrap();

    // Remove the colors sheet. The contract: full re-cascade.
    app.remove_stylesheet(colors);
    app.draw_if_dirty().unwrap();

    assert_eq!(
        crate::style::cascade::computed_of(app.dom(), a).fg,
        rdom_tui_initial_fg(),
        "<a> must re-cascade to initial — the sheet that produced red is gone, \
         even though <a> wasn't in the dirty tracker when the sheet was removed",
    );
}

/// Helper: the cascade's `ComputedStyle::initial()` fg. Centralized so
/// the test above doesn't hard-code the initial-fg value.
fn rdom_tui_initial_fg() -> Color {
    crate::style::ComputedStyle::initial().fg
}

#[test]
fn cascade_with_zero_stylesheets_renders_initial_styles() {
    // The empty-stylesheets case is reachable via `set_stylesheet`
    // (returning the new id) followed by `remove_stylesheet`. The
    // cascade must handle it: no panic, every element resolves to
    // `ComputedStyle::initial()`, paint runs cleanly.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();

    let mut app = test_app(
        dom,
        Stylesheet::bare().rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(255, 0, 0))),
        Rect::new(0, 0, 20, 5),
    );
    // Construction sheet's id isn't returned; we get one via
    // set_stylesheet now that it returns a StylesheetId.
    let id = app.set_stylesheet(
        Stylesheet::bare().rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(0, 255, 0))),
    );
    app.draw_if_dirty().unwrap();
    assert_eq!(
        crate::style::cascade::computed_of(app.dom(), div).fg,
        Color::Rgb(0, 255, 0),
        "green from the post-set sheet"
    );

    // Now remove the only registered sheet.
    app.remove_stylesheet(id);
    assert_eq!(app.style_sheets().len(), 0, "no sheets registered");

    // Paint must succeed and every element must resolve to initial.
    app.draw_if_dirty().unwrap();
    assert_eq!(
        crate::style::cascade::computed_of(app.dom(), div).fg,
        rdom_tui_initial_fg(),
        "div re-cascaded to initial — nothing left to apply"
    );
}

// ── dom_mut + set_stylesheet ────────────────────────────────────────

#[test]
fn dom_mut_and_stylesheet_swap_work() {
    let dom: TuiDom = TuiDom::new();
    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5));

    // Add a div via dom_mut().
    let div = app.dom_mut().create_element("div");
    let root = app.dom_mut().root();
    app.dom_mut().append_child(root, div).unwrap();

    // Swap stylesheet.
    app.set_stylesheet(
        Stylesheet::bare().rule_unchecked("div", TuiStyle::new().fg(Color::Rgb(255, 0, 0))),
    );
    assert!(app.needs_redraw(), "stylesheet swap requests redraw");
}

// ── AppContext::dispatch + queue_dispatch (3.B) ─────────────────────

#[test]
fn context_dispatch_fires_synchronously() {
    use rdom_core::Event;

    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let btn = dom.create_element("btn");
    dom.append_child(root, btn).unwrap();

    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    dom.add_event_listener(btn, "custom", ListenerOptions::default(), move |_| {
        f.set(true);
    })
    .unwrap();

    let btn_id = btn;
    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5)).on_tick(move |ctx| {
        let mut e = Event::new("custom");
        ctx.dispatch(btn_id, &mut e);
        ControlFlow::Continue
    });
    assert!(!fired.get());
    app.tick();
    assert!(fired.get());
}

#[test]
fn context_queue_dispatch_runs_after_tick_returns() {
    use rdom_core::Event;

    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let btn = dom.create_element("btn");
    dom.append_child(root, btn).unwrap();

    let order = Rc::new(std::cell::RefCell::new(Vec::<&'static str>::new()));
    let o1 = order.clone();
    dom.add_event_listener(btn, "queued", ListenerOptions::default(), move |_| {
        o1.borrow_mut().push("queued-fired");
    })
    .unwrap();

    let btn_id = btn;
    let o2 = order.clone();
    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5)).on_tick(move |ctx| {
        o2.borrow_mut().push("tick-start");
        ctx.queue_dispatch(btn_id, Event::new("queued"));
        // Assert queued dispatch hasn't fired yet (still in
        // the tick closure scope).
        // (Can't borrow `order` again to peek — rely on
        // comparing to final expectation.)
        o2.borrow_mut().push("tick-end");
        ControlFlow::Continue
    });

    app.tick();
    assert_eq!(
        *order.borrow(),
        vec!["tick-start", "tick-end", "queued-fired"]
    );
}

// ── AppHandle (3.B) ─────────────────────────────────────────────────

#[test]
fn handle_request_redraw_sets_needs_redraw_on_next_iteration() {
    let dom: TuiDom = TuiDom::new();
    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5));
    app.draw_if_dirty().unwrap();
    assert!(!app.needs_redraw());

    let handle = app.handle();
    std::thread::spawn(move || {
        handle.request_redraw();
    })
    .join()
    .unwrap();

    app.drain_handle_signals();
    assert!(app.needs_redraw());
}

#[test]
fn handle_quit_sets_should_quit() {
    let dom: TuiDom = TuiDom::new();
    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5));

    let handle = app.handle();
    std::thread::spawn(move || {
        handle.quit();
    })
    .join()
    .unwrap();

    app.drain_handle_signals();
    assert!(app.should_quit());
}

#[test]
fn handle_inject_runs_closure_on_loop_thread() {
    let dom: TuiDom = TuiDom::new();
    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5));

    let handle = app.handle();
    std::thread::spawn(move || {
        handle.inject(|ctx: &mut AppContext<'_>| {
            // Mutate DOM from the injected closure.
            let d = ctx.dom.create_element("div");
            let r = ctx.dom.root();
            let _ = ctx.dom.append_child(r, d);
            ctx.request_redraw();
        });
    })
    .join()
    .unwrap();

    app.drain_handle_injections();
    // DirtyTracker should have recorded the mutation.
    assert!(!app.dirty_roots_snapshot().is_empty());
    assert!(app.needs_redraw());
}

#[test]
fn handle_is_send_and_sync() {
    // Compile-time assertion lives in handle.rs; re-confirm here
    // so the test suite fails loudly if someone ever strips it.
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    assert_send::<crate::AppHandle>();
    assert_sync::<crate::AppHandle>();
}

// ── Tab navigation through App ──────────────────────────────────────

#[test]
fn tab_key_moves_focus_through_tabindex_chain() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.set_attribute(a, "tabindex", "0").unwrap();
    dom.set_attribute(b, "tabindex", "0").unwrap();
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();

    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5));
    app.handle_event(key(KeyCode::Tab));
    assert_eq!(app.dom().focused(), Some(a));
    app.handle_event(key(KeyCode::Tab));
    assert_eq!(app.dom().focused(), Some(b));
    app.handle_event(key(KeyCode::Tab));
    assert_eq!(app.dom().focused(), Some(a), "wraps");
}

#[test]
fn shift_tab_moves_focus_backward() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.set_attribute(a, "tabindex", "0").unwrap();
    dom.set_attribute(b, "tabindex", "0").unwrap();
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();
    dom.set_focused(Some(b));

    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5));
    app.handle_event(CtEvent::Key(KeyEvent::new(
        KeyCode::Tab,
        KeyModifiers::SHIFT,
    )));
    assert_eq!(app.dom().focused(), Some(a));
}

#[test]
fn tab_key_preventdefault_blocks_focus_move() {
    // A handler on the focused element can intercept Tab by
    // calling prevent_default on the keydown.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let a = dom.create_element("a");
    let b = dom.create_element("b");
    dom.set_attribute(a, "tabindex", "0").unwrap();
    dom.set_attribute(b, "tabindex", "0").unwrap();
    dom.append_child(root, a).unwrap();
    dom.append_child(root, b).unwrap();
    dom.set_focused(Some(a));

    dom.add_event_listener(a, "keydown", ListenerOptions::default(), |ctx| {
        ctx.event.prevent_default()
    })
    .unwrap();

    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5));
    app.handle_event(key(KeyCode::Tab));
    assert_eq!(
        app.dom().focused(),
        Some(a),
        "prevent_default blocks Tab navigation"
    );
}

/// Bug repro from manual `selectable_text` testing 2026-05-18: a
/// single mousedown on text was reported as immediately selecting
/// the entire paragraph. A `Selection::caret(anchor)` (collapsed)
/// should produce no overlay; this test pins that the runtime
/// path leaves the selection collapsed after exactly one mousedown.
#[test]
fn mousedown_alone_leaves_collapsed_caret_not_whole_paragraph() {
    use crate::layout::Display;
    use crossterm::event::{MouseButton, MouseEvent as CtMouseEvent, MouseEventKind};

    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();

    // Mirror the selectable_text example structure: a <p> with a
    // text node + trailing empty <span> so is_ifc_block fires.
    let prose = dom.create_element("p");
    dom.set_attribute(prose, "tabindex", "0").unwrap();
    let t = dom.create_text_node("Terminal UIs should let you select text easily.");
    dom.append_child(prose, t).unwrap();
    let tail = dom.create_element("span");
    dom.append_child(prose, tail).unwrap();
    dom.append_child(root, prose).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("p", TuiStyle::new().width(Size::Flex(1)))
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));

    let mut app = test_app(dom, sheet, Rect::new(0, 0, 80, 5));
    app.draw_if_dirty().unwrap();

    // Click mid-paragraph (column 10, row 0 — over "U" of "UIs").
    app.handle_event(CtEvent::Mouse(CtMouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: 0,
        modifiers: KeyModifiers::empty(),
    }));

    let sel = app
        .dom()
        .selection()
        .expect("mousedown on text should produce a selection");
    assert!(
        sel.is_collapsed(),
        "single mousedown must leave a collapsed caret, got anchor={:?} focus={:?}",
        sel.anchor,
        sel.focus,
    );
    // A text-selection drag opts into edge autoscroll (DRAG-AUTOSCROLL phase 4
    // hook), so dragging past a scroll container's edge keeps the selection
    // growing — same primitive the grid uses.
    assert!(
        app.dom().drag_autoscroll(),
        "text-selection drag arms autoscroll"
    );
}

/// Companion to `mousedown_alone_leaves_collapsed_caret_not_whole_paragraph`:
/// simulate Down at col 10 → Drag at col 12 (1-2 cells of motion,
/// which is what a finger trying to click and not perfectly steady
/// produces on a real touchpad). Selection should cover *exactly*
/// those few graphemes, not the entire paragraph.
#[test]
fn mousedown_then_small_drag_stays_within_dragged_range() {
    use crate::layout::Display;
    use crossterm::event::{MouseButton, MouseEvent as CtMouseEvent, MouseEventKind};

    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let prose = dom.create_element("p");
    dom.set_attribute(prose, "tabindex", "0").unwrap();
    let t = dom.create_text_node("Terminal UIs should let you select text easily.");
    dom.append_child(prose, t).unwrap();
    let tail = dom.create_element("span");
    dom.append_child(prose, tail).unwrap();
    dom.append_child(root, prose).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked("p", TuiStyle::new().width(Size::Flex(1)))
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));

    let mut app = test_app(dom, sheet, Rect::new(0, 0, 80, 5));
    app.draw_if_dirty().unwrap();

    app.handle_event(CtEvent::Mouse(CtMouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: 0,
        modifiers: KeyModifiers::empty(),
    }));
    app.handle_event(CtEvent::Mouse(CtMouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: 12,
        row: 0,
        modifiers: KeyModifiers::empty(),
    }));

    let sel = app.dom().selection().expect("drag produces a selection");
    // anchor was at column 10 (offset 10 in the prose), focus at column 12.
    // Selection should span exactly 2 graphemes — NOT the whole 47-byte text.
    let (lo, hi) = if sel.anchor.offset <= sel.focus.offset {
        (sel.anchor.offset, sel.focus.offset)
    } else {
        (sel.focus.offset, sel.anchor.offset)
    };
    assert!(
        hi - lo <= 4,
        "small drag should span ~2 graphemes, got {} bytes from {} to {}",
        hi - lo,
        lo,
        hi,
    );
}

#[test]
fn drag_up_over_user_select_none_extends_past_it_not_collapse() {
    // selectable_text repro (upward): anchor a selection in the bottom block,
    // drag UP and rest the cursor over a `user-select: none` bar between
    // blocks. `position_at` returns None there (so a *click* can't start a
    // selection on chrome), and the drag must snap the focus to the nearest
    // SELECTABLE text — not clamp back to the anchor flow, which (being far
    // below) collapsed the whole multi-block selection.
    use crate::layout::{Display, UserSelect};
    use crossterm::event::{MouseButton, MouseEvent as CtMouseEvent, MouseEventKind};
    use rdom_core::Position;

    let mev = |kind, x: u16, y: u16| {
        CtEvent::Mouse(CtMouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: KeyModifiers::empty(),
        })
    };

    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let mk = |dom: &mut TuiDom, txt: &str| -> (NodeId, NodeId) {
        let p = dom.create_element("p");
        let t = dom.create_text_node(txt);
        dom.append_child(p, t).unwrap();
        let tail = dom.create_element("span"); // 2nd inline child → IFC
        dom.append_child(p, tail).unwrap();
        (p, t)
    };
    let (a, t_a) = mk(&mut dom, "AAAA"); // row 0 — selectable
    let (chrome, _t_chrome) = mk(&mut dom, "NN"); // row 1 — user-select:none
    dom.set_attribute(chrome, "class", "chrome").unwrap();
    let (p_mid, _t_mid) = mk(&mut dom, "PPPP"); // row 2 — selectable
    let (b, t_b) = mk(&mut dom, "BBBB"); // row 3 — selectable (anchor here)
    for n in [a, chrome, p_mid, b] {
        dom.append_child(root, n).unwrap();
    }

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(20)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline))
        .rule_unchecked(".chrome", TuiStyle::new().user_select(UserSelect::None));
    let mut app = test_app(dom, sheet, Rect::new(0, 0, 20, 6));
    app.draw_if_dirty().unwrap();

    // Anchor at the end of the bottom block (row 3), then drag UP onto the
    // user-select:none bar (row 1).
    app.handle_event(mev(MouseEventKind::Down(MouseButton::Left), 3, 3));
    app.handle_event(mev(MouseEventKind::Drag(MouseButton::Left), 1, 1));

    let sel = app.dom().selection().expect("drag keeps a selection");
    assert_ne!(
        sel.focus.node, t_b,
        "drag up over user-select:none must not collapse back into the anchor block"
    );
    // Snaps to the nearest selectable flow — the top block's end (doc-order
    // tie-break vs the equidistant block below the bar).
    assert_eq!(sel.focus, Position::new(t_a, 4));
}

#[test]
fn mousedown_focuses_focusable_ancestor() {
    use crossterm::event::{MouseButton, MouseEvent as CtMouseEvent, MouseEventKind};

    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let btn = dom.create_element("btn");
    dom.set_attribute(btn, "tabindex", "0").unwrap();
    dom.append_child(root, btn).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "btn",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(3)),
    );
    let mut app = test_app(dom, sheet, Rect::new(0, 0, 20, 10));
    app.draw_if_dirty().unwrap();

    app.handle_event(CtEvent::Mouse(CtMouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 3,
        row: 1,
        modifiers: KeyModifiers::empty(),
    }));
    assert_eq!(
        app.dom().focused(),
        Some(btn),
        "mousedown focuses focusable ancestor"
    );
}

#[test]
fn handle_injection_can_quit() {
    let dom: TuiDom = TuiDom::new();
    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 20, 5));

    let handle = app.handle();
    handle.inject(|ctx: &mut AppContext<'_>| ctx.quit());

    app.drain_handle_injections();
    assert!(app.should_quit());
}

// ── Clipboard ───────────────────────────────────────────────────────

use crate::runtime::selection::clipboard::MemoryClipboard;

fn ctrl(ch: char) -> CtEvent {
    CtEvent::Key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL))
}

/// An `App` with a paragraph "hello world" and a pre-seeded
/// `MemoryClipboard`. Selection spans "world" (bytes 6..11) on the
/// text node.
fn clipboard_app(
    seeded: Option<&str>,
    selection_range: Option<(usize, usize)>,
) -> (App<TestBackend>, rdom_core::NodeId) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("hello world");
    dom.append_child(p, t).unwrap();
    let span = dom.create_element("span");
    dom.append_child(p, span).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(crate::layout::Display::Block)
                .width(Size::Fixed(20)),
        )
        .rule_unchecked(
            "span",
            TuiStyle::new().display(crate::layout::Display::Inline),
        );

    if let Some((start, end)) = selection_range {
        use rdom_core::{Position, Selection};
        dom.set_selection(Some(Selection::new(
            Position::new(t, start),
            Position::new(t, end),
        )));
    }

    let app = test_app(dom, sheet, Rect::new(0, 0, 40, 10));
    let app = match seeded {
        Some(s) => app.with_clipboard(Box::new(MemoryClipboard::with_text(s))),
        None => app.with_clipboard(Box::new(MemoryClipboard::new())),
    };
    (app, t)
}

#[test]
fn ctrl_c_with_selection_copies_instead_of_quitting() {
    let (mut app, _t) = clipboard_app(None, Some((6, 11)));
    app.handle_event(ctrl('c'));

    // Not quitting — selection was present, so Ctrl-C routed to copy.
    assert!(!app.should_quit());
}

#[test]
fn ctrl_c_without_selection_still_quits() {
    let (mut app, _t) = clipboard_app(None, None);
    app.handle_event(ctrl('c'));

    assert!(app.should_quit());
}

#[test]
fn ctrl_c_with_collapsed_caret_still_quits() {
    // A collapsed selection (caret) is treated as "no selection" for
    // copy purposes — there's nothing to serialize, so Ctrl-C falls
    // through to quit.
    use rdom_core::{Position, Selection};
    let (mut app, t) = clipboard_app(None, None);
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 3))));

    app.handle_event(ctrl('c'));
    assert!(app.should_quit());
}

#[test]
fn copy_fires_copy_event_with_selection_text_in_detail() {
    // `copy` fires on the element owning the anchor (the <p>). We
    // listen on root — the event bubbles — which avoids needing to
    // plumb the <p> id out of the fixture.
    let (mut app, _t) = clipboard_app(None, Some((6, 11)));
    let root = app.dom().root();
    let detail = Rc::new(std::cell::RefCell::new(None));
    {
        let detail = detail.clone();
        app.dom_mut()
            .add_event_listener(root, "copy", ListenerOptions::default(), move |ctx| {
                *detail.borrow_mut() = ctx.event.detail.as_string().map(String::from);
            })
            .unwrap();
    }
    app.handle_event(ctrl('c'));
    assert_eq!(detail.borrow().as_deref(), Some("world"));
}

#[test]
fn copy_event_prevent_default_suppresses_clipboard_write() {
    let (mut app, _t) = clipboard_app(Some("old"), Some((6, 11)));
    let root = app.dom().root();
    app.dom_mut()
        .add_event_listener(root, "copy", ListenerOptions::default(), |ctx| {
            ctx.event.prevent_default();
        })
        .unwrap();

    app.handle_event(ctrl('c'));

    // Clipboard was pre-seeded with "old" — default action was
    // suppressed, so "old" survives the copy gesture.
    // We can't directly read the clipboard off `app` (no accessor —
    // by design the backend is opaque), so instead we verify via a
    // subsequent paste that reads back the seeded value.
    let pasted = Rc::new(std::cell::RefCell::new(None));
    {
        let pasted = pasted.clone();
        let root = app.dom().root();
        app.dom_mut()
            .add_event_listener(root, "paste", ListenerOptions::default(), move |ctx| {
                *pasted.borrow_mut() = ctx.event.detail.as_string().map(String::from);
            })
            .unwrap();
    }
    app.handle_event(ctrl('v'));
    assert_eq!(pasted.borrow().as_deref(), Some("old"));
}

#[test]
fn cut_fires_cut_event_with_selection_text() {
    let (mut app, _t) = clipboard_app(None, Some((0, 5)));
    let root = app.dom().root();
    let detail = Rc::new(std::cell::RefCell::new(None));
    {
        let detail = detail.clone();
        app.dom_mut()
            .add_event_listener(root, "cut", ListenerOptions::default(), move |ctx| {
                *detail.borrow_mut() = ctx.event.detail.as_string().map(String::from);
            })
            .unwrap();
    }
    app.handle_event(ctrl('x'));
    assert_eq!(detail.borrow().as_deref(), Some("hello"));
}

#[test]
fn paste_fires_on_focused_element_with_clipboard_text() {
    let (mut app, _t) = clipboard_app(Some("pasted text"), None);
    let root = app.dom().root();
    let detail = Rc::new(std::cell::RefCell::new(None));
    {
        let detail = detail.clone();
        app.dom_mut()
            .add_event_listener(root, "paste", ListenerOptions::default(), move |ctx| {
                *detail.borrow_mut() = ctx.event.detail.as_string().map(String::from);
            })
            .unwrap();
    }
    app.handle_event(ctrl('v'));
    assert_eq!(detail.borrow().as_deref(), Some("pasted text"));
    assert!(!app.should_quit());
}

#[test]
fn paste_with_empty_clipboard_fires_empty_detail() {
    // MemoryClipboard with no seed — read returns None → paste
    // event carries an empty string.
    let (mut app, _t) = clipboard_app(None, None);
    let root = app.dom().root();
    let detail = Rc::new(std::cell::RefCell::new(None));
    {
        let detail = detail.clone();
        app.dom_mut()
            .add_event_listener(root, "paste", ListenerOptions::default(), move |ctx| {
                *detail.borrow_mut() = ctx.event.detail.as_string().map(String::from);
            })
            .unwrap();
    }
    app.handle_event(ctrl('v'));
    assert_eq!(detail.borrow().as_deref(), Some(""));
}

#[test]
fn cmd_c_also_copies_via_super_modifier() {
    // macOS terminals may report Cmd-C as SUPER.
    let (mut app, _t) = clipboard_app(None, Some((6, 11)));
    let root = app.dom().root();
    let detail = Rc::new(std::cell::RefCell::new(None));
    {
        let detail = detail.clone();
        app.dom_mut()
            .add_event_listener(root, "copy", ListenerOptions::default(), move |ctx| {
                *detail.borrow_mut() = ctx.event.detail.as_string().map(String::from);
            })
            .unwrap();
    }
    app.handle_event(CtEvent::Key(KeyEvent::new(
        KeyCode::Char('c'),
        KeyModifiers::SUPER,
    )));
    assert_eq!(detail.borrow().as_deref(), Some("world"));
    assert!(!app.should_quit());
}

// ── Panic safety ────────────────────────────────────────────────────

use std::panic::{self, AssertUnwindSafe};

#[test]
fn handler_panic_propagates_from_handle_event() {
    // A listener that panics should unwind out of the dispatch and
    // through handle_event — not get swallowed. The catch at
    // App::run is the outer safety net; handle_event on its own
    // lets the panic propagate to the caller who drove it.
    let mut dom = TuiDom::new();
    let root = dom.root();
    dom.add_event_listener(root, "keydown", ListenerOptions::default(), |_| {
        panic!("handler boom");
    })
    .unwrap();

    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 10, 5));

    let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
        app.handle_event(key(KeyCode::Char('x')))
    }));

    assert!(outcome.is_err(), "handler panic should propagate");
    // Payload should be the string we panicked with.
    let payload = outcome.err().unwrap();
    let msg = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(|s| s.as_str()))
        .unwrap_or("");
    assert!(
        msg.contains("handler boom"),
        "expected panic message to surface, got {msg:?}"
    );
}

#[test]
fn dispatch_after_handler_panic_still_works_on_fresh_events() {
    // A listener panics on "click"; the app catches externally. The
    // DOM should still be usable for subsequent events (no wedged
    // state from half-finished dispatch). This checks the
    // "restore listener handler after call" path in rdom-core is
    // robust against unwind.
    let mut dom = TuiDom::new();
    let root = dom.root();
    let survivor_fired = Rc::new(Cell::new(false));

    dom.add_event_listener(root, "keydown", ListenerOptions::default(), |_| {
        panic!("first");
    })
    .unwrap();

    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 10, 5));
    let _ = panic::catch_unwind(AssertUnwindSafe(|| {
        app.handle_event(key(KeyCode::Char('x')));
    }));

    // Register a fresh listener for a different key; it should fire
    // when we dispatch again.
    {
        let fired = survivor_fired.clone();
        app.dom_mut()
            .add_event_listener(root, "resize-noop", ListenerOptions::default(), move |_| {
                fired.set(true);
            })
            .unwrap();
    }

    // Dispatch a resize — it doesn't route to listeners, but the
    // app should still run handle_event without panicking.
    app.handle_event(CtEvent::Resize(20, 10));

    // The "first" listener is gone (panic removed its handler slot).
    // A new dispatch of the same event type should NOT re-panic —
    // the listener's handler was taken out; restore skipped on
    // unwind, so the listener is effectively detached.
    let second = panic::catch_unwind(AssertUnwindSafe(|| {
        app.handle_event(key(KeyCode::Char('x')))
    }));
    assert!(
        second.is_ok(),
        "a second dispatch of the same event must not re-panic"
    );
}

#[test]
fn terminal_guard_drop_runs_leave_tui_mode() {
    // Integration-ish: constructing and dropping a TerminalGuard
    // should run leave_tui_mode. The call is io-best-effort (stdout
    // isn't in alt-screen mode in a unit test), so we just verify
    // it doesn't panic and the guard is disarmable.
    use crate::render::TerminalGuard;

    {
        let _g = TerminalGuard::new();
        // Drops at block exit — runs leave_tui_mode internally.
    }

    let mut g = TerminalGuard::new();
    g.disarm();
    // Disarmed guard drop is a no-op — also shouldn't panic.
    drop(g);
}

#[test]
fn panic_hook_install_is_idempotent() {
    use crate::runtime::app::panic_hook;

    // First install (or no-op if already installed by an earlier
    // test) — must not panic.
    panic_hook::install();
    assert!(panic_hook::is_installed());

    // Second + third calls — must be no-op, not re-installing or
    // leaking the previous hook.
    panic_hook::install();
    panic_hook::install();
    assert!(panic_hook::is_installed());
}

// ── Paste/cut editable defaults ─────────────────────────────────────

use rdom_core::{Position, Selection};

/// Build an editable <p contenteditable="true"> with some text +
/// a trailing <span/> (needed for IFC recognition). Returns
/// (app, p, text_node), app pre-configured with `MemoryClipboard::with_text(seed)`.
fn editable_clipboard_app(
    text: &str,
    seed: &str,
) -> (App<TestBackend>, rdom_core::NodeId, rdom_core::NodeId) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    dom.set_attribute(p, "contenteditable", "true").unwrap();
    let t = dom.create_text_node(text);
    dom.append_child(p, t).unwrap();
    let span = dom.create_element("span");
    dom.append_child(p, span).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(crate::layout::Display::Block)
                .width(Size::Fixed(40)),
        )
        .rule_unchecked(
            "span",
            TuiStyle::new().display(crate::layout::Display::Inline),
        );
    let app = test_app(dom, sheet, Rect::new(0, 0, 60, 10));
    let app = app.with_clipboard(Box::new(MemoryClipboard::with_text(seed)));
    (app, p, t)
}

#[test]
fn ctrl_v_on_editable_inserts_clipboard_text_at_caret() {
    let (mut app, p, t) = editable_clipboard_app("hello ", "world");
    app.dom_mut().set_focused(Some(p));
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 6))));

    app.handle_event(ctrl('v'));

    assert_eq!(app.dom().node(t).node_value(), Some("hello world"));
    let sel = app.dom().selection().unwrap();
    assert!(sel.is_collapsed());
    assert_eq!(sel.focus, Position::new(t, 11));
}

#[test]
fn ctrl_v_on_editable_replaces_range_when_selection_is_non_collapsed() {
    let (mut app, p, t) = editable_clipboard_app("hello WORLD", "there");
    app.dom_mut().set_focused(Some(p));
    app.dom_mut().set_selection(Some(Selection::new(
        Position::new(t, 6),
        Position::new(t, 11),
    )));

    app.handle_event(ctrl('v'));

    assert_eq!(app.dom().node(t).node_value(), Some("hello there"));
}

#[test]
fn ctrl_v_paste_event_prevent_default_blocks_insert() {
    let (mut app, p, t) = editable_clipboard_app("hello ", "world");
    app.dom_mut().set_focused(Some(p));
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 6))));

    app.dom_mut()
        .add_event_listener(p, "paste", ListenerOptions::default(), |ctx| {
            ctx.event.prevent_default();
        })
        .unwrap();

    app.handle_event(ctrl('v'));

    // DOM unchanged.
    assert_eq!(app.dom().node(t).node_value(), Some("hello "));
}

#[test]
fn ctrl_v_on_non_editable_fires_event_but_does_not_mutate_dom() {
    // Regression guard for the "non-editable paste is still a
    // no-op event apps can intercept" contract in V2 §4.2.
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p"); // NOT contenteditable
    let t = dom.create_text_node("hello");
    dom.append_child(p, t).unwrap();
    dom.append_child(root, p).unwrap();
    dom.set_focused(Some(p));

    let detail = Rc::new(std::cell::RefCell::new(None));
    {
        let detail = detail.clone();
        dom.add_event_listener(p, "paste", ListenerOptions::default(), move |ctx| {
            *detail.borrow_mut() = ctx.event.detail.as_string().map(String::from);
        })
        .unwrap();
    }

    let sheet = Stylesheet::bare();
    let mut app = test_app(dom, sheet, Rect::new(0, 0, 30, 5));
    app = app.with_clipboard(Box::new(MemoryClipboard::with_text("X")));

    app.handle_event(ctrl('v'));

    // Event fired...
    assert_eq!(detail.borrow().as_deref(), Some("X"));
    // ...but the DOM wasn't mutated.
    assert_eq!(app.dom().node(t).node_value(), Some("hello"));
}

#[test]
fn ctrl_x_on_editable_deletes_selection_and_copies_to_clipboard() {
    let (mut app, p, t) = editable_clipboard_app("hello world", "");
    app.dom_mut().set_focused(Some(p));
    app.dom_mut().set_selection(Some(Selection::new(
        Position::new(t, 6),
        Position::new(t, 11),
    )));

    app.handle_event(ctrl('x'));

    // Selection is gone from the text.
    assert_eq!(app.dom().node(t).node_value(), Some("hello "));
    // Clipboard received the cut text — verify via a subsequent
    // paste into an editable.
    let t2 = t;
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t2, 6))));
    app.handle_event(ctrl('v'));
    assert_eq!(app.dom().node(t2).node_value(), Some("hello world"));
}

#[test]
fn ctrl_x_on_non_editable_copies_but_does_not_delete() {
    // Selection over non-editable text: Cmd-X should still put
    // the text on the clipboard (useful UX), but must not attempt
    // to mutate the DOM.
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("hello world");
    dom.append_child(p, t).unwrap();
    dom.append_child(root, p).unwrap();
    dom.set_focused(Some(p));
    dom.set_selection(Some(Selection::new(
        Position::new(t, 6),
        Position::new(t, 11),
    )));

    let sheet = Stylesheet::bare();
    let mut app = test_app(dom, sheet, Rect::new(0, 0, 30, 5));
    app = app.with_clipboard(Box::new(MemoryClipboard::new()));

    app.handle_event(ctrl('x'));

    // Text unchanged.
    assert_eq!(app.dom().node(t).node_value(), Some("hello world"));
    // Selection still there (cut didn't touch it; non-editable).
    let sel = app.dom().selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t, 6));
    assert_eq!(sel.focus, Position::new(t, 11));
}

#[test]
fn ctrl_x_prevent_default_blocks_both_copy_and_delete() {
    let (mut app, p, t) = editable_clipboard_app("hello world", "");
    app.dom_mut().set_focused(Some(p));
    app.dom_mut().set_selection(Some(Selection::new(
        Position::new(t, 6),
        Position::new(t, 11),
    )));
    app.dom_mut()
        .add_event_listener(p, "cut", ListenerOptions::default(), |ctx| {
            ctx.event.prevent_default();
        })
        .unwrap();

    app.handle_event(ctrl('x'));

    // Text unchanged + clipboard unchanged (verified via paste).
    assert_eq!(app.dom().node(t).node_value(), Some("hello world"));

    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 0))));
    // Drain any pending paste listener detail.
    let detail = Rc::new(std::cell::RefCell::new(None));
    {
        let detail = detail.clone();
        app.dom_mut()
            .add_event_listener(p, "paste", ListenerOptions::default(), move |ctx| {
                *detail.borrow_mut() = ctx.event.detail.as_string().map(String::from);
            })
            .unwrap();
    }
    app.handle_event(ctrl('v'));
    // Since cut was prevented, clipboard was never written — the
    // MemoryClipboard's initial empty state survives.
    assert_eq!(detail.borrow().as_deref(), Some(""));
}

#[test]
fn paste_into_editable_is_undoable() {
    let (mut app, p, t) = editable_clipboard_app("hello", " world");
    app.dom_mut().set_focused(Some(p));
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 5))));

    app.handle_event(ctrl('v'));
    assert_eq!(app.dom().node(t).node_value(), Some("hello world"));

    // Ctrl-Z reverses the paste.
    app.handle_event(ctrl('z'));
    assert_eq!(app.dom().node(t).node_value(), Some("hello"));
}

#[test]
fn cut_on_editable_is_undoable() {
    let (mut app, p, t) = editable_clipboard_app("hello world", "");
    app.dom_mut().set_focused(Some(p));
    app.dom_mut().set_selection(Some(Selection::new(
        Position::new(t, 6),
        Position::new(t, 11),
    )));

    app.handle_event(ctrl('x'));
    assert_eq!(app.dom().node(t).node_value(), Some("hello "));

    // Ctrl-Z restores the deleted range.
    app.handle_event(ctrl('z'));
    assert_eq!(app.dom().node(t).node_value(), Some("hello world"));
}

// ── C.4a: Enter handling on editables ─────────────────────────────

fn input_app(initial: &str) -> (App<TestBackend>, NodeId, NodeId) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let input = dom.create_element("input");
    dom.set_attribute(input, "value", initial).unwrap();
    dom.append_child(root, input).unwrap();
    let app = test_app(dom, Stylesheet::new(), Rect::new(0, 0, 40, 5));
    let t = app
        .dom()
        .node(input)
        .child_nodes()
        .next()
        .map(|c| c.id())
        .unwrap();
    (app, input, t)
}

fn textarea_app(initial: &str) -> (App<TestBackend>, NodeId, NodeId) {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let ta = dom.create_element("textarea");
    let t = dom.create_text_node(initial);
    dom.append_child(ta, t).unwrap();
    dom.append_child(root, ta).unwrap();
    let app = test_app(dom, Stylesheet::new(), Rect::new(0, 0, 40, 5));
    (app, ta, t)
}

#[test]
fn enter_on_input_does_not_insert_newline() {
    let (mut app, input, t) = input_app("hello");
    app.dom_mut().set_focused(Some(input));
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 5))));

    app.handle_event(key(KeyCode::Enter));

    assert_eq!(app.dom().node(t).node_value(), Some("hello"));
    assert_eq!(app.dom().node(input).get_attribute("value"), Some("hello"));
}

#[test]
fn enter_on_textarea_inserts_newline_at_caret() {
    let (mut app, ta, t) = textarea_app("ab");
    app.dom_mut().set_focused(Some(ta));
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 1))));

    app.handle_event(key(KeyCode::Enter));

    assert_eq!(app.dom().node(t).node_value(), Some("a\nb"));
}

#[test]
fn enter_on_contenteditable_paragraph_inserts_newline() {
    let (mut app, p, t) = editable_clipboard_app("ab", "");
    app.dom_mut().set_focused(Some(p));
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 1))));

    app.handle_event(key(KeyCode::Enter));

    assert_eq!(app.dom().node(t).node_value(), Some("a\nb"));
}

#[test]
fn typing_in_input_mirrors_value_attribute_per_keystroke() {
    let (mut app, input, t) = input_app("");
    app.dom_mut().set_focused(Some(input));
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 0))));

    for ch in ['h', 'i'] {
        app.handle_event(key(KeyCode::Char(ch)));
    }

    assert_eq!(app.dom().node(input).get_attribute("value"), Some("hi"));
}

#[test]
fn typing_in_readonly_input_does_not_change_value() {
    let (mut app, input, t) = input_app("hi");
    app.dom_mut().set_attribute(input, "readonly", "").unwrap();
    app.dom_mut().set_focused(Some(input));
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 2))));

    app.handle_event(key(KeyCode::Char('!')));

    assert_eq!(app.dom().node(t).node_value(), Some("hi"));
    assert_eq!(app.dom().node(input).get_attribute("value"), Some("hi"));
}

#[test]
fn ctrl_enter_on_textarea_does_not_insert_newline() {
    // Modifier combos belong to clipboard / app-level shortcuts —
    // bare Enter alone is the activation gesture.
    let (mut app, ta, t) = textarea_app("ab");
    app.dom_mut().set_focused(Some(ta));
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 2))));

    app.handle_event(CtEvent::Key(KeyEvent::new(
        KeyCode::Enter,
        KeyModifiers::CONTROL,
    )));

    assert_eq!(app.dom().node(t).node_value(), Some("ab"));
}

// ── Step 3: typed transition event detail ───────────────────────────

#[test]
fn transitionend_event_carries_typed_transition_detail() {
    // The dispatched `transitionend` event must populate
    // `event.detail` with `EventDetail::Transition(...)` carrying
    // the property name and elapsed seconds — not a pipe-separated
    // string. Retires D-M3-1.
    use crate::runtime::animation::{AnimatedProp, PendingEvent, TransitionEventKind};
    use std::cell::RefCell;

    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 10, 3));

    let captured: Rc<RefCell<Option<(String, f64)>>> = Rc::new(RefCell::new(None));
    {
        let captured = captured.clone();
        app.dom_mut()
            .add_event_listener(
                div,
                "transitionend",
                ListenerOptions::default(),
                move |ctx| {
                    let t = ctx
                        .event
                        .detail
                        .as_transition()
                        .expect("transition event must carry typed detail");
                    *captured.borrow_mut() = Some((t.property_name.clone(), t.elapsed));
                },
            )
            .unwrap();
    }

    app.animations_mut_for_test()
        .queue_event_for_test(PendingEvent {
            node: div,
            kind: TransitionEventKind::End,
            property: AnimatedProp::Fg,
            elapsed_seconds: 0.1,
        });
    app.dispatch_animation_events_for_test();

    let result = captured.borrow();
    let (property_name, elapsed) = result.as_ref().expect("listener fired");
    assert_eq!(property_name, "color");
    assert!(
        (elapsed - 0.1).abs() < 1e-6,
        "elapsed should round-trip; got {elapsed}"
    );
}

#[test]
fn transitionstart_and_transitioncancel_also_carry_typed_detail() {
    // Coverage for all three transition event variants; ensures
    // the migration in `dispatch_animation_events` doesn't leave
    // Start or Cancel on the old string path.
    use crate::runtime::animation::{AnimatedProp, PendingEvent, TransitionEventKind};
    use std::cell::RefCell;

    let kinds = [
        (TransitionEventKind::Start, "transitionstart"),
        (TransitionEventKind::Cancel, "transitioncancel"),
    ];

    for (kind, event_name) in kinds {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let div = dom.create_element("div");
        dom.append_child(root, div).unwrap();
        let mut app = test_app(dom, Stylesheet::bare(), Rect::new(0, 0, 10, 3));

        let captured: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        {
            let captured = captured.clone();
            app.dom_mut()
                .add_event_listener(div, event_name, ListenerOptions::default(), move |ctx| {
                    let t = ctx
                        .event
                        .detail
                        .as_transition()
                        .unwrap_or_else(|| panic!("{event_name} must carry typed detail"));
                    *captured.borrow_mut() = Some(t.property_name.clone());
                })
                .unwrap();
        }

        app.animations_mut_for_test()
            .queue_event_for_test(PendingEvent {
                node: div,
                kind,
                property: AnimatedProp::Bg,
                elapsed_seconds: 0.0,
            });
        app.dispatch_animation_events_for_test();

        assert_eq!(captured.borrow().as_deref(), Some("background-color"));
    }
}

// ── SPIKE: <canvas> as a focusable interactive widget ───────────────
//
// Verification tests for whether a `<canvas>` can act as a
// focusable, keyboard- and mouse-interactive widget (the
// foundation a canvas-based vtable would need). Added by a
// de-risk spike — does NOT change library behavior.
mod canvas_interactive_spike {
    use super::*;
    use crate::accessors::TuiAccessors;
    use std::cell::RefCell;

    fn canvas_app() -> (App<TestBackend>, NodeId) {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let cv = dom.create_element("canvas");
        dom.set_attribute(cv, "tabindex", "0").unwrap();
        dom.append_child(root, cv).unwrap();
        let sheet = Stylesheet::bare().rule_unchecked(
            "canvas",
            TuiStyle::new()
                .width(Size::Fixed(20))
                .height(Size::Fixed(6)),
        );
        let app = test_app(dom, sheet, Rect::new(0, 0, 40, 10));
        (app, cv)
    }

    // 1. FOCUS — programmatic.
    #[test]
    fn canvas_with_tabindex_is_focusable_programmatically() {
        let (mut app, cv) = canvas_app();
        assert!(crate::runtime::focus::tabindex::is_focusable(app.dom(), cv));
        crate::runtime::focus::focus_node(app.dom_mut(), Some(cv));
        assert_eq!(app.dom().focused(), Some(cv));
    }

    // 1. FOCUS — via Tab key through the full App pipeline.
    #[test]
    fn canvas_receives_focus_via_tab_key() {
        let (mut app, cv) = canvas_app();
        app.draw_if_dirty().unwrap();
        assert_eq!(app.dom().focused(), None);
        app.handle_event(key(KeyCode::Tab));
        assert_eq!(
            app.dom().focused(),
            Some(cv),
            "Tab should land focus on the only focusable element"
        );
    }

    // 1. FOCUS — via click (focus-on-click default action).
    #[test]
    fn canvas_receives_focus_via_click() {
        let (mut app, cv) = canvas_app();
        app.draw_if_dirty().unwrap();
        for ev in click_at(3, 2) {
            app.handle_event(ev);
        }
        assert_eq!(app.dom().focused(), Some(cv));
    }

    // 2. KEYBOARD — keydown dispatches to the focused canvas;
    // listener reads key + modifiers.
    #[test]
    fn focused_canvas_receives_keydown_with_key_and_modifiers() {
        let (mut app, cv) = canvas_app();
        let seen: Rc<RefCell<Vec<(String, bool)>>> = Rc::new(RefCell::new(Vec::new()));
        let s = seen.clone();
        app.dom_mut()
            .add_event_listener(cv, "keydown", ListenerOptions::default(), move |ctx| {
                let kb = ctx
                    .event
                    .detail
                    .as_keyboard()
                    .expect("keydown carries keyboard detail");
                s.borrow_mut().push((kb.key.clone(), kb.modifiers.shift));
            })
            .unwrap();
        app.draw_if_dirty().unwrap();
        crate::runtime::focus::focus_node(app.dom_mut(), Some(cv));

        app.handle_event(CtEvent::Key(KeyEvent::new(
            KeyCode::Down,
            KeyModifiers::empty(),
        )));
        app.handle_event(CtEvent::Key(KeyEvent::new(
            KeyCode::Char('j'),
            KeyModifiers::SHIFT,
        )));

        let got = seen.borrow().clone();
        // Note: the printable `key` is NOT upcased by Shift — DOM
        // `KeyboardEvent.key` would be "J", but rdom's key_translate
        // reports the base char "j" and surfaces Shift via the
        // separate `modifiers.shift` flag. The capability (key +
        // modifier both readable) is what matters here.
        assert_eq!(
            got,
            vec![("ArrowDown".to_string(), false), ("j".to_string(), true)],
            "listener must see translated key + shift modifier"
        );
    }

    // 2. KEYBOARD — prevent_default suppresses the Tab focus-nav
    // default action, so a focused canvas can keep Tab.
    #[test]
    fn canvas_keydown_prevent_default_intercepts_tab() {
        let (mut app, cv) = canvas_app();
        app.dom_mut()
            .add_event_listener(cv, "keydown", ListenerOptions::default(), move |ctx| {
                let kb = ctx.event.detail.as_keyboard().unwrap();
                if kb.key == "Tab" {
                    ctx.event.prevent_default();
                }
            })
            .unwrap();
        app.draw_if_dirty().unwrap();
        crate::runtime::focus::focus_node(app.dom_mut(), Some(cv));
        assert_eq!(app.dom().focused(), Some(cv));

        app.handle_event(key(KeyCode::Tab));
        // prevent_default ran → focus_next was skipped → focus stays.
        assert_eq!(
            app.dom().focused(),
            Some(cv),
            "prevent_default on Tab must suppress focus navigation"
        );
    }

    // 2. KEYBOARD — arrow keys reach the canvas listener even
    // without prevent_default (no built-in steals plain arrows when
    // a canvas is focused and has no selection/editable default).
    #[test]
    fn focused_canvas_receives_arrow_keys() {
        let (mut app, cv) = canvas_app();
        let count = Rc::new(Cell::new(0u32));
        let c = count.clone();
        app.dom_mut()
            .add_event_listener(cv, "keydown", ListenerOptions::default(), move |ctx| {
                if ctx
                    .event
                    .detail
                    .as_keyboard()
                    .unwrap()
                    .key
                    .starts_with("Arrow")
                {
                    c.set(c.get() + 1);
                }
            })
            .unwrap();
        app.draw_if_dirty().unwrap();
        crate::runtime::focus::focus_node(app.dom_mut(), Some(cv));

        for code in [KeyCode::Up, KeyCode::Down, KeyCode::Left, KeyCode::Right] {
            app.handle_event(key(code));
        }
        assert_eq!(count.get(), 4);
    }

    // 3. MOUSE — click reaches the canvas; coords are GLOBAL
    // (client_x/client_y), and bounding_rect lets the app convert
    // to canvas-local coords.
    #[test]
    fn canvas_click_delivers_global_coords_convertible_to_local() {
        let (mut app, cv) = canvas_app();
        let pos: Rc<RefCell<Option<(i32, i32)>>> = Rc::new(RefCell::new(None));
        let p = pos.clone();
        app.dom_mut()
            .add_event_listener(cv, "click", ListenerOptions::default(), move |ctx| {
                let m = ctx.event.detail.as_mouse().expect("mouse detail");
                *p.borrow_mut() = Some((m.client_x, m.client_y));
            })
            .unwrap();
        app.draw_if_dirty().unwrap();

        // Canvas is at viewport origin (0,0) per layout. Click at (5, 3).
        for ev in click_at(5, 3) {
            app.handle_event(ev);
        }

        let (gx, gy) = pos.borrow().expect("click must reach canvas");
        assert_eq!(
            (gx, gy),
            (5, 3),
            "client_x/client_y are GLOBAL screen cells"
        );

        // App converts to canvas-local via bounding_rect.
        let rect = app
            .dom()
            .node(cv)
            .bounding_rect()
            .expect("canvas has a rect");
        let local_x = gx - rect.x;
        let local_y = gy - rect.y;
        assert_eq!(
            (local_x, local_y),
            (5, 3),
            "with canvas at origin, local == global; rect.x/y is the subtractable origin"
        );
    }

    // 3. MOUSE — wheel event reaches the canvas listener with delta.
    #[test]
    fn focused_canvas_receives_wheel_with_delta() {
        let (mut app, cv) = canvas_app();
        let dy: Rc<RefCell<Vec<i32>>> = Rc::new(RefCell::new(Vec::new()));
        let d = dy.clone();
        app.dom_mut()
            .add_event_listener(cv, "wheel", ListenerOptions::default(), move |ctx| {
                let m = ctx.event.detail.as_mouse().unwrap();
                d.borrow_mut().push(m.delta_y);
            })
            .unwrap();
        app.draw_if_dirty().unwrap();

        app.handle_event(CtEvent::Mouse(CtMouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 5,
            row: 3,
            modifiers: KeyModifiers::empty(),
        }));
        app.handle_event(CtEvent::Mouse(CtMouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 5,
            row: 3,
            modifiers: KeyModifiers::empty(),
        }));

        assert_eq!(*dy.borrow(), vec![1, -1], "wheel delta_y reaches canvas");
    }

    // 3. MOUSE — mousedown also reaches the canvas (finer-grained
    // than click; useful for drag-select inside the vtable).
    #[test]
    fn canvas_receives_mousedown() {
        let (mut app, cv) = canvas_app();
        let hits = Rc::new(Cell::new(0u32));
        let h = hits.clone();
        app.dom_mut()
            .add_event_listener(cv, "mousedown", ListenerOptions::default(), move |_| {
                h.set(h.get() + 1);
            })
            .unwrap();
        app.draw_if_dirty().unwrap();
        for ev in click_at(4, 2) {
            app.handle_event(ev);
        }
        assert_eq!(hits.get(), 1);
    }

    // COORDINATE MAPPING: confirm what bounding_rect returns for a
    // DEEPLY NESTED canvas. The doc comment on `bounding_rect` says
    // "parent's coordinate space", but the layout pass bakes
    // ABSOLUTE (viewport-relative) positions into `ext.layout`, so
    // bounding_rect actually returns absolute screen coords — which
    // is exactly what an app needs to subtract from client_x/y.
    #[test]
    fn nested_canvas_bounding_rect_is_absolute_screen_coords() {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let outer = dom.create_element("div");
        let inner = dom.create_element("div");
        let cv = dom.create_element("canvas");
        dom.set_attribute(cv, "tabindex", "0").unwrap();
        dom.append_child(root, outer).unwrap();
        dom.append_child(outer, inner).unwrap();
        dom.append_child(inner, cv).unwrap();
        // Push everything down/right so absolute != parent-relative.
        let sheet = Stylesheet::bare()
            .rule_unchecked(
                "div",
                TuiStyle::new().padding(crate::layout::Padding::all(2)),
            )
            .rule_unchecked(
                "canvas",
                TuiStyle::new()
                    .width(Size::Fixed(10))
                    .height(Size::Fixed(4)),
            );
        let mut app = test_app(dom, sheet, Rect::new(0, 0, 40, 20));
        app.draw_if_dirty().unwrap();

        let rect = app.dom().node(cv).bounding_rect().unwrap();
        // Absolute origin is (4, 4): two nested divs × 2 padding
        // each. bounding_rect returns ABSOLUTE coords — so an app
        // can subtract rect.x/rect.y from a click's client_x/y to
        // get canvas-local coords regardless of nesting depth.
        assert_eq!(
            (rect.x, rect.y),
            (4, 4),
            "bounding_rect returns absolute screen coords, not parent-relative"
        );

        // Prove the end-to-end conversion: a click at absolute (4, 4)
        // maps to canvas-local (0, 0).
        let local_x = 4 - rect.x;
        let local_y = 4 - rect.y;
        assert_eq!((local_x, local_y), (0, 0));
    }
}

/// Regression: dropping the focused node from inside an event handler
/// must not panic. `drop_subtree` fires its `ChildListChanged` mutation
/// while the subtree is still alive, so the dirty-tracker observer (and
/// implicit blur/focusout) can read the removed node — freeing first
/// left them dereferencing a reclaimed arena slot. Surfaced by a chart
/// gallery that swapped demos on a keypress.
#[test]
fn dropping_focused_node_in_handler_does_not_panic() {
    use rdom_core::ListenerOptions;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let btn = dom.create_element("button");
    dom.append_child(root, btn).unwrap();
    dom.add_event_listener(btn, "keydown", ListenerOptions::default(), move |ctx| {
        if let Some(n) = ctx.dom.focused() {
            let _ = ctx.dom.drop_subtree(n);
        }
    })
    .unwrap();

    let mut app = test_app(dom, Stylesheet::new(), Rect::new(0, 0, 20, 5));
    app.dom_mut().set_focused(Some(btn));
    app.handle_event(key(KeyCode::Char('a'))); // must not panic
    assert!(
        !app.dom().contains(btn),
        "the focused node was removed in the handler"
    );
}

#[test]
fn dropping_a_sibling_then_a_dirtied_sibling_does_not_panic() {
    // Dropping one child fires `ChildListChanged`, whose dirty-tracker handler
    // marks every *remaining* sibling dirty (so `:first-child` / `+` / `~`
    // re-evaluate). If one of those freshly-marked siblings is then dropped in
    // the same teardown, it becomes a freed node still queued as a cascade root
    // — the next `draw_if_dirty` must not dereference it. (rdom-virtualtable's
    // column chooser closing: drop the panel, then the tab; the panel-drop marks
    // the tab dirty, then the tab is freed.)
    let mut dom = TuiDom::new();
    let root = dom.root();
    let host = dom.create_element("host");
    dom.append_child(root, host).unwrap();
    let a = dom.create_element("a");
    dom.append_child(host, a).unwrap();
    let b = dom.create_element("b");
    dom.append_child(host, b).unwrap();

    let mut app = test_app(dom, Stylesheet::new(), Rect::new(0, 0, 20, 5));
    app.draw_if_dirty().unwrap(); // drain the create/append dirty roots

    // Drop A → its ChildListChanged marks sibling B a dirty cascade root. Then
    // drop B, freeing it while it's still queued.
    app.dom_mut().drop_subtree(a).unwrap();
    app.dom_mut().drop_subtree(b).unwrap();

    app.draw_if_dirty().unwrap(); // must not panic on the freed B root
    assert!(!app.dom().contains(b));
}

#[test]
fn advance_drives_a_scheduler_interval_deterministically() {
    // Phase-0 gate for timer-driven runtime behavior (autoscroll): `advance`
    // moves the virtual clock and services what comes due, headless and
    // deterministic — no wall clock, no real input.
    use crate::runtime::timers::TuiTimers;
    use std::cell::Cell;
    use std::rc::Rc;

    let mut dom = TuiDom::new();
    let root = dom.root();
    let el = dom.create_element("div");
    dom.append_child(root, el).unwrap();
    let ticks = Rc::new(Cell::new(0u32));
    let t = ticks.clone();
    dom.add_event_listener(el, "keydown", ListenerOptions::default(), move |ctx| {
        let t = t.clone();
        ctx.set_interval(
            move |_| {
                t.set(t.get() + 1);
                true // keep firing
            },
            50,
        );
    })
    .unwrap();

    let mut app = test_app(dom, Stylesheet::new(), Rect::new(0, 0, 10, 3));
    app.dom_mut().set_focused(Some(el));
    app.handle_event(key(KeyCode::Char('a'))); // schedules the 50ms interval
    assert_eq!(ticks.get(), 0, "interval hasn't come due yet");

    app.advance(50).unwrap();
    assert_eq!(ticks.get(), 1, "one period → one tick");
    app.advance(50).unwrap();
    assert_eq!(ticks.get(), 2);
    app.advance(50).unwrap();
    assert_eq!(ticks.get(), 3, "deterministic, one tick per period");
}

#[test]
fn drag_autoscroll_scrolls_a_container_held_at_the_edge() {
    // DRAG-AUTOSCROLL phase 2: a captured drag that opted into autoscroll and
    // dwells at a scroll container's bottom edge scrolls it one step per tick,
    // and stops on release. Driven deterministically via `advance`.
    use crate::layout::Overflow;
    use crate::node::TuiNodeExt;

    let mev = |kind, x: u16, y: u16| {
        CtEvent::Mouse(CtMouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: KeyModifiers::empty(),
        })
    };

    let mut dom = TuiDom::new();
    let root = dom.root();
    let scroller = dom.create_element("scroller");
    dom.append_child(root, scroller).unwrap();
    for _ in 0..10 {
        let r = dom.create_element("r");
        dom.append_child(scroller, r).unwrap(); // 10 rows of 1 → content 10 > viewport 3
    }
    dom.add_event_listener(
        scroller,
        "mousedown",
        ListenerOptions::default(),
        move |ctx| {
            ctx.dom.set_pointer_capture(scroller).unwrap();
            ctx.dom.set_drag_autoscroll(true);
            ctx.event.prevent_default(); // claim the drag
        },
    )
    .unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "scroller",
            TuiStyle::new()
                .width(Size::Fixed(10))
                .height(Size::Fixed(3))
                .overflow(Overflow::Auto),
        )
        .rule_unchecked("r", TuiStyle::new().height(Size::Fixed(1)));
    let mut app = test_app(dom, sheet, Rect::new(0, 0, 12, 6));
    app.draw_if_dirty().unwrap(); // layout → scroll_content_height = 10

    let scroll_y = |app: &App<TestBackend>, id: NodeId| -> usize {
        app.dom()
            .node(id)
            .tui_ext()
            .map(|e| e.scroll_y)
            .unwrap_or(0)
    };

    // Press inside, then drag to the bottom visible row → arm autoscroll-down.
    app.handle_event(mev(MouseEventKind::Down(MouseButton::Left), 2, 1));
    app.handle_event(mev(MouseEventKind::Drag(MouseButton::Left), 2, 2));
    let before = scroll_y(&app, scroller);
    app.advance(50).unwrap();
    let after1 = scroll_y(&app, scroller);
    assert!(
        after1 > before,
        "held at the bottom edge autoscrolls down ({before} -> {after1})"
    );
    app.advance(50).unwrap();
    assert!(
        scroll_y(&app, scroller) > after1,
        "keeps scrolling each period while held"
    );

    // Release ends the drag; further advances don't scroll.
    app.handle_event(mev(MouseEventKind::Up(MouseButton::Left), 2, 2));
    let at_release = scroll_y(&app, scroller);
    app.advance(500).unwrap();
    assert_eq!(
        scroll_y(&app, scroller),
        at_release,
        "no autoscroll after mouseup releases the capture"
    );
}

#[test]
fn text_selection_drag_past_edge_autoscrolls_and_keeps_extending() {
    // DRAG-AUTOSCROLL phase 4, end-to-end: a *native text-selection* drag
    // held past a scroll container's bottom edge must autoscroll AND keep
    // extending the selection into the revealed lines. This is the
    // `selectable_text` repro — the bug was that holding the pointer in the
    // empty space past the edge made `position_at` return None, so the drag
    // collapsed the focus back to the anchor block instead of following the
    // revealed text. Driven deterministically via `advance`.
    use crate::layout::{Display, Overflow};
    use crate::node::TuiNodeExt;

    let mev = |kind, x: u16, y: u16| {
        CtEvent::Mouse(CtMouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: KeyModifiers::empty(),
        })
    };

    let mut dom = TuiDom::new();
    let root = dom.root();
    let scroller = dom.create_element("div");
    dom.append_child(root, scroller).unwrap();
    let p = dom.create_element("p");
    // 40 two-letter tokens wrap (at the spaces) to ~10 rows at width 12 —
    // content well past the 3-row viewport, so there's plenty to autoscroll
    // into. (A space-free string wouldn't wrap: no break opportunities.)
    let text: String = (0..40)
        .map(|i| format!("w{}", i % 10))
        .collect::<Vec<_>>()
        .join(" ");
    let t = dom.create_text_node(&text);
    dom.append_child(p, t).unwrap();
    let tail = dom.create_element("span"); // 2nd inline child → IFC
    dom.append_child(p, tail).unwrap();
    dom.append_child(scroller, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "div",
            TuiStyle::new()
                .width(Size::Fixed(12))
                .height(Size::Fixed(3))
                .overflow(Overflow::Auto),
        )
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(12)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));
    let mut app = test_app(dom, sheet, Rect::new(0, 0, 14, 6));
    app.draw_if_dirty().unwrap();

    let scroll_y = |app: &App<TestBackend>| -> usize {
        app.dom()
            .node(scroller)
            .tui_ext()
            .map(|e| e.scroll_y)
            .unwrap_or(0)
    };
    let focus_off = |app: &App<TestBackend>| -> usize {
        app.dom().selection().map(|s| s.focus.offset).unwrap_or(0)
    };

    // Press near the top of the prose (anchors + captures + arms autoscroll),
    // then hold a drag at the bottom visible row.
    app.handle_event(mev(MouseEventKind::Down(MouseButton::Left), 1, 0));
    assert!(
        app.dom().selection().is_some(),
        "mousedown on text starts a selection"
    );
    assert!(
        app.dom().drag_autoscroll(),
        "text-selection drag arms autoscroll"
    );
    app.handle_event(mev(MouseEventKind::Drag(MouseButton::Left), 1, 2));
    let focus_before = focus_off(&app);

    // Tick the autoscroll several periods while held at the edge.
    for _ in 0..8 {
        app.advance(50).unwrap();
    }

    assert!(
        scroll_y(&app) > 0,
        "held at the bottom edge autoscrolls the container"
    );
    assert!(
        focus_off(&app) > focus_before,
        "the selection keeps extending into revealed lines \
         (focus {} should advance past {focus_before})",
        focus_off(&app)
    );

    // Release stops it; the selection is stable afterward.
    app.handle_event(mev(MouseEventKind::Up(MouseButton::Left), 1, 2));
    let focus_at_release = focus_off(&app);
    app.advance(500).unwrap();
    assert_eq!(
        focus_off(&app),
        focus_at_release,
        "no further extension after release"
    );
}

// ── PAINT-RELATIVE-ABSPOS-DOUBLE regression ─────────────────────────

/// A `position:relative` cell with text, beside a sibling that goes
/// `display:none`, plus an absolute child created then dropped (the
/// rdom-virtualtable show/hide chip + dropdown shape). Under the App's
/// *incremental* cascade the relative cell's glyph used to paint twice — once
/// at its slot and once at the hidden sibling's stale rect. With
/// `LAYOUT-DISPLAY-NONE-STALE-RECT` fixed the stale slot collapses, so the
/// glyph paints exactly once.
#[test]
fn relative_cell_glyph_paints_once_after_sibling_hidden_and_abspos_child_dropped() {
    use crate::layout::{Direction, Display, Flow, Position, ZIndex};
    use crate::node::TuiNodeMutExt;
    use crate::render::{Buffer, LayoutExt, PaintExt};
    use crate::style::Value;

    let vp = Rect::new(0, 0, 40, 6);
    let mut dom = TuiDom::new();
    let root = dom.root();
    let row = dom.create_element("div"); // flex Row header
    {
        let mut s = TuiStyle::new().direction(Direction::Row);
        s.display = Some(Value::Specified(Display::Block));
        s.flow = Some(Value::Specified(Flow::Flex));
        dom.node_mut(row).set_inline_style(s);
    }
    dom.append_child(root, row).unwrap();
    let mk_cell = |dom: &mut TuiDom, parent: NodeId, text: &str| {
        let cell = dom.create_element("div");
        let t = dom.create_text_node(text);
        dom.append_child(cell, t).unwrap();
        dom.append_child(parent, cell).unwrap();
        cell
    };
    let a = mk_cell(&mut dom, row, "aa");
    let b1 = mk_cell(&mut dom, row, "bb"); // visible → hidden (1st)
    let b2 = mk_cell(&mut dom, row, "cc"); // visible → hidden (2nd)
    let chip = mk_cell(&mut dom, row, "X"); // position:relative, the glyph carrier
    {
        let mut s = TuiStyle::new().width(Size::Fixed(3));
        s.position = Some(Value::Specified(Position::Relative));
        dom.node_mut(chip).set_inline_style(s);
    }
    // Like the `<table>` builtin's `size_columns`: stamp a used main-axis width
    // on each cell (flex reads `table_used_width` as the cell's main size).
    for (cell, w) in [(a, 3u16), (b1, 6), (b2, 6), (chip, 3)] {
        dom.node_mut(cell).ext_mut().unwrap().table_used_width = Some(w);
    }

    // `[hide]` → display:none (an attribute toggle, like the real contract).
    let sheet = Stylesheet::bare().rule_unchecked("[hide]", TuiStyle::new().display(Display::None));
    let mut app = test_app(dom, sheet, vp);
    app.draw_if_dirty().unwrap(); // frame 0 — all visible

    // Hide b1 (chip shifts left). Then on b2: open + close an absolute child of
    // the relative chip, then hide b2 (chip shifts left AGAIN). The chip moving
    // across incremental frames is what leaves the stale glyph behind.
    app.dom_mut().set_attribute(b1, "hide", "").unwrap();
    app.draw_if_dirty().unwrap();

    let menu = app.dom_mut().create_element("div");
    {
        let mut s = TuiStyle::new().width(Size::Fixed(4)).height(Size::Fixed(1));
        s.position = Some(Value::Specified(Position::Absolute));
        s.z_index = Some(Value::Specified(ZIndex::Value(10)));
        app.dom_mut().node_mut(menu).set_inline_style(s);
    }
    app.dom_mut().append_child(chip, menu).unwrap();
    app.draw_if_dirty().unwrap(); // dropdown open
    let _ = app.dom_mut().drop_subtree(menu);
    app.draw_if_dirty().unwrap(); // dropdown closed

    app.dom_mut().set_attribute(b2, "hide", "").unwrap();
    app.draw_if_dirty().unwrap();

    // Fresh-paint the App's (incremental-cascade) DOM and count the glyph.
    let dom = app.dom_mut();
    dom.layout_dom(vp);
    let mut buf = Buffer::empty(vp);
    dom.paint_dom(&mut buf, vp);
    let mut glyphs = 0;
    for y in 0..vp.height {
        for x in 0..vp.width {
            if buf.cell(x, y).map(|c| c.symbol()) == Some("X") {
                glyphs += 1;
            }
        }
    }
    assert_eq!(glyphs, 1, "the relative cell's glyph paints exactly once");
}
