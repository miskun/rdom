//! Random-operation fuzzer: build a tree from a mix of append / insert /
//! remove / replace ops, call `validate()` after each, fail on any
//! violation. Catches pointer-maintenance regressions we'd never think to
//! write a targeted unit test for.

use proptest::prelude::*;
use rdom_core::{Dom, NodeId};

#[derive(Debug, Clone)]
enum Op {
    CreateElement,
    CreateText,
    AppendTo(usize),            // (child_idx, parent_idx) — indices into live set
    Remove(usize),              // live_idx of node to detach
    InsertBefore(usize, usize), // (new_idx, reference_idx)
    AddClass(usize, &'static str),
    SetAttribute(usize, &'static str, &'static str),
}

fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        Just(Op::CreateElement),
        Just(Op::CreateText),
        (0usize..40).prop_map(Op::AppendTo),
        (0usize..40).prop_map(Op::Remove),
        (0usize..40, 0usize..40).prop_map(|(a, b)| Op::InsertBefore(a, b)),
        (0usize..40).prop_map(|i| Op::AddClass(i, "active")),
        (0usize..40).prop_map(|i| Op::SetAttribute(i, "role", "banner")),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn invariants_hold_under_random_ops(ops in prop::collection::vec(op_strategy(), 20..120)) {
        let mut dom: Dom = Dom::new();
        let mut live: Vec<NodeId> = vec![dom.root()];

        for op in ops {
            match op {
                Op::CreateElement => {
                    let id = dom.create_element("x");
                    live.push(id);
                }
                Op::CreateText => {
                    let id = dom.create_text_node("t");
                    live.push(id);
                }
                Op::AppendTo(i) => {
                    if live.len() < 2 { continue; }
                    let child = live[i % live.len()];
                    let parent = live[(i + 1) % live.len()];
                    if child == parent { continue; }
                    let _ = dom.append_child(parent, child);
                }
                Op::Remove(i) => {
                    if live.is_empty() { continue; }
                    let n = live[i % live.len()];
                    if n == dom.root() { continue; }
                    if let Some(parent) = dom.node(n).parent_node().map(|p| p.id()) {
                        let _ = dom.remove_child(parent, n);
                    }
                }
                Op::InsertBefore(a, b) => {
                    if live.len() < 3 { continue; }
                    let new = live[a % live.len()];
                    let reference = live[b % live.len()];
                    if new == reference { continue; }
                    if let Some(parent) = dom.node(reference).parent_node().map(|p| p.id()) {
                        let _ = dom.insert_before(parent, new, Some(reference));
                    }
                }
                Op::AddClass(i, cls) => {
                    if live.is_empty() { continue; }
                    let _ = dom.add_class(live[i % live.len()], cls);
                }
                Op::SetAttribute(i, k, v) => {
                    if live.is_empty() { continue; }
                    let _ = dom.set_attribute(live[i % live.len()], k, v);
                }
            }

            let violations = dom.validate();
            prop_assert!(
                violations.is_empty(),
                "invariant violations after op:\n  {:?}",
                violations
            );
        }
    }
}
