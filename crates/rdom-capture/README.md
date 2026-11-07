# rdom-capture

Headless capture sessions for [rdom-tui](https://crates.io/crates/rdom-tui) apps: open an app without a terminal, drive it with keys, text, the mouse and a virtual clock, and capture any moment as a versioned JSON document — for agents and people improving how an app looks and behaves.

- **Deterministic.** The environment (size, color depth, color scheme, SGR extensions, media preferences) is set, never probed, and time moves only when the session advances it. The same steps give the same captures, byte for byte.
- **One contract.** A `Capture` (schema `rdom-capture/1`) is the single source of truth; the text view is derived from it.
- **A session, not a script.** The app is built once and driven step by step, in Rust or over a JSON-RPC session on stdio that any tool can talk to.

## Opt an app in

An app ships a small binary that hands a `CaptureApp` to `serve`:

```rust,no_run
use rdom_capture::{CaptureApp, Environment, FixtureInfo};
use rdom_tui::render::TestBackend;
use rdom_tui::{App, Stylesheet, TuiDom};

struct MyApp;

impl CaptureApp for MyApp {
    fn build(
        &self,
        env: &Environment,
        fixture: Option<&str>,
    ) -> Result<App<TestBackend>, Box<dyn std::error::Error>> {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let p = dom.create_element("p");
        let text = dom.create_text_node(if fixture == Some("empty") { "" } else { "hello" });
        dom.append_child(p, text)?;
        dom.append_child(root, p)?;
        Ok(env.app(dom, Stylesheet::new())?)
    }

    fn fixtures(&self) -> Vec<FixtureInfo> {
        vec![
            FixtureInfo::new("hello", "a greeting"),
            FixtureInfo::new("empty", "no content"),
        ]
    }
}

fn main() -> std::io::Result<()> {
    rdom_capture::serve(MyApp)
}
```

## Drive it

One JSON-RPC 2.0 request per line on stdin, one response per line on stdout:

```text
{"jsonrpc":"2.0","id":1,"method":"describe"}
{"jsonrpc":"2.0","id":2,"method":"open","params":{"fixture":"hello","env":{"viewport":{"width":80,"height":24}}}}
{"jsonrpc":"2.0","id":3,"method":"input","params":{"inputs":[{"type":"key","key":"tab"},{"type":"text","text":"ada"}]}}
{"jsonrpc":"2.0","id":4,"method":"advance","params":{"ms":300}}
{"jsonrpc":"2.0","id":5,"method":"capture","params":{"layers":["grid"],"text":{"bg":true,"fg":true}}}
```

`describe` lists every method with its parameters. Every step answers with what changed on screen — how many cells, where, and the new grid hash — so a client can tell whether a step did anything without fetching a capture.

Then ask about it:

```text
{"jsonrpc":"2.0","id":6,"method":"outline"}
{"jsonrpc":"2.0","id":7,"method":"explain","params":{"at":[12,4]}}
{"jsonrpc":"2.0","id":8,"method":"explain","params":{"select":".panel","properties":["border-top-color"]}}
{"jsonrpc":"2.0","id":9,"method":"query","params":{"color":"#c0c0c0"}}
{"jsonrpc":"2.0","id":10,"method":"capture","params":{"text":{"owners":true}}}
```

- `outline` — landmarks, headings, lists and controls, nested, with roles, names, rects and state: read it first.
- `explain` — why a cell or an element looks as it does: the paint operations on the cell, the element whose paint it shows (through any `opacity` layer), and for each property its winning declaration (value, selector, specificity, `file:line`), what that overrode, or the ancestor it inherits from.
- `query` — who uses a color, what is painted in a rectangle, where a text is, what a selector matches.
- `capture` layers `owners`, `boxes`, `nodes` — which element painted every cell, every element's boxes, what each element is.

Then change it — without a rebuild, and without app code noticing:

```text
{"jsonrpc":"2.0","id":11,"method":"edit","params":{"node":"12","property":"color","value":"#9aa4b2"}}
{"jsonrpc":"2.0","id":12,"method":"force","params":{"node":"20","state":"hover"}}
{"jsonrpc":"2.0","id":13,"method":"patch"}
{"jsonrpc":"2.0","id":14,"method":"patch","params":{"apply":true}}
{"jsonrpc":"2.0","id":15,"method":"reload"}
```

- `edit` — a declaration in place: in the rule an element's property wins by (`node`), a rule by its place (`rule: [sheet, index]`, as `explain` names it), or a new rule (`selector`).
- `force` — an element into `:hover`, `:active`, `:focus`, `:focus-visible` or `:focus-within`, for the cascade only.
- `patch` — the edits as unified diffs against the CSS files their rules are written in (a dry run unless `apply`), and the edits no file can take, with why; `reload` reads the files again.
- `reset` — every edit and forced state undone.

Edits and forced states fire no events and no mutation records in app code, and `matches()` reports the real state; the transitions they start are listed in each step's `suppressed_events` instead of reaching app listeners. Load CSS with `rdom_css::css_file!("src/app.css")` and a debug build rereads the file at every load.

Every answering method takes `detail` (`outline`, `summary`, `full`) and `budget` (about that many tokens); a cut answer lists what it dropped under `truncated`. Rules record where they are written when the CSS is parsed with a name (`rdom_css::from_css_at(text, "src/app.css")`), or built in Rust (the calling line).

## In Rust

```rust
use rdom_capture::{CaptureApp, CaptureRequest, Environment, Input, Layer, Session};
use rdom_tui::render::TestBackend;
use rdom_tui::{App, Stylesheet, TuiDom};

struct Hello;

impl CaptureApp for Hello {
    fn build(
        &self,
        env: &Environment,
        _fixture: Option<&str>,
    ) -> Result<App<TestBackend>, Box<dyn std::error::Error>> {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let p = dom.create_element("p");
        let text = dom.create_text_node("hello");
        dom.append_child(p, text)?;
        dom.append_child(root, p)?;
        Ok(env.app(dom, Stylesheet::new())?)
    }
}

let mut session = Session::new(Hello);
session.open(Environment::default(), None).unwrap();
session.input(&[Input::Key { key: "tab".into() }]).unwrap();
session.advance(100).unwrap();
let capture = session.capture(&CaptureRequest::with_layers([Layer::Grid])).unwrap();
assert_eq!(capture.meta.time_ms, 100);
assert_eq!(capture.grid.unwrap().row_text(0).trim_end(), "hello");
```

## Status

Phases 1–4 of `specs/CAPTURE.md` in the rdom repository: the deterministic environment, the session and its protocol, the grid, owners, boxes and nodes layers, the text view, paint traces, style traces, the outline, `explain`, `query`, and edits, forced states, patches and reloads. Images, audits and motion follow.

## License

MIT
