//! Containment: query containers (CSS Conditional 5 §6.1–§6.3:
//! `container-type`, `container-name`), `contain` (CSS Containment 2 §2)
//! `content-visibility` (§4) and `will-change` (CSS Will Change 1 §2).

use std::sync::Arc;

/// `container-type` (CSS Conditional 5 §6.1): `normal | [ [ size |
/// inline-size ] || scroll-state ]`. A `size` or `inline-size` container
/// answers size queries and applies size containment on those axes (CSS
/// Containment 3); every element is a style container.
///
/// Open (`#[non_exhaustive]`): CSS Anchor Positioning 2 adds `anchored`
/// beside `scroll-state`, so it is built with [`ContainerType::new`] and
/// [`with_scroll_state`](Self::with_scroll_state), not a literal:
///
/// ```compile_fail
/// use rdom_style::layout::{ContainerSize, ContainerType};
/// let _ = ContainerType { size: ContainerSize::Size, scroll_state: false };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ContainerType {
    /// The axes it answers size queries on.
    pub size: ContainerSize,
    /// `scroll-state`: it answers `scroll-state()` queries.
    pub scroll_state: bool,
}

/// The size half of `container-type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContainerSize {
    /// No size queries.
    #[default]
    Normal,
    /// The inline axis: `width` / `inline-size` queries, inline-size
    /// containment.
    InlineSize,
    /// Both axes: size containment.
    Size,
}

impl ContainerType {
    /// A container answering size queries on `size`'s axes (`normal`:
    /// none), and no `scroll-state()` queries.
    pub const fn new(size: ContainerSize) -> Self {
        ContainerType {
            size,
            scroll_state: false,
        }
    }

    /// This type answering `scroll-state()` queries too, when `on`.
    pub const fn with_scroll_state(mut self, on: bool) -> Self {
        self.scroll_state = on;
        self
    }

    /// Whether it answers size queries on the inline axis (horizontal-tb:
    /// the width).
    pub fn queries_inline(self) -> bool {
        self.size != ContainerSize::Normal
    }

    /// Whether it answers size queries on the block axis.
    pub fn queries_block(self) -> bool {
        self.size == ContainerSize::Size
    }

    /// The CSS text.
    pub fn css(self) -> &'static str {
        match (self.size, self.scroll_state) {
            (ContainerSize::Normal, false) => "normal",
            (ContainerSize::Normal, true) => "scroll-state",
            (ContainerSize::InlineSize, false) => "inline-size",
            (ContainerSize::InlineSize, true) => "inline-size scroll-state",
            (ContainerSize::Size, false) => "size",
            (ContainerSize::Size, true) => "size scroll-state",
        }
    }
}

/// `container-name` (CSS Conditional 5 §6.2): `none | <custom-ident>+`,
/// the names an `@container` rule may select this container by
/// (case-sensitive).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContainerName {
    names: Arc<[Arc<str>]>,
}

impl ContainerName {
    /// `none`.
    pub fn none() -> Self {
        Self::default()
    }

    /// `names` (at least one, each a valid `<custom-ident>`: the parser
    /// checks; `none` when empty).
    pub fn new(names: impl IntoIterator<Item = Arc<str>>) -> Self {
        ContainerName {
            names: names.into_iter().collect(),
        }
    }

    /// The names, in order.
    pub fn names(&self) -> &[Arc<str>] {
        &self.names
    }

    /// `none`.
    pub fn is_none(&self) -> bool {
        self.names.is_empty()
    }

    /// Whether `name` is one of the names.
    pub fn has(&self, name: &str) -> bool {
        self.names.iter().any(|n| &**n == name)
    }
}

/// `contain` (CSS Containment 2 §2, Containment 3): `none | strict |
/// content | [ [ size | inline-size ] || layout || style || paint ]` —
/// `strict` is `size layout style paint`, `content` `layout style paint`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Contain {
    /// Size containment on both axes (§3.1).
    pub size: bool,
    /// Size containment on the inline axis (Containment 3).
    pub inline_size: bool,
    /// Layout containment (§3.2).
    pub layout: bool,
    /// Style containment (§3.3).
    pub style: bool,
    /// Paint containment (§3.4).
    pub paint: bool,
}

impl Contain {
    /// `none`.
    pub const NONE: Contain = Contain {
        size: false,
        inline_size: false,
        layout: false,
        style: false,
        paint: false,
    };
    /// `content`: `layout style paint`.
    pub const CONTENT: Contain = Contain {
        size: false,
        inline_size: false,
        layout: true,
        style: true,
        paint: true,
    };
    /// `strict`: `size layout style paint`.
    pub const STRICT: Contain = Contain {
        size: true,
        ..Contain::CONTENT
    };

    /// The CSS text: a keyword where one names the value, else the
    /// containment types in grammar order.
    pub fn css(self) -> String {
        if self == Contain::NONE {
            return "none".to_string();
        }
        if self == Contain::STRICT {
            return "strict".to_string();
        }
        if self == Contain::CONTENT {
            return "content".to_string();
        }
        [
            (self.size, "size"),
            (self.inline_size, "inline-size"),
            (self.layout, "layout"),
            (self.style, "style"),
            (self.paint, "paint"),
        ]
        .iter()
        .filter(|(on, _)| *on)
        .map(|(_, n)| *n)
        .collect::<Vec<_>>()
        .join(" ")
    }
}

/// `will-change` (CSS Will Change 1 §2): `auto | <animateable-feature>#`,
/// the features as written (property names ASCII-lowercased); `auto`
/// when empty.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WillChange {
    features: Arc<[Arc<str>]>,
}

impl WillChange {
    /// `auto`.
    pub fn auto() -> Self {
        Self::default()
    }

    /// `features`, in order (the parser checks each).
    pub fn new(features: impl IntoIterator<Item = Arc<str>>) -> Self {
        WillChange {
            features: features.into_iter().collect(),
        }
    }

    /// The features, in order; empty for `auto`.
    pub fn features(&self) -> &[Arc<str>] {
        &self.features
    }

    /// Whether `feature` is one of them.
    pub fn has(&self, feature: &str) -> bool {
        self.features.iter().any(|f| &**f == feature)
    }
}

/// `content-visibility` (CSS Containment 2 §4): whether the element
/// skips its contents — never (`visible`), always (`hidden`), or while it
/// is not relevant to the user (`auto`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContentVisibility {
    #[default]
    Visible,
    Auto,
    Hidden,
}

impl ContentVisibility {
    /// The CSS keyword.
    pub fn css(self) -> &'static str {
        match self {
            ContentVisibility::Visible => "visible",
            ContentVisibility::Auto => "auto",
            ContentVisibility::Hidden => "hidden",
        }
    }
}
