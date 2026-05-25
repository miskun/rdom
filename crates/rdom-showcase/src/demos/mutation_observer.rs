//! `MutationObserver` demo — live tree mutations produce visible
//! observer records.
//!
//! Click "Add item" → appends an `<li>` to the list → the observer
//! captures a `ChildListChanged` record → the click handler reads
//! the captured records and re-renders the log `<pre>` below.
//!
//! **Reentrancy note:** observer callbacks receive `&mut Dom` but
//! can't mutate it (the `is_observing` guard panics on mutation
//! attempts). The pattern here keeps the observer pure-capture
//! into a shared `Rc<RefCell<Vec<String>>>`; the log-rendering
//! mutation runs in the click handler *after* `append_child`
//! returns. Same pattern any consumer using `MutationObserver`
//! for read-only audit needs to follow.

use std::cell::{Cell, RefCell};
use std::io;
use std::rc::Rc;

use rdom_tui::{
    App, ListenerOptions, Mutation, MutationObserver, NodeId, Stylesheet, TuiDom, TuiExt,
};

use crate::{Category, Demo, Source};

pub const MARKUP: &str = r#"<div class="mo-demo">
  <h1>MutationObserver</h1>
  <p>Click the button to append a list item. The observer logs each tree mutation.</p>
  <button class="add-btn">Add item</button>
  <ul class="items"></ul>
  <h2>Observer log</h2>
  <pre class="log"></pre>
</div>"#;

pub const CSS: &str = r#"
.mo-demo {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding: 1 2;
  gap: 1;
}
.mo-demo h1 {
  height: 1;
  color: rgb(180, 220, 255);
  font-weight: bold;
}
.mo-demo h2 {
  height: 1;
  color: rgb(150, 180, 210);
}
.mo-demo p {
  height: 1;
}
.mo-demo .items {
  max-height: 8;
  overflow: auto;
}
.mo-demo .log {
  flex: 1;
  display: block;
  overflow: auto;
  color: rgb(200, 210, 230);
}
"#;

/// Captured observer record, in human-readable form. Rendered
/// into the log `<pre>` by the click handler.
struct CapturedRecord(String);

/// Pure-capture observer — appends one line per mutation to the
/// shared records buffer. Never mutates the dom (would panic via
/// `is_observing` guard).
struct CaptureMutations {
    records: Rc<RefCell<Vec<CapturedRecord>>>,
}

impl MutationObserver<TuiExt> for CaptureMutations {
    fn observe(&mut self, _dom: &mut TuiDom, record: &Mutation) {
        let line = match record {
            Mutation::ChildListChanged {
                parent,
                added,
                removed,
            } => format!(
                "ChildListChanged on {parent:?}: +{} -{}",
                added.len(),
                removed.len()
            ),
            Mutation::AttributeChanged { id, name, .. } => {
                format!("AttributeChanged on {id:?}: {name:?}")
            }
            Mutation::ClassChanged { id, added, removed } => {
                format!(
                    "ClassChanged on {id:?}: +{} -{}",
                    added.len(),
                    removed.len()
                )
            }
            Mutation::CharacterDataChanged { id, .. } => {
                format!("CharacterDataChanged on {id:?}")
            }
            Mutation::InteractionChanged { kind, prev, next } => {
                format!("InteractionChanged[{kind:?}]: {prev:?} → {next:?}")
            }
            Mutation::SelectionChanged { .. } => "SelectionChanged".to_string(),
            Mutation::PreDetach {
                detached_root,
                focused,
                hovered,
            } => format!("PreDetach {detached_root:?} focused={focused:?} hovered={hovered:?}"),
        };
        self.records.borrow_mut().push(CapturedRecord(line));
    }
}

pub fn build(dom: &mut TuiDom) -> NodeId {
    let root = dom.create_element("div");
    dom.set_attribute(root, "class", "mo-demo").unwrap();

    append_text_block(dom, root, "h1", "MutationObserver");
    append_text_block(
        dom,
        root,
        "p",
        "Click the button to append a list item. The observer logs each tree mutation.",
    );

    let btn = dom.create_element("button");
    dom.set_attribute(btn, "class", "add-btn").unwrap();
    let btn_text = dom.create_text_node("Add item");
    dom.append_child(btn, btn_text).unwrap();
    dom.append_child(root, btn).unwrap();

    let items = dom.create_element("ul");
    dom.set_attribute(items, "class", "items").unwrap();
    dom.append_child(root, items).unwrap();

    append_text_block(dom, root, "h2", "Observer log");

    let log = dom.create_element("pre");
    dom.set_attribute(log, "class", "log").unwrap();
    dom.append_child(root, log).unwrap();

    // Install the observer. The captured-records buffer is shared
    // with the click handler so it can render the log after
    // mutations settle.
    let records: Rc<RefCell<Vec<CapturedRecord>>> = Rc::new(RefCell::new(Vec::new()));
    let obs = Box::new(CaptureMutations {
        records: records.clone(),
    });
    dom.add_mutation_observer(obs);

    // Click handler: append a new <li>, then render the log.
    let counter = Rc::new(Cell::new(0u32));
    let records_for_click = Rc::clone(&records);
    dom.add_event_listener(btn, "click", ListenerOptions::default(), move |ctx| {
        let n = counter.get() + 1;
        counter.set(n);

        // 1. Mutate: append the new <li>. This fires
        //    ChildListChanged; the observer captures it.
        let li = ctx.dom.create_element("li");
        let li_text = ctx.dom.create_text_node(&format!("Item {n}"));
        ctx.dom.append_child(li, li_text).unwrap();
        ctx.dom.append_child(items, li).unwrap();

        // 2. Render the captured log. This is itself a mutation
        //    (replaces the log <pre>'s text); the observer
        //    captures CharacterDataChanged for the new text +
        //    ChildListChanged for the replacement. Subsequent
        //    clicks see those records too — the log is a true
        //    trace of every mutation.
        let rendered = render_log(&records_for_click.borrow());
        let _ = ctx.dom.clear_children(log);
        let log_text = ctx.dom.create_text_node(&rendered);
        let _ = ctx.dom.append_child(log, log_text);
    })
    .unwrap();

    root
}

fn append_text_block(dom: &mut TuiDom, parent: NodeId, tag: &str, text: &str) {
    let el = dom.create_element(tag);
    let t = dom.create_text_node(text);
    dom.append_child(el, t).unwrap();
    dom.append_child(parent, el).unwrap();
}

fn render_log(records: &[CapturedRecord]) -> String {
    let mut out = String::new();
    for (i, r) in records.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&r.0);
    }
    out
}

pub fn stylesheet() -> Stylesheet {
    rdom_css::from_css(CSS)
}

pub fn run_standalone() -> io::Result<()> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let demo_root = build(&mut dom);
    dom.append_child(root, demo_root).unwrap();
    App::new(dom, stylesheet())?.run()
}

pub struct MutationObserverDemo;

impl Demo for MutationObserverDemo {
    fn slug(&self) -> &'static str {
        "events/mutation-observer"
    }

    fn title(&self) -> &'static str {
        "MutationObserver"
    }

    fn category(&self) -> Category {
        Category::Events
    }

    fn build(&self, dom: &mut TuiDom) -> NodeId {
        build(dom)
    }

    fn stylesheet(&self) -> Stylesheet {
        stylesheet()
    }

    fn source(&self) -> Source {
        Source {
            markup: MARKUP,
            css: CSS,
        }
    }
}
