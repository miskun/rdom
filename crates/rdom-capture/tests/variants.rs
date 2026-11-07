//! CAPTURE phase 6 (`specs/CAPTURE.md` §Variants): `diff` with node
//! identity, `whatif`, `matrix`, `stress` and `mockup`, each on an app
//! with known answers.

use std::error::Error;

use rdom_capture::variants::whatif::Candidate;
use rdom_capture::{CaptureApp, EditTarget, Environment, Input, Session, Step};
use rdom_tui::render::TestBackend;
use rdom_tui::{App, ListenerOptions, TuiDom};

/// A titled list: pressing the button adds a row (`data-key`), and the
/// title counts the rows.
struct List;

const CSS: &str = "#title { color: #c0c0c0; } li { color: #ffffff; } button { color: #00ff00; }";

impl CaptureApp for List {
    fn build(
        &self,
        env: &Environment,
        _: Option<&str>,
    ) -> Result<App<TestBackend>, Box<dyn Error>> {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let h = dom.create_element("h1");
        dom.set_attribute(h, "id", "title")?;
        let t = dom.create_text_node("2 rows");
        dom.append_child(h, t)?;
        dom.append_child(root, h)?;
        let ul = dom.create_element("ul");
        dom.append_child(root, ul)?;
        for k in ["a", "b"] {
            let li = dom.create_element("li");
            dom.set_attribute(li, "data-key", k)?;
            let t = dom.create_text_node(&format!("row {k}"));
            dom.append_child(li, t)?;
            dom.append_child(ul, li)?;
        }
        let b = dom.create_element("button");
        let bt = dom.create_text_node("add");
        dom.append_child(b, bt)?;
        dom.append_child(root, b)?;
        let mut n = 2;
        let _ = dom.add_event_listener(b, "click", ListenerOptions::default(), move |ctx| {
            n += 1;
            let li = ctx.dom.create_element("li");
            ctx.dom
                .set_attribute(li, "data-key", &format!("k{n}"))
                .unwrap();
            let t = ctx.dom.create_text_node(&format!("row {n}"));
            ctx.dom.append_child(li, t).unwrap();
            // A new row at the top of the list.
            let first = ctx.dom.node(ul).first_child().map(|c| c.id());
            ctx.dom.insert_before(ul, li, first).unwrap();
            ctx.dom.set_text_content(h, &format!("{n} rows")).unwrap();
        });
        Ok(env.app(dom, rdom_css::from_css(CSS))?)
    }
}

fn open() -> Session<List> {
    let mut s = Session::new(List);
    s.open(Environment::default().with_viewport(20, 8), None)
        .unwrap();
    s
}

fn press(s: &mut Session<List>) {
    s.input(&[
        Input::Key { key: "tab".into() },
        Input::Key {
            key: "enter".into(),
        },
    ])
    .unwrap();
}

// --- diff ---------------------------------------------------------------

#[test]
fn a_diff_matches_elements_by_identity_not_by_node() {
    let mut s = open();
    s.mark("before").unwrap();
    press(&mut s);
    let d = s.diff("before", None).unwrap();
    // The new row at the top: one element added, matched by data-key; the
    // old rows moved down a line, and still match by their keys.
    assert_eq!(d.added.len(), 1, "{d:#?}");
    assert_eq!(d.added[0].key, "ul/[key=k3]");
    assert!(d.removed.is_empty(), "{d:#?}");
    let moved: Vec<&str> = d.boxes.iter().map(|b| b.key.as_str()).collect();
    assert!(
        moved.contains(&"ul/[key=a]") && moved.contains(&"ul/[key=b]"),
        "{moved:?}"
    );
    assert_eq!(d.matched.id, 1, "the title");
    assert_eq!(d.matched.data_key, 2);
    // The title's text changed on its row.
    let title = d.rows.iter().find(|r| r.y == 0).expect("row 0 changed");
    assert!(
        title.before.contains('2') && title.after.contains('3'),
        "{title:?}"
    );
    assert!(d.cells_changed > 0 && d.bounds.is_some());
}

