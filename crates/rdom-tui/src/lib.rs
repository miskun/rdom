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
pub mod layout;
pub mod node;
pub mod prelude;
pub mod render;
pub mod runtime;
pub mod style;
pub mod tui_event;

mod sealed;

#[cfg(test)]
mod test_alloc;

pub use accessors::{
    GridTracks, ScrollBehaviorOption, ScrollIntoViewOptions, ScrollLogicalPosition, ScrollRange,
    ScrollToOptions, TuiAccessors, TuiAccessorsMut, TuiDocAccessors,
};
pub use cssom::{extend_from_style_tags, extend_from_style_tags_with_loader, seed_inline_styles};
pub use tui_event::{TuiDispatchExt, TuiEvent};

pub use ext::{StaticPosition, TuiExt};
pub use layout::{
    Align, AlignProperty, Alignment, AppliedDecorations, AppliedLine, AspectRatio,
    BackgroundAttachment, BackgroundRepeat, BlockEllipsis, Border, BorderRadius, BorderSpacing,
    BorderStyle, BorderWeight, BorderWidth, BoxOrient, BoxShadow, BoxSizing, Clear,
    ContainIntrinsicSize, Continue, CornerStyle, Corners, Direction, Display, FlexBasis,
    FlexDirection, FlexWrap, Float, FloatSide, Flow, Font, FontFamily, FontSize, FontSizeKeyword,
    FontStretch, FontStretchKeyword, FontStyle, FontVariant, FontWeight, GapValue, GridAutoFlow,
    GridLine, GridTemplate, GridTemplateAreas, Hyphens, IntrinsicSize, LayoutRect, Length,
    LineBreak, LineHeight, LineNameItem, LineNameList, ListStyleImage, ListStylePosition,
    ListStyleType, Margin, MarginTrim, MarginValue, MarkerSide, MaxSize, MinSize, NamedArea,
    Overflow, OverflowAlign, OverflowClipMargin, OverflowWrap, OverscrollBehavior, Padding,
    PaddingValue, PaintLength, RepeatCount, RepeatStyle, ScrollPadding, ScrollSnapAlign,
    ScrollSnapAxis, ScrollSnapStop, ScrollSnapStrictness, ScrollSnapType, ScrollbarColor,
    ScrollbarGutter, ScrollbarWidth, Sides, Size, SnapAlign, Spacing, SystemFont, TabSize,
    TextAlign, TextAlignKeyword, TextAlignLast, TextCase, TextDecoration, TextDecorationLine,
    TextDecorationSkipInk, TextDecorationStyle, TextDecorationThickness, TextDecorations,
    TextDirection, TextIndent, TextJustify, TextOverflow, TextOverflowSide, TextStyle,
    TextTransform, TextUnderlineOffset, TextUnderlinePosition, TextWrapMode, TextWrapStyle,
    TrackBreadth, TrackList, TrackListItem, TrackRepeat, TrackSize, UserSelect, VerticalAlign,
    Visibility, VisualBox, WhiteSpace, WhiteSpaceCollapse, WordBreak, WritingMode, ZIndex,
};
pub use node::{TuiNodeExt, TuiNodeMutExt};
/// `@import` resolution for the document's `<style>` sheets
/// ([`App::set_import_loader`], [`extend_from_style_tags_with_loader`]).
pub use rdom_css::{ImportLoader, LoadedSheet};
/// Math expressions (`calc()`, `min()`, …, CSS Values 4 §10): the
/// [`CalcExpr`](calc::CalcExpr) a `Size::Calc`, `MinSize::Calc`,
/// `MaxSize::Calc` or `Length::Calc` holds, and what resolves it.
pub use rdom_style::calc;
/// The size the viewport-percentage units (`vw`, `vh`, …) resolve
/// against: the document's ([`CascadeExt::set_viewport`]).
pub use rdom_style::calc::Viewport;
/// Test-only VT emulator; see [`render::virtual_screen`].
#[cfg(any(test, feature = "test-util"))]
pub use render::VirtualScreen;
pub use render::{
    Backend, Buffer, Cell, CellDiff, CompletedFrame, CrosstermBackend, LayoutExt, PaintExt, Rect,
    SgrCapabilities, Style, Terminal, TerminalGuard, TestBackend,
};
/// The canvas paint surface a `<canvas>` `set_paint` callback receives.
/// (Re-exported here as the canonical `RenderContext`; the old, unused
/// `render::RenderContext` was removed in `RENDERCTX-DEDUP-1`.)
pub use runtime::builtins::canvas::RenderContext;
pub use runtime::builtins::form::SubmitOutcome;
/// A `popover` attribute's state (HTML §6.12), what
/// [`runtime::builtins::popover::popover_state`] returns.
pub use runtime::builtins::popover::PopoverState;
pub use runtime::builtins::validation::ValidityState;
/// `focus(options)` (HTML `FocusOptions`), `TuiAccessorsMut::focus_with`.
pub use runtime::focus::FocusOptions;
/// The timer API on event contexts (`set_timeout`, `request_animation_frame`, …).
pub use runtime::timers::TuiTimers;
pub use runtime::{
    App, AppContext, AppHandle, ControlFlow, HitTestExt, RouteOutcome, Router, StylesheetId,
};
pub use style::{
    CascadeExt, Color, ColorContext, ColorFunction, ColorScheme, ColorSchemeList, ComputedStyle,
    Content, ContentContext, CounterOp, CounterStyle, CounterStyleName, CustomValue, DirtyTracker,
    FontDeclarations, ImportantMask, LayerId, Modifier, PropertyRegistration, PropertySyntax,
    PropertySyntaxError, PseudoElementTarget, QuoteKind, QuotePair, Quotes, RegisterPropertyError,
    Rule, RuleContext, RuleOrigin, Specificity, StyleError, StyleSelector, Stylesheet, SystemColor,
    TextDeclarations, TextDecorationDeclarations, TuiColor, TuiStyle, UserActionState, Value,
    VarMap, parse_color, resolve_tui_color,
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
/// An attribute selector's case flag (Selectors 4 §6.3), the `case` of
/// `core_api::selectors::SimpleSelector::Attribute`.
pub use rdom_core::selectors::AttrCase;
pub use rdom_core::{
    AdjacentPosition, ContentEditableState, ControlState, Directionality, DocumentPosition,
    DomError, Event, EventDetail, EventPhase, FormEnctype, FormMethod, Highlight,
    HighlightRegistry, HighlightType, HighlightsMut, InputDetail, InputType, InputTypeState,
    InteractionKind, KeyboardDetail, KeyboardModifiers, ListenerId, ListenerOptions, MouseButton,
    MouseDetail, Mutation, MutationObserver, NodeData, NodeId, NodeType, ObserverId, Position,
    Range, Result, Selection, SelectionSerial, SubmitDetail, ToggleDetail, ToggleState,
    TopLayerKind, TransitionDetail, ValidityHook,
};

#[cfg(test)]
mod tests {
    use super::*;

    /// `C1G-REEXPORTS`: the types of the App-level style APIs —
    /// `App::register_property` (CSS Properties and Values API 1 §3),
    /// `App::set_import_loader` (CSS Cascade 5 §3), cascade layers and
    /// `Stylesheet::add_style_rule` — are nameable from `rdom_tui`
    /// alone, without a direct `rdom-style` / `rdom-css` dependency.
    #[test]
    fn app_style_api_types_are_reexported() {
        let reg = PropertyRegistration::new("--c", "<color>", true, Some("red")).unwrap();
        let syntax: &PropertySyntax = &reg.syntax;
        assert!(syntax.matches("blue"));
        fn takes_loader(_: &dyn ImportLoader) {}
        takes_loader(&|_: &str| Ok(String::new()));
        let loaded = LoadedSheet::new("a.css", "p { width: 1 }");
        assert_eq!(loaded.url, "a.css");
        let mut sheet = Stylesheet::bare();
        let layer: Option<LayerId> = sheet.declare_layer(None, &["base"]);
        let selector = StyleSelector::parse("p").unwrap();
        sheet.add_style_rule(
            &selector,
            TuiStyle::new(),
            RuleContext::default().in_layer(layer),
        );
        assert_eq!(sheet.rules()[0].layer, layer);
    }

    /// `P7G-API-NAMES-1`: the rdom-core form / editing vocabulary a
    /// consumer names alongside `FormMethod` is re-exported here.
    #[test]
    fn form_and_editing_vocabulary_is_reexported() {
        let mut dom: TuiDom = TuiDom::new();
        let input = dom.create_element("input");
        assert_eq!(dom.input_type_state(input), Some(InputTypeState::Text));
        dom.set_attribute(input, "contenteditable", "").unwrap();
        assert_eq!(
            dom.content_editable_state(input),
            Some(ContentEditableState::True)
        );
        let hook: ValidityHook<TuiExt> = |_, _| true;
        dom.set_validity_hook(Some(hook));
        let serial: SelectionSerial = dom.selection_serial();
        assert_eq!(serial, dom.selection_serial());
    }

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
        // Geometry lives in `inline_style` (empty by default) since
        // EXT-LAYOUT-SETTERS-1 removed the raw `ext` geometry fields.
        assert!(ext.inline_style.is_none());
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
        // Geometry setters now write `inline_style`, and `set_inline_style`
        // replaces it wholesale — so seed the inline style first, then let
        // the geometry setters patch onto it.
        dom.node_mut(div)
            .set_inline_style(
                TuiStyle::new()
                    .fg(Color::Rgb(255, 255, 255))
                    .bg(Color::Rgb(0, 0, 0)),
            )
            .set_width(Size::Fixed(80))
            .set_height(Size::Flex(1.0))
            .set_padding(Padding::symmetric(2, 1))
            .set_border(Border::ring(crate::layout::BorderStyle::Double))
            .set_gap(1)
            .set_direction(Direction::Row);

        dom.append_child(root, div).unwrap();

        let n = dom.node(div);
        assert_eq!(n.width(), Some(Size::Fixed(80)));
        assert_eq!(n.padding(), Some(Padding::symmetric(2, 1)));
        assert_eq!(
            n.border(),
            Some(Border::ring(crate::layout::BorderStyle::Double))
        );
        assert_eq!(
            n.inline_style().unwrap().fg,
            Some(Value::Specified(TuiColor::Literal(Color::Rgb(
                255, 255, 255
            ))))
        );
    }
}

/// The README's examples, compiled and run as doctests so they keep
/// compiling as the API moves.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
