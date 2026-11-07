//! Unit tests for the session, the grid, the views and the protocol, one
//! section per behavior.

use std::error::Error;

use crossterm::event::{Event, KeyCode, KeyModifiers};
use rdom_tui::render::TestBackend;
use rdom_tui::{App, Color, ColorDepth, ListenerOptions, TuiDom};

use crate::capture::{Attr, ColorValue, Grid, Layer};
use crate::env::{Depth, Environment, Viewport};
use crate::input::{Input, MouseAction, parse_key};
use crate::protocol::{METHODS, code, handle_line};
use crate::session::{CaptureRequest, Session, SessionError};
use crate::views::text::{TextOptions, text};
use crate::{CaptureApp, FixtureInfo};

// ── fixtures ──

/// A page with a line of text and a button; clicking the button (or
/// pressing Enter on it) appends "!" to the text. Fixture "panic-build"
/// panics while building; "panic-click" panics in the click listener;
/// "wrong-size" builds a 10×5 terminal whatever the environment says.
struct Page;

const CSS: &str = "p { color: #c0c0c0; } button { background: #224466; }";

impl CaptureApp for Page {
    fn build(
        &self,
        env: &Environment,
        fixture: Option<&str>,
    ) -> Result<App<TestBackend>, Box<dyn Error>> {
        if fixture == Some("panic-build") {
            panic!("boom while building");
        }
        let mut dom = TuiDom::new();
        let root = dom.root();
        let p = dom.create_element("p");
        let t = dom.create_text_node("hello");
        dom.append_child(p, t)?;
        dom.append_child(root, p)?;
        let b = dom.create_element("button");
        let label = dom.create_text_node("go");
        dom.append_child(b, label)?;
        dom.append_child(root, b)?;
        let panics = fixture == Some("panic-click");
        let _listener =
            dom.add_event_listener(b, "click", ListenerOptions::default(), move |ctx| {
                if panics {
                    panic!("boom in a listener");
                }
                let text = ctx.dom.text_content(t);
                ctx.dom
                    .node_mut(t)
                    .set_node_value(&format!("{text}!"))
                    .unwrap();
            });
        let sheet = rdom_css::from_css(CSS);
        if fixture == Some("wrong-size") {
            let terminal = rdom_tui::render::Terminal::new(TestBackend::new(10, 5))?;
            return Ok(App::with_backend(dom, sheet, terminal)?);
        }
        Ok(env.app(dom, sheet)?)
    }

    fn fixtures(&self) -> Vec<FixtureInfo> {
        ["plain", "panic-build", "panic-click", "wrong-size"]
            .into_iter()
            .map(|n| FixtureInfo::new(n, n))
            .collect()
    }
}

fn small() -> Environment {
    Environment {
        viewport: Viewport {
            width: 20,
            height: 4,
        },
        ..Environment::default()
    }
}

fn open(fixture: &str) -> Session<Page> {
    let mut s = Session::new(Page);
    s.open(small(), Some(fixture)).unwrap();
    s
}

fn grid(s: &Session<Page>) -> Grid {
    s.capture(&CaptureRequest::with_layers([Layer::Grid]))
        .unwrap()
        .grid
        .unwrap()
}

// ── colors ──

#[test]
fn color_values_round_trip_through_their_string_form() {
    for (c, s) in [
        (ColorValue::Default, "default"),
        (ColorValue::Rgb(0x1e, 0x1e, 0x1e), "#1e1e1e"),
        (ColorValue::Rgba(1, 2, 3, 4), "#01020304"),
        (ColorValue::Ansi(208), "ansi:208"),
    ] {
        assert_eq!(c.to_string(), s);
        assert_eq!(s.parse::<ColorValue>().unwrap(), c);
    }
    for bad in ["", "#12345", "#gggggg", "ansi:256", "red"] {
        assert!(bad.parse::<ColorValue>().is_err(), "{bad:?} parsed");
    }
    assert_eq!(ColorValue::from(Color::Indexed(3)), ColorValue::Ansi(3));
}

// ── keys ──

#[test]
fn keys_parse_modifiers_names_and_single_characters() {
    let k = parse_key("ctrl+s").unwrap();
    assert_eq!(
        (k.code, k.modifiers),
        (KeyCode::Char('s'), KeyModifiers::CONTROL)
    );
    let k = parse_key("shift+tab").unwrap();
    assert_eq!(k.code, KeyCode::BackTab);
    let k = parse_key("Ctrl+Alt+PageDown").unwrap();
    assert_eq!(
        (k.code, k.modifiers),
        (KeyCode::PageDown, KeyModifiers::CONTROL | KeyModifiers::ALT)
    );
    assert_eq!(parse_key("+").unwrap().code, KeyCode::Char('+'));
    let k = parse_key("ctrl++").unwrap();
    assert_eq!(
        (k.code, k.modifiers),
        (KeyCode::Char('+'), KeyModifiers::CONTROL)
    );
    assert_eq!(parse_key("A").unwrap().code, KeyCode::Char('A'));
    assert_eq!(parse_key("f12").unwrap().code, KeyCode::F(12));
    assert_eq!(parse_key("space").unwrap().code, KeyCode::Char(' '));
    for bad in ["", "f25", "hyper+a", "nosuchkey", "a+"] {
        assert!(parse_key(bad).is_err(), "{bad:?} parsed");
    }
}

