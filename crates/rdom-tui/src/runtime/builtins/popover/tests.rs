//! The `popover` attribute (HTML §6.12): its states, the show / hide /
//! toggle algorithms with their events, the auto and hint stacks, focus,
//! `popovertarget` invokers and the attribute's change steps.

use std::cell::RefCell;
use std::rc::Rc;

use rdom_core::{DomError, EventDetail, ListenerOptions, NodeId, ToggleState, TopLayerKind};

use super::{PopoverState, hide_popover, popover_state, show_popover, toggle_popover};
use crate::TuiDom;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::style::Stylesheet;

fn el(dom: &mut TuiDom, parent: NodeId, tag: &str, attrs: &[(&str, &str)]) -> NodeId {
    let e = dom.create_element(tag);
    for (k, v) in attrs {
        dom.set_attribute(e, k, v).unwrap();
    }
    dom.append_child(parent, e).unwrap();
    e
}

fn app(dom: TuiDom) -> App<TestBackend> {
    let terminal = Terminal::new(TestBackend::new(30, 8)).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();
    app.advance(0).unwrap();
    app
}

/// Every `beforetoggle` / `toggle` fired at `ids`, as
/// `"<type> <id> <old>-><new>"`, plus whether `beforetoggle` was
/// cancelable and the event's source.
type Log = Rc<RefCell<Vec<String>>>;

fn log_toggles(dom: &mut TuiDom, ids: &[NodeId]) -> Log {
    let log: Log = Rc::default();
    for &id in ids {
        for ty in ["beforetoggle", "toggle"] {
            let log = log.clone();
            dom.add_event_listener(id, ty, ListenerOptions::default(), move |ctx| {
                let EventDetail::Toggle(t) = &ctx.event.detail else {
                    panic!("{ty} without a toggle detail");
                };
                let state = |s| match s {
                    ToggleState::Open => "open",
                    ToggleState::Closed => "closed",
                };
                let mut line = format!(
                    "{ty} {} {}->{}",
                    id.n(),
                    state(t.old_state),
                    state(t.new_state)
                );
                if ctx.event.cancelable {
                    line.push_str(" cancelable");
                }
                if let Some(s) = t.source {
                    line.push_str(&format!(" from {}", s.n()));
                }
                log.borrow_mut().push(line);
            })
            .unwrap();
        }
    }
    log
}

/// A short, stable name for a node in the logs.
trait Short {
    fn n(&self) -> String;
}

impl Short for NodeId {
    fn n(&self) -> String {
        format!("{self:?}")
    }
}

fn showing(dom: &TuiDom, id: NodeId) -> bool {
    dom.top_layer_kind(id) == Some(TopLayerKind::Popover)
}

// ── The attribute's states ─────────────────────────────────────────

/// HTML §6.12: `popover` is an enumerated attribute — `auto` (and the
/// empty string) the auto state, `manual`, `hint`; the invalid value
/// default is manual, the missing value default no popover. Keywords
/// match ASCII case-insensitively.
#[test]
fn the_popover_attribute_states() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    for (value, state) in [
        (Some(""), Some(PopoverState::Auto)),
        (Some("auto"), Some(PopoverState::Auto)),
        (Some("AuTo"), Some(PopoverState::Auto)),
        (Some("manual"), Some(PopoverState::Manual)),
        (Some("hint"), Some(PopoverState::Hint)),
        (Some("bogus"), Some(PopoverState::Manual)),
        (None, None),
    ] {
        let e = el(&mut dom, root, "div", &[]);
        if let Some(v) = value {
            dom.set_attribute(e, "popover", v).unwrap();
        }
        assert_eq!(popover_state(&dom, e), state, "{value:?}");
    }
}

// ── showPopover / hidePopover / togglePopover ──────────────────────

