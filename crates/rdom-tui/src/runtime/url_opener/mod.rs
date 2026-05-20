//! URL opening — cross-platform hand-off to the OS for external
//! links. Used by the `<a href>` click default action (Phase C.2).
//!
//! ## Trait seam
//!
//! Apps plug a concrete [`UrlOpener`] into `App::with_url_opener`.
//! Production uses [`SystemUrlOpener`] (shells out via the `open`
//! crate); tests use [`MemoryUrlOpener`] which records calls
//! without touching the real OS.
//!
//! Mirrors the `Clipboard` trait in
//! `runtime::selection::clipboard` — same "stub the side-effect
//! boundary" pattern, same test ergonomics.
//!
//! ## Scope (v1)
//!
//! The runtime only calls `open` for URLs with an HTML-standard
//! **external** scheme: `http`, `https`, `mailto`, `ftp`, `file`.
//! (Plus `tel`, `sms`, `data`, `blob` — matching MDN's `<a href>`
//! spec. `javascript:` is intentionally excluded for security.)
//! Anything else is treated as internal — routing is the app's
//! job.

use std::cell::RefCell;

/// Open `url` in the system's default handler. Implementations are
/// interior-mutable so a single `Rc<dyn UrlOpener>` can be shared
/// across the App, its event listeners, and test assertions.
pub trait UrlOpener: 'static {
    /// Hand `url` off. Errors are swallowed at this boundary —
    /// the user has already clicked, there's nothing a handler
    /// can do about a missing platform handler.
    fn open(&self, url: &str);
}

/// Production opener. Wraps the `open` crate which probes for the
/// right platform invocation (`xdg-open` on Linux, `open` on macOS,
/// `start` on Windows). Zero-cost when idle.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemUrlOpener;

impl UrlOpener for SystemUrlOpener {
    fn open(&self, url: &str) {
        let _ = open::that(url);
    }
}

/// Test opener. Records every URL opened in an internal log,
/// retrievable via [`opened`](Self::opened). Used by app tests
/// so they don't shell out to the user's browser when
/// `<a href>` click defaults fire.
#[derive(Debug, Default)]
pub struct MemoryUrlOpener {
    opened: RefCell<Vec<String>>,
}

impl MemoryUrlOpener {
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of every URL `open` was called with, in order.
    pub fn opened(&self) -> Vec<String> {
        self.opened.borrow().clone()
    }
}

impl UrlOpener for MemoryUrlOpener {
    fn open(&self, url: &str) {
        self.opened.borrow_mut().push(url.to_string());
    }
}

/// Scheme of `url` — the substring before the first colon, or
/// empty when there's no colon (relative path / fragment).
///
/// Not a full RFC 3986 scheme parse — we just need a lowercase
/// token to match against the external allowlist. Case-insensitive:
/// `HTTP://Example.com` → `"http"`.
pub fn scheme_of(url: &str) -> &str {
    match url.find(':') {
        Some(i) => &url[..i],
        None => "",
    }
}

/// Is `scheme` one the runtime will hand off to the OS?
/// The allowlist matches MDN's `<a href>` scheme list minus
/// `javascript:` (security risk, explicitly excluded).
pub fn is_external_scheme(scheme: &str) -> bool {
    // Case-insensitive comparison: `Mailto:`, `MAILTO:`, `mailto:`
    // all count. Allocates once per check but schemes are short
    // and this is called at most once per click.
    matches!(
        scheme.to_ascii_lowercase().as_str(),
        "http" | "https" | "mailto" | "tel" | "sms" | "ftp" | "file" | "data" | "blob"
    )
}

#[cfg(test)]
mod tests;
