//! # rdom-tui — terminal rendering for rdom-core
//!
//! Wraps `rdom_core::Dom<TuiExt>` with presentation data: flexbox layout,
//! TUI styles, scroll state, pseudo-elements (`::before`/`::after`), and
//! a CSS-faithful cascade. The pure DOM tree lives in `rdom-core`; this
//! crate adds the "how should it render to a terminal" layer.
//!
//! ## Quick start
//!
//! ```
//! use rdom_tui::prelude::*;
//!
//! let mut dom: TuiDom = TuiDom::new();
//! let root = dom.root();
//! let div = dom.create_element("div");
//! dom.node_mut(div).add_class("hero").unwrap();
//! dom.append_child(root, div).unwrap();
//!
//! let sheet = Stylesheet::new()
//!     .rule(".hero", TuiStyle::new().fg(Color::Rgb(255, 0, 0)).padding(Padding::all(1)))
//!     .unwrap();
//! dom.cascade(&sheet);
//!
//! let computed = dom.node(div).computed().unwrap();
//! assert_eq!(computed.fg, Color::Rgb(255, 0, 0));
//! assert_eq!(computed.padding, Padding::all(1));
//! ```
//!
//! ## What's here
//!
//! - **`TuiExt`** — per-element presentation data (inline style,
//!   computed style cache, dirty flags, geometry, scroll state).
//! - **`TuiStyle`** / **`ComputedStyle`** — author-input vs.
//!   post-cascade style; `TuiStyle` uses `Option<Value<T>>` to carry
//!   specified/inherit/initial; `ComputedStyle` is fully concrete.
//! - **`Stylesheet`** + **`CascadeExt`** — rule storage + cascade
//!   engine with 4-tuple specificity, `!important` ladder,
//!   pseudo-elements, `content`, and `var(--…)` resolution.
//! - **`DirtyTracker`** — `MutationObserver` impl that drives
//!   incremental re-cascade via `cascade_subtrees(&sheet, &roots)`.
//! - **`:hover`/`:focus`** pseudo-classes driven by `Dom::set_hovered`
//!   / `set_focused`.
//!
//! ## Type aliases
//!
//! - `TuiDom`       → `rdom_core::Dom<TuiExt>`
//! - `TuiNodeRef`   → `rdom_core::NodeRef<'_, TuiExt>`
//! - `TuiNodeMut`   → `rdom_core::NodeMut<'_, TuiExt>`
//! - `TuiEventCtx`  → `rdom_core::EventCtx<'_, TuiExt>`

use rdom_core as core;

pub mod accessors;
pub mod cssom;
pub mod editing;
pub mod ext;
pub mod node;
pub mod prelude;
pub mod render;
pub mod runtime;
pub mod style;
pub mod tui_event;

// `layout` moved to `rdom-style` (M4b mid-stream restructure). The
// `rdom_tui::layout::*` path stays valid through this re-export.
pub use rdom_style::layout;

pub use accessors::{TuiAccessors, TuiAccessorsMut, TuiDocAccessors};
pub use cssom::{extend_from_style_tags, seed_inline_styles};
pub use tui_event::{TuiDispatchExt, TuiEvent};

pub use ext::{PseudoLayout, TuiExt};
pub use layout::{
    Align, Border, Direction, Display, LayoutRect, Overflow, Padding, Size, UserSelect, WhiteSpace,
};
pub use node::{TuiNodeExt, TuiNodeMutExt};
pub use render::{
    Backend, Buffer, Cell, CellDiff, CompletedFrame, CrosstermBackend, LayoutExt, PaintExt, Rect,
    RenderContext, Style, Terminal, TerminalGuard, TestBackend, VirtualScreen,
};
pub use runtime::{
    App, AppContext, AppHandle, ControlFlow, HitTestExt, RouteOutcome, Router, StylesheetId,
};
pub use style::{
    CascadeExt, Color, ComputedStyle, Content, DirtyTracker, INHERITS_MASK, ImportantMask,
    LAYOUT_MASK, Modifier, PropMask, PseudoElementTarget, Rule, RuleOrigin, Specificity,
    StyleError, Stylesheet, TuiColor, TuiStyle, Value, VarMap, parse_color, resolve_tui_color,
};

