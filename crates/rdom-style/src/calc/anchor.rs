//! The anchor functions (CSS Anchor Positioning 1 §5): `anchor()`, an
//! anchor's edge, in the inset properties, and `anchor-size()`, its size,
//! in the inset, sizing and margin properties — math leaves
//! ([`CalcExpr::Anchor`]) layout resolves against the anchor once it is
//! laid out ([`CalcExpr::substitute_anchors`]).

use std::sync::Arc;

use super::CalcExpr;

/// An `anchor()` or `anchor-size()` (§5.1, §5.2): the anchor named (or the
/// default anchor), what of it, and the fallback used when the anchor is
/// missing.
///
/// Open (`#[non_exhaustive]`): Anchor Positioning 2 may add functions; a
/// resolver that meets one it does not know takes its fallback.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum AnchorFunction {
    /// `anchor( <anchor-name>? && <anchor-side>, <length-percentage>? )`.
    Edge {
        name: Option<Arc<str>>,
        side: AnchorSide,
        fallback: Option<CalcExpr>,
    },
    /// `anchor-size( [ <anchor-name> || <anchor-size> ]?,
    /// <length-percentage>? )`; no size is the axis of the property.
    Size {
        name: Option<Arc<str>>,
        size: Option<AnchorSize>,
        fallback: Option<CalcExpr>,
    },
}

impl AnchorFunction {
    /// The anchor named, `None` for the default anchor.
    pub fn name(&self) -> Option<&str> {
        match self {
            AnchorFunction::Edge { name, .. } | AnchorFunction::Size { name, .. } => {
                name.as_deref()
            }
        }
    }

    /// The fallback, when written.
    pub fn fallback(&self) -> Option<&CalcExpr> {
        match self {
            AnchorFunction::Edge { fallback, .. } | AnchorFunction::Size { fallback, .. } => {
                fallback.as_ref()
            }
        }
    }
}

/// An `<anchor-side>` (§5.1.1).
///
/// Closed (DESIGN): the resolver reads each.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnchorSide {
    /// The anchor's edge on the inset's own side.
    Inside,
    /// The opposite edge.
    Outside,
    Top,
    Left,
    Right,
    Bottom,
    /// The start edge in the inset's axis, by the containing block's
    /// writing mode and direction.
    Start,
    End,
    /// By the positioned box's own.
    SelfStart,
    SelfEnd,
    /// Halfway: `50%`.
    Center,
    /// That far from the start edge (a percentage, `0..=100` typically).
    Percent(f64),
}

impl AnchorSide {
    /// Every keyword with its CSS spelling (the percentage aside).
    pub const KEYWORDS: &'static [(&'static str, AnchorSide)] = &[
        ("inside", AnchorSide::Inside),
        ("outside", AnchorSide::Outside),
        ("top", AnchorSide::Top),
        ("left", AnchorSide::Left),
        ("right", AnchorSide::Right),
        ("bottom", AnchorSide::Bottom),
        ("start", AnchorSide::Start),
        ("end", AnchorSide::End),
        ("self-start", AnchorSide::SelfStart),
        ("self-end", AnchorSide::SelfEnd),
        ("center", AnchorSide::Center),
    ];
}

/// An `<anchor-size>` (§5.2.1).
///
/// Closed (DESIGN): the resolver reads each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnchorSize {
    Width,
    Height,
    Block,
    Inline,
    SelfBlock,
    SelfInline,
}

impl AnchorSize {
    /// Every keyword with its CSS spelling.
    pub const KEYWORDS: &'static [(&'static str, AnchorSize)] = &[
        ("width", AnchorSize::Width),
        ("height", AnchorSize::Height),
        ("block", AnchorSize::Block),
        ("inline", AnchorSize::Inline),
        ("self-block", AnchorSize::SelfBlock),
        ("self-inline", AnchorSize::SelfInline),
    ];

    /// The keyword's CSS spelling.
    pub fn keyword(self) -> &'static str {
        Self::KEYWORDS
            .iter()
            .find(|(_, s)| *s == self)
            .map_or("width", |(name, _)| name)
    }
}

impl CalcExpr {
    /// Whether an anchor function is anywhere in the expression.
    pub fn contains_anchor(&self) -> bool {
        match self {
            CalcExpr::Anchor(_) => true,
            CalcExpr::Binary { lhs, rhs, .. } => lhs.contains_anchor() || rhs.contains_anchor(),
            CalcExpr::Function { args, .. } => args.iter().any(CalcExpr::contains_anchor),
            _ => false,
        }
    }

    /// The expression with each anchor function replaced by what
    /// `resolve` makes of it — a length in cells — or, where it gives
    /// `None` (no acceptable anchor), by the function's fallback (§5:
    /// "if the anchor is invalid … the fallback"); `None` when one has
    /// neither, which makes the property invalid at computed-value time.
    pub fn substitute_anchors(
        &self,
        resolve: &mut dyn FnMut(&AnchorFunction) -> Option<i32>,
    ) -> Option<CalcExpr> {
        Some(match self {
            CalcExpr::Anchor(f) => match resolve(f) {
                Some(cells) => CalcExpr::Length(cells),
                None => f.fallback()?.substitute_anchors(resolve)?,
            },
            CalcExpr::Binary { op, lhs, rhs } => CalcExpr::binary(
                *op,
                lhs.substitute_anchors(resolve)?,
                rhs.substitute_anchors(resolve)?,
            ),
            CalcExpr::Function { func, args } => CalcExpr::function(
                *func,
                args.iter()
                    .map(|a| a.substitute_anchors(resolve))
                    .collect::<Option<Vec<_>>>()?,
            ),
            other => other.clone(),
        })
    }
}
