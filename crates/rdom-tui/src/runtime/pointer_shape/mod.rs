//! The pointer shape (CSS UI 4 §4.1 `cursor`): the `App` sends the shape
//! of the element under the pointer to a terminal that takes OSC 22
//! pointer shapes (`ESC ] 22 ; <name> ST`, the CSS names — kitty, foot,
//! WezTerm, Ghostty), after each pointer event and each frame (the
//! element under a still pointer may change, or its `:hover` style), and
//! only when it changed. A terminal without them is sent nothing.
//!
//! The image fallbacks of a `cursor` list are inert — a terminal draws no
//! image pointer — and the keyword after them is the shape. `auto` is the
//! text pointer over text a user can select or edit, `default` elsewhere;
//! `none` is `default` (no terminal protocol hides the pointer).
//!
//! Terminal state: once a shape was sent, leaving TUI mode — a normal
//! exit, the `TerminalGuard`'s drop, the panic hook — sends `default`
//! back (`backend_crossterm::restore_terminal`).

use std::sync::atomic::{AtomicBool, Ordering};

use crate::TuiDom;
use crate::layout::CursorKeyword;
use crate::node::TuiNodeExt;
use crate::runtime::hit_test::HitTestExt;

#[cfg(test)]
mod tests;

/// The pointer-shape protocol a terminal takes.
///
/// Open (`#[non_exhaustive]`): another protocol may be added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum PointerShapes {
    /// None: the pointer is left as the terminal draws it.
    #[default]
    None,
    /// OSC 22 with the CSS pointer names.
    Osc22,
}

impl PointerShapes {
    /// The protocol of the terminal this process writes to, guessed from
    /// its environment ([`detect`](Self::detect) has the table).
    /// [`App::new`](crate::App::new) uses it;
    /// [`App::with_pointer_shapes`](crate::App::with_pointer_shapes)
    /// overrides it.
    pub fn from_env() -> Self {
        Self::detect(|name| std::env::var(name).ok())
    }

    /// [`from_env`](Self::from_env) reading the variables through `var`.
    /// Conservative, as [`SgrCapabilities::detect`](crate::SgrCapabilities::detect)
    /// is: a multiplexer — GNU screen (`STY`), Zellij (`ZELLIJ`), tmux
    /// (`TMUX`, `TERM_PROGRAM=tmux`, a `TERM` of `screen*` / `tmux*`) —
    /// would have to pass OSC 22 through, so it gets [`None`](Self::None);
    /// [`Osc22`](Self::Osc22) goes to a `TERM` naming kitty, foot,
    /// WezTerm or Ghostty, `TERM_PROGRAM` WezTerm, ghostty or kitty, or
    /// `KITTY_WINDOW_ID` — the terminals that take the CSS names; any
    /// other terminal gets `None` (xterm's OSC 22 takes X cursor-font
    /// names).
    pub fn detect(var: impl Fn(&str) -> Option<String>) -> Self {
        let term = var("TERM").unwrap_or_default();
        let program = var("TERM_PROGRAM").unwrap_or_default();
        if var("STY").is_some()
            || var("ZELLIJ").is_some()
            || var("TMUX").is_some()
            || program == "tmux"
            || term.starts_with("screen")
            || term.starts_with("tmux")
        {
            return Self::None;
        }
        let named_term = ["kitty", "foot", "wezterm", "ghostty"]
            .iter()
            .any(|t| term.contains(t));
        let named_program = ["WezTerm", "ghostty", "kitty"]
            .iter()
            .any(|p| program.eq_ignore_ascii_case(p));
        if named_term || named_program || var("KITTY_WINDOW_ID").is_some() {
            Self::Osc22
        } else {
            Self::None
        }
    }
}

/// Whether this process sent a pointer shape, so leaving TUI mode must
/// send `default` back. Process-wide, as the terminal is: the panic hook
/// that restores it has no `App` to ask.
static SHAPE_SENT: AtomicBool = AtomicBool::new(false);

/// Note that a shape was sent (the `App`, as it sends one).
pub(crate) fn note_shape_sent() {
    SHAPE_SENT.store(true, Ordering::SeqCst);
}

/// The bytes that put the pointer back once a shape was sent: `OSC 22 ;
/// default ST`. Every later restore sends them again (harmless, and no
/// restore can take them from another).
pub(crate) fn restore_bytes() -> Option<&'static [u8]> {
    SHAPE_SENT
        .load(Ordering::SeqCst)
        .then_some(b"\x1b]22;default\x1b\\".as_slice())
}

/// `OSC 22 ; name ST`.
pub(crate) fn osc22(name: &str) -> Vec<u8> {
    format!("\x1b]22;{name}\x1b\\").into_bytes()
}

/// The pointer name over `(x, y)`: the `cursor` keyword of the element
/// there (inherited from its ancestors), `auto` resolved.
pub(crate) fn shape_at(dom: &TuiDom, x: u16, y: u16) -> &'static str {
    let Some(hit) = dom.hit_test(x, y) else {
        return "default";
    };
    // `TuiNodeExt::computed`: the root fragment's style too (the root
    // element, C14G-ROOT-ELEMENT), which `ext().computed` does not hold.
    let keyword = dom
        .node(hit)
        .computed()
        .map_or(CursorKeyword::Auto, |c| c.ui.cursor.keyword);
    match keyword {
        CursorKeyword::Auto if over_text(dom, hit, x, y) => "text",
        CursorKeyword::Auto | CursorKeyword::None => "default",
        k => k.keyword(),
    }
}

/// Whether `(x, y)`, over `hit`, is text a user can edit or select: an
/// editable text control or host, or a line of selectable inline content.
fn over_text(dom: &TuiDom, hit: rdom_core::NodeId, x: u16, y: u16) -> bool {
    use crate::node::TuiNodeExt;
    if dom.node(hit).is_editable() || dom.is_editable_or_editing_host(hit) {
        return true;
    }
    crate::runtime::hit_test::over_selectable_text(dom, x, y)
}
