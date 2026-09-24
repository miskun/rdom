//! Counter state for the cascade walk (CSS Lists 3 §3.1).
//!
//! A counter instance is created by `counter-reset` (or implicitly by
//! an increment / read of a counter not in scope) on an element `E`
//! and is in scope for `E`, `E`'s descendants, and `E`'s following
//! siblings with their descendants. That is exactly "until `E`'s
//! parent is left", so instances record the parent they were created
//! under and are dropped when the walk leaves that parent.

use rdom_core::NodeId;

use crate::style::CounterOp;

#[derive(Debug, Clone)]
struct Instance {
    name: String,
    value: i32,
    /// The parent of the element that created the instance; the
    /// instance dies when the walk leaves this element.
    scope_parent: Option<NodeId>,
}

/// Counter instances in creation order (later = innermost).
#[derive(Debug, Default, Clone)]
pub(super) struct CounterState {
    instances: Vec<Instance>,
}

impl CounterState {
    /// Apply an element's `counter-reset` then `counter-increment`
    /// (CSS Lists 3 §3.1.1 – §3.1.2). `parent` is the element's parent,
    /// which bounds the scope of anything created here.
    pub(super) fn enter(
        &mut self,
        parent: Option<NodeId>,
        reset: &[CounterOp],
        increment: &[CounterOp],
    ) {
        for op in reset {
            self.instances.push(Instance {
                name: op.name.clone(),
                value: op.value,
                scope_parent: parent,
            });
        }
        for op in increment {
            match self.instances.iter_mut().rev().find(|i| i.name == op.name) {
                Some(inst) => inst.value = inst.value.saturating_add(op.value),
                // Incrementing a counter not in scope implicitly resets
                // it to 0 on this element first.
                None => self.instances.push(Instance {
                    name: op.name.clone(),
                    value: op.value,
                    scope_parent: parent,
                }),
            }
        }
    }

    /// Leave `element`: instances created by its children go out of
    /// scope.
    pub(super) fn exit(&mut self, element: NodeId) {
        self.instances.retain(|i| i.scope_parent != Some(element));
    }

    /// The innermost instance of `name` in scope, or 0 (CSS Lists 3
    /// §3.2: a missing counter reads as 0).
    pub(super) fn value(&self, name: &str) -> i32 {
        self.instances
            .iter()
            .rev()
            .find(|i| i.name == name)
            .map_or(0, |i| i.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rdom_core::Dom;

    fn op(name: &str, value: i32) -> CounterOp {
        CounterOp {
            name: name.into(),
            value,
        }
    }

    /// `ol > li` numbering with a nested list: the inner reset is
    /// scoped to the inner `<ol>`, the outer count resumes after it,
    /// and a following sibling `<ol>` starts over.
    #[test]
    fn nested_lists_scope_and_resume() {
        let mut dom: Dom = Dom::new();
        let root = dom.root();
        let ol = dom.create_element("ol");
        let li1 = dom.create_element("li");
        let inner = dom.create_element("ol");
        let inner_li = dom.create_element("li");
        let li2 = dom.create_element("li");
        let ol2 = dom.create_element("ol");
        dom.append_child(root, ol).unwrap();
        dom.append_child(ol, li1).unwrap();
        dom.append_child(li1, inner).unwrap();
        dom.append_child(inner, inner_li).unwrap();
        dom.append_child(ol, li2).unwrap();
        dom.append_child(root, ol2).unwrap();

        let mut st = CounterState::default();
        st.enter(Some(root), &[op("list-item", 0)], &[]); // <ol>
        st.enter(Some(ol), &[], &[op("list-item", 1)]); // <li> 1
        assert_eq!(st.value("list-item"), 1);
        st.enter(Some(li1), &[op("list-item", 0)], &[]); // inner <ol>
        st.enter(Some(inner), &[], &[op("list-item", 1)]); // inner <li>
        assert_eq!(st.value("list-item"), 1);
        st.exit(inner_li);
        st.exit(inner); // inner <ol> closes: its own instance survives until li1 exits
        assert_eq!(
            st.value("list-item"),
            1,
            "still in the inner <ol>'s scope (a following sibling would see it)"
        );
        st.exit(li1); // leaving <li> 1 drops the inner <ol>'s instance
        assert_eq!(st.value("list-item"), 1, "outer count resumes");
        st.enter(Some(ol), &[], &[op("list-item", 1)]); // <li> 2
        assert_eq!(st.value("list-item"), 2);
        st.exit(li2);
        st.exit(ol);
        // <ol> 2 is a following sibling of <ol> 1: the first instance is
        // still in scope, and the reset creates a fresh one.
        st.enter(Some(root), &[op("list-item", 0)], &[]);
        st.enter(Some(ol2), &[], &[op("list-item", 1)]);
        assert_eq!(st.value("list-item"), 1);
    }

    #[test]
    fn increment_without_reset_creates_the_counter_and_missing_reads_zero() {
        let mut st = CounterState::default();
        assert_eq!(st.value("x"), 0);
        st.enter(None, &[], &[op("x", 5)]);
        assert_eq!(st.value("x"), 5);
        st.enter(None, &[], &[op("x", -2)]);
        assert_eq!(st.value("x"), 3);
    }
}
