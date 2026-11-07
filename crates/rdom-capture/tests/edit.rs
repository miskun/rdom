//! CAPTURE phase 4 (`specs/CAPTURE.md` §Edits): edits and forced states
//! change what the app shows and nothing app code can see; `patch` writes
//! them to the CSS file as a diff; edit → patch → reload gives the same
//! frame; `reset` gives back the first one.

use std::cell::Cell;
use std::error::Error;
use std::path::PathBuf;
use std::rc::Rc;

use rdom_capture::{CaptureApp, CaptureRequest, EditTarget, Environment, Session};
use rdom_tui::core_api::{ForcedState, Mutation, MutationObserver};
use rdom_tui::render::TestBackend;
use rdom_tui::{App, ListenerOptions, TuiDom, TuiExt};

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
        Ok(env.app(dom, sheet)?)
    }
}

/// A session over a page reading `name`.css, its file, and its app
/// observer's and app listener's counts.
type Opened = (Session<Page>, PathBuf, Rc<Cell<usize>>, Rc<Cell<usize>>);

fn page(name: &str) -> Opened {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("rdom-capture-edit");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{name}.css"));
    std::fs::write(&path, CSS).unwrap();
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
    let (files, dropped, _) = s.reload().unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(dropped.len(), 2, "the edits are in the file now");
    s.advance(100).unwrap();
    assert_eq!(hash(&s), edited, "the file holds what the edits showed");
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
