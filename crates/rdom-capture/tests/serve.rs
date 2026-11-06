//! The stdio server end to end: requests in, one response per request
//! out, and `capture` writing its files where `out` says.

use std::error::Error;
use std::path::PathBuf;

use rdom_capture::protocol::serve_io;
use rdom_capture::{CaptureApp, Environment};
use rdom_tui::render::TestBackend;
use rdom_tui::{App, Stylesheet, TuiDom};

struct Hello;

impl CaptureApp for Hello {
    fn build(
        &self,
        env: &Environment,
        _: Option<&str>,
    ) -> Result<App<TestBackend>, Box<dyn Error>> {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let p = dom.create_element("p");
        let t = dom.create_text_node("hello");
        dom.append_child(p, t)?;
        dom.append_child(root, p)?;
        Ok(env.app(dom, Stylesheet::new())?)
    }
}

fn serve(script: &str) -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    serve_io(Hello, script.as_bytes(), &mut out).unwrap();
    String::from_utf8(out)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[test]
fn every_request_gets_one_response_in_order() {
    let responses = serve(concat!(
        r#"{"jsonrpc":"2.0","id":1,"method":"describe"}"#,
        "\n\n",
        r#"{"jsonrpc":"2.0","method":"open"}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":"two","method":"advance","params":{"ms":10}}"#,
        "\n",
    ));
    assert_eq!(
        responses.len(),
        2,
        "the blank line and the notification get none"
    );
    assert_eq!(responses[0]["id"], 1);
    assert_eq!(responses[1]["id"], "two");
    assert_eq!(responses[1]["result"]["time_ms"], 10);
}

#[test]
fn capture_writes_its_files_where_out_says() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("rdom-capture-serve");
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("shot.v1");
    let script = format!(
        "{}\n{}\n",
        r#"{"jsonrpc":"2.0","id":1,"method":"open","params":{"env":{"viewport":{"width":10,"height":2}}}}"#,
        serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "capture",
            "params": {"layers": ["grid"], "text": {}, "out": out}}),
    );
    let responses = serve(&script);
    let files = responses[1]["result"]["files"].as_array().unwrap();
    assert_eq!(files.len(), 2);
    let json = std::fs::read_to_string(dir.join("shot.v1.json")).unwrap();
    let capture: rdom_capture::Capture = serde_json::from_str(&json).unwrap();
    assert_eq!(capture.grid.unwrap().row_text(0), "hello     ");
    let text = std::fs::read_to_string(dir.join("shot.v1.txt")).unwrap();
    assert_eq!(text, "hello\n\n");
    assert!(
        responses[1]["result"].get("grid").is_none(),
        "written, not returned"
    );
}
