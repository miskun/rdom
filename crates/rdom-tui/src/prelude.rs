//! One-stop import for the common API. `use rdom_tui::prelude::*;`
//! brings in the types a typical app needs: `TuiDom`, the node
//! accessor traits, the core style types, and extension traits whose
//! methods (`cascade`, `set_width`, `computed`, ...) would otherwise
//! be invisible until imported.
//!
//! ## M4 accessor surface
//!
//! The prelude re-exports the M4b accessor traits so a single
//! `use rdom_tui::prelude::*;` brings them in scope:
//!
//! - [`TuiAccessors`](crate::TuiAccessors) — per-element read
//!   methods (`value`, `checked`, `style`, per-tag accessors).
//! - [`TuiAccessorsMut`](crate::TuiAccessorsMut) — per-element
//!   write methods (`set_value`, `style_mut`, `focus`, `click`,
//!   …).
//! - [`TuiDocAccessors`](crate::TuiDocAccessors) — document-level
//!   read methods (`element_from_point`,
//!   `caret_position_from_point`).
//!
//! ### Smart vs narrow accessors
//!
//! Every form-control reading method ships in two shapes:
//!
//! - **Smart** (e.g. [`value()`](crate::TuiAccessors::value)) —
//!   dispatches on tag at runtime. `dom.node(id).value()` works
//!   whether `id` is an `<input>`, `<textarea>`, or `<select>`;
//!   returns `None` on any other tag.
//! - **Narrow** (e.g.
//!   [`input_value()`](crate::TuiAccessors::input_value),
//!   [`select_value()`](crate::TuiAccessors::select_value),
//!   [`option_value()`](crate::TuiAccessors::option_value), and
//!   the rest of the per-tag set) —
//!   returns `None` for any tag *other than the named one*.
//!
//! Reach for the narrow variant when you already know the tag
//! (or want a compile-time-readable assertion at the call site
//! that "this should be an input"); reach for the smart variant
//! at generic walk-the-tree code where the tag is dynamic.
//!
//! ## `NodeMutHtml` (not in the prelude)
//!
//! The `set_inner_html` / `set_outer_html` / `insert_adjacent_html`
//! extension trait lives in `rdom-parser` and is intentionally
//! NOT re-exported here — `rdom-tui` doesn't depend on
//! `rdom-parser`. Apps that need it write
//! `use rdom_parser::NodeMutHtml;` alongside the prelude
//! import.
//!
//! For the full surface use `rdom_tui::*` directly; for access to
//! `rdom-core` internals use `rdom_tui::core_api::…`.

pub use crate::{
    // Selected rdom-core re-exports most apps will need
    AdjacentPosition,
    // Ext + layout
    Align,
    // Runtime primitives
    App,
    AppContext,
    AppHandle,
    // Render primitives (paint layer)
    Backend,
    Border,
    Buffer,
    // Style types (cascade layer)
    CascadeExt,
    Cell,
    CellDiff,
    Color,
    CompletedFrame,
    ComputedStyle,
    Content,
    ControlFlow,
    CrosstermBackend,
    Direction,
    DirtyTracker,
    Display,
    DomError,
    Event,
    EventPhase,
    Flow,
    HitTestExt,
    ImportantMask,
    InteractionKind,
    LayoutExt,
    LayoutRect,
    ListenerOptions,
    Modifier,
    Mutation,
    MutationObserver,
    NodeData,
    NodeId,
    NodeType,
    ObserverId,
    Overflow,
    Padding,
    PaintExt,
    // Selection types (re-exported from rdom-core)
    Position,
    PropMask,
    PseudoElementTarget,
    Range,
    Rect,
    RenderContext,
    Result,
    RouteOutcome,
    Router,
    Rule,
    RuleOrigin,
    Selection,
    Size,
    Specificity,
    Style,
    StyleError,
    Stylesheet,
    Terminal,
    TerminalGuard,
    TestBackend,
    // Author-facing accessor traits (M4b)
    TuiAccessors,
    TuiAccessorsMut,
    TuiColor,
    // TUI event wrapper + dispatch extension
    TuiDispatchExt,
    TuiDocAccessors,
    // Core aliases
    TuiDom,
    TuiEvent,
    TuiEventCtx,
    TuiExt,
    // Extension traits (methods invisible without them)
    TuiNodeExt,
    TuiNodeMut,
    TuiNodeMutExt,
    TuiNodeRef,
    TuiStyle,
    UserSelect,
    Value,
    VarMap,
    VirtualScreen,
    WhiteSpace,
};
