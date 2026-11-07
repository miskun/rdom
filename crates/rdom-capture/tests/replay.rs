//! CAPTURE phase 9 (`specs/CAPTURE.md` §Record and replay): what a replay
//! reproduces — mouse, resize and paste included — and what it refuses or
//! stops at, saying why: an App configured otherwise, a session that took
//! steps, an unreplayable mark, a replaced log.

use std::cell::RefCell;
use std::error::Error;
use std::io::Write;
use std::rc::Rc;

use rdom_capture::{CaptureApp, Environment, Input, MouseAction, Session};
use rdom_tui::render::TestBackend;
use rdom_tui::runtime::record::Recorder;
use rdom_tui::{App, ListenerOptions};

/// A button counting clicks into a paragraph, and a text field.
struct Page;

impl CaptureApp for Page {
    fn build(
        &self,
        env: &Environment,
        _: Option<&str>,
    ) -> Result<App<TestBackend>, Box<dyn Error>> {
        let (mut dom, _) = rdom_parser::parse(r#"<button>press</button><p>0</p><input value="">"#)?;
        let b = dom.query_selector("button").unwrap().id();
        let p = dom.query_selector("p").unwrap().id();
        let mut n = 0;
        let _ = dom.add_event_listener(b, "click", ListenerOptions::default(), move |ctx| {
            n += 1;
            ctx.dom.set_text_content(p, &n.to_string()).unwrap();
        });
        Ok(env.app(dom, rdom_css::from_css("button { color: #00ff00; }"))?)
    }
}

#[derive(Clone, Default)]
struct Shared(Rc<RefCell<Vec<u8>>>);

impl Write for Shared {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn env() -> Environment {
    Environment::default().with_viewport(30, 6)
}

/// The App a session opens, recording into `out`, driven through
/// `inputs` (each followed by its frame and `gap` ms).
fn record(out: impl Write + 'static, inputs: &[Input], gap: u64) -> App<TestBackend> {
    let env = env();
    let mut app = env
        .enforce(Page.build(&env, None).unwrap())
        .with_recorder(Recorder::new(out));
    app.advance(0).unwrap();
    Page.after_first_frame(&mut app, None);
    app.advance(0).unwrap();
    for input in inputs {
        if let Input::Resize { width, height } = input {
            app.terminal_mut().backend_mut().resize(*width, *height);
        }
        for event in input.events().unwrap() {
            app.handle_event(event);
            app.advance(0).unwrap();
        }
        app.advance(gap).unwrap();
    }
    app
}

fn text(log: &Shared) -> String {
    String::from_utf8(log.0.borrow().clone()).unwrap()
}

fn open() -> Session<Page> {
    let mut s = Session::new(Page);
    s.open(env(), None).unwrap();
    s
}

#[test]
fn mouse_resize_and_paste_replay_frame_for_frame() {
    let log = Shared::default();
    let inputs = [
        Input::Mouse {
            action: MouseAction::Click,
            x: 1,
            y: 0,
            button: Default::default(),
        },
        Input::Key { key: "tab".into() },
        Input::Key { key: "tab".into() },
        Input::Paste {
            text: "two words".into(),
        },
        Input::Resize {
            width: 24,
            height: 6,
        },
    ];
    let app = record(log.clone(), &inputs, 40);
    let mut s = Session::new(Page);
    s.open(env(), None).unwrap();
    let r = s.replay(&text(&log), None).unwrap();
    assert_eq!(
        (r.divergence.clone(), r.unreplayable.clone()),
        (None, None),
        "{r:?}\n{}",
        text(&log)
    );
    assert!(r.events >= 5, "{r:?}");
    assert_eq!(
        s.app().unwrap().terminal().buffer().content_hash(),
        app.terminal().buffer().content_hash()
    );
}

#[test]
fn a_recording_of_an_app_configured_otherwise_is_refused_by_name() {
    let log = Shared::default();
    record(log.clone(), &[Input::Key { key: "tab".into() }], 10);
    // The recorded App blinked its caret; the session's does not.
    let tampered = text(&log).replacen("caret=off", "caret=530", 1);
    let err = open().replay(&tampered, None).unwrap_err().to_string();
    assert!(err.contains("caret is 530 there and off here"), "{err}");
}

#[test]
fn a_replay_needs_a_fresh_session_and_stops_at_what_it_cannot_replay() {
    let log = Shared::default();
    record(log.clone(), &[Input::Key { key: "tab".into() }], 10);
    let mut stepped = open();
    stepped.advance(5).unwrap();
    assert!(
        stepped
            .replay(&text(&log), None)
            .unwrap_err()
            .to_string()
            .contains("freshly opened")
    );
    // A closure another thread injected: the replay stops there, with why.
    let mut lines: Vec<String> = text(&log).lines().map(str::to_string).collect();
    lines.insert(2, "0 unreplayable another%20thread%20injected".into());
    let r = open().replay(&lines.join("\n"), None).unwrap();
    assert_eq!(
        r.unreplayable.as_deref(),
        Some("another thread injected"),
        "{r:?}"
    );
}

#[test]
fn follow_waits_for_a_whole_line_and_refuses_a_replaced_log() {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("rdom-capture-replay");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("follow.rec");
    let log = Shared::default();
    record(log.clone(), &[Input::Key { key: "tab".into() }], 10);
    let full = text(&log);
    // Half of a line: nothing past it is taken yet.
    let cut = full.rfind(" key ").map(|i| i + 3).unwrap();
    std::fs::write(&path, &full[..cut]).unwrap();
    let mut s = open();
    let first = s.follow(&path).unwrap();
    assert_eq!(first.events, 0, "{first:?}");
    std::fs::write(&path, &full).unwrap();
    let next = s.follow(&path).unwrap();
    assert_eq!(
        (next.events, next.divergence.clone()),
        (1, None),
        "{next:?}"
    );
    // A new run in the same file: refused, not replayed on top.
    std::fs::write(&path, full.lines().next().unwrap().to_string() + "\n").unwrap();
    let err = s.follow(&path).unwrap_err().to_string();
    assert!(err.contains("another recording"), "{err}");
}
