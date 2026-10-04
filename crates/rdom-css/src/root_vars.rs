//! The `:root` mirror: a parsed sheet's `:root` custom properties,
//! published as its sheet-level variables (`Stylesheet::vars`, DESIGN
//! "`:root` custom properties are published twice on purpose").
//!
//! `:root` matches the tree's root node, and every element inherits the
//! root's custom properties through the sheet-level map, so the mirror
//! is computed in cascade order over the sheet's `:root` rules — rules
//! whose selector is exactly `:root` (a selector-list item included),
//! not scoped:
//!
//! - an `!important` declaration beats a normal one (CSS Cascade 4
//!   §6.4);
//! - normal: unlayered beats every layer, a later layer beats an earlier
//!   one; important: the reverse (CSS Cascade 5 §6.4);
//! - then order of appearance — an `@import`ed sheet's rules sit at the
//!   import's position.
//!
//! Specificity is the same for every such rule. Across the sheets of a
//! cascade, the later sheet's map wins per name (the cascade's
//! `merge_root_vars`).

use rdom_core::selectors::{PseudoClass, SimpleSelector};
use rdom_style::{LayerOrder, PseudoElementTarget, Rule, Stylesheet};

/// Publish `sheet`'s `:root` custom properties as its sheet-level
/// variables, in cascade order (module doc).
pub(crate) fn mirror_root_vars(sheet: &mut Stylesheet) {
    let order = LayerOrder::new(&[&*sheet]);
    let mut winners: Vec<(String, String, Precedence)> = Vec::new();
    for (index, rule) in sheet.rules().iter().enumerate() {
        if !is_root_rule(rule) {
            continue;
        }
        for d in &rule.style.custom_properties {
            let candidate = Precedence {
                important: d.important,
                rank: order.rank(0, rule.layer),
                index,
            };
            match winners.iter_mut().find(|(name, _, _)| *name == d.name) {
                Some(slot) if candidate.beats(&slot.2) => {
                    slot.1 = d.value.clone();
                    slot.2 = candidate;
                }
                Some(_) => {}
                None => winners.push((d.name.clone(), d.value.clone(), candidate)),
            }
        }
    }
    for (name, value, _) in winners {
        sheet.define_var_mut(&name, &value);
    }
}

/// Is `rule`'s selector exactly `:root`, unscoped?
fn is_root_rule(rule: &Rule) -> bool {
    let [complex] = rule.selector.0.as_slice() else {
        return false;
    };
    rule.scope.is_none()
        && rule.pseudo == PseudoElementTarget::None
        && complex.ancestors.is_empty()
        && matches!(
            complex.subject.simples.as_slice(),
            [SimpleSelector::Pseudo(PseudoClass::Root)]
        )
}

/// Where a `:root` declaration sits in the cascade (module doc).
struct Precedence {
    important: bool,
    /// Layer rank (`LayerOrder::UNLAYERED` for unlayered rules).
    rank: u32,
    /// Order of appearance (the rule's index in the sheet).
    index: usize,
}

impl Precedence {
    fn beats(&self, other: &Precedence) -> bool {
        if self.important != other.important {
            return self.important;
        }
        if self.rank != other.rank {
            return if self.important {
                self.rank < other.rank
            } else {
                self.rank > other.rank
            };
        }
        self.index > other.index
    }
}
