//! The seal on rdom-tui's call-only extension traits (DESIGN "Which
//! rdom-tui traits a consumer implements"): [`Sealed`] is public in a
//! private module, so no other crate can name it, and every trait that
//! only rdom implements takes it as a supertrait. A method added to one
//! of them is then not a breaking change. The traits a consumer is meant
//! to implement — `Backend`, `Clipboard`, `UrlOpener` — are not sealed.

use rdom_core::{Dom, EventCtx, NodeMut, NodeRef};

use crate::ext::TuiExt;

/// Implemented for exactly the types rdom-tui's extension traits extend.
pub trait Sealed {}

impl Sealed for Dom<TuiExt> {}
impl Sealed for NodeRef<'_, TuiExt> {}
impl Sealed for NodeMut<'_, TuiExt> {}
impl Sealed for EventCtx<'_, TuiExt> {}

/// Each sealed trait, implemented in full outside rdom-tui, fails to
/// compile.
#[cfg(doctest)]
#[doc = include_str!("doctests.md")]
struct SealedTraitsDoctests;
