//! The list properties (CSS Lists 3 §3): what a list item's marker
//! shows, where it sits, and on which side.

use crate::counters::CounterStyle;

/// `list-style-type` (§3.4): the marker's content when `::marker`'s
/// `content` is `normal`. Inherited; initial `disc`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ListStyleType {
    /// No marker content (the marker is not generated).
    None,
    /// The `list-item` counter in a counter style, between the style's
    /// prefix and suffix.
    Style(CounterStyle),
    /// A string, the marker's content as it is.
    String(String),
}

impl Default for ListStyleType {
    fn default() -> Self {
        ListStyleType::Style(CounterStyle::disc())
    }
}

impl ListStyleType {
    /// The CSS text.
    pub fn to_css(&self) -> String {
        match self {
            ListStyleType::None => "none".to_string(),
            ListStyleType::Style(style) => style.to_css(),
            ListStyleType::String(s) => rdom_core::css_syntax::serialize_string(s),
        }
    }
}

/// `list-style-position` (§3.5). Inherited; initial `outside`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum ListStylePosition {
    /// The marker hangs outside the list item's box, beside its first
    /// line.
    #[default]
    Outside,
    /// The marker is the first inline box of the list item's first line.
    Inside,
}

/// `list-style-image` (§3.3): parsed and kept; a terminal has no pixels
/// to draw an image marker into, so the marker always comes from
/// `list-style-type` (DIVERGENCES §1). Inherited; initial `none`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum ListStyleImage {
    #[default]
    None,
    /// An `<image>`, as its CSS text.
    Image(String),
}

impl ListStyleImage {
    /// The CSS text.
    pub fn to_css(&self) -> String {
        match self {
            ListStyleImage::None => "none".to_string(),
            ListStyleImage::Image(text) => text.clone(),
        }
    }
}

/// `marker-side` (CSS Lists 3 §3.6): which inline side an outside marker
/// hangs on. Inherited; initial `match-self`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum MarkerSide {
    /// The list item's own inline-start side (its `direction`).
    #[default]
    MatchSelf,
    /// The inline-start side of the list item's parent.
    MatchParent,
}
