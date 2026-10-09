//! [`Shared`]: a style group held behind a shared pointer, copied on
//! write (C15G-STYLE-SIZE).
//!
//! Every element carries a [`ComputedStyle`](crate::ComputedStyle), and
//! every rule and inline style a [`TuiStyle`](crate::TuiStyle); most of
//! their groups — the effects, multi-column, anchor positioning and UI
//! properties, the mask declarations — are never set on most elements. A
//! `Shared` group is one pointer: the styles that hold the same value
//! share it, the default value is one per group type, and a write copies
//! the group first when anything else holds it ([`DerefMut`], as
//! `Arc::make_mut`). Reading a field is unchanged (`style.effects.filter`,
//! through [`Deref`]); naming the group's own type takes a `*`
//! (`*style.effects == EffectsStyle::default()`).

use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, OnceLock};

/// A style group shared between the styles holding the same value, copied
/// on write (module doc). Closed: a wrapper, its pointer private.
#[derive(Clone)]
pub struct Shared<T>(Arc<T>);

impl<T> Shared<T> {
    /// A group holding `value`, shared by nothing else yet.
    pub fn new(value: T) -> Self {
        Shared(Arc::new(value))
    }

    /// Whether `a` and `b` are the same shared group (not merely equal):
    /// what a test of the sharing asks.
    pub fn ptr_eq(a: &Self, b: &Self) -> bool {
        Arc::ptr_eq(&a.0, &b.0)
    }
}

impl<T> Deref for Shared<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: Clone> DerefMut for Shared<T> {
    /// The group, copied first when another style holds it.
    fn deref_mut(&mut self) -> &mut T {
        Arc::make_mut(&mut self.0)
    }
}

impl<T: PartialEq> PartialEq for Shared<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || *self.0 == *other.0
    }
}

impl<T: fmt::Debug> fmt::Debug for Shared<T> {
    /// As the group itself: the sharing does not show.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> From<T> for Shared<T> {
    fn from(value: T) -> Self {
        Shared::new(value)
    }
}

/// `Default` for `Shared<$t>`: one default per group type, shared by every
/// style that has not written the group.
macro_rules! shared_default {
    ($($t:ty),* $(,)?) => {$(
        impl Default for Shared<$t> {
            fn default() -> Self {
                static DEFAULT: OnceLock<Arc<$t>> = OnceLock::new();
                Shared(DEFAULT.get_or_init(|| Arc::new(<$t>::default())).clone())
            }
        }
    )*};
}

shared_default!(
    crate::layout::EffectsStyle,
    crate::layout::MulticolStyle,
    crate::layout::AnchorStyle,
    crate::layout::UiStyle,
    crate::tui_style::EffectsDeclarations,
    crate::tui_style::MaskDeclarations,
    crate::tui_style::MulticolDeclarations,
    crate::tui_style::AnchorDeclarations,
    crate::tui_style::UiDeclarations,
);