#[test]
fn a_diff_reports_declared_style_changes_and_nothing_for_the_same_moment() {
    let mut s = open();
    s.mark("a").unwrap();
    let same = s.diff("a", None).unwrap();
    assert_eq!(
        (same.cells_changed, same.styles.len(), same.added.len()),
        (0, 0, 0)
    );
    assert_eq!(same.from, same.to);
    let title = {
        let dom = s.app().unwrap().dom();
        rdom_capture::capture::node_key(dom.get_element_by_id("title").unwrap())
    };
    s.edit(
        &EditTarget::Winner { node: title },
        "color",
        Some("#ff0000"),
        false,
    )
    .unwrap();
    s.mark("b").unwrap();
    let d = s.diff("a", Some("b")).unwrap();
    assert_eq!(d.styles.len(), 1, "{d:#?}");
    assert_eq!(d.styles[0].key, "#title");
    assert_eq!(d.styles[0].property, "color");
    assert_eq!(d.styles[0].after.as_deref(), Some("red"));
    let unknown = s.diff("nope", None).unwrap_err().to_string();
    assert!(unknown.contains("a, b"), "{unknown}");
}

// --- whatif -------------------------------------------------------------

#[test]
fn whatif_tries_each_candidate_from_the_same_base_and_leaves_the_session() {
    let mut s = open();
    press(&mut s);
    let before = s.snapshot().unwrap();
    let title = {
        let dom = s.app().unwrap().dom();
        rdom_capture::capture::node_key(dom.get_element_by_id("title").unwrap())
    };
    let recolor = |v: &str| Step::Edit {
        target: EditTarget::Winner {
            node: title.clone(),
        },
        property: "color".into(),
        value: Some(v.into()),
        important: false,
    };
    let outcomes = s
        .whatif(
            &[
                Candidate::new("red", vec![recolor("#ff0000")]),
                Candidate::new("same", vec![recolor("#c0c0c0")]),
                Candidate::new(
                    "add-another",
                    vec![Step::Input {
                        inputs: vec![Input::Key {
                            key: "enter".into(),
                        }],
                    }],
                ),
                Candidate::new("bad", vec![recolor("not-a-color")]),
            ],
            200,
            None,
        )
        .unwrap();
    let by = |n: &str| outcomes.iter().find(|o| o.name == n).unwrap();
    // Red: the title's cells change, nothing else; its patch has no file
    // (rules built from a string) and says so.
    let red = by("red").diff.as_ref().unwrap();
    assert_eq!(red.bounds.map(|b| b[1]), Some(0), "the title row");
    assert_eq!(red.styles.len(), 1);
    assert!(!by("red").patch.as_ref().unwrap().unpatched.is_empty());
    // The same color: no change at all.
    assert_eq!(by("same").diff.as_ref().unwrap().cells_changed, 0);
    // Another press adds one more row — from the same base, not after red.
    let more = by("add-another").diff.as_ref().unwrap();
    assert_eq!(more.added.len(), 1);
    assert!(more.styles.is_empty(), "red was not carried over");
    // A refused value fails its candidate only, with why.
    assert!(
        by("bad").error.as_deref().unwrap().contains("color"),
        "{:?}",
        by("bad")
    );
    // The session is as it was.
    let after = s.snapshot().unwrap();
    assert_eq!(after.grid_hash(), before.grid_hash());
    assert_eq!(s.log().len(), 1, "the press only");
}

// --- matrix -------------------------------------------------------------

