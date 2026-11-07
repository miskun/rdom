//! Changing one declaration of a rule in place — CSSOM's
//! `CSSStyleRule.style.setProperty()` / `removeProperty()` (CSSOM §6.6.1)
//! — for tools that try a change before it is written to the source
//! (CAPTURE phase 4, `specs/CAPTURE.md` §Edits). The rule keeps its
//! selector, specificity, source order, layer, scope, condition and
//! location; its block changes and, as when it was built, keeps only what
//! its pseudo-element takes and its direction-mapped forms are rebuilt.

use super::Stylesheet;
use super::style_selector::fit_to_pseudo;
use crate::property_dispatch::{self, DispatchError};

/// Why a rule's declaration could not be changed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EditError {
    /// The sheet has no rule at that index.
    NoSuchRule(usize),
    /// The property or its value does not parse (`property_dispatch::set`).
    Invalid(DispatchError),
    /// The rule's pseudo-element takes no such property (`::marker` only
    /// takes a few).
    NotApplicable,
}

impl std::fmt::Display for EditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EditError::NoSuchRule(i) => write!(f, "the sheet has no rule {i}"),
            EditError::Invalid(e) => write!(f, "{e:?}"),
            EditError::NotApplicable => {
                f.write_str("the rule's pseudo-element takes no such property")
            }
        }
    }
}

impl std::error::Error for EditError {}

impl Stylesheet {
    /// The declared value of `name` in rule `index` and whether it is
    /// `!important`; `None` when the rule does not declare it.
    pub fn rule_declaration(&self, index: usize, name: &str) -> Option<(String, bool)> {
        let rule = self.rules().get(index)?;
        match name.strip_prefix("--") {
            Some(custom) => rule
                .style
                .custom_properties
                .iter()
                .rev()
                .find(|c| c.name == custom)
                .map(|c| (c.value.as_str().to_string(), c.important)),
            None => property_dispatch::serialize(name, &rule.style)
                .map(|v| (v, property_dispatch::is_important(name, &rule.style))),
        }
    }

    /// Declare `name: value` (`!important` when `important`) in rule
    /// `index`, replacing any declaration of it there.
    pub fn set_rule_declaration(
        &mut self,
        index: usize,
        name: &str,
        value: &str,
        important: bool,
    ) -> Result<(), EditError> {
        self.edit_rule(index, |style| {
            property_dispatch::set(name, value, style).map_err(EditError::Invalid)?;
            property_dispatch::set_important(name, important, style);
            Ok(())
        })?;
        // A property the pseudo-element does not take was dropped by its
        // subset: say so rather than report a change that did nothing.
        match self.rule_declaration(index, name) {
            Some(_) => Ok(()),
            None => Err(EditError::NotApplicable),
        }
    }

    /// Remove `name` from rule `index`; `Ok(false)` when it did not
    /// declare it.
    pub fn remove_rule_declaration(&mut self, index: usize, name: &str) -> Result<bool, EditError> {
        let mut removed = false;
        self.edit_rule(index, |style| {
            removed = property_dispatch::remove(name, style);
            Ok(())
        })?;
        Ok(removed)
    }

    /// Change rule `index`'s block through `f`, then fit it to the rule's
    /// pseudo-element and rebuild its direction-mapped forms.
    fn edit_rule(
        &mut self,
        index: usize,
        f: impl FnOnce(&mut crate::TuiStyle) -> Result<(), EditError>,
    ) -> Result<(), EditError> {
        let rule = self.rules.get(index).ok_or(EditError::NoSuchRule(index))?;
        let mut style = rule.style.clone();
        f(&mut style)?;
        let pseudo = rule.pseudo.clone();
        let (style, directional) = fit_to_pseudo(&pseudo, &style).unwrap_or_else(|| {
            let directional = style.directional_overlays().map(std::sync::Arc::new);
            (style, directional)
        });
        let rule = &mut self.rules[index];
        rule.style = style;
        rule.directional = directional;
        // The rule index keys rules by selector, which did not change; the
        // version tells the cascade the sheet did.
        self.touch();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{PseudoElementTarget, Stylesheet, TuiStyle};

    fn sheet() -> Stylesheet {
        Stylesheet::bare()
            .rule_unchecked(".a", TuiStyle::new())
            .rule_unchecked("li::marker", TuiStyle::new())
    }

    #[test]
    fn a_declaration_is_set_read_back_and_removed() {
        let mut s = sheet();
        let before = s.version();
        s.set_rule_declaration(0, "color", "#ff0000", false)
            .unwrap();
        assert_ne!(s.version(), before, "the cascade sees the change");
        let (v, important) = s.rule_declaration(0, "color").unwrap();
        assert_eq!(v, "red", "as CSSOM serializes #ff0000");
        assert!(!important);
        s.set_rule_declaration(0, "color", "blue", true).unwrap();
        assert!(s.rule_declaration(0, "color").unwrap().1, "now !important");
        assert!(s.remove_rule_declaration(0, "color").unwrap());
        assert_eq!(s.rule_declaration(0, "color"), None);
        assert!(!s.remove_rule_declaration(0, "color").unwrap());
    }

    #[test]
    fn custom_properties_and_errors() {
        let mut s = sheet();
        s.set_rule_declaration(0, "--accent", "#123456", false)
            .unwrap();
        assert_eq!(
            s.rule_declaration(0, "--accent").unwrap().0.trim(),
            "#123456"
        );
        assert!(matches!(
            s.set_rule_declaration(9, "color", "red", false),
            Err(super::EditError::NoSuchRule(9))
        ));
        assert!(matches!(
            s.set_rule_declaration(0, "colour", "red", false),
            Err(super::EditError::Invalid(_))
        ));
        assert_eq!(s.rules()[1].pseudo, PseudoElementTarget::Marker);
        assert!(matches!(
            s.set_rule_declaration(1, "padding-top", "2", false),
            Err(super::EditError::NotApplicable)
        ));
        s.set_rule_declaration(1, "color", "red", false).unwrap();
    }

    #[test]
    fn an_edit_keeps_the_rules_place_and_rebuilds_its_mapped_forms() {
        let mut s = sheet();
        let (spec, idx) = (s.rules()[0].specificity, s.rules()[0].source_idx);
        s.set_rule_declaration(0, "margin-inline-start", "2", false)
            .unwrap();
        let r = &s.rules()[0];
        assert_eq!((r.specificity, r.source_idx), (spec, idx));
        assert!(
            r.directional_overlay(crate::layout::TextDirection::Ltr)
                .is_some()
        );
    }
}
