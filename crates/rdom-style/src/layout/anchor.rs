//! The CSS Anchor Positioning 1 values: the anchor names (`anchor-name`,
//! `anchor-scope`, `position-anchor`), the position fallbacks
//! (`position-try-fallbacks`, `position-try-order`, §4) and
//! `position-visibility` (§5) — with [`AnchorStyle`], their computed
//! group. `position-area` (§3.1) is `position_area`'s; the `anchor()` /
//! `anchor-size()` functions are math leaves
//! ([`AnchorFunction`](crate::calc::AnchorFunction)).

use std::sync::Arc;

use super::PositionArea;

/// `anchor-name` (§2.1): `none | <dashed-ident>#` — the names other
/// boxes find this one by as an anchor.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AnchorName {
    names: Arc<[Arc<str>]>,
}

impl AnchorName {
    /// `none`.
    pub fn none() -> Self {
        Self::default()
    }

    /// `names` (each a `<dashed-ident>`: the parser checks; `none` when
    /// empty).
    pub fn new(names: impl IntoIterator<Item = Arc<str>>) -> Self {
        AnchorName {
            names: names.into_iter().collect(),
        }
    }

    /// The names, in order.
    pub fn names(&self) -> &[Arc<str>] {
        &self.names
    }

    /// Whether `name` is one of them.
    pub fn has(&self, name: &str) -> bool {
        self.names.iter().any(|n| &**n == name)
    }
}

/// `anchor-scope` (§2.2): `none | all | <dashed-ident>#` — the anchor
/// names this box's subtree keeps to itself.
///
/// Closed (DESIGN): the lookup decides each.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum AnchorScope {
    /// No scope (the initial value).
    #[default]
    None,
    /// Every anchor name in the subtree.
    All,
    /// These names.
    Names(Arc<[Arc<str>]>),
}

impl AnchorScope {
    /// Whether the scope keeps `name` inside the subtree.
    pub fn scopes(&self, name: &str) -> bool {
        match self {
            AnchorScope::None => false,
            AnchorScope::All => true,
            AnchorScope::Names(names) => names.iter().any(|n| &**n == name),
        }
    }
}

/// `position-anchor` (§2.3): `auto | none | <anchor-name>` — the default
/// anchor: the implicit one (a popover's invoker, HTML §6.12) under
/// `auto`, none under `none`, else the named one.
///
/// Closed (DESIGN): the lookup decides each.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PositionAnchor {
    /// The implicit anchor element (the initial value).
    #[default]
    Auto,
    /// No default anchor.
    None,
    /// The anchor named so.
    Name(Arc<str>),
}

/// The try tactics of a position option (§4.1): `flip-block`,
/// `flip-inline`, `flip-start` (and Anchor Positioning 2's `flip-x`,
/// `flip-y`), applied in the order written.
///
/// Closed (DESIGN): the fallback engine applies each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TryTactic {
    FlipBlock,
    FlipInline,
    FlipStart,
    FlipX,
    FlipY,
}

impl TryTactic {
    /// Every tactic with its CSS spelling.
    pub const KEYWORDS: &'static [(&'static str, TryTactic)] = &[
        ("flip-block", TryTactic::FlipBlock),
        ("flip-inline", TryTactic::FlipInline),
        ("flip-start", TryTactic::FlipStart),
        ("flip-x", TryTactic::FlipX),
        ("flip-y", TryTactic::FlipY),
    ];

    /// The tactic's CSS spelling.
    pub fn keyword(self) -> &'static str {
        Self::KEYWORDS
            .iter()
            .find(|(_, t)| *t == self)
            .map_or("flip-block", |(name, _)| name)
    }
}

/// One entry of `position-try-fallbacks` (§4.1): a `@position-try` rule's
/// name, try tactics, or both — or a `position-area`.
///
/// `#[non_exhaustive]`: built by [`TryFallback::rule`],
/// [`TryFallback::tactics`] / [`TryFallback::area`]; the cascade attaches the named rule's
/// declarations ([`declarations`](Self::declarations)).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TryFallback {
    /// The `@position-try` rule's `<dashed-ident>`.
    pub name: Option<Arc<str>>,
    /// The tactics, in order.
    pub tactics: Vec<TryTactic>,
    /// A `position-area` to try (the `<'position-area'>` form).
    pub area: Option<PositionArea>,
    /// The named rule's declarations, attached by the cascade from the
    /// sheets' `@position-try` rules; `None` when none has the name.
    pub(crate) declarations: Option<Arc<crate::TuiStyle>>,
}

