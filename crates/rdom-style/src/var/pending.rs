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
    /// The property is a flow-relative inline-axis one (CSS Logical 1):
    /// it maps by the element's `direction`, so it waits for the cascade
    /// even without a substitution function.
    pub directional: bool,
    /// The declaration is `!important` — recorded per declaration
    /// ([`property_dispatch::set_important`](crate::property_dispatch::set_important)),
    /// since an inline-axis one's physical side is not known until the
    /// cascade.
    pub important: bool,
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
            directional: crate::property_dispatch::is_directional(name),
            important: false,
        }
    }
}

impl TuiStyle {
    /// Does this style hold declarations waiting for substitution?
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Does a kept declaration hold a substitution function (`var()`,
    /// `attr()`), which only the element can resolve? Without one, the
    /// kept declarations depend on the element's `direction` alone, and
    /// a rule builds their form for each direction once
    /// ([`Rule::directional_overlay`](crate::Rule::directional_overlay)).
    pub fn needs_substitution(&self) -> bool {
        self.pending.iter().any(|d| d.has_substitution)
    }

    /// The kept declarations of a style that holds an inline-axis
    /// flow-relative property and no substitution function, replayed in
    /// source order for an `ltr` and an `rtl` element — what
    /// [`substituted_pending`](Self::substituted_pending) gives for each
    /// direction, built once (`Rule::directional_overlay`). `None` when
    /// nothing is kept or a substitution function is.
    pub(crate) fn directional_overlays(&self) -> Option<[TuiStyle; 2]> {
        if !self.has_pending() || self.needs_substitution() {
            return None;
        }
        let vars = HashMap::new();
        let overlay = |direction| {
            let cx = SubstitutionContext::new().with_direction(direction);
            self.substituted_pending(&vars, &cx)
        };
        Some([
            overlay(crate::layout::TextDirection::Ltr),
            overlay(crate::layout::TextDirection::Rtl),
        ])
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
        // `margin` / `padding` keep their four sides in one field, which a
        // replayed side longhand updates: it starts from the block's own
        // sides (`margin: 1; margin-left: var(--x)` keeps 1 elsewhere).
        let mut out = TuiStyle {
            important: self.important,
            margin: self.margin.clone(),
            padding: self.padding.clone(),
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
            let set = |tokens: &[Token], out: &mut TuiStyle| {
                crate::property_dispatch::set_parsed_in(&decl.name, tokens, out, cx.direction)
                    .is_ok()
            };
            let parsed = if decl.has_substitution {
                substitute_at(
                    &decl.value,
                    0,
                    Some(&decl.heads),
                    &mut |n| lookup_in(vars, n),
                    cx.attrs,
                )
                .is_ok_and(|t| set(&t, out))
            } else {
                set(&decl.value, out)
            };
            if !parsed {
                crate::property_dispatch::set_unset_in(&decl.name, out, cx.direction);
            }
        }
    }
}
