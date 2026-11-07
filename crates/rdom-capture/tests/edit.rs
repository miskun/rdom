//! CAPTURE phase 4 (`specs/CAPTURE.md` §Edits): edits and forced states
//! change what the app shows and nothing app code can see; `patch` writes
//! them to the CSS file as a diff — exactly what the edit showed, or not
//! at all and why; edit → patch → reload gives the same frame; `reset`
//! gives back the first one.

use std::cell::Cell;
use std::error::Error;
use std::path::PathBuf;
use std::rc::Rc;

use rdom_capture::{CaptureApp, CaptureRequest, EditTarget, Environment, Session};
use rdom_tui::core_api::{ForcedState, Mutation, MutationObserver};
use rdom_tui::render::TestBackend;
use rdom_tui::{App, ControlFlow, ListenerOptions, TuiDom, TuiExt};

const CSS: &str = ".panel {\n  color: #c0c0c0;\n  transition: color 50ms;\n}\n\n.panel:hover {\n  color: #ff0000;\n}\n";

/// A page reading its sheet from `path`, with an app observer and an app
/// listener counting what app code sees.
struct Page {
    path: PathBuf,
    records: Rc<Cell<usize>>,
    events: Rc<Cell<usize>>,
}

struct Count(Rc<Cell<usize>>);
impl MutationObserver<TuiExt> for Count {
    fn observe(&mut self, _: &mut TuiDom, _: &Mutation) {
        self.0.set(self.0.get() + 1);
    }
}

impl CaptureApp for Page {
    fn build(
        &self,
        env: &Environment,
        _: Option<&str>,
    ) -> Result<App<TestBackend>, Box<dyn Error>> {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let p = dom.create_element("p");
        dom.set_attribute(p, "class", "panel")?;
        let t = dom.create_text_node("hello");
        dom.append_child(p, t)?;
        dom.append_child(root, p)?;
        let b = dom.create_element("button");
        let bt = dom.create_text_node("go");
        dom.append_child(b, bt)?;
        dom.append_child(root, b)?;
        let e = self.events.clone();
        let _ = dom.add_event_listener(p, "transitionend", ListenerOptions::default(), move |_| {
            e.set(e.get() + 1)
        });
        dom.add_mutation_observer(Box::new(Count(self.records.clone())));
        let text = std::fs::read_to_string(&self.path)?;
        let sheet = rdom_css::from_css_at(&text, self.path.to_str().unwrap());
        // A test asks the app to set its own sheet (a theme switch) by
        // writing `<name>.swap`; the app's next tick does.
        let swap = self.path.with_extension("swap");
        Ok(env.app(dom, sheet)?.with_tick_handler(move |ctx| {
            if std::fs::remove_file(&swap).is_ok() {
                ctx.set_stylesheet(rdom_css::from_css(".panel { color: #0000ff; }"));
            }
            ControlFlow::Continue
        }))
    }
}

/// A session over a page reading `name`.css, its file, and its app
/// observer's and app listener's counts.
type Opened = (Session<Page>, PathBuf, Rc<Cell<usize>>, Rc<Cell<usize>>);

fn page(name: &str) -> Opened {
    page_with(name, CSS)
}

fn page_with(name: &str, css: &str) -> Opened {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("rdom-capture-edit");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{name}.css"));
    std::fs::write(&path, css).unwrap();
    let records = Rc::new(Cell::new(0));
    let events = Rc::new(Cell::new(0));
    let mut s = Session::new(Page {
        path: path.clone(),
        records: records.clone(),
        events: events.clone(),
    });
    s.open(Environment::default().with_viewport(20, 2), None)
        .unwrap();
    records.set(0);
    (s, path, records, events)
}

fn hash(s: &Session<Page>) -> String {
    s.capture(&CaptureRequest::default())
        .unwrap()
        .meta
        .grid_hash
}

fn panel_key(s: &Session<Page>) -> String {
    let dom = s.app().unwrap().dom();
    rdom_capture::capture::node_key(dom.query_selector(".panel").unwrap().id())
}