/// HTML §6.12.2 "show popover": validity first — no popover attribute is
/// `NotSupportedError`, a disconnected element `InvalidStateError`; then
/// a cancelable `beforetoggle` (closed → open), the top layer,
/// `:popover-open`, and `toggle`. Showing a shown popover does nothing.
#[test]
fn show_popover_fires_beforetoggle_and_toggle_and_enters_the_top_layer() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", &[("popover", "")]);
    let plain = el(&mut dom, root, "div", &[]);
    let loose = dom.create_element("div");
    dom.set_attribute(loose, "popover", "").unwrap();
    let log = log_toggles(&mut dom, &[p]);
    assert!(matches!(
        show_popover(&mut dom, plain),
        Err(DomError::NotSupported(_))
    ));
    assert!(matches!(
        show_popover(&mut dom, loose),
        Err(DomError::InvalidState(_))
    ));
    show_popover(&mut dom, p).unwrap();
    assert!(showing(&dom, p));
    assert!(dom.matches(p, ":popover-open").unwrap());
    assert_eq!(
        *log.borrow(),
        [
            format!("beforetoggle {} closed->open cancelable", p.n()),
            format!("toggle {} closed->open", p.n())
        ]
    );
    show_popover(&mut dom, p).unwrap();
    assert_eq!(log.borrow().len(), 2, "already showing: nothing");
}

/// A canceled `beforetoggle` keeps the popover hidden, and no `toggle`
/// fires.
#[test]
fn a_canceled_beforetoggle_keeps_the_popover_hidden() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", &[("popover", "manual")]);
    let log = log_toggles(&mut dom, &[p]);
    dom.add_event_listener(p, "beforetoggle", ListenerOptions::default(), |ctx| {
        ctx.event.prevent_default()
    })
    .unwrap();
    show_popover(&mut dom, p).unwrap();
    assert!(!showing(&dom, p));
    assert_eq!(log.borrow().len(), 1);
}

/// HTML §6.12.2 "hide popover": a `beforetoggle` that cannot be
/// canceled (open → closed), out of the top layer, `toggle`; hiding a
/// hidden popover does nothing. `togglePopover(force)` shows or hides
/// and returns whether it is showing.
#[test]
fn hide_and_toggle() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", &[("popover", "manual")]);
    let log = log_toggles(&mut dom, &[p]);
    hide_popover(&mut dom, p).unwrap();
    assert!(log.borrow().is_empty());
    assert!(toggle_popover(&mut dom, p, None).unwrap());
    assert!(toggle_popover(&mut dom, p, Some(true)).unwrap());
    assert!(!toggle_popover(&mut dom, p, None).unwrap());
    assert!(!toggle_popover(&mut dom, p, Some(false)).unwrap());
    assert!(!showing(&dom, p));
    assert_eq!(
        log.borrow()[2..],
        [
            format!("beforetoggle {} open->closed", p.n()),
            format!("toggle {} open->closed", p.n())
        ]
    );
    let plain = el(&mut dom, root, "div", &[]);
    assert!(matches!(
        hide_popover(&mut dom, plain),
        Err(DomError::NotSupported(_))
    ));
}

/// A modal dialog cannot also be shown as a popover (HTML §6.12.2 "check
/// popover validity": its *is modal* flag is an `InvalidStateError`).
#[test]
fn a_modal_dialog_is_no_popover_to_show() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let d = el(&mut dom, root, "dialog", &[("popover", "")]);
    crate::runtime::builtins::dialog::show_modal(&mut dom, d);
    assert!(matches!(
        show_popover(&mut dom, d),
        Err(DomError::InvalidState(_))
    ));
    assert_eq!(dom.top_layer_kind(d), Some(TopLayerKind::ModalDialog));
}

// ── The auto and hint stacks ───────────────────────────────────────

