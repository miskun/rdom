//! The at-rules rdom-css evaluates, by name: the one table the top-level
//! dispatch reads (`top_level::consume_at_rule`), so [`at_rule_names`]
//! lists every at-rule a sheet can use — any other is consumed whole and
//! reported as `WarningKind::UnsupportedAtRule` (CSS Syntax 3 §5.4.2).

/// What a dispatched at-rule is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AtRule {
    Layer,
    Property,
    Keyframes,
    PositionTry,
    CounterStyle,
    Scope,
    /// `@media`, `@supports`, `@container` (CSS Conditional 3 §2).
    Conditional,
    StartingStyle,
}

/// The at-rules `consume_at_rule` dispatches, by ASCII-lowercase name.
/// `@import`, read before the rules (`import::leading_at_rule`), is not
/// among them.
const DISPATCHED: &[(&str, AtRule)] = &[
    ("layer", AtRule::Layer),
    ("property", AtRule::Property),
    ("keyframes", AtRule::Keyframes),
    ("position-try", AtRule::PositionTry),
    ("counter-style", AtRule::CounterStyle),
    ("scope", AtRule::Scope),
    ("media", AtRule::Conditional),
    ("supports", AtRule::Conditional),
    ("container", AtRule::Conditional),
    ("starting-style", AtRule::StartingStyle),
];

impl AtRule {
    /// The dispatched at-rule named `name` (ASCII case-insensitive).
    pub(crate) fn of(name: &str) -> Option<AtRule> {
        DISPATCHED
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, rule)| *rule)
    }
}

/// The name of every at-rule rdom-css evaluates — `@import` and the ones
/// the top-level dispatch reads from its table — without the `@`, in no
/// particular order. The acid page's coverage test asks for each.
pub fn at_rule_names() -> impl Iterator<Item = &'static str> {
    std::iter::once("import").chain(DISPATCHED.iter().map(|(name, _)| *name))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every listed at-rule parses without an `UnsupportedAtRule` warning,
    /// and one rdom-css does not evaluate warns.
    #[test]
    fn every_listed_at_rule_is_evaluated() {
        let instance = |name: &str| match name {
            "import" => "@import url(x.css);".to_string(),
            "layer" => "@layer a;".to_string(),
            "property" => "@property --x { syntax: '*'; inherits: false; }".to_string(),
            "keyframes" => "@keyframes k { from { color: red } }".to_string(),
            "position-try" => "@position-try --t { top: 0 }".to_string(),
            "counter-style" => "@counter-style c { system: cyclic; symbols: a; }".to_string(),
            "scope" => "@scope (.a) { p { color: red } }".to_string(),
            "starting-style" => "@starting-style { p { color: red } }".to_string(),
            conditional => format!("@{conditional} (width > 1) {{ p {{ color: red }} }}"),
        };
        for name in at_rule_names() {
            let parsed = crate::parse(&instance(name));
            assert!(
                !parsed
                    .warnings
                    .iter()
                    .any(|w| matches!(w.kind, crate::WarningKind::UnsupportedAtRule(_))),
                "@{name}: {:?}",
                parsed.warnings
            );
        }
        let parsed = crate::parse("@font-face { font-family: x }");
        assert!(
            parsed
                .warnings
                .iter()
                .any(|w| matches!(w.kind, crate::WarningKind::UnsupportedAtRule(_)))
        );
    }
}
