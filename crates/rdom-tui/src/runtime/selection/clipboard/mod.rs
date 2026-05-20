//! Clipboard — copy / cut / paste wiring for Phase 6.5.6.
//!
//! Three default actions fired from `App::handle_event` when the
//! user hits Ctrl-C, Ctrl-X or Ctrl-V (or the Cmd-variant on
//! macOS):
//!
//! - **`copy`**: serialize the current selection, fire `copy` on
//!   the anchor node with the serialized text in `event.detail`;
//!   if not prevented, write it to the [`Clipboard`] backend.
//! - **`cut`**: same as copy, plus a `cut` event whose default
//!   action also copies. Actually removing the selected text is
//!   up to a listener on an editable element — read-only views
//!   just get the copy behavior.
//! - **`paste`**: read text from the clipboard, fire `paste` on
//!   `dom.focused()` with the text in `event.detail`. Listeners
//!   insert or transform the pasted text; without a listener the
//!   event is a no-op.
//!
//! ## Clipboard backends
//!
//! The [`Clipboard`] trait abstracts the system clipboard so tests
//! can inject a [`MemoryClipboard`]. [`SystemClipboard`] (the
//! production default) wraps `arboard` and silently degrades to a
//! no-op if initialization fails (no display server, headless
//! sandbox, WSL without `clip.exe`, etc.) — apps keep running,
//! copy/paste just doesn't do anything.

use rdom_core::{NodeId, NodeType, Range};

use crate::TuiDom;

mod serialize;

pub use serialize::serialize_selection;

/// Trait for pluggable clipboard backends. Implemented by
/// [`SystemClipboard`] (arboard) and [`MemoryClipboard`] (tests).
/// Apps can register their own via `AppBuilder::clipboard(...)`
/// — useful for OSC 52 in remote-over-SSH scenarios (post-v1).
pub trait Clipboard: 'static {
    /// Read the current clipboard contents as UTF-8 text. `None`
    /// when the clipboard is empty, unavailable, or holds non-text
    /// data (images, files, etc.).
    fn read_text(&mut self) -> Option<String>;

    /// Write UTF-8 text to the clipboard, replacing whatever was
    /// there. Errors are swallowed — the runtime treats clipboard
    /// failures as best-effort.
    fn write_text(&mut self, text: String);
}

/// arboard-backed system clipboard — the production default. Lazy
/// inside: each call creates a fresh `arboard::Clipboard` so a
/// transient platform error doesn't permanently disable copy/paste
/// for the rest of the session.
pub struct SystemClipboard;

impl SystemClipboard {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SystemClipboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Clipboard for SystemClipboard {
    fn read_text(&mut self) -> Option<String> {
        arboard::Clipboard::new().ok()?.get_text().ok()
    }

    fn write_text(&mut self, text: String) {
        if let Ok(mut cb) = arboard::Clipboard::new() {
            let _ = cb.set_text(text);
        }
    }
}

/// In-memory clipboard for tests and headless harnesses. Roundtrips
/// via a single `String`. Supports pre-seeding for paste tests.
#[derive(Debug, Default)]
pub struct MemoryClipboard {
    buf: Option<String>,
}

impl MemoryClipboard {
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed the clipboard — useful for testing `paste` flows without
    /// going through a `write_text` first.
    pub fn with_text(text: impl Into<String>) -> Self {
        Self {
            buf: Some(text.into()),
        }
    }

    /// Peek at the current contents without consuming them.
    pub fn peek(&self) -> Option<&str> {
        self.buf.as_deref()
    }
}

impl Clipboard for MemoryClipboard {
    fn read_text(&mut self) -> Option<String> {
        self.buf.clone()
    }

    fn write_text(&mut self, text: String) {
        self.buf = Some(text);
    }
}

/// OSC 52 terminal-mediated clipboard.
///
/// Emits `ESC ] 5 2 ; c ; <BASE64> ESC \\` sequences to the
/// configured writer — almost always the TTY. Terminals that
/// support OSC 52 (alacritty, iTerm2, wezterm, kitty, tmux with
/// `set-clipboard on`, modern xterm) place the base64-decoded
/// payload in the system clipboard, even across SSH.
///
/// ## When to use
///
/// - SSH into a remote dev box and want `y` / Ctrl-C to reach
///   your local clipboard? Swap the default `SystemClipboard`
///   (which only sees the remote box's X/Wayland) for
///   `OscClipboard::stdout()`.
/// - Headless CI with no display server: OSC 52 is harmless;
///   the bytes just write to stdout.
///
/// ## Read support
///
/// v1 is **write-only**. Reading via OSC 52 requires parsing the
/// terminal's response payload back through stdin; we don't ship
/// that machinery yet. `read_text` returns `None`. Apps that
/// need read use `SystemClipboard` on the local machine or keep
/// a memory cache of what they wrote.
///
/// ## Size limits
///
/// Most terminals cap OSC 52 payloads around 64 KiB–100 KiB.
/// Writing larger content is silently truncated / rejected by
/// the terminal. v1 doesn't split; apps that need huge payloads
/// manage chunking themselves.
pub struct OscClipboard<W: std::io::Write + Send + 'static> {
    writer: W,
}

