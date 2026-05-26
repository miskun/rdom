//! Regression: `<details>` source disclosure must toggle on every
//! click. Reproduced as: expand → collapse → re-expand third click
//! fell on a stale `<pre>` rect (display: none child, layout pass
//! doesn't update its rect, so the OPEN-state rect lingered after
//! collapse). Hit-test now skips `display: none` per CSS Display 3 §2.5.

use crossterm::event::{
    Event as CtEvent, KeyModifiers, MouseButton, MouseEvent as CtMouseEvent, MouseEventKind,
};
use rdom_showcase::{
    DEMOS, ShowcaseState, build_shell, mount_demo, shell::base_stylesheet, wire_focus_hints,
    wire_scroll_indicator, wire_sidebar_click, wire_sidebar_keys,
};
use rdom_tui::node::TuiNodeExt;
use rdom_tui::render::{Terminal, TestBackend};
use rdom_tui::runtime::app::App;
use rdom_tui::{NodeId, TuiDom};
use std::cell::RefCell;
use std::rc::Rc;

fn click(app: &mut App<TestBackend>, x: u16, y: u16) {
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Up(MouseButton::Left),
    ] {
        app.handle_event(CtEvent::Mouse(CtMouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: KeyModifiers::empty(),
        }));
    }
    app.draw_if_dirty().unwrap();
}

fn open_attr_present(app: &App<TestBackend>, id: NodeId) -> bool {
    app.dom().node(id).has_attribute("open")
}

fn summary_y(app: &App<TestBackend>, summary: NodeId) -> i32 {
    app.dom().node(summary).tui_ext().unwrap().layout.y
}

fn summary_x(app: &App<TestBackend>, summary: NodeId) -> i32 {
    app.dom().node(summary).tui_ext().unwrap().layout.x
}

#[test]
fn expand_collapse_expand_via_mouse_router() {
    // Build full showcase app the same way `main.rs` does it.
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);
    let state = Rc::new(RefCell::new(ShowcaseState::from_handles(&handles)));
    mount_demo(&mut state.borrow_mut(), &mut dom, 0);

    wire_sidebar_click(&mut dom, handles.sidebar, Rc::clone(&state));
    wire_sidebar_keys(&mut dom, handles.sidebar, Rc::clone(&state));
    wire_scroll_indicator(&mut dom, handles.main, handles.status_bar);
    wire_focus_hints(&mut dom, handles.status_bar);

    let backend = TestBackend::new(123, 26);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, base_stylesheet(), terminal).unwrap();
    for demo in DEMOS {
        app.push_stylesheet(demo.stylesheet());
    }

    // Initial draw — populates layout rects.
    app.draw_if_dirty().unwrap();

    let src = handles.source_disclosure;
    let summary = app
        .dom()
        .node(src)
        .child_nodes()
        .find(|c| c.tag_name() == Some("summary"))
        .map(|n| n.id())
        .expect("<summary>");

    let log = |app: &App<TestBackend>, label: &str| {
        let sy = summary_y(app, summary);
        let sx = summary_x(app, summary);
        eprintln!(
            "{label}: open={} summary at ({sx}, {sy})",
            open_attr_present(app, src)
        );
    };

    log(&app, "initial");

    // Click 1 — expand. Click in the middle of the summary's row.
    let sx = summary_x(&app, summary) as u16;
    let sy = summary_y(&app, summary) as u16;
    click(&mut app, sx + 5, sy);
    log(&app, "after expand 1");
    let after1 = open_attr_present(&app, src);

    // Click 2 — collapse. Summary's coords may have changed (open
    // state moves the disclosure UP, closed moves it DOWN).
    let sx = summary_x(&app, summary) as u16;
    let sy = summary_y(&app, summary) as u16;
    click(&mut app, sx + 5, sy);
    log(&app, "after collapse");
    let after2 = open_attr_present(&app, src);

    // Click 3 — expand again. The regression case: the previously-
    // visible `<pre>` children of `<details>` get `display: none`
    // when [open] is removed (UA `details:not([open]) > *:not(summary)`
    // rule), but the layout pass doesn't update their rects — they
    // hold their last visible position. Without the hit-test
    // display:none guard, this third click lands on the stale `<pre>`
    // rect at (29, 20, 91, 8) instead of the summary at (29, 23).
    let sx = summary_x(&app, summary) as u16;
    let sy = summary_y(&app, summary) as u16;
    click(&mut app, sx + 5, sy);
    log(&app, "after expand 2");
    let after3 = open_attr_present(&app, src);

    assert!(after1, "click 1 should expand");
    assert!(!after2, "click 2 should collapse");
    assert!(
        after3,
        "click 3 should re-expand — regressed when display: none child \
         rects intercepted clicks meant for the summary"
    );
}
