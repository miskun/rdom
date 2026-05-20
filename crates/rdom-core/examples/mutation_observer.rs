//! `MutationObserver` — react to DOM changes.
//!
//! Demonstrates the W3C-style observer pattern. Every mutation entry
//! point (set_attribute, add_class, append_child, etc.) fires a
//! `Mutation` record to every registered observer.
//!
//! Run with: `cargo run --example mutation_observer -p rdom-core`

use std::cell::RefCell;
use std::rc::Rc;

use rdom_core::{Dom, Mutation, MutationObserver, NodeId};

/// A simple observer that logs every mutation to a shared Vec.
struct Logger {
    log: Rc<RefCell<Vec<String>>>,
}

impl MutationObserver<()> for Logger {
    fn observe(&mut self, _dom: &mut Dom<()>, record: &Mutation) {
        let line = match record {
            Mutation::AttributeChanged { id, name, old, new } => {
                format!(
                    "attr[{}]: {:?} ({} → {})",
                    id_fmt(*id),
                    name,
                    opt(old.as_deref()),
                    opt(new.as_deref()),
                )
            }
            Mutation::ClassChanged { id, added, removed } => {
                format!("class[{}]: +{:?} -{:?}", id_fmt(*id), added, removed)
            }
            Mutation::ChildListChanged {
                parent,
                added,
                removed,
            } => {
                format!(
                    "children[{}]: +{} -{}",
                    id_fmt(*parent),
                    added.len(),
                    removed.len()
                )
            }
            Mutation::CharacterDataChanged { id, old, new } => {
                format!("text[{}]: {:?} → {:?}", id_fmt(*id), old, new)
            }
            Mutation::InteractionChanged { prev, next, kind } => {
                format!(
                    "interact[{:?}]: {:?} → {:?}",
                    kind,
                    prev.map(id_fmt),
                    next.map(id_fmt)
                )
            }
            Mutation::SelectionChanged { prev, next } => {
                format!("selection: {:?} → {:?}", prev.is_some(), next.is_some())
            }
        };
        self.log.borrow_mut().push(line);
    }
}

fn id_fmt(id: NodeId) -> u32 {
    id.as_u32()
}

fn opt(s: Option<&str>) -> String {
    match s {
        Some(v) => format!("\"{v}\""),
        None => "none".to_string(),
    }
}

fn main() {
    let mut dom: Dom = Dom::new();
    let log = Rc::new(RefCell::new(Vec::new()));

    let logger = Logger { log: log.clone() };
    let _id = dom.add_mutation_observer(Box::new(logger));

    // Do some mutations.
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    dom.set_attribute(div, "id", "hero").unwrap();
    dom.add_class(div, "primary").unwrap();
    dom.add_class(div, "card").unwrap();
    dom.set_attribute(div, "role", "banner").unwrap();

    let child = dom.create_element("span");
    dom.append_child(div, child).unwrap();
    dom.set_attribute(div, "id", "banner").unwrap(); // change from hero → banner
    dom.remove_class(div, "primary").unwrap();

    dom.set_hovered(Some(div));
    dom.set_hovered(None);

    // Print the log.
    println!(
        "── Mutation log ({} entries) ──────────────────",
        log.borrow().len()
    );
    for line in log.borrow().iter() {
        println!("  {line}");
    }
}