/// HTML §6.12.2: showing an auto popover hides the auto popovers that
/// are not its ancestors — through the DOM, or through the element that
/// invoked it — and leaves manual ones alone.
#[test]
fn showing_an_auto_popover_hides_the_unrelated_ones() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let a = el(&mut dom, root, "div", &[("popover", "auto")]);
    let child = el(&mut dom, a, "div", &[("popover", "auto")]);
    let invoker = el(&mut dom, a, "button", &[]);
    let invoked = el(&mut dom, root, "div", &[("popover", "auto")]);
    let other = el(&mut dom, root, "div", &[("popover", "auto")]);
    let manual = el(&mut dom, root, "div", &[("popover", "manual")]);
    show_popover(&mut dom, manual).unwrap();
    show_popover(&mut dom, a).unwrap();
    show_popover(&mut dom, child).unwrap();
    assert!(
        showing(&dom, a) && showing(&dom, child),
        "a DOM descendant nests"
    );
    super::show_popover_from(&mut dom, invoked, Some(invoker)).unwrap();
    assert!(
        showing(&dom, a) && !showing(&dom, child) && showing(&dom, invoked),
        "an invoker inside `a` nests `invoked` in it; `child` was not its ancestor"
    );
    show_popover(&mut dom, other).unwrap();
    assert!(!showing(&dom, a) && !showing(&dom, invoked) && showing(&dom, other));
    assert!(
        showing(&dom, manual),
        "manual popovers are not in the stack"
    );
}

/// Hiding an auto popover hides the ones nested above it first, top
/// down ("hide all popovers until").
#[test]
fn hiding_an_auto_popover_hides_its_descendants_first() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let a = el(&mut dom, root, "div", &[("popover", "")]);
    let b = el(&mut dom, a, "div", &[("popover", "")]);
    let c = el(&mut dom, b, "div", &[("popover", "")]);
    for p in [a, b, c] {
        show_popover(&mut dom, p).unwrap();
    }
    let log = log_toggles(&mut dom, &[a, b, c]);
    hide_popover(&mut dom, a).unwrap();
    assert!(dom.top_layer().is_empty());
    let order: Vec<String> = log
        .borrow()
        .iter()
        .filter(|l| l.starts_with("toggle"))
        .cloned()
        .collect();
    assert_eq!(
        order,
        [c, b, a].map(|p| format!("toggle {} open->closed", p.n()))
    );
}

/// HTML §6.12.2 (hint popovers): a hint opens above the auto stack
/// without closing it and closes the other hints; an auto popover
/// opening closes every hint.
#[test]
fn hints_stack_above_autos() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let menu = el(&mut dom, root, "div", &[("popover", "auto")]);
    let tip = el(&mut dom, root, "div", &[("popover", "hint")]);
    let tip2 = el(&mut dom, root, "div", &[("popover", "hint")]);
    let other = el(&mut dom, root, "div", &[("popover", "auto")]);
    show_popover(&mut dom, menu).unwrap();
    show_popover(&mut dom, tip).unwrap();
    assert!(showing(&dom, menu) && showing(&dom, tip));
    show_popover(&mut dom, tip2).unwrap();
    assert!(showing(&dom, menu) && !showing(&dom, tip) && showing(&dom, tip2));
    show_popover(&mut dom, other).unwrap();
    assert!(!showing(&dom, tip2) && !showing(&dom, menu) && showing(&dom, other));
}

// ── Focus ──────────────────────────────────────────────────────────

/// HTML §6.12.2 "popover focusing steps": showing focuses the popover's
/// autofocus delegate; hiding with focus inside returns it to the
/// element focused before (an auto popover remembers it).
#[test]
fn focus_moves_in_on_show_and_back_on_hide() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let opener = el(&mut dom, root, "button", &[]);
    let p = el(&mut dom, root, "div", &[("popover", "")]);
    let field = el(&mut dom, p, "input", &[("autofocus", "")]);
    let mut app = app(dom);
    crate::runtime::focus::focus_node(app.dom_mut(), Some(opener));
    show_popover(app.dom_mut(), p).unwrap();
    assert_eq!(app.dom().focused(), Some(field));
    hide_popover(app.dom_mut(), p).unwrap();
    assert_eq!(app.dom().focused(), Some(opener));
    // Focus moved out before the hide: it stays where it went.
    show_popover(app.dom_mut(), p).unwrap();
    let elsewhere = app.dom_mut().create_element("button");
    app.dom_mut().append_child(root, elsewhere).unwrap();
    crate::runtime::focus::focus_node(app.dom_mut(), Some(elsewhere));
    hide_popover(app.dom_mut(), p).unwrap();
    assert_eq!(app.dom().focused(), Some(elsewhere));
}

