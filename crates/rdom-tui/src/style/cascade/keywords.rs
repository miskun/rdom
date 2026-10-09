//! The CSS-wide keywords of one ladder pass and the resolution every
//! applicator shares (`apply`): a declared `Value<T>` is its specified
//! value, or — `inherit` / `initial` / `revert` / `revert-layer` — the
//! field of a whole computed style ([`Keywords::resolve`]): the parent's,
//! `ComputedStyle::initial()` (via [`Initials`], the table every element's
//! cascade starts from), or the ladder's rollback state. Normal and
//! `!important` declarations apply in separate passes ([`matches_pass`]).

use crate::style::{ComputedStyle, Value};

/// Where the CSS-wide keywords of one ladder pass take their values
/// from.
pub(super) struct Keywords<'a> {
    /// `inherit`: the parent's computed style (CSS Cascade 4 §7.2).
    pub parent: &'a ComputedStyle,
    /// The document's preferred color scheme (CSS Color Adjust 1
    /// §2.1), which the colors cascaded so far resolve under.
    pub preferred_scheme: rdom_style::color::ColorScheme,
    /// `initial`: the initial values (§7.1).
    pub initial: &'a Initials,
    /// `revert`: the cascade rolled back to the previous origin
    /// (§7.3), computed on first use.
    pub revert: &'a dyn Fn() -> &'a ComputedStyle,
    /// `revert-layer`: the cascade rolled back to the previous cascade
    /// layer (Cascade 5 §7.4), computed on first use.
    pub revert_layer: &'a dyn Fn() -> &'a ComputedStyle,
}

/// A declared value, resolved for one pass: the specified value, or
/// the computed style a CSS-wide keyword copies the field from.
pub(super) enum Resolved<'v, 'a, T> {
    Specified(&'v T),
    From(&'a ComputedStyle),
}

impl<'a> Keywords<'a> {
    pub(super) fn resolve<'v, T>(&self, value: &'v Value<T>) -> Resolved<'v, 'a, T> {
        match value {
            Value::Specified(x) => Resolved::Specified(x),
            Value::Inherit => Resolved::From(self.parent),
            Value::Initial => Resolved::From(self.initial.get()),
            Value::Revert => Resolved::From((self.revert)()),
            Value::RevertLayer => Resolved::From((self.revert_layer)()),
        }
    }
}

/// Should this declaration actually apply during the current pass?
/// Normal pass applies normal declarations; important pass applies
/// important ones.
#[inline]
pub(super) fn matches_pass(important_prop: bool, important_pass: bool) -> bool {
    important_prop == important_pass
}

/// The initial values `initial` resolves to: `ComputedStyle::initial()`,
/// the same table every element's cascade starts from, so the two
/// cannot drift (`P6G-APPLY-INITIALS-1`). Built on the first `initial`
/// keyword an element's cascade meets; most elements never build it.
#[derive(Default)]
pub(super) struct Initials(std::cell::OnceCell<ComputedStyle>);

impl Initials {
    pub(super) fn get(&self) -> &ComputedStyle {
        self.0.get_or_init(ComputedStyle::initial)
    }
}

/// The CSS-wide keyword resolution every property shares: specified
/// as written, a keyword the `field` of its source style
/// ([`Keywords::resolve`]) — `inherit` the parent's computed value
/// (inherited property or not — CSS Cascade 4 §7.2), `initial` the
/// property's initial value, `revert` the rolled-back cascade's.
pub(super) fn apply_value<T: Clone>(
    target: &mut T,
    value: &Option<Value<T>>,
    important_prop: bool,
    important_pass: bool,
    kw: &Keywords<'_>,
    field: fn(&ComputedStyle) -> &T,
) {
    apply_converted(
        target,
        value,
        important_prop,
        important_pass,
        kw,
        field,
        |v| Some(v.clone()),
    );
}

/// [`apply_value`] for a property whose declared form `S` differs from
/// its computed one `T`, or may lie outside its grammar: `to` computes a
/// declared value, `None` ignoring the declaration (the value stays).
pub(super) fn apply_converted<S, T: Clone>(
    target: &mut T,
    value: &Option<Value<S>>,
    important_prop: bool,
    important_pass: bool,
    kw: &Keywords<'_>,
    field: fn(&ComputedStyle) -> &T,
    to: impl Fn(&S) -> Option<T>,
) {
    if let Some(x) = resolved(value, important_prop, important_pass, kw, field, to) {
        *target = x;
    }
}

/// The value one declaration gives its field in this pass — `None` when it
/// does not apply (absent, the other pass) or `to` ignores it.
pub(super) fn resolved<S, T: Clone>(
    value: &Option<Value<S>>,
    important_prop: bool,
    important_pass: bool,
    kw: &Keywords<'_>,
    field: fn(&ComputedStyle) -> &T,
    to: impl Fn(&S) -> Option<T>,
) -> Option<T> {
    let v = value.as_ref()?;
    if !matches_pass(important_prop, important_pass) {
        return None;
    }
    match kw.resolve(v) {
        Resolved::Specified(x) => to(x),
        Resolved::From(source) => Some(field(source).clone()),
    }
}