#[test]
fn an_edit_changes_the_frame_unseen_and_patches_its_file() {
    let (mut s, path, records, events) = page("edit");
    let first = hash(&s);
    let node = panel_key(&s);
    let (edited, _) = s
        .edit(
            &EditTarget::Winner { node },
            "color",
            Some("#00ff00"),
            false,
        )
        .unwrap();
    // `transition: color 50ms`: the edit's frame is the transition's first.
    assert!(s.advance(100).unwrap().cells_changed > 0);
    assert_eq!(edited.selector, ".panel");
    assert_eq!(edited.previous.as_deref(), Some("silver"), "{edited:?}");
    assert!(
        edited.source.as_deref().unwrap().ends_with("edit.css:1:1"),
        "{edited:?}"
    );
    assert_ne!(hash(&s), first);
    assert_eq!(records.get(), 0, "an edit is no DOM mutation");
    assert_eq!(events.get(), 0, "the edit's transition is the inspector's");
    // A dry run: the diff, the file untouched.
    let patch = s.patch(false).unwrap();
    assert!(patch.unpatched.is_empty(), "{patch:?}");
    assert_eq!(patch.files.len(), 1);
    assert!(
        patch.files[0]
            .diff
            .contains("-  color: #c0c0c0;\n+  color: #00ff00;\n"),
        "{}",
        patch.files[0].diff
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), CSS);
}

#[test]
fn edit_patch_reload_gives_the_same_frame() {
    let (mut s, path, _, _) = page("round-trip");
    let node = panel_key(&s);
    s.edit(
        &EditTarget::Winner { node },
        "color",
        Some("#123456"),
        false,
    )
    .unwrap();
    s.edit(
        &EditTarget::NewRule {
            selector: "button".into(),
        },
        "color",
        Some("#654321"),
        false,
    )
    .unwrap();
    s.advance(100).unwrap();
    let edited = hash(&s);
    let patch = s.patch(true).unwrap();
    assert!(patch.applied);
    let written = std::fs::read_to_string(&path).unwrap();
    assert!(written.contains("color: #123456;"), "{written}");
    assert!(
        written.contains("button {\n  color: #654321;\n}"),
        "{written}"
    );
    // Applying reads the files back: the edits are in them now.
    s.advance(100).unwrap();
    assert_eq!(hash(&s), edited, "the file holds what the edits showed");
    let reloaded = s.reload().unwrap();
    assert_eq!(reloaded.files.len(), 1);
    assert!(reloaded.dropped.is_empty(), "{reloaded:?}");
    s.advance(100).unwrap();
    assert_eq!(hash(&s), edited);
}

#[test]
fn a_forced_state_shows_and_reset_takes_everything_back() {
    let (mut s, _, records, events) = page("force");
    let first = hash(&s);
    let node = panel_key(&s);
    s.force(&node, ForcedState::Hover, true).unwrap();
    let c = s.advance(100).unwrap();
    assert_ne!(hash(&s), first, "styled as hovered");
    assert!(
        c.suppressed_events
            .iter()
            .any(|e| e.event == "transitionend"),
        "{c:?}"
    );
    assert_eq!((records.get(), events.get()), (0, 0));
    s.edit(
        &EditTarget::Winner { node: node.clone() },
        "color",
        Some("#0000ff"),
        false,
    )
    .unwrap();
    s.reset().unwrap();
    s.advance(100).unwrap();
    assert_eq!(hash(&s), first, "back to the first frame");
    assert_eq!(records.get(), 0);
}

#[test]
fn a_ua_rules_edit_has_no_file_and_says_why() {
    let (mut s, _, _, _) = page("ua");
    let dom = s.app().unwrap().dom();
    let button = rdom_capture::capture::node_key(dom.query_selector("button").unwrap().id());
    s.edit(
        &EditTarget::Winner { node: button },
        "color",
        Some("#ffffff"),
        false,
    )
    .unwrap();
    let patch = s.patch(false).unwrap();
    assert!(patch.files.is_empty());
    assert_eq!(patch.unpatched.len(), 1);
    assert!(
        patch.unpatched[0].reason.contains("user-agent"),
        "{patch:?}"
    );
}

