//! C12-KEYFRAMES — CSS Animations 1 §3 `@keyframes`: a named list of
//! keyframe blocks, each a `<keyframe-selector>#` (`from`, `to`,
//! percentages) and a declaration list. The sheet keeps every rule in
//! source order with its cascade layer; which rule a name resolves to
//! (the last, by layer order) and how keyframes with the same selector
//! cascade is the backend's (`rdom_style::keyframes`).

use rdom_css::{WarningKind, parse};

fn offsets(rule: &rdom_style::keyframes::KeyframesRule) -> Vec<Vec<f32>> {
    rule.keyframes
        .iter()
        .map(|k| k.selectors.iter().map(|s| s.offset()).collect())
        .collect()
}

/// §3: `from` is `0%`, `to` is `100%`; a selector list names several
/// offsets for one block; the blocks keep their declarations.
#[test]
fn keyframes_parse_their_selectors_and_blocks() {
    let r = parse(
        "@keyframes slide { from { color: red } 25%, 75% { color: blue } to { color: green } } \
         .a { animation-name: slide }",
    );
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let sheet = &r.stylesheet;
    assert_eq!(sheet.rules().len(), 1, "the style rule after it is kept");
    let rules = sheet.keyframes();
    assert_eq!(rules.len(), 1);
    assert_eq!(&*rules[0].name, "slide");
    assert_eq!(offsets(&rules[0]), [vec![0.0], vec![0.25, 0.75], vec![1.0]]);
    assert!(rules[0].keyframes.iter().all(|k| k.style.fg.is_some()));
}

/// §3: the name is a `<custom-ident>` or a `<string>`; `none` and the
/// CSS-wide keywords cannot name keyframes as identifiers (CSS Values 4
/// §4.2), but can as strings.
#[test]
fn keyframes_names_are_idents_or_strings() {
    let r =
        parse("@keyframes \"none\" { to { color: red } } @keyframes Fade { to { color: red } }");
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let names: Vec<&str> = r.stylesheet.keyframes().iter().map(|k| &*k.name).collect();
    assert_eq!(
        names,
        ["none", "Fade"],
        "case kept: names are case-sensitive"
    );
    for bad in ["none", "initial", "inherit", "default", "a b", ""] {
        let r = parse(&format!(
            "@keyframes {bad} {{ to {{ color: red }} }} .x {{ color: red }}"
        ));
        assert!(r.stylesheet.keyframes().is_empty(), "{bad:?}");
        assert!(
            matches!(&r.warnings[..], [w] if matches!(&w.kind, WarningKind::InvalidAtRulePrelude { name, .. } if name == "keyframes")),
            "{bad:?}: {:?}",
            r.warnings
        );
        assert_eq!(r.stylesheet.rules().len(), 1, "the next rule survives");
    }
}

/// §3: a keyframe whose selector list holds an invalid selector — a
/// percentage outside [0%, 100%], an unknown keyword — is ignored, and
/// the rest of the rule kept.
#[test]
fn an_invalid_keyframe_selector_drops_its_block() {
    let r = parse(
        "@keyframes a { 0% { color: red } 150% { color: blue } 50%, middle { color: green } to { color: white } }",
    );
    let rules = r.stylesheet.keyframes();
    assert_eq!(offsets(&rules[0]), [vec![0.0], vec![1.0]]);
    let dropped: Vec<&WarningKind> = r.warnings.iter().map(|w| &w.kind).collect();
    assert!(
        matches!(&dropped[..], [WarningKind::InvalidKeyframeSelector(a), WarningKind::InvalidKeyframeSelector(b)] if a == "150%" && b == "50%, middle"),
        "{dropped:?}"
    );
}

/// §3: a declaration marked `!important` in a keyframe is ignored —
/// dropped, with a warning — and the block's other declarations kept.
#[test]
fn important_declarations_in_keyframes_are_ignored() {
    let r = parse("@keyframes a { from { color: red !important; background-color: blue } }");
    let k = &r.stylesheet.keyframes()[0].keyframes[0];
    assert!(k.style.fg.is_none(), "the important one is ignored");
    assert!(k.style.bg.is_some());
    assert!(k.style.important.is_empty());
    assert!(
        matches!(&r.warnings[..], [w] if w.kind == WarningKind::ImportantInKeyframe("color".into())),
        "{:?}",
        r.warnings
    );
}

/// §3: two keyframes with the same selector both stay in the rule — they
/// cascade (the later one's declarations win), which the backend
/// resolves (`KeyframesRule::resolve`).
#[test]
fn keyframes_with_one_selector_cascade() {
    let r =
        parse("@keyframes a { 50% { color: red; background-color: blue } 50% { color: green } }");
    let rule = &r.stylesheet.keyframes()[0];
    assert_eq!(rule.keyframes.len(), 2);
    let resolved = rule.resolve();
    assert_eq!(resolved.len(), 1, "one offset");
    assert_eq!(resolved[0].offset, 0.5);
    assert_eq!(resolved[0].blocks.len(), 2, "both blocks, in order");
}

/// CSS Cascade 5 §6.4.3: a `@keyframes` rule sits in the cascade layer
/// of its place in the sheet.
#[test]
fn keyframes_record_their_layer() {
    let r = parse(
        "@layer base { @keyframes a { to { color: red } } } @keyframes a { to { color: blue } }",
    );
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let rules = r.stylesheet.keyframes();
    assert_eq!(rules.len(), 2);
    assert!(rules[0].layer.is_some());
    assert!(rules[1].layer.is_none());
}

/// CSS Animations 1 §3: `animation-timing-function` in a keyframe is the
/// easing to the next keyframe; `animation-composition` (CSS Animations
/// 2 §3.2) its composite operation. Both are kept on the block.
#[test]
fn keyframes_keep_their_easing_and_composition() {
    let r = parse(
        "@keyframes a { from { color: red; animation-timing-function: steps(2); animation-composition: add } }",
    );
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let k = &r.stylesheet.keyframes()[0].keyframes[0];
    assert!(k.easing().is_some());
    assert_eq!(
        k.composition(),
        Some(rdom_style::keyframes::AnimationComposition::Add)
    );
}

/// Scroll-driven Animations 1 §4.4: a keyframe selector may name a
/// timeline range and a percentage of it (`entry 20%`), any percentage —
/// the point it names may fall outside the animation's range.
#[test]
fn keyframe_selectors_may_name_a_timeline_range() {
    use rdom_style::keyframes::TimelineRangeName;
    let r = parse("@keyframes a { entry 0%, exit 120% { color: red } cover 50% { color: blue } }");
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let rule = &r.stylesheet.keyframes()[0];
    let sel: Vec<(Option<TimelineRangeName>, f32)> = rule
        .keyframes
        .iter()
        .flat_map(|k| k.selectors.iter().map(|s| (s.range(), s.offset())))
        .collect();
    assert_eq!(
        sel,
        [
            (Some(TimelineRangeName::Entry), 0.0),
            (Some(TimelineRangeName::Exit), 1.2),
            (Some(TimelineRangeName::Cover), 0.5)
        ]
    );
    let bad = parse("@keyframes a { sideways 10% { color: red } entry { color: red } }");
    assert_eq!(bad.warnings.len(), 2, "{:?}", bad.warnings);
}
