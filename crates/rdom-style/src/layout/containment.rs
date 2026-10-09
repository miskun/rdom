//! Query containers (CSS Conditional 5 §6.1–§6.3): `container-type` and
//! `container-name`.

use std::sync::Arc;

/// `container-type` (CSS Conditional 5 §6.1): `normal | [ [ size |
/// inline-size ] || scroll-state ]`. A `size` or `inline-size` container
/// answers size queries and applies size containment on those axes (CSS
/// Containment 3); every element is a style container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
