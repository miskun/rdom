//! `<a href>` click default — scheme-based dispatch.
//!
//! ## Contract
//!
//! On a click event that reaches document root (bubble phase):
//!
//! 1. Walk up from `event.target` via `closest("a[href]")` to find
//!    the enclosing anchor. If none, no-op.
//! 2. If `event.default_prevented()` is set, no-op — an author
//!    handler already handled the click.
//! 3. Read `href`. If the scheme is in the external allowlist
//!    (http, https, mailto, tel, sms, ftp, file, data, blob),
//!    call `opener.open(href)`.
//! 4. Internal schemes (relative paths, custom schemes, fragments)
//!    → no-op. Apps implement their own router as a `click`
//!    listener that filters `<a[href]>` targets and calls
//!    `preventDefault` before this default runs.
//!
//! ## Why a root listener (not per-element setup)
//!
//! Browsers don't install a listener on each `<a>`. They have
//! built-in default actions that fire after bubbling completes.
//! Our bubble-phase root listener is the same idea in rdom-tui's
//! event model: one registration, handles every anchor the
//! document will ever contain.

use std::cell::RefCell;
use std::rc::Rc;

use rdom_core::{ListenerOptions, NodeId};

use crate::TuiDom;
use crate::runtime::url_opener::{UrlOpener, is_external_scheme, scheme_of};

/// Shareable "current url opener" handle. Double-`Rc` lets `App`
/// keep one of these as a field while the click listener holds
/// an independent clone; inner `RefCell` lets
/// [`App::with_url_opener`] swap the backend at any time, with the
/// swap visible to the listener on the next click.
pub type SharedOpener = Rc<RefCell<Rc<dyn UrlOpener>>>;

/// Install the anchor-click default action. Called once from
/// `App::build`. The listener is registered at the document root
/// in the bubble phase, so it runs AFTER target + ancestor
/// listeners and respects `event.preventDefault()`.
pub fn install(dom: &mut TuiDom, opener: SharedOpener) {
    let root = dom.root();
    dom.add_event_listener(root, "click", ListenerOptions::default(), move |ctx| {
        if ctx.event.default_prevented() {
            return;
        }
        let Some(target) = ctx.event.target else {
            return;
        };
        let Some(anchor) = anchor_with_href(ctx.dom, target) else {
            return;
        };
        let href = ctx
            .dom
            .node(anchor)
            .get_attribute("href")
            .map(str::to_owned);
        let Some(href) = href else {
            return;
        };
        let scheme = scheme_of(&href);
        if is_external_scheme(scheme) {
            // Read the *current* opener from the shared cell
            // so `App::with_url_opener` swaps propagate to
            // already-installed listeners.
            let opener_snapshot = opener.borrow().clone();
            opener_snapshot.open(&href);
        }
        // Internal schemes: no-op. Apps route themselves.
    })
    .expect("root click listener install");
}

/// Walk up from `id` (inclusive) to the nearest `<a>` element
/// with an `href` attribute. Mirrors the DOM `closest("a[href]")`
/// idiom. Returns `None` when no such ancestor exists.
fn anchor_with_href(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
    let mut cur = Some(id);
    while let Some(n) = cur {
        let node = dom.node(n);
        if node.tag_name() == Some("a") && node.has_attribute("href") {
            return Some(n);
        }
        cur = node.parent_node().map(|p| p.id());
    }
    None
}

#[cfg(test)]
mod tests;
