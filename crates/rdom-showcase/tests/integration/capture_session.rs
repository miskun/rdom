//! CAPTURE phase 1 (`specs/CAPTURE.md` §Deterministic environment): a
//! capture session over the showcase gives byte-identical captures for the
//! same steps — in one process and in two — and an acid page's capture is
//! exactly what the acid harness paints.

use std::io::Write;
use std::process::{Command, Stdio};

use rdom_capture::{CaptureRequest, Environment, Grid, Input, Layer, Session};
use rdom_showcase::DEMOS;
use rdom_showcase::capture::Showcase;
use rdom_showcase::demos::acid::{self, PAGE_HEIGHT, PAGE_WIDTH};
use rdom_tui::ColorDepth;

use crate::acid::compare::open_page;

fn acid_env() -> Environment {
    Environment::default().with_viewport(PAGE_WIDTH, PAGE_HEIGHT)
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

/// CAPTURE phase 6: every demo forks to the same frame after input — the
/// footing of `whatif`, `matrix` and `stress`.
#[test]
fn every_demo_forks_to_its_own_frame_after_input() {
    let steps = [
        Input::Key { key: "tab".into() },
        Input::Key { key: "down".into() },
        Input::Key {
            key: "enter".into(),
        },
    ];
    for demo in DEMOS {
        let fixture = format!("demo:{}", demo.slug());
        let mut session = Session::new(Showcase);
        session
            .open(Environment::default(), Some(&fixture))
            .unwrap();
        session.input(&steps).unwrap();
        session.advance(300).unwrap();
        let fork = session
            .fork()
            .unwrap_or_else(|e| panic!("{fixture} did not fork: {e}"));
        assert_eq!(
            fork.snapshot().unwrap().grid_hash(),
            session.snapshot().unwrap().grid_hash(),
            "{fixture}"
        );
    }
}

/// CAPTURE phase 6: an acid page across schemes and depths.
#[test]
fn an_acid_page_varies_by_environment() {
    let mut session = Session::new(Showcase);
    session.open(acid_env(), Some("acid:1")).unwrap();
    let axes = vec![
        vec![
            serde_json::json!({"color_scheme": "dark"}),
            serde_json::json!({"color_scheme": "light"}),
        ],
        vec![
            serde_json::json!({"color_depth": "truecolor"}),
            serde_json::json!({"color_depth": "16"}),
        ],
    ];
    let variants = session.matrix(&axes, 0).unwrap();
    assert_eq!(variants.len(), 4);
    let hash = |i: usize| {
        variants[i]
            .grid_hash
            .clone()
            .unwrap_or_else(|| panic!("{:?}", variants[i]))
    };
    // Page 1 has no color-scheme rule: the scheme changes nothing, the
    // depth does (dark/truecolor, dark/16, light/truecolor, light/16).
    assert_eq!(hash(0), hash(2), "the scheme changes nothing on page 1");
    assert_eq!(hash(1), hash(3));
    assert_ne!(hash(0), hash(1), "16 colors show other colors");
    // Page 6 holds the media-query tile: the scheme shows there.
    let mut media = Session::new(Showcase);
    media.open(acid_env(), Some("acid:6")).unwrap();
    let schemes = media.matrix(&axes[..1], 0).unwrap();
    assert_ne!(
        schemes[0].grid_hash, schemes[1].grid_hash,
        "acid page 6 reads the scheme"
    );
}

/// The node key of the first element `selector` matches in `session`.
fn key_of(session: &Session<Showcase>, selector: &str) -> String {
    let dom = session.app().unwrap().dom();
    rdom_capture::capture::node_key(dom.query_selector(selector).unwrap().id())
}

/// CAPTURE phase 6, known answers on the showcase: a click adds one list
/// item, and the diff says so by identity.
#[test]
fn a_demo_click_diffs_as_one_added_item() {
    let mut session = Session::new(Showcase);
    session
        .open(
            Environment::default(),
            Some("demo:events/mutation-observer"),
        )
        .unwrap();
    let button = {
        let dom = session.app().unwrap().dom();
        let b = dom.query_selector("button").unwrap().id();
        rdom_tui::capture::box_model(dom, b).unwrap().border
    };
    session.mark("before").unwrap();
    session
        .input(&[Input::Mouse {
            action: rdom_capture::MouseAction::Click,
            x: button.x as u16 + 1,
            y: button.y as u16,
            button: Default::default(),
        }])
        .unwrap();
    let d = session.diff("before", None).unwrap();
    assert_eq!(d.added.len(), 1, "{d:#?}");
    assert!(d.added[0].path.ends_with("li"), "{:?}", d.added[0]);
    assert!(d.removed.is_empty());
}

/// CAPTURE phase 6, known answers: recoloring acid page 1's `mark.c-ua`
/// changes its two cells and nothing else; a mockup of the page is its
/// own text view, and one changed word is one difference.
#[test]
fn an_acid_page_answers_whatif_and_mockup_exactly() {
    use rdom_capture::variants::whatif::Candidate;
    use rdom_capture::{EditTarget, Step};
    let mut session = Session::new(Showcase);
    session.open(acid_env(), Some("acid:1")).unwrap();
    let mark = key_of(&session, ".c-ua");
    let outcomes = session
        .whatif(
            &[Candidate::new(
                "red",
                vec![Step::Edit {
                    target: EditTarget::Winner { node: mark },
                    property: "color".into(),
                    value: Some("#ff0000".into()),
                    important: false,
                }],
            )],
            0,
            None,
        )
        .unwrap();
    let d = outcomes[0]
        .diff
        .as_ref()
        .unwrap_or_else(|| panic!("{:?}", outcomes[0]));
    assert_eq!(d.bounds, Some([1, 1, 2, 1]), "{d:#?}");
    assert_eq!(d.cells_changed, 2);
    assert_eq!(d.styles.len(), 1);
    assert_eq!(d.styles[0].property, "color");

    let grid = session
        .capture(&CaptureRequest::with_layers([Layer::Grid]))
        .unwrap()
        .grid
        .unwrap();
    let view = rdom_capture::views::text::text(&grid, rdom_capture::views::text::TextOptions::ALL);
    assert!(session.mockup(&view, [0, 0], None).unwrap().matches);
    let edited = view.replacen("ua late", "UA late", 1);
    let r = session.mockup(&edited, [0, 0], None).unwrap();
    assert_eq!(r.glyphs.len(), 1, "{r:#?}");
    assert_eq!((r.glyphs[0].y, r.glyphs[0].x), (1, 1));
    assert_eq!(
        (r.glyphs[0].expected.as_str(), r.glyphs[0].actual.as_str()),
        ("UA", "ua")
    );
}

/// CAPTURE phase 6, known answers: acid page 1's first tile label is one
/// clipped line 38 cells wide — text that fits shows whole, text past it
/// is cut, and nothing wraps or moves.
#[test]
fn an_acid_label_is_cut_by_exactly_the_rewrites_that_do_not_fit() {
    use rdom_capture::variants::stress::Rewrite;
    let mut session = Session::new(Showcase);
    session.open(acid_env(), Some("acid:1")).unwrap();
    let r = session
        .stress(".acid-label:first-child", &Rewrite::ALL, 0)
        .unwrap();
    for s in &r {
        use rdom_capture::variants::stress::Kind;
        let kinds: Vec<Kind> = s.findings.iter().map(|f| f.kind).collect();
        let cut = matches!(s.rewrite, Rewrite::Quadruple | Rewrite::LongWord);
        assert_eq!(
            kinds,
            if cut { vec![Kind::Truncated] } else { vec![] },
            "{:?}: {s:?}",
            s.rewrite
        );
        assert_eq!(s.before, s.after, "{:?}: one clipped line", s.rewrite);
    }
}

/// CAPTURE phase 7: a full audit of every acid page runs every check or
/// says why not, and finds what it found when the audits were reviewed —
/// a change of these counts is a change of a check (or of a page) to look
/// at, not to accept blindly. The acid pages are hostile on purpose: they
/// test the platform's edges, not good design.
#[test]
fn every_acid_page_takes_a_full_audit() {
    use rdom_capture::audit::{AuditRequest, Check};
    const KNOWN: &[(u8, &[(&str, usize)])] = &[
        (
            1,
            &[
                ("alignment", 5),
                ("contrast", 17),
                ("tokens", 8),
                ("truncation", 3),
            ],
        ),
        (
            2,
            &[
                ("alignment", 16),
                ("contrast", 13),
                ("depth-loss", 4),
                ("focus-visible", 4),
                ("occlusion", 11),
                ("tokens", 16),
                ("truncation", 9),
            ],
        ),
        (
            3,
            &[
                ("alignment", 1),
                ("contrast", 6),
                ("depth-loss", 1),
                ("landmarks", 1),
                ("palette", 1),
                ("targets", 1),
                ("tokens", 17),
                ("truncation", 4),
            ],
        ),
        (
            4,
            &[
                ("alignment", 1),
                ("contrast", 24),
                ("occlusion", 8),
                ("tokens", 15),
                ("truncation", 5),
            ],
        ),
        (5, &[("contrast", 1), ("occlusion", 4), ("tokens", 6)]),
        (
            6,
            &[
                ("alignment", 17),
                ("contrast", 5),
                ("focus-visible", 1),
                ("occlusion", 2),
                ("tokens", 23),
                ("truncation", 4),
            ],
        ),
        (
            7,
            &[
                ("alignment", 2),
                ("contrast", 2),
                ("focus-visible", 1),
                ("tokens", 8),
            ],
        ),
        (
            8,
            &[
                ("alignment", 13),
                ("contrast", 11),
                ("focus-visible", 1),
                ("targets", 1),
                ("tokens", 10),
                ("truncation", 3),
            ],
        ),
        (
            9,
            &[
                ("contrast", 3),
                ("focus-visible", 2),
                ("palette", 1),
                ("reachability", 1),
                ("tokens", 10),
            ],
        ),
        (
            10,
            &[
                ("contrast", 1),
                ("focus-visible", 1),
                ("landmarks", 1),
                ("reachability", 1),
                ("tokens", 9),
            ],
        ),
        (
            11,
            &[
                ("contrast", 1),
                ("focus-visible", 10),
                ("reachability", 2),
                ("tokens", 3),
            ],
        ),
        (12, &[("alignment", 3), ("focus-visible", 2), ("tokens", 7)]),
        (
            13,
            &[
                ("alignment", 3),
                ("contrast", 3),
                ("tokens", 6),
                ("truncation", 2),
            ],
        ),
    ];
    for &(page, known) in KNOWN {
        let mut session = Session::new(Showcase);
        session
            .open(acid_env(), Some(&format!("acid:{page}")))
            .unwrap();
        let a = session.audit(&AuditRequest::all()).unwrap();
        for c in Check::AUDIT {
            assert!(
                a.checked.contains(&c) || a.not_checked.iter().any(|n| n.check == c.name()),
                "page {page}: {c:?}"
            );
        }
        let mut by: std::collections::BTreeMap<&str, usize> = Default::default();
        for f in &a.findings {
            *by.entry(f.check.name()).or_default() += 1;
        }
        let want: std::collections::BTreeMap<&str, usize> = known.iter().copied().collect();
        assert_eq!(by, want, "page {page}");
    }
}

/// CAPTURE phase 8, the acid motion tile I20 (`specs/ACID.md`): a click
/// opens tile 48 — a panel transitioning `translate: -100% 0` → `0` over
/// 200 ms linear, a box sliding in by `@keyframes` — and the film's
/// timelines put both at −width, −width/2 and 0 at 0, 100 and 200 ms, in
/// whole cells; the `rotate()` spinner paints the same cell throughout.
#[test]
fn a_film_of_acid_tile_48_gives_its_specd_timeline() {
    use rdom_capture::motion::FilmRequest;
    let mut session = Session::new(Showcase);
    session.open(acid_env(), Some("acid:6")).unwrap();
    let go = {
        let dom = session.app().unwrap().dom();
        let b = dom.query_selector(".acid-t48 .go").unwrap().id();
        rdom_tui::capture::box_model(dom, b).unwrap().border
    };
    let click = Input::Mouse {
        action: rdom_capture::MouseAction::Click,
        x: go.x as u16,
        y: go.y as u16,
        button: Default::default(),
    };
    let film = session
        .film(
            &FilmRequest::of(300)
                .with_step(50)
                .with_inputs(vec![click])
                .with_nodes(vec![
                    ".acid-t48 .sl".into(),
                    ".acid-t48 .kf".into(),
                    ".acid-t48 .sp".into(),
                ]),
        )
        .unwrap();
    // The tile's content starts at column 61.
    for line in &film.nodes[..2] {
        let x_at = |t: u64| {
            line.changes
                .iter()
                .rev()
                .find(|c| c.t_ms <= t)
                .and_then(|c| c.border)
                .map(|b| b[0])
        };
        assert_eq!(
            (x_at(0), x_at(100), x_at(200), x_at(300)),
            (Some(51), Some(56), Some(61), Some(61)),
            "{}: {line:#?}",
            line.selector
        );
        let painted_at = |t: u64| {
            line.changes
                .iter()
                .rev()
                .find(|c| c.t_ms <= t)
                .map(|c| c.cells)
        };
        assert_eq!(
            (painted_at(0), painted_at(100), painted_at(200)),
            (Some(0), Some(5), Some(10))
        );
    }
    assert_eq!(film.nodes[2].changes.len(), 1, "the spinner moves nothing");
    // The tile ignores prefers-reduced-motion: the film says so.
    assert!(
        film.findings
            .iter()
            .any(|f| f.check == rdom_capture::audit::Check::ReducedMotion),
        "{:?}",
        film.findings
    );
    assert_eq!(film.frames.len(), 7);
    assert!(film.frames[1..5].iter().all(|f| f.bytes > 0) && film.frames[6].bytes == 0);
}