fn winner(s: &mut Session<Page>, property: &str, value: Option<&str>) {
    let node = panel_key(s);
    s.edit(&EditTarget::Winner { node }, property, value, false)
        .unwrap();
}

#[test]
fn a_selector_lists_rule_shows_the_edit_on_every_item_as_its_file_will() {
    let css = ".panel, button {\n  color: #c0c0c0;\n}\n";
    let (mut s, path, _, _) = page_with("list", css);
    let first = hash(&s);
    winner(&mut s, "color", Some("#00ff00"));
    s.advance(100).unwrap();
    let edited = hash(&s);
    let button_only = {
        // The button shows the edit too: one block, one declaration.
        let (mut t, _, _, _) = page_with(
            "list-check",
            ".panel, button {\n  color: #c0c0c0;\n}\nbutton { color: #00ff00; }\n.panel { color: #00ff00; }\n",
        );
        t.advance(100).unwrap();
        hash(&t)
    };
    assert_ne!(edited, first);
    assert_eq!(edited, button_only, "both items changed");
    s.patch(true).unwrap();
    s.advance(100).unwrap();
    assert_eq!(hash(&s), edited, "the file shows what the edit did");
    assert!(
        std::fs::read_to_string(&path)
            .unwrap()
            .contains(".panel, button {\n  color: #00ff00;")
    );
}

#[test]
fn a_commented_declaration_is_found_and_removed() {
    let css = ".panel {\n  padding-left: 1;\n  /* the brand color */\n  color: #c0c0c0;\n}\n";
    let (mut s, path, _, _) = page_with("comment", css);
    winner(&mut s, "color", None);
    s.advance(100).unwrap();
    let edited = hash(&s);
    let patch = s.patch(true).unwrap();
    assert!(patch.unpatched.is_empty(), "{patch:?}");
    let written = std::fs::read_to_string(&path).unwrap();
    assert!(!written.contains("color: #c0c0c0"), "{written}");
    s.advance(100).unwrap();
    assert_eq!(hash(&s), edited);
}

