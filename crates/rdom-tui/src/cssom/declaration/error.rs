//! [`SetPropertyError`] — the typed failure channel surfaced by the
//! `try_*` setters on [`super::StyleDeclarationMut`].

use rdom_style::property_dispatch::DispatchError;

/// Failure modes for [`StyleDeclarationMut::try_set_property`] /
/// [`StyleDeclarationMut::try_set_property_important`].
///
/// The shipped [`StyleDeclarationMut::set_property`] family
/// silently swallows [`Parse`] errors per browser CSSOM —
/// `el.style.color = "not-a-color"` doesn't throw. That's
/// debugger-friendly in a browser (devtools console surfaces the
/// drop) but a real footgun in a terminal app where stdout is
/// often redirected. The `try_*` variants expose the parse
/// channel so callers who want to know can.
///
/// `Tree` wraps a `rdom_core::DomError` from the underlying
/// `set_attribute` write. In practice this only fires when the
/// node has been detached between the borrow check and the
/// attribute write — vanishingly rare for typical author
/// usage, but surfaced rather than swallowed.
///
/// [`Parse`]: SetPropertyError::Parse
/// [`StyleDeclarationMut::try_set_property`]: super::StyleDeclarationMut::try_set_property
/// [`StyleDeclarationMut::try_set_property_important`]: super::StyleDeclarationMut::try_set_property_important
/// [`StyleDeclarationMut::set_property`]: super::StyleDeclarationMut::set_property
#[derive(Debug)]
pub enum SetPropertyError {
    /// `name` wasn't in the property-dispatch table or `value`
    /// failed to parse. Wraps the inner [`DispatchError`].
    Parse(DispatchError),
    /// The attribute-write step (`style="…"`) failed. Wraps
    /// the inner `rdom_core::DomError`.
    Tree(rdom_core::DomError),
}

impl From<DispatchError> for SetPropertyError {
    fn from(e: DispatchError) -> Self {
        Self::Parse(e)
    }
}

impl From<rdom_core::DomError> for SetPropertyError {
    fn from(e: rdom_core::DomError) -> Self {
        Self::Tree(e)
    }
}

impl core::fmt::Display for SetPropertyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse(DispatchError::UnknownProperty) => {
                write!(f, "unknown CSS property")
            }
            Self::Parse(DispatchError::InvalidValue) => {
                write!(f, "invalid value for property")
            }
            Self::Tree(e) => write!(f, "DOM tree error: {e:?}"),
        }
    }
}

impl std::error::Error for SetPropertyError {}