#[test]
fn matrix_replays_the_steps_across_environments() {
    let mut s = open();
    press(&mut s);
    let axes = vec![
        vec![
            serde_json::json!({"viewport": {"width": 20}}),
            serde_json::json!({"viewport": {"width": 12}}),
        ],
        vec![
            serde_json::json!({"color_scheme": "dark"}),
            serde_json::json!({"color_scheme": "light"}),
        ],
    ];
    let variants = s.matrix(&axes, 0).unwrap();
    assert_eq!(variants.len(), 4);
    assert_eq!(variants[1].label, "viewport.width=20 color_scheme=light");
    // Every variant replayed the press: three rows in the title.
    for v in &variants {
        let grid = v.grid.as_ref().unwrap_or_else(|| panic!("{v:?}"));
        assert!(
            grid.row_text(0).starts_with("3 rows"),
            "{}: {}",
            v.label,
            grid.row_text(0)
        );
    }
    assert_eq!(variants[2].env.viewport.width, 12);
    // A contact sheet of the four, two to a row.
    let tiles: Vec<(String, &rdom_capture::Grid)> = variants
        .iter()
        .map(|v| (v.label.clone(), v.grid.as_ref().unwrap()))
        .collect();
    let sheet = rdom_capture::raster::sheet::sheet(&tiles, 2);
    assert_eq!(sheet.width, 20 + 1 + 20);
    // An axis value that is no environment fails the call, with why.
    let bad = s
        .matrix(&[vec![serde_json::json!(3)]], 0)
        .unwrap_err()
        .to_string();
    assert!(bad.contains("object"), "{bad}");
}

// --- stress -------------------------------------------------------------

/// A card 12 cells wide with a label, a paragraph below it, and a narrow
/// clipped badge.
struct Card;

const CARD_CSS: &str =
    ".card { width: 12; } .badge { width: 6; overflow: hidden; white-space: nowrap; }";

impl CaptureApp for Card {
    fn build(
        &self,
        env: &Environment,
        _: Option<&str>,
    ) -> Result<App<TestBackend>, Box<dyn Error>> {
        let (dom, _) = rdom_parser::parse(
            r#"<div class="card"><p id="label">Save</p></div><p id="below">next</p><div class="badge" id="badge">ok</div>"#,
        )?;
        Ok(env.app(dom, rdom_css::from_css(CARD_CSS))?)
    }
}

#[test]
fn stress_reports_what_each_rewrite_did_to_the_layout() {
    use rdom_capture::variants::stress::Rewrite;
    let mut s = Session::new(Card);
    s.open(Environment::default().with_viewport(30, 10), None)
        .unwrap();
    let r = s
        .stress("#label", &[Rewrite::Double, Rewrite::Quadruple], 0)
        .unwrap();
    use rdom_capture::variants::stress::Kind;
    let kinds = |i: usize| -> Vec<Kind> { r[i].findings.iter().map(|f| f.kind).collect() };
    assert_eq!(r[0].key, "#label");
    assert!(kinds(0).is_empty(), "\"Save Save\" fits: {:?}", r[0]);
    assert!(kinds(1).contains(&Kind::Wrapped), "{:?}", r[1]);
    assert!(
        !kinds(1).contains(&Kind::Truncated),
        "wrapped text is all there: {:?}",
        r[1]
    );
    let shifted = r[1]
        .findings
        .iter()
        .find(|f| f.kind == Kind::Shifted)
        .expect("the paragraph below moved");
    assert!(shifted.nodes.contains(&"#below".to_string()), "{shifted:?}");
    // A long word in a clipped, unwrapping badge: cut off.
    let b = s.stress("#badge", &[Rewrite::LongWord], 0).unwrap();
    assert!(
        b[0].findings
            .iter()
            .any(|f| f.kind == rdom_capture::variants::stress::Kind::Truncated),
        "{:?}",
        b[0]
    );
    // The session is as it was, and its log holds no rewrite.
    assert!(s.log().is_empty());
    assert!(s.stress("#nope", &[Rewrite::Empty], 0).is_err());
}

#[test]
fn a_variant_says_which_logged_steps_do_not_carry_over() {
    let mut s = open();
    s.input(&[Input::Resize {
        width: 24,
        height: 8,
    }])
    .unwrap();
    let variants = s
        .matrix(&[vec![serde_json::json!({"viewport": {"width": 30}})]], 0)
        .unwrap();
    let notes = variants[0].notes.join(" ");
    assert!(notes.contains("resize"), "{variants:?}");
}
