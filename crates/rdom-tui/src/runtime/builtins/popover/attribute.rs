//! The `popover` attribute's change steps (HTML §6.12): a showing popover
//! whose attribute changes to another state — or is removed — is hidden,
//! focusing the previous element and firing its events. A mutation
//! observer may not change the tree, so [`PopoverAttributes`] records the
//! changes and [`PopoverAttributes::flush`] hides the popovers at the
//! `App`'s next event or frame, as `<select>`'s selectedness is settled.

use std::cell::RefCell;
use std::rc::Rc;

use rdom_core::{Dom, Mutation, MutationObserver, NodeId};

use super::algorithms::{self, Hide};
use super::{PopoverState, is_showing};
use crate::TuiDom;
use crate::ext::TuiExt;

/// The popovers whose attribute changed state while showing.
#[derive(Debug, Default)]
pub(crate) struct PopoverAttributes {
    pending: Rc<RefCell<Vec<NodeId>>>,
}

struct Watch {
    pending: Rc<RefCell<Vec<NodeId>>>,
}

impl MutationObserver<TuiExt> for Watch {
    fn observe(&mut self, dom: &mut Dom<TuiExt>, record: &Mutation) {
        if let Mutation::AttributeChanged { id, name, old, new } = record
            && name == "popover"
            && PopoverState::of(old.as_deref()) != PopoverState::of(new.as_deref())
            && is_showing(dom, *id)
        {
            self.pending.borrow_mut().push(*id);
        }
    }
}

impl PopoverAttributes {
    /// Register the observer on `dom`.
    pub(crate) fn install(dom: &mut TuiDom) -> Self {
        let pending = Rc::default();
        dom.add_mutation_observer(Box::new(Watch {
            pending: Rc::clone(&pending),
        }));
        Self { pending }
    }

    /// Hide the recorded popovers that still show: "hide popover
    /// algorithm" with focus restored, events fired, no exception, and
    /// the attribute's state ignored.
    pub(crate) fn flush(&self, dom: &mut TuiDom) {
        let queued = std::mem::take(&mut *self.pending.borrow_mut());
        for id in queued {
            if dom.contains(id) && is_showing(dom, id) {
                let _ = algorithms::hide(
                    dom,
                    id,
                    Hide {
                        throw: false,
                        ignore_dom_state: true,
                        ..Hide::THROWING
                    },
                );
            }
        }
    }
}
