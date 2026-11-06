//! CAPTURE phase 1 (`specs/CAPTURE.md` §Deterministic environment): a
//! capture session over the showcase gives byte-identical captures for the
//! same steps — in one process and in two — and an acid page's capture is
//! exactly what the acid harness paints.

use std::io::Write;
use std::process::{Command, Stdio};

use rdom_capture::{CaptureRequest, Environment, Grid, Input, Layer, Session, Viewport};
use rdom_showcase::DEMOS;
use rdom_showcase::capture::Showcase;
use rdom_showcase::demos::acid::{self, PAGE_HEIGHT, PAGE_WIDTH};
use rdom_tui::ColorDepth;

use crate::acid::compare::open_page;

fn acid_env() -> Environment {
    let mut env = Environment::default();
    env.viewport = Viewport {
        width: PAGE_WIDTH,
        height: PAGE_HEIGHT,
    };
    env
}

/// `fixture` opened in `env`, `steps` applied, captured with its grid, as
/// JSON.
fn capture_json(fixture: &str, env: &Environment, steps: &[Input]) -> String {
    let mut session = Session::new(Showcase);
    session.open(env.clone(), Some(fixture)).unwrap();
    if !steps.is_empty() {
        session.input(steps).unwrap();
    }
    session.advance(500).unwrap();
    let capture = session
        .capture(&CaptureRequest::with_layers([Layer::Grid]))
        .unwrap();
    capture.to_json()
}

#[test]
fn every_acid_page_captures_byte_identically_twice() {
    for page in 1..=acid::page_count() {
        let fixture = format!("acid:{page}");
        let a = capture_json(&fixture, &acid_env(), &[]);
        let b = capture_json(&fixture, &acid_env(), &[]);
        assert_eq!(a, b, "{fixture} captured differently");
    }
}

#[test]
fn every_demo_captures_byte_identically_twice_after_input() {
    let steps = [
        Input::Key { key: "tab".into() },
        Input::Key { key: "down".into() },
        Input::Key {
            key: "enter".into(),
        },
    ];
    for demo in DEMOS {
        let fixture = format!("demo:{}", demo.slug());
        let a = capture_json(&fixture, &Environment::default(), &steps);
        let b = capture_json(&fixture, &Environment::default(), &steps);
        assert_eq!(a, b, "{fixture} captured differently");
    }
}

#[test]
fn an_acid_capture_is_what_the_acid_harness_paints() {
    for page in 1..=acid::page_count() {
        let mut harness = open_page(page, None);
        harness.advance(0).unwrap();
        let expected = Grid::from_buffer(harness.terminal().buffer(), ColorDepth::TrueColor);
        let mut session = Session::new(Showcase);
        session
            .open(acid_env(), Some(&format!("acid:{page}")))
            .unwrap();
        let capture = session
            .capture(&CaptureRequest::with_layers([Layer::Grid]))
            .unwrap();
        assert!(
            capture.grid.as_ref() == Some(&expected),
            "acid page {page}: the session's capture differs from the harness's paint"
        );
    }
}

/// The capture binary's stdout for `script`, one request per line.
fn run_binary(script: &str) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rdom-showcase-capture"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the capture binary starts");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "the capture binary failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn the_capture_binary_answers_byte_identically_in_two_processes() {
    let mut script = String::new();
    let mut id = 0;
    let mut line = |method: &str, params: &str| {
        id += 1;
        script.push_str(&format!(
            r#"{{"jsonrpc":"2.0","id":{id},"method":"{method}","params":{params}}}"#
        ));
        script.push('\n');
    };
    for page in 1..=acid::page_count() {
        line(
            "open",
            &format!(
                r#"{{"fixture":"acid:{page}","env":{{"viewport":{{"width":{PAGE_WIDTH},"height":{PAGE_HEIGHT}}}}}}}"#
            ),
        );
        line("advance", r#"{"ms":250}"#);
        line(
            "capture",
            r#"{"layers":["grid"],"text":{"bg":true,"fg":true}}"#,
        );
    }
    line("open", r#"{"fixture":"demo:forms/tab-form"}"#);
    line(
        "input",
        r#"{"inputs":[{"type":"key","key":"tab"},{"type":"text","text":"ada"},{"type":"mouse","action":"click","x":40,"y":10}]}"#,
    );
    line("capture", r#"{"layers":["grid"]}"#);
    let a = run_binary(&script);
    let b = run_binary(&script);
    assert_eq!(a.lines().count(), id, "one response per request");
    assert!(!a.contains(r#""error""#), "no request failed: {a}");
    assert_eq!(a, b, "two processes answered differently");
}