/// `Dom<TuiExt>` — the full TUI document.
pub type TuiDom = core::Dom<TuiExt>;

/// Borrowed node accessor for `TuiDom`.
pub type TuiNodeRef<'a> = core::NodeRef<'a, TuiExt>;

/// Mutable node accessor for `TuiDom`.
pub type TuiNodeMut<'a> = core::NodeMut<'a, TuiExt>;

/// Event dispatch context for `TuiDom`.
pub type TuiEventCtx<'a> = core::EventCtx<'a, TuiExt>;

// Re-export the bits a caller typically needs so `use rdom_tui::*` is
// a reasonable start. The full rdom-core API is accessible via
// `rdom_tui::core_api::…` (re-exported below).
pub use rdom_core as core_api;
pub use rdom_core::{
    AdjacentPosition, DocumentPosition, DomError, Event, EventDetail, EventPhase, InputDetail,
    InputType, InteractionKind, KeyboardDetail, KeyboardModifiers, ListenerId, ListenerOptions,
    MouseButton, MouseDetail, Mutation, MutationObserver, NodeData, NodeId, NodeType, ObserverId,
    Position, Range, Result, Selection, SubmitDetail, ToggleDetail, ToggleState, TransitionDetail,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tui_dom_aliases_compile() {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let div = dom.create_element("div");
        dom.append_child(root, div).unwrap();

        // Accessor alias works.
        let node: TuiNodeRef<'_> = dom.node(div);
        assert_eq!(node.tag_name(), Some("div"));

        // Mutating alias works.
        let mut node_mut: TuiNodeMut<'_> = dom.node_mut(div);
        node_mut.set_id("hero").unwrap();
        assert_eq!(dom.node(div).get_attribute("id"), Some("hero"));
    }

    #[test]
    fn default_ext_is_default_tui_ext() {
        let ext: TuiExt = TuiExt::default();
        assert_eq!(ext.direction, Direction::Column);
        assert_eq!(ext.width, Size::Auto);
    }

    #[test]
    fn event_dispatch_works_over_tui_ext() {
        use std::cell::Cell;
        use std::rc::Rc;

        let mut dom: TuiDom = TuiDom::new();
        let btn = dom.create_element("button");
        let fired = Rc::new(Cell::new(false));
        let f = fired.clone();
        dom.add_event_listener(
            btn,
            "click",
            ListenerOptions::default(),
            move |_ctx: &mut TuiEventCtx<'_>| {
                f.set(true);
            },
        )
        .unwrap();

        let mut e = Event::new("click");
        dom.dispatch_event(btn, &mut e).unwrap();
        assert!(fired.get());
    }

    #[test]
    fn query_selector_works_over_tui_ext() {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let div = dom.create_element("div");
        dom.node_mut(div).set_id("target").unwrap();
        dom.append_child(root, div).unwrap();

        assert_eq!(dom.query_selector_in(root, "#target").unwrap(), Some(div));
    }

    #[test]
    fn presentation_builder_end_to_end() {
        let mut dom: TuiDom = TuiDom::new();
        let root = dom.root();
        let div = dom.create_element("div");
        dom.node_mut(div)
            .set_width(Size::Fixed(80))
            .set_height(Size::Flex(1))
            .set_padding(Padding::symmetric(2, 1))
            .set_border(Border::Rounded)
            .set_gap(1)
            .set_direction(Direction::Row)
            .set_inline_style(
                TuiStyle::new()
                    .fg(Color::Rgb(255, 255, 255))
                    .bg(Color::Rgb(0, 0, 0)),
            );

        dom.append_child(root, div).unwrap();

        let n = dom.node(div);
        assert_eq!(n.width(), Some(Size::Fixed(80)));
        assert_eq!(n.padding(), Some(Padding::symmetric(2, 1)));
        assert_eq!(n.border(), Some(Border::Rounded));
        assert_eq!(
            n.inline_style().unwrap().fg,
            Some(Value::Specified(TuiColor::Literal(Color::Rgb(
                255, 255, 255
            ))))
        );
    }
}