#[test]
fn an_edit_the_file_cannot_express_is_unpatched_and_apply_writes_nothing() {
    let css = ".panel {\n  margin: 1;\n  color: #c0c0c0;\n}\n";
    let (mut s, path, _, _) = page_with("shorthand", css);
    winner(&mut s, "color", Some("#00ff00"));
    winner(&mut s, "margin-top", None);
    let patch = s.patch(false).unwrap();
    assert_eq!(patch.unpatched.len(), 1, "{patch:?}");
    assert!(patch.unpatched[0].reason.contains("`margin`"), "{patch:?}");
    // Applying would read the file back and lose that edit: refused.
    let err = s.patch(true).unwrap_err().to_string();
    assert!(err.contains("e2"), "{err}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), css);
}

#[test]
fn applying_twice_writes_each_edit_once() {
    let (mut s, path, _, _) = page("twice");
    winner(&mut s, "color", Some("#111111"));
    s.edit(
        &EditTarget::NewRule {
            selector: "button".into(),
        },
        "color",
        Some("#222222"),
        false,
    )
    .unwrap();
    s.patch(true).unwrap();
    winner(&mut s, "color", Some("#333333"));
    let patch = s.patch(true).unwrap();
    assert!(patch.unpatched.is_empty(), "{patch:?}");
    let written = std::fs::read_to_string(&path).unwrap();
    assert_eq!(written.matches("button {").count(), 1, "{written}");
    assert!(written.contains(".panel {\n  color: #333333;"), "{written}");
    assert!(!written.contains("#111111"), "{written}");
}

#[test]
fn a_file_changed_since_it_was_parsed_is_not_patched_blindly() {
    let (mut s, path, _, _) = page("moved");
    winner(&mut s, "color", Some("#00ff00"));
    // Someone edits the file: the rule is not at its line any more.
    std::fs::write(&path, format!("a {{ color: red; }}\n{CSS}")).unwrap();
    let patch = s.patch(false).unwrap();
    assert_eq!(patch.unpatched.len(), 1, "{patch:?}");
    assert!(patch.unpatched[0].reason.contains("no longer"), "{patch:?}");
    // A file that cannot be read is reported, not an error mid-write.
    std::fs::remove_file(&path).unwrap();
    let patch = s.patch(false).unwrap();
    assert!(
        patch.unpatched[0].reason.contains("cannot read"),
        "{patch:?}"
    );
}

#[test]
fn text_that_would_escape_its_declaration_is_refused() {
    let (mut s, _, _, _) = page("escape");
    let node = panel_key(&s);
    for value in ["1; } * { color: red", "red !important", "x }"] {
        let err = s.edit(
            &EditTarget::Winner { node: node.clone() },
            "--x",
            Some(value),
            false,
        );
        assert!(err.is_err(), "{value}");
    }
    let err = s.edit(
        &EditTarget::NewRule {
            selector: "button {} p".into(),
        },
        "color",
        Some("red"),
        false,
    );
    assert!(err.is_err());
    // A refused new rule leaves nothing behind.
    let bad = s.edit(
        &EditTarget::NewRule {
            selector: "button".into(),
        },
        "color",
        Some("not-a-color"),
        false,
    );
    assert!(bad.is_err());
    assert!(s.patch(false).unwrap().files.is_empty());
}

#[test]
fn an_edit_to_a_sheet_the_app_replaced_is_dropped_and_says_why() {
    let (mut s, path, _, _) = page("replaced");
    winner(&mut s, "color", Some("#00ff00"));
    // The app sets its own sheet (a theme switch).
    std::fs::write(path.with_extension("swap"), "").unwrap();
    s.advance(200).unwrap();
    let app_frame = hash(&s);
    let patch = s.patch(false).unwrap();
    assert!(patch.files.is_empty(), "{patch:?}");
    assert!(
        patch
            .unpatched
            .iter()
            .any(|n| n.reason.contains("replaced")),
        "{patch:?}"
    );
    let reset = s.reset().unwrap();
    s.advance(100).unwrap();
    assert_eq!(hash(&s), app_frame, "the app's sheet stays");
    let _ = reset;
}

#[test]
fn a_refused_edit_changes_no_rule_of_its_block() {
    // `li::marker` takes no padding: the edit is refused — and `.panel`,
    // the block's other rule, must not keep it.
    let css = ".panel, li::marker {\n  color: #c0c0c0;\n}\n";
    let (mut s, _, _, _) = page_with("atomic", css);
    let first = hash(&s);
    let (sheet, index) = {
        let app = s.app().unwrap();
        let id = app.stylesheet_ids()[0];
        let rules = app.stylesheet(id).unwrap().rules();
        let index = rules
            .iter()
            .position(|r| r.source_text == ".panel")
            .unwrap();
        (0, index)
    };
    let refused = s.edit(
        &EditTarget::Rule { sheet, index },
        "padding-left",
        Some("2"),
        false,
    );
    assert!(refused.is_err());
    s.advance(100).unwrap();
    assert_eq!(hash(&s), first, "nothing changed");
    assert!(s.patch(false).unwrap().files.is_empty());
    assert!(s.fork().is_ok(), "the log still rebuilds the session");
}

#[test]
fn a_file_changed_since_open_stops_a_fork_and_says_which() {
    let (mut s, path, _, _) = page("changed");
    std::fs::write(&path, CSS.replace("#c0c0c0", "#101010")).unwrap();
    s.reload().unwrap();
    match s.fork() {
        Err(rdom_capture::SessionError::Unreplayable(why)) => {
            assert!(why.contains("changed.css"), "{why}")
        }
        other => panic!("{:?}", other.map(|_| ())),
    }
}