impl OscClipboard<std::io::Stdout> {
    /// Default construction — writes OSC 52 sequences to stdout.
    pub fn stdout() -> Self {
        Self {
            writer: std::io::stdout(),
        }
    }
}

impl<W: std::io::Write + Send + 'static> OscClipboard<W> {
    /// Construct with a custom writer. Handy for tests or for
    /// apps that want to route the bytes through a log buffer,
    /// `/dev/tty`, or an alternative fd.
    pub fn with_writer(writer: W) -> Self {
        Self { writer }
    }
}

impl<W: std::io::Write + Send + 'static> Clipboard for OscClipboard<W> {
    /// v1 is write-only — reading requires parsing the terminal's
    /// OSC 52 response, which we defer to polish. Always returns
    /// `None`.
    fn read_text(&mut self) -> Option<String> {
        None
    }

    fn write_text(&mut self, text: String) {
        // OSC 52 sequence: ESC ] 5 2 ; c ; <BASE64> ESC \
        // The `c` selects the clipboard buffer; `p` would be the
        // primary selection. We always use the clipboard.
        let encoded = base64_encode(text.as_bytes());
        // Swallow write errors — clipboard failures are best-effort.
        let _ = write!(self.writer, "\x1b]52;c;{}\x1b\\", encoded);
        let _ = self.writer.flush();
    }
}

// ── Minimal base64 encoder ────────────────────────────────────────
//
// OSC 52 is the only consumer here — a 14-line standard encoder
// avoids pulling in the full `base64` crate for one spot. Deals
// with the three-byte → four-char round trip + standard '=' pad.

const BASE64_TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    let mut chunks = bytes.chunks_exact(3);
    for chunk in chunks.by_ref() {
        let b0 = chunk[0] as u32;
        let b1 = chunk[1] as u32;
        let b2 = chunk[2] as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(BASE64_TABLE[((n >> 18) & 0x3F) as usize] as char);
        out.push(BASE64_TABLE[((n >> 12) & 0x3F) as usize] as char);
        out.push(BASE64_TABLE[((n >> 6) & 0x3F) as usize] as char);
        out.push(BASE64_TABLE[(n & 0x3F) as usize] as char);
    }
    let rem = chunks.remainder();
    match rem.len() {
        0 => {}
        1 => {
            let n = (rem[0] as u32) << 16;
            out.push(BASE64_TABLE[((n >> 18) & 0x3F) as usize] as char);
            out.push(BASE64_TABLE[((n >> 12) & 0x3F) as usize] as char);
            out.push('=');
            out.push('=');
        }
        2 => {
            let n = ((rem[0] as u32) << 16) | ((rem[1] as u32) << 8);
            out.push(BASE64_TABLE[((n >> 18) & 0x3F) as usize] as char);
            out.push(BASE64_TABLE[((n >> 12) & 0x3F) as usize] as char);
            out.push(BASE64_TABLE[((n >> 6) & 0x3F) as usize] as char);
            out.push('=');
        }
        _ => unreachable!(),
    }
    out
}

// ── Target resolution for copy / cut ────────────────────────────────

/// Find the element to target `copy` / `cut` events on. The spec
/// says "element owning the selection anchor" — walk up from the
/// anchor's text node to the nearest Element ancestor.
///
/// Returns `None` when there's no selection or the anchor sits in
/// a disconnected/invalid node.
pub(crate) fn copy_target(dom: &TuiDom) -> Option<NodeId> {
    let sel = dom.selection()?;
    element_ancestor(dom, sel.anchor.node)
}

/// Walk up from `id` to the nearest element ancestor (including
/// `id` itself if it's already an element).
fn element_ancestor(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
    let mut cur = Some(id);
    while let Some(n) = cur {
        if dom.node(n).node_type() == NodeType::Element {
            return Some(n);
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    }
    None
}

/// Serialize + return the current selection text. Thin wrapper
/// around [`serialize_selection`] that returns the text and the
/// selection's range; `None` when no selection / collapsed.
pub(crate) fn current_selection_text(dom: &TuiDom) -> Option<(String, Range)> {
    let range = dom.selection_range()?;
    if range.is_collapsed() {
        return None;
    }
    let text = serialize_selection(dom, &range);
    Some((text, range))
}

#[cfg(test)]
mod tests;
