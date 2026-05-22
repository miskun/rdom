//! The `Demo` trait — every entry in the showcase registry
//! implements this. See [`crate::registry::DEMOS`] for the table.

use rdom_tui::{NodeId, Stylesheet, TuiDom};

/// One showcase entry. Implementations describe a small, focused
/// example of an rdom primitive: a button, a form, a flex layout,
/// a transition, etc.
///
/// All methods return `'static` borrows or owned values so the
/// registry can be a plain `&[&dyn Demo]` constant.
pub trait Demo {
    /// Path-like identifier used for CLI deep-linking and as the
    /// stable key for goldens. Format: `"category/demo-name"` in
    /// kebab-case (e.g. `"layout/flex-row"`, `"forms/checkbox"`).
    /// Must be unique across the registry.
    fn slug(&self) -> &'static str;

    /// Human-readable title shown in the sidebar.
    fn title(&self) -> &'static str;

    /// Which sidebar category this demo lives under.
    fn category(&self) -> Category;

    /// Build the demo's subtree under `dom`. The returned `NodeId`
    /// is the root of the demo's content — the shell appends it to
    /// the main-view container. The implementation does **not**
    /// attach to any parent itself.
    ///
    /// Demos may create text nodes, set attributes, register event
    /// listeners, etc. — anything a downstream consumer would do.
    fn build(&self, dom: &mut TuiDom) -> NodeId;

    /// CSS that applies while this demo is mounted. The showcase
    /// pushes it onto the App's sheet stack when the demo activates
    /// and removes it on deactivation. Use `Stylesheet::bare()` if
    /// the demo doesn't need its own styles.
    fn stylesheet(&self) -> Stylesheet;

    /// Source strings shown in the demo's Source tab (M7). Stub for
    /// M2 — empty markup + empty css is acceptable until a real
    /// source pipeline exists.
    fn source(&self) -> Source;
}

/// Sidebar taxonomy. Final form decided in M3 (see
/// `specs/SHOWCASE.md` M3 deliverables). Listed in display order.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum Category {
    Layout,
    Positioning,
    Forms,
    Text,
    Editing,
    Selection,
    Cascade,
    PseudoElements,
    Events,
    Animations,
    BuiltIns,
}

impl Category {
    /// Human-readable label for the sidebar header.
    pub fn title(&self) -> &'static str {
        match self {
            Self::Layout => "Layout",
            Self::Positioning => "Positioning",
            Self::Forms => "Forms",
            Self::Text => "Text",
            Self::Editing => "Editing",
            Self::Selection => "Selection",
            Self::Cascade => "Cascade",
            Self::PseudoElements => "Pseudo-elements",
            Self::Events => "Events",
            Self::Animations => "Animations",
            Self::BuiltIns => "Built-ins",
        }
    }
}

/// Source-tab content for a demo. M7 wires the Source view; M2
/// only stores the strings.
#[derive(Copy, Clone, Debug)]
pub struct Source {
    /// HTML-ish markup the demo's `build` constructs. Hand-written
    /// to match what `build` produces — the showcase doesn't
    /// reverse-engineer it from the live tree.
    pub markup: &'static str,
    /// CSS the demo's `stylesheet` carries. Same authoring
    /// principle as `markup`.
    pub css: &'static str,
}

impl Source {
    /// An empty `Source` — useful as a stub during scaffolding.
    pub const fn empty() -> Self {
        Self {
            markup: "",
            css: "",
        }
    }
}
