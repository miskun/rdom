//! `Value<T>` — the CSS tri-state for any style property.
//!
//! CSS has three states a property can be in, distinct from "not
//! mentioned by the author":
//!
//! - `Specified(T)` — the author wrote `color: red;`.
//! - `Inherit` — the author wrote `color: inherit;`, forcing the value
//!   from the parent even on properties that don't inherit by default.
//! - `Initial` — the author wrote `color: initial;` (or `unset` with no
//!   parent context), forcing the property back to its spec initial.
//!
//! The outer `Option<Value<T>>` on `TuiStyle` fields adds a fourth state:
//!
//! - `None` — the author didn't touch this property. Unlike `Inherit`
//!   (which is a spec-defined override), `None` means "use whatever the
//!   cascade would produce here naturally", which depends on whether the
//!   property is inherited (falls through to parent) or not (initial).
//!
//! Reference: [CSS Values and Units §6.3](https://www.w3.org/TR/css-values-4/#common-keywords).

/// CSS value states: specified, inherit, or initial.
///
/// `Copy` whenever `T: Copy`, so this adds zero heap overhead for the
/// common case where `T` is `Color`, `bool`, `u16`, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Value<T> {
    /// Literal author-specified value: `color: red;`.
    Specified(T),
    /// Force parent's value: `color: inherit;`.
    Inherit,
    /// Force spec initial: `color: initial;` / `color: unset;` when the
    /// property is not inherited.
    Initial,
}

impl<T> Value<T> {
    /// Return the literal value if this is `Specified`; `None` for
    /// `Inherit` / `Initial`. The cascade uses this to distinguish
    /// "author wrote a concrete value" from "author asked for a
    /// keyword-based resolution".
    pub fn as_specified(&self) -> Option<&T> {
        match self {
            Value::Specified(v) => Some(v),
            _ => None,
        }
    }

    pub fn is_specified(&self) -> bool {
        matches!(self, Value::Specified(_))
    }

    pub fn is_inherit(&self) -> bool {
        matches!(self, Value::Inherit)
    }

    pub fn is_initial(&self) -> bool {
        matches!(self, Value::Initial)
    }

    /// Apply a function to the contained value. Keyword variants pass
    /// through unchanged.
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Value<U> {
        match self {
            Value::Specified(v) => Value::Specified(f(v)),
            Value::Inherit => Value::Inherit,
            Value::Initial => Value::Initial,
        }
    }
}

impl<T> From<T> for Value<T> {
    /// Convenience: `Color::Rgb(255, 0, 0).into()` becomes `Value::Specified(Color::Rgb(255, 0, 0))`.
    fn from(v: T) -> Self {
        Value::Specified(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specified_round_trip() {
        let v: Value<u16> = Value::Specified(42);
        assert_eq!(v.as_specified(), Some(&42));
        assert!(v.is_specified());
        assert!(!v.is_inherit());
        assert!(!v.is_initial());
    }

    #[test]
    fn inherit_variant() {
        let v: Value<u16> = Value::Inherit;
        assert!(v.is_inherit());
        assert!(!v.is_specified());
        assert_eq!(v.as_specified(), None);
    }

    #[test]
    fn initial_variant() {
        let v: Value<u16> = Value::Initial;
        assert!(v.is_initial());
        assert!(!v.is_specified());
        assert_eq!(v.as_specified(), None);
    }

    #[test]
    fn map_preserves_keywords() {
        let v: Value<u16> = Value::Inherit;
        assert_eq!(v.map(|x| x * 2), Value::Inherit);

        let v: Value<u16> = Value::Initial;
        assert_eq!(v.map(|x| x * 2), Value::Initial);

        let v: Value<u16> = Value::Specified(10);
        assert_eq!(v.map(|x| x * 2), Value::Specified(20));
    }

    #[test]
    fn from_t_gives_specified() {
        let v: Value<u16> = 7.into();
        assert_eq!(v, Value::Specified(7));
    }

    #[test]
    fn copy_when_t_is_copy() {
        let a = Value::Specified(3u16);
        let b = a; // Copy
        assert_eq!(a, b);
    }
}
