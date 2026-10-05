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
//! - [`TuiAccessors`] — per-element read
//!   methods (`value`, `checked`, `style`, per-tag accessors).
//! - [`TuiAccessorsMut`] — per-element
//!   write methods (`set_value`, `style_mut`, `focus`, `click`,
//!   …).
//! - [`TuiDocAccessors`] — document-level
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

/// Test-only VT emulator (`test-util` feature).
#[cfg(any(test, feature = "test-util"))]
pub use crate::VirtualScreen;
/// The math expression a `Size::Calc`, `MinSize::Calc`, `MaxSize::Calc`
/// or `Length::Calc` holds (`rdom_tui::calc` has the rest).
pub use crate::calc::CalcExpr;
pub use crate::{
    // Selected rdom-core re-exports most apps will need
    AdjacentPosition,
    // Ext + layout
    Align,
    // Runtime primitives
    App,
    AppContext,
    AppHandle,
    AspectRatio,
    Backend,
    // Render primitives (paint layer)
    BackgroundAttachment,
    BackgroundRepeat,
    Border,
    BorderRadius,
    BorderSpacing,
    BorderStyle,
    BorderWeight,
    BorderWidth,
    BoxShadow,
    BoxSizing,
    Buffer,
    // Style types (cascade layer)
    CascadeExt,
    Cell,
    CellDiff,
    Color,
    ColorContext,
    ColorFunction,
    ColorScheme,
    ColorSchemeList,
    CompletedFrame,
    ComputedStyle,
    Content,
    ContentContext,
    ControlFlow,
    CornerStyle,
    Corners,
    CrosstermBackend,
    CustomValue,
    Direction,
    DirtyTracker,
    Display,
    DomError,
    Event,
    EventPhase,
    FlexBasis,
    Flow,
    HitTestExt,
    ImportantMask,
    InteractionKind,
    IntrinsicSize,
    LayoutExt,
    LayoutRect,
    ListenerOptions,
    MaxSize,
    MinSize,
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
    PaintLength,
    // Selection types (re-exported from rdom-core)
    Position,
    PseudoElementTarget,
    Range,
    Rect,
    RenderContext,
    RepeatStyle,
    Result,
    RouteOutcome,
    Router,
    Rule,
    RuleOrigin,
    Selection,
    Sides,
    Size,
    Specificity,
    Style,
    StyleError,
    Stylesheet,
    SystemColor,
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
    Viewport,
    VisualBox,
    WhiteSpace,
    parse_color,
    resolve_tui_color,
};