impl TryFallback {
    /// A `@position-try` rule `name` with `tactics` (either may be
    /// absent, not both — the parser checks).
    pub fn rule(name: Option<Arc<str>>, tactics: Vec<TryTactic>) -> Self {
        TryFallback {
            name,
            tactics,
            area: None,
            declarations: None,
        }
    }

    /// A tactics-only option (`flip-block`, `flip-inline flip-start`, …):
    /// the box's own style, flipped.
    pub fn tactics(tactics: Vec<TryTactic>) -> Self {
        Self::rule(None, tactics)
    }

    /// A `position-area` option.
    pub fn area(area: PositionArea) -> Self {
        TryFallback {
            name: None,
            tactics: Vec::new(),
            area: Some(area),
            declarations: None,
        }
    }

    /// The named `@position-try` rule's declarations, once the cascade
    /// found them.
    pub fn declarations(&self) -> Option<&crate::TuiStyle> {
        self.declarations.as_deref()
    }

    /// This option with the named rule's declarations attached.
    pub fn with_declarations(mut self, declarations: Option<Arc<crate::TuiStyle>>) -> Self {
        self.declarations = declarations;
        self
    }
}

/// `position-try-order` (§4.2): `normal | <try-size>`.
///
/// Closed (DESIGN): the fallback engine orders by each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum PositionTryOrder {
    /// The options in the order written (the initial value).
    #[default]
    Normal,
    MostWidth,
    MostHeight,
    MostBlockSize,
    MostInlineSize,
}

impl PositionTryOrder {
    /// Every keyword with its CSS spelling.
    pub const KEYWORDS: &'static [(&'static str, PositionTryOrder)] = &[
        ("normal", PositionTryOrder::Normal),
        ("most-width", PositionTryOrder::MostWidth),
        ("most-height", PositionTryOrder::MostHeight),
        ("most-block-size", PositionTryOrder::MostBlockSize),
        ("most-inline-size", PositionTryOrder::MostInlineSize),
    ];

    /// The keyword's CSS spelling.
    pub fn keyword(self) -> &'static str {
        Self::KEYWORDS
            .iter()
            .find(|(_, o)| *o == self)
            .map_or("normal", |(name, _)| name)
    }
}

/// `position-visibility` (§5): `always | [ anchors-valid ||
/// anchors-visible || no-overflow ]` — when an anchor-positioned box is
/// hidden. Initial `anchors-visible`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PositionVisibility {
    /// Hidden when an anchor it needs is missing.
    pub anchors_valid: bool,
    /// Hidden when its default anchor is clipped out of view.
    pub anchors_visible: bool,
    /// Hidden when it overflows its containing block whatever it tries.
    pub no_overflow: bool,
}

impl PositionVisibility {
    /// `always`: never hidden.
    pub const ALWAYS: Self = PositionVisibility {
        anchors_valid: false,
        anchors_visible: false,
        no_overflow: false,
    };
}

impl Default for PositionVisibility {
    fn default() -> Self {
        PositionVisibility {
            anchors_visible: true,
            ..Self::ALWAYS
        }
    }
}

/// The computed CSS Anchor Positioning 1 properties of an element
/// ([`ComputedStyle::anchor`](crate::ComputedStyle::anchor)); none
/// inherit.
///
/// Closed (DESIGN), as the other style groups. `Default` is the initial
/// values.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnchorStyle {
    /// `anchor-name` (§2.1).
    pub anchor_name: AnchorName,
    /// `anchor-scope` (§2.2).
    pub anchor_scope: AnchorScope,
    /// `position-anchor` (§2.3).
    pub position_anchor: PositionAnchor,
    /// `position-area` (§3.1).
    pub position_area: PositionArea,
    /// `position-try-fallbacks` (§4.1); empty is `none`.
    pub position_try_fallbacks: Vec<TryFallback>,
    /// `position-try-order` (§4.2).
    pub position_try_order: PositionTryOrder,
    /// `position-visibility` (§5).
    pub position_visibility: PositionVisibility,
}

impl AnchorStyle {
    /// Whether the values give layout anything to do for a positioned
    /// box: an anchor reference or a fallback.
    pub fn is_anchored(&self) -> bool {
        !self.position_area.is_none() || !self.position_try_fallbacks.is_empty()
    }
}