#[test]
fn a_click_is_a_move_a_press_and_a_release() {
    let click = Input::Mouse {
        action: MouseAction::Click,
        x: 1,
        y: 2,
        button: Default::default(),
    };
    let events = click.events().unwrap();
    assert_eq!(events.len(), 3);
    assert!(
        events
            .iter()
            .all(|e| matches!(e, Event::Mouse(m) if (m.column, m.row) == (1, 2)))
    );
    let text = Input::Text { text: "ab".into() }.events().unwrap();
    assert_eq!(text.len(), 2);
}

#[test]
fn inputs_read_from_their_json_form() {
    let inputs: Vec<Input> = serde_json::from_str(
        r#"[{"type":"key","key":"tab"},{"type":"mouse","action":"wheel-down","x":3,"y":4},
            {"type":"resize","width":80,"height":24},{"type":"paste","text":"x"}]"#,
    )
    .unwrap();
    assert_eq!(inputs.len(), 4);
    assert!(serde_json::from_str::<Input>(r#"{"type":"key","key":"a","extra":1}"#).is_err());
}

// ── grid ──

#[test]
fn runs_tile_every_row_exactly() {
    let s = open("plain");
    let g = grid(&s);
    assert_eq!((g.width, g.height), (20, 4));
    for runs in &g.rows {
        let mut x = 0;
        for r in runs {
            assert_eq!(r.x, x, "runs are contiguous");
            assert!(r.w > 0);
            x += r.w;
        }
        assert_eq!(x, 20, "runs cover the row");
    }
    assert_eq!(g.row_text(0), format!("{:<20}", "hello"));
    let hello = &g.rows[0][0];
    assert_eq!(
        (hello.text.as_str(), hello.fg),
        ("hello", ColorValue::Rgb(0xc0, 0xc0, 0xc0))
    );
}

#[test]
fn a_wide_glyph_is_one_glyph_over_two_cells() {
    let mut buf = rdom_tui::Buffer::empty(rdom_tui::Rect::new(0, 0, 4, 1));
    buf.set_string(0, 0, "中a", rdom_tui::Style::default());
    let g = Grid::from_buffer(&buf, ColorDepth::TrueColor);
    let runs = &g.rows[0];
    assert_eq!(runs.len(), 1);
    assert_eq!((runs[0].text.as_str(), runs[0].w), ("中a ", 4));
}

#[test]
fn attributes_and_shown_colors_are_recorded() {
    let mut buf = rdom_tui::Buffer::empty(rdom_tui::Rect::new(0, 0, 2, 1));
    let style = rdom_tui::Style::default()
        .fg(Color::Rgb(250, 10, 10))
        .add_modifier(rdom_tui::Modifier::BOLD | rdom_tui::Modifier::UNDERLINED);
    buf.set_string(0, 0, "x", style);
    let true_color = Grid::from_buffer(&buf, ColorDepth::TrueColor);
    let x = &true_color.rows[0][0];
    assert_eq!(x.mods, vec![Attr::Bold, Attr::Underline]);
    assert_eq!(x.fg_shown, None, "truecolor shows what was painted");
    let ansi16 = Grid::from_buffer(&buf, ColorDepth::Ansi16);
    assert!(matches!(
        ansi16.rows[0][0].fg_shown,
        Some(ColorValue::Ansi(_))
    ));
}

// ── text view ──

#[test]
fn the_text_view_shows_glyphs_then_keyed_layers() {
    let s = open("plain");
    let g = grid(&s);
    let glyphs = text(&g, TextOptions::GLYPHS);
    assert!(glyphs.starts_with("hello\n"), "{glyphs}");
    assert!(!glyphs.contains("──"), "no layers asked for");
    let all = text(&g, TextOptions::ALL);
    assert!(all.contains("── background ──\n"), "{all}");
    assert!(all.contains("── foreground ──\naaaaa\n"), "{all}");
    assert!(all.contains("a #c0c0c0\n"), "{all}");
    assert!(
        all.contains(" #224466\n"),
        "the button's background is keyed: {all}"
    );
}

// ── session ──

#[test]
fn steps_before_open_fail_with_not_open() {
    let mut s = Session::new(Page);
    assert!(matches!(s.advance(1), Err(SessionError::NotOpen)));
    assert!(matches!(
        s.capture(&CaptureRequest::default()),
        Err(SessionError::NotOpen)
    ));
}

#[test]
fn an_unknown_fixture_is_refused_with_the_known_ones() {
    let mut s = Session::new(Page);
    match s.open(small(), Some("nope")) {
        Err(SessionError::UnknownFixture { known, .. }) => assert!(known.contains(&"plain".into())),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_click_changes_cells_and_reports_where() {
    let mut s = open("plain");
    let before = s
        .capture(&CaptureRequest::default())
        .unwrap()
        .meta
        .grid_hash;
    let click = Input::Mouse {
        action: MouseAction::Click,
        x: 0,
        y: 1,
        button: Default::default(),
    };
    let change = s.input(&[click]).unwrap();
    assert!(change.cells_changed > 0);
    assert_ne!(change.grid_hash, before);
    assert_eq!(grid(&s).row_text(0).trim_end(), "hello!");
    let [_, y, _, _] = change.bounds.unwrap();
    assert_eq!(y, 0, "the text row changed");
}

#[test]
fn an_invalid_input_applies_none_of_the_inputs() {
    let mut s = open("plain");
    let before = grid(&s);
    let click = Input::Mouse {
        action: MouseAction::Click,
        x: 0,
        y: 1,
        button: Default::default(),
    };
    let bad = Input::Key {
        key: "nosuchkey".into(),
    };
    assert!(matches!(
        s.input(&[click, bad]),
        Err(SessionError::InvalidInput(_))
    ));
    assert_eq!(grid(&s), before);
}

#[test]
fn advance_moves_the_virtual_clock() {
    let mut s = open("plain");
    s.advance(250).unwrap();
    let c = s.advance(50).unwrap();
    assert_eq!(c.time_ms, 300);
    assert_eq!(c.cells_changed, 0, "nothing animates");
    assert_eq!(
        s.capture(&CaptureRequest::default()).unwrap().meta.time_ms,
        300
    );
}

#[test]
fn a_resize_changes_the_grid_size() {
    let mut s = open("plain");
    s.input(&[Input::Resize {
        width: 30,
        height: 6,
    }])
    .unwrap();
    let g = grid(&s);
    assert_eq!((g.width, g.height), (30, 6));
}

#[test]
fn a_panic_while_building_closes_the_session() {
    let mut s = Session::new(Page);
    assert!(matches!(
        s.open(small(), Some("panic-build")),
        Err(SessionError::AppPanicked(m)) if m.contains("boom while building")
    ));
    assert!(!s.is_open());
}

#[test]
fn a_panic_in_a_listener_closes_the_session() {
    let mut s = open("panic-click");
    let click = Input::Mouse {
        action: MouseAction::Click,
        x: 0,
        y: 1,
        button: Default::default(),
    };
    assert!(matches!(
        s.input(&[click]),
        Err(SessionError::AppPanicked(m)) if m.contains("boom in a listener")
    ));
    assert!(!s.is_open());
    s.open(small(), Some("plain")).unwrap();
    assert!(s.is_open(), "the session opens again");
}

#[test]
fn an_app_of_the_wrong_size_is_refused() {
    let mut s = Session::new(Page);
    assert!(
        matches!(s.open(small(), Some("wrong-size")), Err(SessionError::Build(m)) if m.contains("10×5"))
    );
}

#[test]
fn the_environment_overrides_the_apps_own_settings() {
    // The app builds at its own depth; the session runs it at the
    // environment's.
    struct Sixteen;
    impl CaptureApp for Sixteen {
        fn build(
            &self,
            env: &Environment,
            _: Option<&str>,
        ) -> Result<App<TestBackend>, Box<dyn Error>> {
            Ok(Page
                .build(env, Some("plain"))?
                .with_color_depth(ColorDepth::Ansi16))
        }
    }
    let mut s = Session::new(Sixteen);
    s.open(small(), None).unwrap();
    assert_eq!(s.app().unwrap().color_depth(), ColorDepth::TrueColor);
    let env = Environment {
        color_depth: Depth::Ansi256,
        ..small()
    };
    s.open(env, None).unwrap();
    assert_eq!(s.app().unwrap().color_depth(), ColorDepth::Ansi256);
}

#[test]
fn an_invalid_environment_is_refused() {
    let mut s = Session::new(Page);
    let env = Environment {
        viewport: Viewport {
            width: 0,
            height: 3,
        },
        ..Environment::default()
    };
    assert!(matches!(
        s.open(env, None),
        Err(SessionError::InvalidEnvironment(_))
    ));
}

#[test]
fn the_same_steps_give_byte_identical_captures() {
    let run = || {
        let mut s = open("plain");
        s.input(&[
            Input::Key { key: "tab".into() },
            Input::Key {
                key: "enter".into(),
            },
        ])
        .unwrap();
        s.advance(1000).unwrap();
        serde_json::to_string(
            &s.capture(&CaptureRequest::with_layers([Layer::Grid]))
                .unwrap(),
        )
        .unwrap()
    };
    let (a, b) = (run(), run());
    assert_eq!(a, b);
    assert!(
        a.contains("hello!"),
        "tab focused the button and enter clicked it: {a}"
    );
}

#[test]
fn environments_read_partially_from_json() {
    let env: Environment =
        serde_json::from_str(r#"{"color_depth":"16","viewport":{"width":80,"height":24}}"#)
            .unwrap();
    assert_eq!(env.color_depth, Depth::Ansi16);
    assert_eq!(
        env.viewport,
        Viewport {
            width: 80,
            height: 24
        }
    );
    assert_eq!(env.animation_fps, 60, "unnamed fields keep their defaults");
    assert!(serde_json::from_str::<Environment>(r#"{"colour_depth":"16"}"#).is_err());
}

// ── protocol ──

fn call(s: &mut Session<Page>, line: &str) -> serde_json::Value {
    let r = handle_line(s, line).expect("a request gets a response");
    serde_json::to_value(r).unwrap()
}

#[test]
fn every_listed_method_is_answered_and_nothing_else() {
    let mut s = Session::new(Page);
    for m in METHODS {
        let r = call(
            &mut s,
            &format!(r#"{{"jsonrpc":"2.0","id":1,"method":"{}"}}"#, m.name),
        );
        assert_ne!(
            r["error"]["code"],
            code::METHOD_NOT_FOUND,
            "{} is listed but not answered",
            m.name
        );
    }
    let r = call(&mut s, r#"{"jsonrpc":"2.0","id":1,"method":"teleport"}"#);
    assert_eq!(r["error"]["code"], code::METHOD_NOT_FOUND);
}

#[test]
fn describe_lists_methods_fixtures_and_the_default_environment() {
    let mut s = Session::new(Page);
    let r = call(&mut s, r#"{"jsonrpc":"2.0","id":7,"method":"describe"}"#);
    assert_eq!(r["id"], 7);
    let result = &r["result"];
    assert_eq!(result["schema"], crate::SCHEMA);
    assert_eq!(result["methods"].as_array().unwrap().len(), METHODS.len());
    assert_eq!(result["fixtures"][0]["name"], "plain");
    assert_eq!(result["default_env"]["viewport"]["width"], 120);
}

#[test]
fn a_session_runs_end_to_end_over_the_protocol() {
    let mut s = Session::new(Page);
    let r = call(
        &mut s,
        r#"{"jsonrpc":"2.0","id":1,"method":"open","params":{"fixture":"plain","env":{"viewport":{"width":20,"height":4}}}}"#,
    );
    assert!(
        r["result"]["grid_hash"]
            .as_str()
            .unwrap()
            .starts_with("fnv1a64:"),
        "{r}"
    );
    let r = call(
        &mut s,
        r#"{"jsonrpc":"2.0","id":2,"method":"input","params":{"inputs":[{"type":"mouse","action":"click","x":0,"y":1}]}}"#,
    );
    assert!(r["result"]["cells_changed"].as_u64().unwrap() > 0, "{r}");
    let r = call(
        &mut s,
        r#"{"jsonrpc":"2.0","id":3,"method":"capture","params":{"text":{"bg":false}}}"#,
    );
    assert!(
        r["result"]["text"]
            .as_str()
            .unwrap()
            .starts_with("hello!\n"),
        "{r}"
    );
    assert!(
        r["result"].get("grid").is_none(),
        "the grid only when asked for"
    );
    assert_eq!(r["result"]["schema"], crate::SCHEMA);
    let r = call(&mut s, r#"{"jsonrpc":"2.0","id":4,"method":"close"}"#);
    assert!(r["result"].is_object());
    let r = call(
        &mut s,
        r#"{"jsonrpc":"2.0","id":5,"method":"advance","params":{"ms":1}}"#,
    );
    assert_eq!(r["error"]["code"], code::NOT_OPEN);
}

#[test]
fn malformed_requests_get_the_right_errors() {
    let mut s = Session::new(Page);
    assert_eq!(
        call(&mut s, "{not json")["error"]["code"],
        code::PARSE_ERROR
    );
    assert_eq!(call(&mut s, "[]")["error"]["code"], code::INVALID_REQUEST);
    assert_eq!(
        call(&mut s, r#"{"jsonrpc":"1.0","id":1,"method":"describe"}"#)["error"]["code"],
        code::INVALID_REQUEST
    );
    assert_eq!(
        call(
            &mut s,
            r#"{"jsonrpc":"2.0","id":1,"method":"advance","params":{"seconds":1}}"#
        )["error"]["code"],
        code::INVALID_PARAMS
    );
    let r = call(
        &mut s,
        r#"{"jsonrpc":"2.0","id":1,"method":"open","params":{"fixture":"x"}}"#,
    );
    assert_eq!(r["error"]["code"], code::UNKNOWN_FIXTURE);
    assert!(r["error"]["data"]["known"].as_array().is_some());
}

#[test]
fn notifications_and_blank_lines_get_no_response() {
    let mut s = Session::new(Page);
    assert!(handle_line(&mut s, "").is_none());
    assert!(
        handle_line(
            &mut s,
            r#"{"jsonrpc":"2.0","method":"open","params":{"fixture":"plain"}}"#
        )
        .is_none()
    );
    assert!(s.is_open(), "the notification ran");
}

// ── review follow-ups (phase 1) ──

#[test]
fn typing_sends_what_a_terminal_sends() {
    let events = Input::Text {
        text: "aB\n\t".into(),
    }
    .events()
    .unwrap();
    let keys: Vec<_> = events
        .iter()
        .map(|e| match e {
            Event::Key(k) => (k.code, k.modifiers),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(
        keys,
        [
            (KeyCode::Char('a'), KeyModifiers::empty()),
            (KeyCode::Char('B'), KeyModifiers::SHIFT),
            (KeyCode::Enter, KeyModifiers::empty()),
            (KeyCode::Tab, KeyModifiers::empty()),
        ]
    );
    assert!(
        Input::Text {
            text: "\u{7}".into()
        }
        .events()
        .is_err()
    );
}

#[test]
fn cells_that_join_when_resplit_keep_their_glyphs() {
    // Two regional indicators in two cells: their text re-segments into
    // one two-cell flag, so the run keeps each cell's glyph.
    let mut buf = rdom_tui::Buffer::empty(rdom_tui::Rect::new(0, 0, 3, 1));
    buf.set_symbol(0, 0, "🇫", rdom_tui::Style::default());
    buf.set_symbol(1, 0, "🇮", rdom_tui::Style::default());
    let g = Grid::from_buffer(&buf, ColorDepth::TrueColor);
    let run = &g.rows[0][0];
    assert_eq!(run.cells(), ["🇫", "🇮", " "]);
    assert!(run.glyphs.is_some());
    let plain = Grid::from_buffer(
        &rdom_tui::Buffer::empty(rdom_tui::Rect::new(0, 0, 3, 1)),
        ColorDepth::TrueColor,
    );
    assert!(plain.rows[0][0].glyphs.is_none(), "only where needed");
}

#[test]
fn key_layers_show_the_colors_the_terminal_shows() {
    let mut buf = rdom_tui::Buffer::empty(rdom_tui::Rect::new(0, 0, 1, 1));
    buf.set_string(
        0,
        0,
        "x",
        rdom_tui::Style::default().bg(Color::Rgb(250, 10, 10)),
    );
    let g = Grid::from_buffer(&buf, ColorDepth::Ansi16);
    let t = text(&g, TextOptions::default());
    assert!(t.contains("a ansi:"), "{t}");
}

#[test]
fn the_grid_hash_is_pinned_to_its_encoding() {
    // The encoding is the hash's contract: a change here is a change of
    // every stored hash, so it must be deliberate.
    let mut buf = rdom_tui::Buffer::empty(rdom_tui::Rect::new(0, 0, 2, 1));
    buf.set_string(
        0,
        0,
        "hi",
        rdom_tui::Style::default().fg(Color::Rgb(1, 2, 3)),
    );
    let g = Grid::from_buffer(&buf, ColorDepth::TrueColor);
    assert_eq!(crate::capture::grid_hash(&g), "fnv1a64:15e342e075b1afd0");
}

#[test]
fn a_null_id_is_answered() {
    let mut s = Session::new(Page);
    let r = call(&mut s, r#"{"jsonrpc":"2.0","id":null,"method":"describe"}"#);
    assert!(r["id"].is_null());
    assert!(r["result"].is_object());
}

// ── owners, boxes, nodes (phase 2) ──

#[test]
fn owners_say_which_elements_paint_decided_each_cell() {
    let s = open("plain");
    let c = s
        .capture(&CaptureRequest::with_layers([Layer::Owners]))
        .unwrap();
    let owners = c.owners.unwrap();
    let nodes = c
        .nodes
        .expect("a layer that names elements brings the nodes table");
    for runs in &owners.rows {
        let mut x = 0;
        for r in runs {
            assert_eq!(r.x, x, "owner runs tile the row");
            x += r.w;
        }
        assert_eq!(x, 20);
    }
    let hello = &owners.rows[0][0];
    assert_eq!((hello.part.as_str(), hello.w), ("text", 5));
    assert_eq!(nodes[hello.node.as_ref().unwrap()].tag, "p");
    let button = owners.rows[1]
        .iter()
        .find(|r| r.node.as_ref().is_some_and(|n| nodes[n].tag == "button"))
        .expect("the button painted row 1");
    assert_eq!(
        button.part, "pseudo:before",
        "the UA bracket `[ ` is the button's ::before: {button:?}"
    );
}

#[test]
fn boxes_place_every_element_and_nodes_name_them() {
    let s = open("plain");
    let c = s
        .capture(&CaptureRequest::with_layers([Layer::Boxes]))
        .unwrap();
    let boxes = c.boxes.unwrap();
    let nodes = c.nodes.unwrap();
    let p = nodes
        .iter()
        .find(|(_, n)| n.tag == "p")
        .map(|(k, _)| k)
        .unwrap();
    assert_eq!(boxes[p].border, [0, 0, 20, 1]);
    assert_eq!(nodes[p].path, "p");
    let b = nodes
        .iter()
        .find(|(_, n)| n.tag == "button")
        .map(|(k, _)| k)
        .unwrap();
    assert_eq!(boxes[b].border[1], 1, "the button is on row 1");
}

#[test]
fn the_text_view_keys_owners_by_element_and_part() {
    let mut s = Session::new(Page);
    let r = call(
        &mut s,
        r#"{"jsonrpc":"2.0","id":1,"method":"open","params":{"fixture":"plain","env":{"viewport":{"width":20,"height":4}}}}"#,
    );
    assert!(r["result"].is_object());
    let r = call(
        &mut s,
        r#"{"jsonrpc":"2.0","id":2,"method":"capture","params":{"text":{"bg":false,"owners":true}}}"#,
    );
    let text = r["result"]["text"].as_str().unwrap();
    assert!(text.contains("── owners ──\naaaaa"), "{text}");
    assert!(text.contains("a text p (#"), "{text}");
    assert!(
        r["result"].get("owners").is_none(),
        "the layer only for the text view"
    );
    assert!(r["result"].get("nodes").is_none());
}

#[test]
fn nodes_keep_document_order_and_paths_escape_their_names() {
    struct Many;
    impl CaptureApp for Many {
        fn build(
            &self,
            env: &Environment,
            _: Option<&str>,
        ) -> Result<App<TestBackend>, Box<dyn Error>> {
            let mut dom = TuiDom::new();
            let root = dom.root();
            for i in 0..12 {
                let p = dom.create_element("p");
                dom.set_attribute(p, "class", &format!("n{i} a:b"))?;
                dom.append_child(root, p)?;
            }
            Ok(env.app(dom, rdom_tui::Stylesheet::new())?)
        }
    }
    let mut s = Session::new(Many);
    s.open(small(), None).unwrap();
    let c = s
        .capture(&CaptureRequest::with_layers([Layer::Nodes]))
        .unwrap();
    let nodes = c.nodes.unwrap();
    let classes: Vec<String> = nodes
        .iter()
        .map(|(_, n)| {
            n.classes
                .iter()
                .find(|c| c.starts_with('n'))
                .unwrap()
                .clone()
        })
        .collect();
    let expected: Vec<String> = (0..12).map(|i| format!("n{i}")).collect();
    assert_eq!(classes, expected, "document order, not key order");
    let json = serde_json::to_string(&nodes).unwrap();
    let back: crate::capture::NodeMap<crate::capture::NodeInfo> =
        serde_json::from_str(&json).unwrap();
    assert_eq!(back, nodes, "the order survives a round trip");
    assert!(
        nodes.iter().next().unwrap().1.path.contains(".a\\:b"),
        "{json}"
    );
}

#[test]
fn owners_look_through_a_groups_layer() {
    struct Faded;
    impl CaptureApp for Faded {
        fn build(
            &self,
            env: &Environment,
            _: Option<&str>,
        ) -> Result<App<TestBackend>, Box<dyn Error>> {
            let mut dom = TuiDom::new();
            let root = dom.root();
            let g = dom.create_element("div");
            dom.set_attribute(g, "class", "g")?;
            let p = dom.create_element("p");
            let t = dom.create_text_node("ghost");
            dom.append_child(p, t)?;
            dom.append_child(g, p)?;
            dom.append_child(root, g)?;
            Ok(env.app(
                dom,
                rdom_css::from_css(".g { opacity: 0.5; color: #00ff00; }"),
            )?)
        }
    }
    let mut s = Session::new(Faded);
    s.open(small(), None).unwrap();
    let c = s
        .capture(&CaptureRequest::with_layers([Layer::Owners]))
        .unwrap();
    let nodes = c.nodes.unwrap();
    let run = &c.owners.unwrap().rows[0][0];
    assert_eq!((run.part.as_str(), run.w), ("text", 5));
    assert_eq!(nodes[run.node.as_deref().unwrap()].tag, "p");
    assert_eq!(
        nodes[run.via.as_deref().unwrap()].classes,
        ["g"],
        "through the group's layer"
    );
    assert!(c.meta.warnings.is_empty(), "{:?}", c.meta.warnings);
}

// ── outline, explain, query (phase 3) ──

fn opened() -> Session<Page> {
    let mut s = Session::new(Page);
    call(
        &mut s,
        r#"{"jsonrpc":"2.0","id":1,"method":"open","params":{"fixture":"plain","env":{"viewport":{"width":20,"height":4}}}}"#,
    );
    s
}

#[test]
fn the_outline_names_the_button_where_it_is() {
    let mut s = opened();
    let r = call(&mut s, r#"{"jsonrpc":"2.0","id":2,"method":"outline"}"#);
    let outline = r["result"]["outline"].as_array().unwrap();
    let button = outline
        .iter()
        .find(|n| n["role"] == "button")
        .expect("the button");
    assert_eq!(button["name"], "go");
    assert_eq!(button["rect"][1], 1);
}

#[test]
fn explain_at_a_cell_traces_its_paint_and_its_color() {
    let mut s = opened();
    let r = call(
        &mut s,
        r#"{"jsonrpc":"2.0","id":2,"method":"explain","params":{"at":[1,0]}}"#,
    );
    let e = &r["result"];
    assert_eq!(e["cell"]["glyph"], "e", "{e}");
    assert_eq!(e["target"]["tag"], "p");
    assert_eq!(e["target"]["part"], "text");
    let color = e["style"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["property"] == "color")
        .unwrap();
    assert_eq!(color["from"], "declared");
    assert_eq!(color["declaration"]["selector"], "p");
    assert!(
        e["paint"]
            .as_array()
            .unwrap()
            .iter()
            .any(|op| op["part"] == "text")
    );
}

#[test]
fn explain_an_element_by_selector_lists_its_declarations() {
    let mut s = opened();
    let r = call(
        &mut s,
        r#"{"jsonrpc":"2.0","id":2,"method":"explain","params":{"select":"button","properties":["background-color","color"]}}"#,
    );
    let style = r["result"]["style"].as_array().unwrap();
    assert_eq!(style[0]["property"], "background-color");
    assert_eq!(
        style[0]["value"], "rgb(34, 68, 102)",
        "as CSSOM serializes it"
    );
    let color = &style[1];
    assert!(["declared", "inherited", "initial"].contains(&color["from"].as_str().unwrap()));
    let r = call(
        &mut s,
        r#"{"jsonrpc":"2.0","id":3,"method":"explain","params":{"select":"nope"}}"#,
    );
    assert_eq!(r["error"]["code"], code::INVALID_PARAMS);
    let r = call(
        &mut s,
        r#"{"jsonrpc":"2.0","id":4,"method":"explain","params":{"at":[0,0],"select":"p"}}"#,
    );
    assert_eq!(
        r["error"]["code"],
        code::INVALID_PARAMS,
        "exactly one target"
    );
}

#[test]
fn queries_find_by_selector_color_rect_and_text() {
    let mut s = opened();
    let q = |s: &mut Session<Page>, params: &str| {
        call(
            s,
            &format!(r#"{{"jsonrpc":"2.0","id":2,"method":"query","params":{params}}}"#),
        )["result"]["found"]
            .as_array()
            .unwrap()
            .clone()
    };
    let by_sel = q(&mut s, r#"{"selector":"p"}"#);
    assert_eq!(by_sel.len(), 1);
    let by_color = q(&mut s, r##"{"color":"#c0c0c0"}"##);
    assert!(
        by_color.iter().any(|f| f["by"] == "fg" && f["cells"] == 5),
        "{by_color:?}"
    );
    let in_rect = q(&mut s, r#"{"rect":[0,1,4,1]}"#);
    assert!(
        in_rect
            .iter()
            .all(|f| f["path"].as_str().unwrap().ends_with("button")),
        "{in_rect:?}"
    );
    let by_text = q(&mut s, r#"{"text":"llo"}"#);
    assert_eq!(by_text[0]["at"], serde_json::json!([2, 0]));
}

#[test]
fn a_budget_cuts_an_answer_and_says_where() {
    let mut s = opened();
    let r = call(
        &mut s,
        r#"{"jsonrpc":"2.0","id":2,"method":"explain","params":{"select":"button","detail":"full","budget":40}}"#,
    );
    let e = &r["result"];
    assert!(e["truncated"].is_object(), "{e}");
}

// ── phase 3 review follow-ups ──

/// A fixture app: `markup` styled by `css`.
struct Markup(&'static str, &'static str);

impl CaptureApp for Markup {
    fn build(
        &self,
        env: &Environment,
        _: Option<&str>,
    ) -> Result<App<TestBackend>, Box<dyn Error>> {
        let mut dom = TuiDom::new();
        let root = dom.root();
        rdom_parser::parse_into(&mut dom, self.0, root)?;
        Ok(env.app(dom, rdom_css::from_css_at(self.1, "t.css"))?)
    }
}

fn markup_session(markup: &'static str, css: &'static str) -> Session<Markup> {
    let mut s = Session::new(Markup(markup, css));
    s.open(Environment::default().with_viewport(30, 6), None)
        .unwrap();
    s
}

fn explain_prop(s: &mut Session<Markup>, select: &str, prop: &str) -> serde_json::Value {
    let line = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "explain",
        "params": {"select": select, "properties": [prop]}})
    .to_string();
    let r = serde_json::to_value(handle_line(s, &line).unwrap()).unwrap();
    assert!(r.get("error").is_none(), "{r}");
    r["result"]["style"][0].clone()
}

#[test]
fn keywords_resolve_to_what_they_mean() {
    let mut s = markup_session(
        r#"<div class="p"><p class="i">x</p><button class="r">b</button></div>"#,
        ".p { color: #102030; }\n.i { color: inherit; }\n.r { color: revert; }",
    );
    let i = explain_prop(&mut s, ".i", "color");
    assert_eq!(
        (i["from"].as_str(), i["keyword"].as_str()),
        (Some("inherited"), Some("inherit")),
        "{i}"
    );
    assert_eq!(i["declaration"]["selector"], ".p");
    let r = explain_prop(&mut s, ".r", "color");
    assert_eq!(r["from"], "reverted", "{r}");
    assert_eq!(r["declaration"]["origin"], "ua");
    assert_eq!(r["keyword"], "revert");
}

#[test]
fn a_registered_property_that_does_not_inherit_is_initial_below() {
    let mut s = markup_session(
        r#"<div class="p"><p class="c">x</p></div>"#,
        "@property --x { syntax: '*'; inherits: false; }\n.p { --x: 3; --y: 4; }",
    );
    assert_eq!(explain_prop(&mut s, ".c", "--x")["from"], "initial");
    let y = explain_prop(&mut s, ".c", "--y");
    assert_eq!(y["from"], "inherited", "{y}");
    assert_eq!(y["computed"], "4");
}

#[test]
fn a_details_slot_is_named_not_skipped() {
    let mut s = markup_session(
        r#"<details open class="d"><summary>s</summary><p class="in">x</p></details>"#,
        ".d::details-content { color: #00ff00; }",
    );
    let c = explain_prop(&mut s, ".in", "color");
    assert_eq!(c["from"], "inherited", "{c}");
    assert!(
        c["note"].as_str().unwrap().contains("::details-content"),
        "{c}"
    );
}

#[test]
fn text_after_a_wide_glyph_is_found_at_its_cell() {
    let mut s = markup_session(r#"<p>中a</p>"#, "");
    let r = call_m(
        &mut s,
        r#"{"jsonrpc":"2.0","id":2,"method":"query","params":{"text":"a"}}"#,
    );
    assert_eq!(
        r["result"]["found"][0]["at"],
        serde_json::json!([2, 0]),
        "{r}"
    );
    let r = call_m(
        &mut s,
        r#"{"jsonrpc":"2.0","id":3,"method":"explain","params":{"select":"p","properties":["colour"]}}"#,
    );
    assert_eq!(
        r["error"]["code"],
        code::INVALID_PARAMS,
        "unknown properties are refused"
    );
}

fn call_m(s: &mut Session<Markup>, line: &str) -> serde_json::Value {
    serde_json::to_value(handle_line(s, line).unwrap()).unwrap()
}

#[test]
fn the_outline_scopes_headers_and_names_by_labelledby_first() {
    let mut s = markup_session(
        r#"<header>top</header><article><header>card</header><h2 id="t">Title</h2><nav aria-label="a" aria-labelledby="t">x</nav></article><img alt="logo">"#,
        "",
    );
    let r = call_m(
        &mut s,
        r#"{"jsonrpc":"2.0","id":2,"method":"outline","params":{"detail":"full"}}"#,
    );
    let text = r["result"].to_string();
    assert_eq!(
        text.matches("\"banner\"").count(),
        1,
        "only the page's header: {text}"
    );
    assert!(
        text.contains(r#""name":"Title","node""#) || text.contains(r#""name":"Title""#),
        "{text}"
    );
    let nav = r["result"]["outline"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["role"] == "article")
        .unwrap()["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["role"] == "navigation")
        .unwrap()
        .clone();
    assert_eq!(nav["name"], "Title", "aria-labelledby wins over aria-label");
}

#[test]
fn delivering_inspector_events_is_reserved_and_refused() {
    let mut s = Session::new(Page);
    let r = call(
        &mut s,
        r#"{"jsonrpc":"2.0","id":1,"method":"open","params":{"fixture":"plain","inspector_events":"deliver"}}"#,
    );
    assert_eq!(r["error"]["code"], code::INVALID_PARAMS);
    assert!(r["error"]["message"].as_str().unwrap().contains("reserved"));
    let r = call(
        &mut s,
        r#"{"jsonrpc":"2.0","id":2,"method":"open","params":{"fixture":"plain","inspector_events":"suppress"}}"#,
    );
    assert!(r["result"].is_object(), "{r}");
}

// ── the png view (phase 5) ──

#[test]
fn a_png_view_is_drawn_with_the_profile_and_scale_asked_for() {
    let mut s = open("plain");
    let r = call(
        &mut s,
        r#"{"jsonrpc":"2.0","id":1,"method":"capture","params":{"png":{"profile":"xterm-light","scale":2}}}"#,
    );
    let png = &r["result"]["png"];
    assert_eq!(png["profile"], "xterm-light", "{r}");
    assert_eq!(png["scale"], 2);
    let (w, h) = (small().viewport.width, small().viewport.height);
    assert_eq!(png["width"], u32::from(w) * 16);
    assert_eq!(png["height"], u32::from(h) * 32);
    // The PNG signature, base64.
    assert!(
        png["base64"].as_str().unwrap().starts_with("iVBORw0KGgo"),
        "{png}"
    );
    assert!(
        r["result"].get("grid").is_none(),
        "the grid layer was not asked for"
    );
    // describe lists the profiles, the default first.
    let d = call(&mut s, r#"{"jsonrpc":"2.0","id":2,"method":"describe"}"#);
    assert_eq!(d["result"]["profiles"][0], "xterm-dark");
}

#[test]
fn a_png_view_refuses_an_unknown_profile_or_scale() {
    let mut s = open("plain");
    let r = call(
        &mut s,
        r#"{"jsonrpc":"2.0","id":1,"method":"capture","params":{"png":{"profile":"amber"}}}"#,
    );
    let msg = r["error"]["message"].as_str().unwrap();
    assert!(msg.contains("xterm-dark"), "the known ones: {msg}");
    let r = call(
        &mut s,
        r#"{"jsonrpc":"2.0","id":2,"method":"capture","params":{"png":{"scale":9}}}"#,
    );
    assert_eq!(r["error"]["code"], code::INVALID_PARAMS, "{r}");
}

#[test]
fn a_report_is_one_deterministic_page_with_every_elements_explanation() {
    let s = open("plain");
    let profile = crate::Profile::default_profile();
    let a = s.report(&profile, 1).unwrap();
    assert_eq!(
        a,
        s.report(&profile, 1).unwrap(),
        "the same frame, the same bytes"
    );
    assert!(a.starts_with("<!doctype html>"));
    assert!(a.contains("data:image/png;base64,"));
    // The embedded data: the frame's image, and the paragraph explained
    // with its winning `color` declaration.
    let data = a
        .split_once(r#"<script type="application/json" id="data">"#)
        .unwrap()
        .1
        .split_once("</script>")
        .unwrap()
        .0;
    let d: serde_json::Value = serde_json::from_str(&data.replace("<\\/", "</")).unwrap();
    assert!(d["png"].as_str().unwrap().starts_with("iVBORw0KGgo"));
    let explained = d["explain"].as_array().unwrap();
    let p = explained
        .iter()
        .find(|e| e[1]["target"]["tag"] == "p")
        .expect("the paragraph is explained");
    let color = p[1]["style"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["property"] == "color")
        .expect("its color");
    assert_eq!(color["declaration"]["selector"], "p", "{color}");
    // Document order, as pairs: node keys are numbers.
    assert!(d["nodes"].is_array());
}
