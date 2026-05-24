//! M8 — `mutation_observer` demo behavioral test.
//!
//! Exercises the click → mutation → log-update flow: clicking the
//! "Add item" button appends a new `<li>` (substrate fires
//! `ChildListChanged`), the observer captures the record, and
//! the click handler renders the captured log into the `<pre>`
//! block.

use rdom_showcase::demos::mutation_observer;
use rdom_tui::{Event, NodeId, TuiDom};

fn find_descendant_by_tag(dom: &TuiDom, root: NodeId, tag: &str) -> Option<NodeId> {
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        if dom.node(id).tag_name() == Some(tag) {
            return Some(id);
        }
        for child in dom.node(id).child_nodes() {
            stack.push(child.id());
        }
    }
    None
}

fn pre_text(dom: &TuiDom, pre: NodeId) -> String {
    let mut out = String::new();
    for child in dom.node(pre).child_nodes() {
        if let Some(t) = child.node_value() {
            out.push_str(t);
        }
    }
    out
}

#[test]
fn click_appends_li_and_logs_mutation_record() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let demo_root = mutation_observer::build(&mut dom);
    dom.append_child(root, demo_root).unwrap();

    let btn = find_descendant_by_tag(&dom, demo_root, "button").expect("button");
    let log = find_descendant_by_tag(&dom, demo_root, "pre").expect("log");
    let list = find_descendant_by_tag(&dom, demo_root, "ul").expect("items");

    assert_eq!(pre_text(&dom, log), "", "log starts empty");

    let mut click = Event::new("click");
    dom.dispatch_event(btn, &mut click).unwrap();

    // <ul> now has one <li>.
    let li_count = dom
        .node(list)
        .child_nodes()
        .filter(|c| c.tag_name() == Some("li"))
        .count();
    assert_eq!(li_count, 1, "one item appended");

    // Log records the ChildListChanged that the append fired.
    let log_text = pre_text(&dom, log);
    assert!(
        log_text.contains("ChildListChanged"),
        "log captures the ChildListChanged record (got {log_text:?})"
    );
}

#[test]
fn multiple_clicks_accumulate_log_lines() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let demo_root = mutation_observer::build(&mut dom);
    dom.append_child(root, demo_root).unwrap();

    let btn = find_descendant_by_tag(&dom, demo_root, "button").unwrap();
    let log = find_descendant_by_tag(&dom, demo_root, "pre").unwrap();

    for _ in 0..3 {
        let mut click = Event::new("click");
        dom.dispatch_event(btn, &mut click).unwrap();
    }

    // Each click fires the user's append (ChildListChanged on
    // <ul>) plus the log re-render (clear_children +
    // ChildListChanged on <pre>). So each click contributes
    // multiple records to the log — the log keeps growing.
    let log_text = pre_text(&dom, log);
    let lines: Vec<_> = log_text.lines().collect();
    assert!(
        lines.len() >= 3,
        "log accumulates at least one line per click (got {} lines)",
        lines.len()
    );
}
