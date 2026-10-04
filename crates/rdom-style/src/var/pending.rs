//! The declarations a style keeps for substitution
//! ([`PendingDeclaration`]) and their replay per element.

use std::collections::HashMap;

use super::{CustomValue, SubstitutionContext, lookup_in, substitute_at};
use crate::TuiStyle;
use crate::parse::token::Token;

/// A declaration kept as tokens until the cascade substitutes `var()`
/// and `attr()`.
///
/// Once a block holds such a declaration, every later declaration of
/// the block is recorded here too (with `has_substitution: false`) so
/// the cascade replays them in source order: `padding: var(--p);
/// padding-left: 1` keeps the later longhand on top.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PendingDeclaration {
    /// The canonical property name.
    pub name: String,
    /// The value's tokens, `!important` stripped.
    pub value: Vec<Token>,
    /// The value contains an arbitrary substitution function: `var()` or
    /// `attr()`.
    pub has_substitution: bool,
    /// The value's `attr()` names and types, parsed once here rather
    /// than at every substitution.
    pub(crate) heads: crate::attr::AttrHeads,
}

impl PendingDeclaration {
    pub fn new(name: &str, value: &[Token], has_substitution: bool) -> Self {
        PendingDeclaration {
            name: name.to_string(),
            value: value.to_vec(),
            has_substitution,
            heads: if has_substitution {
                crate::attr::AttrHeads::of(value)
            } else {
                Default::default()
            },
        }
    }
}

impl TuiStyle {
    /// Does this style hold declarations waiting for substitution?
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// This style with its pending declarations substituted from `vars`
    /// (the element's custom properties) and `cx` (its attributes), and
    /// parsed, in source order. A declaration invalid at computed-value
    /// time sets its property to `unset` (§3.1).
    pub fn substituted(
        &self,
        vars: &HashMap<String, CustomValue>,
        cx: &SubstitutionContext<'_, '_>,
    ) -> TuiStyle {
        let mut out = self.clone();
        out.pending.clear();
        self.replay_pending(vars, cx, &mut out);
        out
    }

    /// Only the substituted pending declarations of this style, on an
    /// otherwise empty style that keeps this one's `!important` bits:
    /// applying this style and then the result is applying
    /// [`substituted`](Self::substituted) — the pending declarations
    /// come after the rest of the block in source order — without
    /// copying the block. What the cascade uses per element, with the
    /// element's attributes in `cx` (CSS Values 5 §8.7).
    pub fn substituted_pending(
        &self,
        vars: &HashMap<String, CustomValue>,
        cx: &SubstitutionContext<'_, '_>,
    ) -> TuiStyle {
        let mut out = TuiStyle {
            important: self.important,
            ..TuiStyle::default()
        };
        self.replay_pending(vars, cx, &mut out);
        out
    }

    fn replay_pending(
        &self,
        vars: &HashMap<String, CustomValue>,
        cx: &SubstitutionContext<'_, '_>,
        out: &mut TuiStyle,
    ) {
        for decl in &self.pending {
            let parsed = if decl.has_substitution {
                substitute_at(
                    &decl.value,
                    0,
                    Some(&decl.heads),
                    &mut |n| lookup_in(vars, n),
                    cx.attrs,
                )
                .is_ok_and(|t| crate::property_dispatch::set_parsed(&decl.name, &t, out).is_ok())
            } else {
                crate::property_dispatch::set_parsed(&decl.name, &decl.value, out).is_ok()
            };
            if !parsed {
                crate::property_dispatch::set_unset(&decl.name, out);
            }
        }
    }
}
