//! CAPTURE phase 6 (`specs/CAPTURE.md` §Variants): a fork is the session
//! rebuilt from its step log — the same frame, then independent of it —
//! and a session it cannot rebuild says why instead of diverging.

use std::error::Error;

use rdom_capture::{
    CaptureApp, CaptureRequest, EditTarget, Environment, Input, Session, SessionError, Step,
};
use rdom_tui::core_api::ForcedState;
use rdom_tui::render::TestBackend;
use rdom_tui::{App, ListenerOptions, TuiDom};

/// A button counting its presses into a paragraph, and a transition.
struct Counter;

const CSS: &str = "p { color: #c0c0c0; transition: color 100ms; } p:hover { color: #ff0000; } button { color: #ffffff; }";

impl CaptureApp for Counter {
    fn build(
        &self,
        env: &Environment,
        _: Option<&str>,
    ) -> Result<App<TestBackend>, Box<dyn Error>> {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let b = dom.create_element("button");
        let bt = dom.create_text_node("press");
        dom.append_child(b, bt)?;
        dom.append_child(root, b)?;
        let p = dom.create_element("p");
        let t = dom.create_text_node("0");
        dom.append_child(p, t)?;
        dom.append_child(root, p)?;
        let mut n = 0;
        let _ = dom.add_event_listener(b, "click", ListenerOptions::default(), move |ctx| {
            n += 1;
            ctx.dom.set_text_content(p, &n.to_string()).unwrap();
        });
        Ok(env.app(dom, rdom_css::from_css(CSS))?)
    }
}

fn hash(s: &Session<Counter>) -> String {
    s.capture(&CaptureRequest::default())
        .unwrap()
        .meta
        .grid_hash
}

fn key_of(s: &Session<Counter>, selector: &str) -> String {
    let dom = s.app().unwrap().dom();
    rdom_capture::capture::node_key(dom.query_selector(selector).unwrap().id())
}

/// A session driven through every kind of step.
fn driven() -> Session<Counter> {
    let mut s = Session::new(Counter);
    s.open(Environment::default().with_viewport(20, 3), None)
        .unwrap();
    s.input(&[
        Input::Key { key: "tab".into() },
        Input::Key {
            key: "enter".into(),
        },
    ])
    .unwrap();
    s.advance(50).unwrap();
    let p = key_of(&s, "p");
    s.edit(
        &EditTarget::Winner { node: p.clone() },
        "color",
        Some("#00ff00"),
        false,
    )
    .unwrap();
    s.force(&p, ForcedState::Hover, true).unwrap();
    s.advance(30).unwrap();
    s
}

#[test]
fn a_fork_shows_the_sessions_frame_and_then_goes_its_own_way() {
    let mut s = driven();
    let mut f = s.fork().unwrap();
    assert_eq!(hash(&f), hash(&s), "the same frame");
    assert_eq!(f.log(), s.log(), "the same steps");
    // Independent: the fork's steps leave the session as it was.
    let before = hash(&s);
    f.input(&[Input::Key {
        key: "enter".into(),
    }])
    .unwrap();
    f.advance(500).unwrap();
    assert_ne!(hash(&f), before);
    assert_eq!(hash(&s), before);
    // And the session's own steps go on.
    s.advance(500).unwrap();
}

#[test]
fn the_log_lists_the_steps_that_made_the_frame() {
    let s = driven();
    let kinds: Vec<&str> = s
        .log()
        .iter()
        .map(|step| match step {
            Step::Input { .. } => "input",
            Step::Advance { .. } => "advance",
            Step::Edit { .. } => "edit",
            Step::Force { .. } => "force",
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, ["input", "advance", "edit", "force", "advance"]);
}

#[test]
fn a_session_driven_outside_its_steps_is_not_forked() {
    let mut s = driven();
    s.app_mut().unwrap().advance(10).unwrap();
    match s.fork() {
        Err(SessionError::Unreplayable(why)) => assert!(why.contains("app_mut"), "{why}"),
        other => panic!("{:?}", other.map(|_| ())),
    }
}

#[test]
fn a_fork_with_another_environment_replays_the_steps_there() {
    let s = driven();
    let wide = Environment::default().with_viewport(40, 3);
    let f = s.fork_in(wide).unwrap();
    assert_eq!(f.app().unwrap().terminal().buffer().area.width, 40);
    let text = f
        .capture(&CaptureRequest::with_layers([rdom_capture::Layer::Grid]))
        .unwrap();
    // The press counted there too.
    assert!(text.grid.unwrap().row_text(1).starts_with('1'));
}

#[test]
fn the_log_holds_what_was_changed_not_how_it_was_found() {
    let s = driven();
    // The edit targeted "the rule p's color wins by": the log holds that
    // rule, so a replay in another environment edits the same rule.
    let edit = s
        .log()
        .iter()
        .find(|st| matches!(st, Step::Edit { .. }))
        .unwrap();
    assert!(
        matches!(
            edit,
            Step::Edit {
                target: EditTarget::Rule { .. },
                ..
            }
        ),
        "{edit:?}"
    );
    // A forced state names its element by identity, not by arena node.
    let force = s
        .log()
        .iter()
        .find(|st| matches!(st, Step::Force { .. }))
        .unwrap();
    assert!(
        matches!(force, Step::Force { node, .. } if node == "p"),
        "{force:?}"
    );
}