// ── popovertarget ──────────────────────────────────────────────────

/// HTML §6.12.3: a button's `popovertarget` toggles the popover it names
/// when activated (the `toggle` reporting the button as its source);
/// `popovertargetaction=show` / `hide` only show / only hide; a disabled
/// button and a form's submit button invoke nothing.
#[test]
fn popovertarget_buttons_invoke_their_popover() {
    use crate::accessors::TuiAccessorsMut;
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", &[("popover", ""), ("id", "p")]);
    let toggle = el(&mut dom, root, "button", &[("popovertarget", "p")]);
    let show = el(
        &mut dom,
        root,
        "input",
        &[
            ("type", "button"),
            ("popovertarget", "p"),
            ("popovertargetaction", "show"),
        ],
    );
    let hide = el(
        &mut dom,
        root,
        "button",
        &[("popovertarget", "p"), ("popovertargetaction", "HIDE")],
    );
    let off = el(
        &mut dom,
        root,
        "button",
        &[("popovertarget", "p"), ("disabled", "")],
    );
    let form = el(&mut dom, root, "form", &[]);
    let submit = el(&mut dom, form, "button", &[("popovertarget", "p")]);
    let log = log_toggles(&mut dom, &[p]);
    let mut app = app(dom);
    app.dom_mut().node_mut(toggle).click();
    assert!(showing(app.dom(), p));
    assert_eq!(
        log.borrow()[1],
        format!("toggle {} closed->open from {}", p.n(), toggle.n())
    );
    app.dom_mut().node_mut(show).click();
    assert!(showing(app.dom(), p), "show does not hide");
    app.dom_mut().node_mut(toggle).click();
    assert!(!showing(app.dom(), p));
    app.dom_mut().node_mut(hide).click();
    assert!(!showing(app.dom(), p), "hide does not show");
    app.dom_mut().node_mut(show).click();
    app.dom_mut().node_mut(hide).click();
    assert!(!showing(app.dom(), p));
    app.dom_mut().node_mut(off).click();
    app.dom_mut().node_mut(submit).click();
    assert!(!showing(app.dom(), p));
}

// ── The attribute's change steps ───────────────────────────────────

/// HTML §6.12 (the `popover` attribute change steps): a showing
/// popover whose attribute changes state, or goes, is hidden — with its
/// events — before the App's next event or frame.
#[test]
fn changing_the_attribute_hides_a_showing_popover() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = el(&mut dom, root, "div", &[("popover", "auto")]);
    let q = el(&mut dom, root, "div", &[("popover", "manual")]);
    let log = log_toggles(&mut dom, &[p, q]);
    let mut app = app(dom);
    show_popover(app.dom_mut(), p).unwrap();
    show_popover(app.dom_mut(), q).unwrap();
    app.dom_mut().set_attribute(p, "popover", "manual").unwrap();
    app.dom_mut().set_attribute(q, "popover", "bogus").unwrap();
    app.advance(0).unwrap();
    assert!(!showing(app.dom(), p), "auto → manual hides");
    assert!(
        showing(app.dom(), q),
        "manual → invalid (manual) is the same state"
    );
    app.dom_mut().remove_attribute(q, "popover").unwrap();
    app.advance(0).unwrap();
    assert!(!showing(app.dom(), q));
    assert!(
        log.borrow()
            .iter()
            .any(|l| *l == format!("toggle {} open->closed", q.n()))
    );
}
